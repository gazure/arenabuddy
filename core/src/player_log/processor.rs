use std::{collections::VecDeque, path::Path};

use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};
use tracing::{debug, error};

use crate::{
    Result,
    errors::ParseError,
    events::{
        business::RequestTypeBusinessEvent, client::RequestTypeClientToMatchServiceMessage,
        draft::RequestTypeDraftNotify, gre::RequestTypeGREToClientEvent, mgrsc::RequestTypeMGRSCEvent,
    },
};

#[derive(Debug)]
pub struct PlayerLogProcessor {
    player_log_reader: BufReader<File>,
    json_events: VecDeque<String>,
    current_json_str: Option<String>,
    bracket_depth: usize,
}

impl PlayerLogProcessor {
    /// # Errors
    ///
    /// Will return an error if the player log file cannot be opened
    pub async fn try_new(player_log_path: &Path) -> Result<Self> {
        Ok(Self {
            player_log_reader: BufReader::new(File::open(player_log_path).await?),
            json_events: VecDeque::new(),
            current_json_str: None,
            bracket_depth: 0,
        })
    }

    // try to find the json strings in the logs. ignoring all other info
    // purges whitespace from the internal json strings, but I don't think that will cause
    // any issues given the log entries seen
    pub fn process_line(&mut self, log_line: &str) -> Vec<String> {
        let mut completed_json_strings = Vec::new();
        log_line.chars().for_each(|char| match char {
            '{' => {
                if self.current_json_str.is_none() {
                    self.current_json_str = Some(String::new());
                }
                if let Some(json_str) = &mut self.current_json_str {
                    json_str.push('{');
                }
                self.bracket_depth += 1;
            }
            '}' => {
                if let Some(json_str) = &mut self.current_json_str {
                    json_str.push('}');
                    self.bracket_depth -= 1;
                    if self.bracket_depth == 0 {
                        completed_json_strings.push(json_str.clone());
                        self.current_json_str = None;
                    }
                }
            }
            ' ' | '\n' | '\r' => {}
            _ => {
                if let Some(json_str) = &mut self.current_json_str {
                    json_str.push(char);
                }
            }
        });
        completed_json_strings
    }

    async fn process_lines(&mut self) {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            match self.player_log_reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => lines.push(line),
                Err(e) => {
                    error!("Error reading line: {:?}", e);
                    break;
                }
            }
        }
        for line in lines {
            let json_strings = self.process_line(&line);
            self.json_events.extend(json_strings);
        }
    }

    /// # Errors
    ///
    /// Errors when json events that look parseable do not parse, or when no events are found
    pub async fn get_next_event(&mut self) -> Result<ParseOutput> {
        self.process_lines().await;
        let event = self.json_events.pop_front().ok_or(ParseError::NoEvent)?;
        parse(&event).map_err(|e| {
            error!("Error parsing event: {}", e);
            debug!("Event: {}", event);
            ParseError::Error(event).into()
        })
    }
}

#[derive(Debug)]
pub enum ParseOutput {
    GREMessage(RequestTypeGREToClientEvent),
    ClientMessage(RequestTypeClientToMatchServiceMessage),
    MGRSCMessage(RequestTypeMGRSCEvent),
    BusinessMessage(RequestTypeBusinessEvent),
    DraftNotify(RequestTypeDraftNotify),
    NoEvent,
}

/// # Errors
///
/// Errors if event appears to be a relevant json string, but does not decode properly
pub fn parse(event: &str) -> Result<ParseOutput> {
    if event.contains("clientToMatchServiceMessage") {
        let client_to_match_service_message: RequestTypeClientToMatchServiceMessage = serde_json::from_str(event)?;
        Ok(ParseOutput::ClientMessage(client_to_match_service_message))
    } else if event.contains("matchGameRoomStateChangedEvent") {
        let mgrsc_event: RequestTypeMGRSCEvent = serde_json::from_str(event)?;
        Ok(ParseOutput::MGRSCMessage(mgrsc_event))
    } else if event.contains("greToClientEvent") {
        let request_gre_to_client_event: RequestTypeGREToClientEvent = serde_json::from_str(event)?;
        Ok(ParseOutput::GREMessage(request_gre_to_client_event))
    } else if let Ok(business_event) = serde_json::from_str::<RequestTypeBusinessEvent>(event) {
        Ok(ParseOutput::BusinessMessage(business_event))
    } else if let Ok(draft_event) = serde_json::from_str::<RequestTypeDraftNotify>(event) {
        Ok(ParseOutput::DraftNotify(draft_event))
    } else {
        Ok(ParseOutput::NoEvent)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use tokio::{fs::File, io::BufReader};

    use super::*;

    async fn make_processor() -> PlayerLogProcessor {
        let tmp = std::env::temp_dir().join("arenabuddy_test_empty.log");
        tokio::fs::write(&tmp, b"").await.expect("write temp file");
        PlayerLogProcessor {
            player_log_reader: BufReader::new(File::open(&tmp).await.expect("open temp file")),
            json_events: VecDeque::new(),
            current_json_str: None,
            bracket_depth: 0,
        }
    }

    // -- process_line tests ---------------------------------------------------

    #[tokio::test]
    async fn process_line_extracts_single_json_object() {
        let mut proc = make_processor().await;
        let result = proc.process_line(r#"some log prefix {"key":"value"} trailing"#);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], r#"{"key":"value"}"#);
    }

    #[tokio::test]
    async fn process_line_extracts_multiple_json_objects() {
        let mut proc = make_processor().await;
        let result = proc.process_line(r#"{"a":1} noise {"b":2}"#);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], r#"{"a":1}"#);
        assert_eq!(result[1], r#"{"b":2}"#);
    }

    #[tokio::test]
    async fn process_line_handles_nested_braces() {
        let mut proc = make_processor().await;
        let result = proc.process_line(r#"{"outer":{"inner":1}}"#);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], r#"{"outer":{"inner":1}}"#);
    }

    #[tokio::test]
    async fn process_line_strips_whitespace() {
        let mut proc = make_processor().await;
        let result = proc.process_line(r#"{ "key" : "value" }"#);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], r#"{"key":"value"}"#);
    }

    #[tokio::test]
    async fn process_line_spans_multiple_calls() {
        let mut proc = make_processor().await;
        let r1 = proc.process_line(r#"prefix {"key":"#);
        assert!(r1.is_empty(), "incomplete JSON should not produce output");

        let r2 = proc.process_line(r#""value"}"#);
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0], r#"{"key":"value"}"#);
    }

    #[tokio::test]
    async fn process_line_no_json() {
        let mut proc = make_processor().await;
        let result = proc.process_line("just some plain log text with no braces");
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn process_line_deeply_nested() {
        let mut proc = make_processor().await;
        let input = r#"{"a":{"b":{"c":{"d":1}}}}"#;
        let result = proc.process_line(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], input);
    }

    #[tokio::test]
    async fn process_line_empty_object() {
        let mut proc = make_processor().await;
        let result = proc.process_line("{}");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "{}");
    }

    // -- parse() dispatch tests -----------------------------------------------

    #[test]
    fn parse_unrecognized_json_returns_no_event() {
        let result = parse(r#"{"someRandomField": 42}"#).expect("should not error");
        assert!(matches!(result, ParseOutput::NoEvent));
    }

    #[test]
    fn parse_non_json_returns_no_event() {
        let result = parse("not json at all");
        assert!(matches!(result, Ok(ParseOutput::NoEvent)));
    }
}
