//! Unit Tests Module
//!
//! This module contains unit tests organized by component.
//! Unit tests focus on testing individual functions and modules in isolation.

#[cfg(feature = "native")]
pub mod app;
#[cfg(feature = "native")]
pub mod audio;
pub mod core;
pub mod instruments;
pub mod utils;
