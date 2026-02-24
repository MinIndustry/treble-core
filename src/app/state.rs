use super::AppMode;

/// Runtime state of the application, shared between App and the command thread.
pub struct AppState {
    pub mode: AppMode,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::Setup,
        }
    }
}
