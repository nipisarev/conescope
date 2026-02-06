/// Events emitted by a PTY reader thread. No UI types — clean boundary.
#[derive(Debug, Clone)]
pub enum PtyEvent {
    Output(Vec<u8>),
    Exit(Option<i32>),
    Error(String),
}
