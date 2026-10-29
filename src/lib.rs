//! ssbu-desync-detector: records what ended an online Smash Ultimate 13.0.5
//! match and keeps the per-frame evidence a vanilla replay needs to show which
//! console desynced. Read-only: hooks observe registers and memory the game is
//! already reading; nothing is sent and no game state changes.
pub mod model;
#[cfg(target_os = "switch")]
mod runtime;
