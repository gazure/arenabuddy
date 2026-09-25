/// Limits memory retained by an incomplete or corrupt log entry.
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, PartialEq)]
pub(super) enum Frame {
    Object(Vec<u8>),
    Rejected(String),
}

/// Frames objects in a byte stream without modifying their contents.
#[derive(Debug, Default)]
pub(super) struct JsonFramer {
    bytes: Vec<u8>,
    depth: usize,
    in_string: bool,
    escaped: bool,
    discarding: bool,
}

impl JsonFramer {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Frame> {
        let mut frames = Vec::new();
        for &byte in chunk {
            if self.depth == 0 {
                if byte == b'{' {
                    self.depth = 1;
                    self.bytes.push(byte);
                }
                continue;
            }
            if !self.discarding {
                self.bytes.push(byte);
            }
            if self.in_string {
                if self.escaped {
                    self.escaped = false;
                } else if byte == b'\\' {
                    self.escaped = true;
                } else if byte == b'"' {
                    self.in_string = false;
                }
            } else {
                match byte {
                    b'"' => self.in_string = true,
                    b'{' => self.depth = self.depth.saturating_add(1),
                    b'}' => self.depth -= 1,
                    _ => {}
                }
            }
            if self.bytes.len() > MAX_FRAME_BYTES {
                // Retain lexical state until the oversized object ends, so
                // nested objects cannot be mistaken for new top-level events.
                self.bytes.clear();
                self.discarding = true;
                frames.push(Frame::Rejected(format!("JSON object exceeds {MAX_FRAME_BYTES} bytes")));
            }
            if self.depth == 0 {
                if !self.discarding {
                    frames.push(Frame::Object(std::mem::take(&mut self.bytes)));
                }
                *self = Self::default();
            }
        }
        frames
    }

    pub fn finish(&mut self) -> Option<Frame> {
        let incomplete = self.depth != 0 && !self.discarding;
        *self = Self::default();
        incomplete.then(|| Frame::Rejected("Incomplete JSON object at the end of a log file".into()))
    }
}

#[cfg(test)]
#[path = "framing_tests.rs"]
mod tests;
