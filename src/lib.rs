//! ssbu-desync-detector: records what ended an online Smash Ultimate 13.0.5
//! match and keeps the per-frame evidence a vanilla replay needs to show which
//! console desynced. Hooks observe simulation state without changing it or
//! sending data. The communication-error sentence, the Canceled notice, and
//! the reconnect warning can receive an evidence-based reason in the native
//! UI buffer.
pub mod dialog;
pub mod disconnect;
pub mod evidence;
pub mod lastsession;
pub mod matchlog;
pub mod model;
#[cfg(target_os = "switch")]
mod runtime;
