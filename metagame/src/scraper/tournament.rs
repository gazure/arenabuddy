use std::collections::HashSet;

use anyhow::Result;
use arenabuddy_data::{
    MetagameRepository,
    metagame_models::{MetagameDeck, MetagameTournament},
};
use chrono::NaiveDate;
use scraper::{Html, Selector};
use tracing::info;

use super::{Fetcher, deck};

/// Scrape a single tournament by its `MTGGoldfish` ID, importing all decklists.
///
/// # Panics
///
/// Panics if hardcoded CSS selectors are invalid (should never happen).
pub async fn scrape_single_tournament(
    repo: &impl MetagameRepository,
    fetcher: &Fetcher,
    goldfish_id: i32,
    format: &str,
) -> Result<()> {
    let base_url = fetcher.base_url();
    let url = format!("{base_url}/tournament/{goldfish_id}");
    info!("Scraping tournament {goldfish_id}: {url}");

    let html = fetcher.fetch(&format!("/tournament/{goldfish_id}")).await?;
    let document = Html::parse_document(&html);

    let title_sel = Selector::parse("h2, h1, .title").expect("valid selector");
    let name = document.select(&title_sel).next().map_or_else(
        || format!("Tournament {goldfish_id}"),
        |el| el.text().collect::<String>().trim().to_string(),
    );

    let tournament = MetagameTournament {
        goldfish_id,
        name: name.clone(),
        format: format.to_string(),
        date: chrono::Utc::now().date_naive(),
        url: url.clone(),
    };
    let tournament_db_id = repo.upsert_metagame_tournament(&tournament).await?;
    info!("Tournament: {name} - id={tournament_db_id} url={url}");

    let decks = parse_tournament_decks(&document, base_url, format);
    info!("  Found {} decks", decks.len());

    for (deck_info, archetype_name) in &decks {
        let archetype_id = if let Some(name) = archetype_name {
            Some(repo.upsert_metagame_archetype(name, format, None).await?)
        } else {
            None
        };

        match deck::fetch_deck_cards(fetcher, deck_info.goldfish_id).await {
            Ok(cards) => {
                let deck_db_id = repo
                    .upsert_metagame_deck(deck_info, Some(tournament_db_id), archetype_id, &cards)
                    .await?;
                info!(
                    "    Deck {} ({}): {} cards - db_id={}",
                    deck_info.goldfish_id,
                    deck_info.player_name.as_deref().unwrap_or("unknown"),
                    cards.len(),
                    deck_db_id,
                );
            }
            Err(e) => {
                tracing::warn!("    Failed to fetch deck {}: {e:#}", deck_info.goldfish_id);
            }
        }
    }

    Ok(())
}

/// Scrape tournaments matching a format and date range, including all decklists.
pub async fn scrape_tournaments(
    repo: &impl MetagameRepository,
    fetcher: &Fetcher,
    format: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<()> {
    let base_url = fetcher.base_url();
    let date_range = format!("{} - {}", from.format("%m/%d/%Y"), to.format("%m/%d/%Y"));
    info!("Searching {format} tournaments from {date_range}");

    let mut page = 1u32;
    loop {
        info!("Fetching search results page {page}");
        let tournaments = search_tournaments(fetcher, base_url, format, &date_range, page).await?;

        let Some(tournaments) = tournaments else {
            info!("Pagination exhausted at page {page}");
            break;
        };

        if tournaments.is_empty() {
            info!("No more tournaments found on page {page}, stopping");
            break;
        }

        info!("Found {} tournaments on page {page}", tournaments.len());

        for tournament in &tournaments {
            let tournament_db_id = repo.upsert_metagame_tournament(tournament).await?;
            info!(
                "Tournament: {} ({}) - id={} url={}",
                tournament.name, tournament.date, tournament_db_id, tournament.url
            );

            let decks = scrape_tournament_decks(fetcher, base_url, tournament.goldfish_id, format).await?;
            info!("  Found {} decks", decks.len());

            for (deck_info, archetype_name) in &decks {
                let archetype_id = if let Some(name) = archetype_name {
                    Some(repo.upsert_metagame_archetype(name, format, None).await?)
                } else {
                    None
                };

                match deck::fetch_deck_cards(fetcher, deck_info.goldfish_id).await {
                    Ok(cards) => {
                        let deck_db_id = repo
                            .upsert_metagame_deck(deck_info, Some(tournament_db_id), archetype_id, &cards)
                            .await?;
                        info!(
                            "    Deck {} ({}): {} cards - db_id={}",
                            deck_info.goldfish_id,
                            deck_info.player_name.as_deref().unwrap_or("unknown"),
                            cards.len(),
                            deck_db_id,
                        );
                    }
                    Err(e) => {
                        tracing::warn!("    Failed to fetch deck {}: {e:#}", deck_info.goldfish_id);
                    }
                }
            }
        }

        page += 1;
    }

    Ok(())
}

/// Search for tournaments using the tournament search endpoint.
/// Returns `None` if the page returns 400 (pagination past the end).
async fn search_tournaments(
    fetcher: &Fetcher,
    base_url: &str,
    format: &str,
    date_range: &str,
    page: u32,
) -> Result<Option<Vec<MetagameTournament>>> {
    let encoded_range = urlencoding::encode(date_range);
    let path = format!(
        "/tournament_searches/create?tournament_search%5Bname%5D=&tournament_search%5Bformat%5D={format}&tournament_search%5Bdate_range%5D={encoded_range}&commit=Search&page={page}"
    );
    let Some(html) = fetcher.fetch_optional(&path).await? else {
        return Ok(None);
    };
    let document = Html::parse_document(&html);
    Ok(Some(parse_tournament_search_results(&document, base_url, format)))
}

/// Parse tournament search results table.
fn parse_tournament_search_results(document: &Html, base_url: &str, format: &str) -> Vec<MetagameTournament> {
    let mut tournaments = Vec::new();

    let row_sel = Selector::parse("table tr").expect("valid selector");
    let td_sel = Selector::parse("td").expect("valid selector");
    let link_sel = Selector::parse("a[href*='/tournament/']").expect("valid selector");

    for row in document.select(&row_sel) {
        let tds: Vec<_> = row.select(&td_sel).collect();
        if tds.len() < 3 {
            continue;
        }

        let Some(link) = row.select(&link_sel).next() else {
            continue;
        };

        let Some(href) = link.value().attr("href") else {
            continue;
        };

        let Some(goldfish_id) = extract_tournament_id(href) else {
            continue;
        };

        let name = link.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            continue;
        }

        let date_text = tds[0].text().collect::<String>().trim().to_string();
        let date =
            NaiveDate::parse_from_str(&date_text, "%Y-%m-%d").unwrap_or_else(|_| chrono::Utc::now().date_naive());

        tournaments.push(MetagameTournament {
            goldfish_id,
            name,
            format: format.to_string(),
            date,
            url: format!("{base_url}{href}"),
        });
    }

    tournaments
}

/// Parse a single tournament page to extract deck entries.
/// Returns `(MetagameDeck, Option<archetype_name>)` pairs.
async fn scrape_tournament_decks(
    fetcher: &Fetcher,
    base_url: &str,
    tournament_goldfish_id: i32,
    format: &str,
) -> Result<Vec<(MetagameDeck, Option<String>)>> {
    let html = fetcher.fetch(&format!("/tournament/{tournament_goldfish_id}")).await?;
    tracing::debug!(
        "Tournament {tournament_goldfish_id} page: {} bytes, contains '/deck/': {}",
        html.len(),
        html.contains("/deck/")
    );
    let document = Html::parse_document(&html);
    Ok(parse_tournament_decks(&document, base_url, format))
}

fn parse_tournament_decks(document: &Html, base_url: &str, format: &str) -> Vec<(MetagameDeck, Option<String>)> {
    let mut decks = Vec::new();
    let mut seen_ids = HashSet::new();

    let deck_link_sel = Selector::parse("a[href*='/deck/']").expect("valid selector");

    for deck_link in document.select(&deck_link_sel) {
        let Some(href) = deck_link.value().attr("href") else {
            continue;
        };

        let Some(goldfish_deck_id) = extract_deck_id(href) else {
            continue;
        };

        if !seen_ids.insert(goldfish_deck_id) {
            continue;
        }

        let archetype_name = deck_link.text().collect::<String>().trim().to_string();
        let archetype = if archetype_name.is_empty() {
            None
        } else {
            Some(archetype_name)
        };

        let player_name = find_sibling_link(&deck_link, "/player/");

        decks.push((
            MetagameDeck {
                goldfish_id: goldfish_deck_id,
                archetype_name: archetype.clone(),
                player_name,
                placement: None,
                format: format.to_string(),
                date: None,
                url: format!("{base_url}{href}"),
            },
            archetype,
        ));
    }

    decks
}

/// Walk up from an element to find a sibling link matching a pattern.
fn find_sibling_link(element: &scraper::ElementRef<'_>, href_contains: &str) -> Option<String> {
    for ancestor in element.ancestors() {
        if let Some(node) = ancestor.value().as_element()
            && node.name() == "tr"
        {
            let link_sel = Selector::parse(&format!("a[href*='{href_contains}']")).expect("valid selector");
            let ancestor_ref = scraper::ElementRef::wrap(ancestor)?;
            return ancestor_ref
                .select(&link_sel)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string());
        }
    }
    None
}

fn extract_tournament_id(href: &str) -> Option<i32> {
    href.strip_prefix("/tournament/")
        .and_then(|s| s.split(['?', '#']).next())
        .and_then(|s| s.parse().ok())
}

fn extract_deck_id(href: &str) -> Option<i32> {
    let suffix = href.strip_prefix("/deck/")?;
    if suffix.starts_with("download") {
        return None;
    }
    suffix.split(['?', '#', '/']).next().and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tournament_id() {
        assert_eq!(extract_tournament_id("/tournament/62266"), Some(62266));
        assert_eq!(extract_tournament_id("/tournament/62266?page=1"), Some(62266));
        assert_eq!(extract_tournament_id("/other/path"), None);
    }

    #[test]
    fn test_extract_deck_id() {
        assert_eq!(extract_deck_id("/deck/7677856"), Some(7_677_856));
        assert_eq!(extract_deck_id("/deck/7677856#online"), Some(7_677_856));
        assert_eq!(extract_deck_id("/deck/download/7677856"), None);
        assert_eq!(extract_deck_id("/other/path"), None);
    }
}
