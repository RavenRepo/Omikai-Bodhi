//! # Theasus Session
//!
//! SQLite-based session persistence for the Theasus AI terminal.
//!
//! Provides:
//! - Session storage and retrieval
//! - Conversation history management
//! - Auto-save functionality
//! - Session listing and cleanup
//!
//! ## Example
//!
//! ```rust,ignore
//! use theasus_session::{SessionStore, Session};
//!
//! let store = SessionStore::open_default()?;
//! let session = store.create_session("my-session", "gpt-4o")?;
//! store.add_message(&session.id, role, content)?;
//! ```

mod error;
mod schema;
mod session;
mod store;

pub use error::{Result, SessionError};
pub use session::{Session, SessionMessage, SessionSummary};
pub use store::SessionStore;

#[cfg(test)]
mod tests;
