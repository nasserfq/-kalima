//! The AI agent inside كلمة (وكيل): talks to Claude, Gemini, OpenRouter and any OpenAI-compatible
//! service (Groq, Ollama on this computer, free models), streams the reply, and runs the tools the
//! model calls through [`run::Tools`], which the app supplies. Settings and keys live in the app's
//! settings folder (keys in the macOS Keychain). No interface code, so it is tested headlessly.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod error;
pub mod providers;
pub mod run;
pub mod secrets;
pub mod settings;
pub mod sse;
pub mod types;

pub use error::{Error, Result};

/// The key a tool input carries when the model sent JSON that doesn't parse (or isn't an object),
/// so the tool can say so instead of acting on a guess.
pub const INVALID_INPUT: &str = "__invalid_input";

/// The most text a tool result sends back to the model.
pub const MAX_RESULT: usize = 100_000;
