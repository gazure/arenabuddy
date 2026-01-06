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
use yup_oauth2;

use crate::{Result, display::match_details::MatchDetails};

fn match_details_sheet_row(md: &MatchDetails) -> Vec<serde_json::Value> {
    let mut row = vec![
        json!(md.id),
        json!(md.created_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        json!(md.did_controller_win),
        json!(md.controller_player_name),
        json!(md.opponent_player_name),
    ];

    // Game results summary (W-L format)
    let wins = md
        .game_results
        .iter()
        .filter(|g| g.winning_player == md.controller_player_name)
        .count();
    let losses = md.game_results.len() - wins;
    row.push(json!(format!("{}-{}", wins, losses)));

    // Primary deck archetype
    let player_decklist = md.primary_decklist.as_ref().map_or_else(
        || "Player Deck Not Found".to_string(),
        super::display::deck::DeckDisplayRecord::pretty_print,
    );
    row.push(json!(player_decklist));

    // Opponent deck archetype
    let opponent_deck = md.opponent_deck.as_ref().map_or_else(
        || "Unknown".to_string(),
        super::display::deck::DeckDisplayRecord::pretty_print,
    );
    row.push(json!(opponent_deck));

    // Sideboarding
    let game_two_sideboard = md
        .differences
        .as_ref()
        .and_then(|ds| ds.first().map(super::display::deck::Difference::pretty_print))
        .unwrap_or("No Game 2 Sideboarding".to_string());
    row.push(json!(game_two_sideboard));

    let game_three_sideboard = md
        .differences
        .as_ref()
        .and_then(|ds| ds.get(1).map(super::display::deck::Difference::pretty_print))
        .unwrap_or("No Game 3 Sideboarding".to_string());
    row.push(json!(game_three_sideboard));

    // Mulligan information
    let game_one_mulligans = md
        .mulligans
        .iter()
        .filter(|m| m.game_number == 1)
        .map(super::display::mulligan::Mulligan::pretty_print)
        .collect::<Vec<_>>()
        .join("\n\n");
    row.push(json!(game_one_mulligans));

    let game_two_mulligans = md
        .mulligans
        .iter()
        .filter(|m| m.game_number == 2)
        .map(super::display::mulligan::Mulligan::pretty_print)
        .collect::<Vec<_>>()
        .join("\n\n");
    row.push(json!(game_two_mulligans));

    let game_three_mulligans = md
        .mulligans
        .iter()
        .filter(|m| m.game_number == 3)
        .map(super::display::mulligan::Mulligan::pretty_print)
        .collect::<Vec<_>>()
        .join("\n\n");
    row.push(json!(game_three_mulligans));

    // Game details (individual game results)
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

pub fn sheet_headers() -> Vec<&'static str> {
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

pub struct SheetsClient {
    sheets: Sheets<hyper_rustls::HttpsConnector<HttpConnector>>,
    spreadsheet_id: String,
}

impl SheetsClient {
    /// # Errors
    ///
    /// If there is an issue with TLS config
    pub async fn new(spreadsheet_id: String) -> Result<Self> {
        let key = yup_oauth2::read_service_account_key("./keys/arenabuddy-7632406ac057.json").await?;
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

    /// Append a `MatchDetails` record to the spreadsheet
    /// # Errors
    ///
    /// If there is an issue with the google sheets API
    pub async fn append_match(&self, match_details: &MatchDetails, sheet_name: &str) -> Result<()> {
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

        tracingx::info!("Successfully appended match {} to sheet", match_details.id);
        Ok(())
    }

    /// Batch write multiple `MatchDetails` records
    /// # Errors
    ///
    /// If there is a google sheets API error
    pub async fn batch_append_matches(&self, matches: &[MatchDetails], sheet_name: &str) -> Result<()> {
        if matches.is_empty() {
            return Ok(());
        }

        let range = format!("{sheet_name}!A:Z");
        let values: Vec<Vec<serde_json::Value>> = matches.iter().map(match_details_sheet_row).collect();

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

        tracingx::info!("Successfully appended {} matches to sheet", matches.len());
        Ok(())
    }

    /// Initialize sheet with headers if needed
    ///
    /// # Errors
    /// if there is a Sheets API issue
    pub async fn initialize_sheet_headers(&self, sheet_name: &str) -> Result<()> {
        let range = format!("{sheet_name}!A1:Z1");

        // Check if headers already exist
        let result = self
            .sheets
            .spreadsheets()
            .values_get(&self.spreadsheet_id, &range)
            .doit()
            .await;

        if let Ok((_, value_range)) = result
            && let Some(ref values) = value_range.values
            && values.is_empty()
        {
            tracingx::info!("Headers already exist in sheet {}", sheet_name);
            return Ok(());
        }

        // Write headers
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

        tracingx::info!("Initialized headers for sheet {}", sheet_name);
        Ok(())
    }
}

/// Writes `match_details` to the specified spreadsheet idea into the "Matches" sheet
///
/// # Errors
/// If there was an error interacting with the Google Sheets API.
async fn write_match_to_sheets(match_details: &MatchDetails, spreadsheet_id: &str) -> Result<()> {
    let client = SheetsClient::new(spreadsheet_id.to_string()).await?;

    // Initialize headers if needed
    client.initialize_sheet_headers("Matches").await?;

    // Append the match
    client.append_match(match_details, "Matches").await?;

    Ok(())
}

/// Writes `match_details` to my spreadsheet
///
/// # Errors
/// If there was an error interacting with the Google Sheets API.
pub async fn write_to_arenadata(match_details: &MatchDetails) -> Result<()> {
    write_match_to_sheets(match_details, "1lU9YcqenIR5T5zUAHObwOAR6BfP3DQXX0E-ooS8RNQs").await
}
