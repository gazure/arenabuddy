use arenabuddy_core::{
    cards::CardsDatabase,
    display::{
        deck::{DeckDisplayRecord, Difference},
        game::GameResultDisplay,
        match_details::MatchDetails,
        mulligan::Mulligan,
    },
};
use arenabuddy_data::{ArenabuddyRepository, MatchDB};
use google_sheets4::{
    Sheets,
    api::ValueRange,
    hyper_rustls,
    hyper_util::{
        client::legacy::{Client, connect::HttpConnector},
        rt::TokioExecutor,
    },
};
use serde_json::json;
use tracingx::{error, info};
use uuid::Uuid;

fn match_details_sheet_row(md: &MatchDetails) -> Vec<serde_json::Value> {
    let mut row = vec![
        json!(md.id),
        json!(md.created_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        json!(md.did_controller_win),
        json!(md.controller_player_name),
        json!(md.opponent_player_name),
    ];

    let wins = md
        .game_results
        .iter()
        .filter(|g| g.winning_player == md.controller_player_name)
        .count();
    let losses = md.game_results.len() - wins;
    row.push(json!(format!("{}-{}", wins, losses)));

    let player_decklist = md
        .primary_decklist
        .as_ref()
        .map_or_else(|| "Player Deck Not Found".to_string(), DeckDisplayRecord::pretty_print);
    row.push(json!(player_decklist));

    let opponent_deck = md
        .opponent_deck
        .as_ref()
        .map_or_else(|| "Unknown".to_string(), DeckDisplayRecord::pretty_print);
    row.push(json!(opponent_deck));

    let game_two_sideboard = md
        .differences
        .as_ref()
        .and_then(|ds| ds.first().map(Difference::pretty_print))
        .unwrap_or("No Game 2 Sideboarding".to_string());
    row.push(json!(game_two_sideboard));

    let game_three_sideboard = md
        .differences
        .as_ref()
        .and_then(|ds| ds.get(1).map(Difference::pretty_print))
        .unwrap_or("No Game 3 Sideboarding".to_string());
    row.push(json!(game_three_sideboard));

    for game_num in 1..=3 {
        let mulligans = md
            .mulligans
            .iter()
            .filter(|m| m.game_number == game_num)
            .map(Mulligan::pretty_print)
            .collect::<Vec<_>>()
            .join("\n\n");
        row.push(json!(mulligans));
    }

    for game in &md.game_results {
        row.push(json!(format!(
            "Game {}: {}",
            game.game_number,
            if game.winning_player == md.controller_player_name {
                "W"
            } else {
                "L"
            }
        )));
    }

    row
}

fn sheet_headers() -> Vec<&'static str> {
    vec![
        "Match ID",
        "Date/Time",
        "Won Match",
        "Player Name",
        "Opponent Name",
        "Game Score",
        "Player Deck",
        "Opponent Deck",
        "Game 2 Sideboarding",
        "Game 3 Sideboarding",
        "Game 1 Mulligans",
        "Game 2 Mulligans",
        "Game 3 Mulligans",
        "Game 1 Result",
        "Game 2 Result",
        "Game 3 Result",
    ]
}

struct SheetsClient {
    sheets: Sheets<hyper_rustls::HttpsConnector<HttpConnector>>,
    spreadsheet_id: String,
}

impl SheetsClient {
    async fn new(spreadsheet_id: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let key_path = std::env::var("GOOGLE_SERVICE_ACCOUNT_KEY")
            .unwrap_or_else(|_| "./keys/arenabuddy-7632406ac057.json".to_string());
        let key = yup_oauth2::read_service_account_key(key_path).await?;
        let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(key).build().await?;
        let client = Client::builder(TokioExecutor::new()).build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()?
                .https_only()
                .enable_http1()
                .enable_http2()
                .build(),
        );

        let sheets = Sheets::new(client, authenticator);
        Ok(SheetsClient { sheets, spreadsheet_id })
    }

    async fn append_match(
        &self,
        match_details: &MatchDetails,
        sheet_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let range = format!("{sheet_name}!A:Z");
        let values = vec![match_details_sheet_row(match_details)];

        let value_range = ValueRange {
            range: Some(range.clone()),
            major_dimension: Some("ROWS".to_string()),
            values: Some(values),
        };

        self.sheets
            .spreadsheets()
            .values_append(value_range, &self.spreadsheet_id, &range)
            .value_input_option("USER_ENTERED")
            .insert_data_option("INSERT_ROWS")
            .doit()
            .await?;

        info!("Successfully appended match {} to sheet", match_details.id);
        Ok(())
    }

    async fn initialize_sheet_headers(&self, sheet_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let range = format!("{sheet_name}!A1:Z1");

        let result = self
            .sheets
            .spreadsheets()
            .values_get(&self.spreadsheet_id, &range)
            .doit()
            .await;

        if let Ok((_, value_range)) = result
            && let Some(ref values) = value_range.values
            && !values.is_empty()
        {
            info!("Headers already exist in sheet {}", sheet_name);
            return Ok(());
        }

        let headers: Vec<serde_json::Value> = sheet_headers().iter().map(|h| json!(h)).collect();

        let value_range = ValueRange {
            range: Some(range.clone()),
            major_dimension: Some("ROWS".to_string()),
            values: Some(vec![headers]),
        };

        self.sheets
            .spreadsheets()
            .values_update(value_range, &self.spreadsheet_id, &range)
            .value_input_option("USER_ENTERED")
            .doit()
            .await?;

        info!("Initialized headers for sheet {}", sheet_name);
        Ok(())
    }
}

async fn build_match_details(
    db: &MatchDB,
    cards: &CardsDatabase,
    match_id: &str,
    user_id: Option<Uuid>,
) -> Result<MatchDetails, Box<dyn std::error::Error + Send + Sync>> {
    let (mtga_match, result) = db.get_match(match_id, user_id).await?;

    let mut details = MatchDetails {
        id: match_id.to_string(),
        controller_seat_id: mtga_match.controller_seat_id(),
        controller_player_name: mtga_match.controller_player_name().to_string(),
        opponent_player_name: mtga_match.opponent_player_name().to_string(),
        created_at: mtga_match.created_at(),
        did_controller_win: result.is_some_and(|r| r.is_winner(mtga_match.controller_seat_id())),
        ..Default::default()
    };

    details.decklists = db.list_decklists(match_id).await.unwrap_or_default();

    details.primary_decklist = details
        .decklists
        .first()
        .map(|d| DeckDisplayRecord::from_decklist(d, cards));

    details.decklists.windows(2).for_each(|pair| {
        if let [prev, next] = pair {
            let diff = Difference::diff(prev, next, cards);
            details.differences.get_or_insert_with(Vec::new).push(diff);
        }
    });

    let raw_mulligans = db.list_mulligans(match_id).await.unwrap_or_default();
    details.mulligans = raw_mulligans.iter().map(|m| Mulligan::from_model(m, cards)).collect();
    details.mulligans.sort();

    details.game_results = db
        .list_match_results(match_id)
        .await
        .unwrap_or_default()
        .iter()
        .map(|mr| {
            GameResultDisplay::from_match_result(
                mr,
                details.controller_seat_id,
                &details.controller_player_name,
                &details.opponent_player_name,
            )
        })
        .collect();

    details.opponent_deck = db
        .get_opponent_deck(match_id)
        .await
        .map(|deck| DeckDisplayRecord::from_decklist(&deck, cards))
        .ok();

    Ok(details)
}

/// Spawn an async task to sync a match to Google Sheets.
/// This runs in the background and does not block the gRPC response.
pub(crate) fn spawn_sheets_sync(
    db: MatchDB,
    cards: CardsDatabase,
    match_id: String,
    user_id: Option<Uuid>,
    spreadsheet_id: String,
) {
    tokio::spawn(async move {
        if let Err(e) = sync_match(db, &cards, &match_id, user_id, &spreadsheet_id).await {
            error!("Sheets sync failed for match {match_id}: {e}");
        }
    });
}

async fn sync_match(
    db: MatchDB,
    cards: &CardsDatabase,
    match_id: &str,
    user_id: Option<Uuid>,
    spreadsheet_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let details = build_match_details(&db, cards, match_id, user_id).await?;
    let client = SheetsClient::new(spreadsheet_id.to_string()).await?;
    client.initialize_sheet_headers("Matches").await?;
    client.append_match(&details, "Matches").await?;
    Ok(())
}
