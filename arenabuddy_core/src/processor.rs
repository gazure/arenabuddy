use std::{
    collections::VecDeque,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    result::Result as StdResult,
};

use anyhow::Result;
use tracing::{debug, error};

use crate::mtga_events::{
    business::RequestTypeBusinessEvent, client::RequestTypeClientToMatchServiceMessage,
    gre::RequestTypeGREToClientEvent, mgrsc::RequestTypeMGRSCEvent,
};

pub trait EventSource {
    /// # Errors
    ///
    /// Errors when json events that look parseable do not parse, or when no events are found
    fn get_next_event(&mut self) -> StdResult<ParseOutput, ParseError>;
}

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
    pub fn try_new(player_log_path: PathBuf) -> Result<Self> {
        let reader = BufReader::new(File::open(player_log_path)?);
        Ok(Self {
            player_log_reader: reader,
            json_events: VecDeque::new(),
            current_json_str: None,
            bracket_depth: 0,
        })
    }

    // try to find the json strings in the logs. ignoring all other info
    // purges whitespace from the internal json strings, but I don't think that will cause
    // any issues given the log entries I've read
    pub fn process_line(&mut self, log_line: &str) -> Vec<String> {
        let mut completed_json_strings = Vec::new();
        for char in log_line.chars() {
            match char {
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
            }
        }
        completed_json_strings
    }

    fn process_lines(&mut self) {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            match self.player_log_reader.read_line(&mut line) {
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
}

impl EventSource for PlayerLogProcessor {
    fn get_next_event(&mut self) -> StdResult<ParseOutput, ParseError> {
        self.process_lines();
        let event = self.json_events.pop_front().ok_or(ParseError::NoEvent)?;
        parse(&event).map_err(|e| {
            error!("Error parsing event: {}", e);
            debug!("Event: {}", event);
            ParseError::Error(event)
        })
    }
}

#[derive(Debug)]
pub enum ParseOutput {
    GREMessage(RequestTypeGREToClientEvent),
    ClientMessage(RequestTypeClientToMatchServiceMessage),
    MGRSCMessage(RequestTypeMGRSCEvent),
    BusinessMessage(RequestTypeBusinessEvent),
    NoEvent,
}

pub enum ParseError {
    NoEvent,
    Error(String),
}

/// # Errors
///
/// Errors if event appears to be a relevant json string, but does not decode properly
pub fn parse(event: &str) -> Result<ParseOutput> {
    if event.contains("clientToMatchServiceMessage") {
        let client_to_match_service_message: RequestTypeClientToMatchServiceMessage =
            serde_json::from_str(event)?;
        Ok(ParseOutput::ClientMessage(client_to_match_service_message))
    } else if event.contains("matchGameRoomStateChangedEvent") {
        let mgrsc_event: RequestTypeMGRSCEvent = serde_json::from_str(event)?;
        Ok(ParseOutput::MGRSCMessage(mgrsc_event))
    } else if event.contains("greToClientEvent") {
        let request_gre_to_client_event: RequestTypeGREToClientEvent = serde_json::from_str(event)?;
        Ok(ParseOutput::GREMessage(request_gre_to_client_event))
    } else if let Ok(business_event) = serde_json::from_str::<RequestTypeBusinessEvent>(event) {
        Ok(ParseOutput::BusinessMessage(business_event))
    } else {
        Ok(ParseOutput::NoEvent)
    }
}
