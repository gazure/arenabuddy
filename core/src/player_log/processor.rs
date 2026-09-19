use std::{collections::VecDeque, path::Path};

use super::{
    follower::{LogFollower, ReadResult},
    framing::{Frame, JsonFramer},
};
use crate::{
    Result,
    errors::ParseError,
    events::{
        business::RequestTypeBusinessEvent, client::RequestTypeClientToMatchServiceMessage,
        draft::RequestTypeDraftNotify, gre::RequestTypeGREToClientEvent, mgrsc::RequestTypeMGRSCEvent,
    },
};

// Keep events inline: each item is consumed immediately, never stored in a collection.
#[expect(clippy::large_enum_variant)]
pub(super) enum LogItem {
    Event(ParseOutput),
    Boundary,
    Progress,
    Eof,
}

/// Reads framed events from a player log without changing JSON string contents.
#[derive(Debug)]
pub struct PlayerLogProcessor {
    follower: LogFollower,
    framer: JsonFramer,
    frames: VecDeque<Frame>,
    boundary_pending: bool,
}

impl PlayerLogProcessor {
    /// Opens a snapshot of the file for one-shot processing.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be opened as a regular file.
    pub async fn try_new(path: &Path) -> Result<Self> {
        Self::with_follow(path, false).await
    }

    pub(super) async fn with_follow(path: &Path, follow: bool) -> Result<Self> {
        Ok(Self {
            follower: LogFollower::new(path, follow).await?,
            framer: JsonFramer::default(),
            frames: VecDeque::new(),
            boundary_pending: false,
        })
    }

    fn decode(frame: Frame) -> Result<LogItem> {
        match frame {
            Frame::Rejected(reason) => Err(ParseError::Error(reason).into()),
            Frame::Object(bytes) => {
                let text =
                    String::from_utf8(bytes).map_err(|_| ParseError::Error("Invalid UTF-8 in JSON object".into()))?;
                parse(&text)
                    .map(LogItem::Event)
                    .map_err(|_| ParseError::Error(text).into())
            }
        }
    }

    // One bounded read or queued frame per call lets the service check shutdown
    // even while scanning a large file or a continuously growing stream.
    pub(super) async fn next_item(&mut self) -> Result<LogItem> {
        if let Some(frame) = self.frames.pop_front() {
            return Self::decode(frame);
        }
        if self.boundary_pending {
            self.boundary_pending = false;
            return Ok(LogItem::Boundary);
        }
        match self.follower.read().await? {
            ReadResult::Bytes(bytes) => {
                self.frames.extend(self.framer.push(&bytes));
                self.frames.pop_front().map_or(Ok(LogItem::Progress), Self::decode)
            }
            ReadResult::Boundary => {
                if let Some(incomplete) = self.framer.finish() {
                    self.boundary_pending = true;
                    Self::decode(incomplete)
                } else {
                    Ok(LogItem::Boundary)
                }
            }
            ReadResult::Eof => Ok(LogItem::Eof),
        }
    }

    pub(super) fn finish(&mut self) -> Result<()> {
        match self.framer.finish() {
            Some(frame) => Self::decode(frame).map(|_| ()),
            None => Ok(()),
        }
    }

    /// Returns the next parsed object in the file snapshot.
    ///
    /// # Errors
    ///
    /// Returns a parse error for malformed events or an incomplete final object,
    /// `NoEvent` at EOF, or an I/O error if the file cannot be read.
    pub async fn get_next_event(&mut self) -> Result<ParseOutput> {
        loop {
            match self.next_item().await? {
                LogItem::Event(event) => return Ok(event),
                LogItem::Eof => {
                    self.finish()?;
                    return Err(ParseError::NoEvent.into());
                }
                LogItem::Progress | LogItem::Boundary => tokio::task::yield_now().await,
            }
        }
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

/// Decodes an event based on its top-level envelope fields.
///
/// # Errors
///
/// Returns an error for malformed JSON or a malformed recognized event.
/// Unknown valid JSON objects return `NoEvent`.
pub fn parse(event: &str) -> Result<ParseOutput> {
    let value: serde_json::Value = serde_json::from_str(event)?;
    if value.get("clientToMatchServiceMessage").is_some() {
        Ok(ParseOutput::ClientMessage(serde_json::from_value(value)?))
    } else if value.get("matchGameRoomStateChangedEvent").is_some() {
        Ok(ParseOutput::MGRSCMessage(serde_json::from_value(value)?))
    } else if value.get("greToClientEvent").is_some() {
        Ok(ParseOutput::GREMessage(serde_json::from_value(value)?))
    } else if let Ok(business) = serde_json::from_value::<RequestTypeBusinessEvent>(value.clone()) {
        Ok(ParseOutput::BusinessMessage(business))
    } else if let Ok(draft) = serde_json::from_value::<RequestTypeDraftNotify>(value) {
        Ok(ParseOutput::DraftNotify(draft))
    } else {
        Ok(ParseOutput::NoEvent)
    }
}

#[cfg(test)]
#[path = "processor_tests.rs"]
mod tests;
