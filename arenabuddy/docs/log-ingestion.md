# Log ingestion

The desktop app and CLI use the same ingestion service in `arenabuddy_core`.

## Following a running game

The reader preserves its byte offset while MTGA appends to `Player.log`. Filesystem
notifications wake the reader; they do not trigger a restart. Changes to sibling
files are ignored. Polling remains active when notifications are disabled or the
watcher cannot start, including when the log directory does not exist yet.

Following waits for a missing log to appear. When MTGA replaces the file, the
reader drains the old file to its current end before opening the replacement at
byte zero. File identity, length, and content checkpoints detect replacement,
truncation, and common cases of truncation followed by rapid regrowth. A confirmed
boundary clears incomplete JSON and unfinished match and draft state. Configured
writers remain attached, and completed records remain in storage.

Content checkpoints sample the first 256 bytes and the last 256 bytes read. A
rewrite that preserves both samples and regrows past the read offset between
checks cannot always be detected. Bytes appended to the old file after the reader
switches to its replacement are not consumed.

## Parsing and memory

The reader processes chunks of at most 64 KiB. JSON framing preserves whitespace,
Unicode, escaped quotes, and braces inside strings, even across reads. Event
routing uses top-level JSON fields. Complete malformed frames generate parse
errors without preventing subsequent frames from being processed.

Each JSON object is limited to 16 MiB. An oversized object generates one error;
the parser discards it through its closing brace. An unclosed object cannot be
reliably separated from later entries: the parser waits for its closing brace or
a file boundary. A boundary discards an incomplete object and reports an error.

## One-shot parsing and shutdown

One-shot parsing stops at the file length recorded when it opens the log. It
reports an incomplete final JSON object and exits. A missing file is an error in
this mode. This is a length limit, not a filesystem snapshot: avoid rewriting the
file while parsing it.

Following checks shutdown between chunks and events, including while catching up
on a large log. Shutdown waits for the current callback or writer to finish. The
service owns and releases its filesystem watcher. Hosts can supply a shutdown
channel; CLI callers can opt into Tokio's shared Ctrl+C handling.

## Upgrading

This change requires no database migration, server update, or wire protocol
change. Existing clients and data on the server remain compatible. Upgrade the
desktop app or CLI to use the corrected reader.

Read offsets remain in memory. Restarting ingestion reads the current log from
its beginning; it does not resume from a persisted cursor. The durable upload
queue handles upload retries independently. Previously stored records are not
reparsed or repaired by this upgrade.
