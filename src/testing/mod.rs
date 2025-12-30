//! Testing utilities for the rustic audio system
//!
//! This module provides a simple GUI for testing basic audio functionality
//! without requiring a full frontend. It's feature-gated behind the "testing"
//! feature flag.

mod gui;

pub use gui::run_testing_gui;
