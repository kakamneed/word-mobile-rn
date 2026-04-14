use std::path::PathBuf;

/// Coarse platform abstraction for runtime services.
///
/// Mobile and desktop implementations provide platform-specific behavior
/// while the shared core remains platform-agnostic.
///
/// # Implementing PlatformRuntime
///
/// Desktop (Tauri): Delegates to Tauri's AppHandle APIs
/// Mobile (React Native): Delegates to native module bindings
/// Test: Mock implementations can be provided for unit tests
pub trait PlatformRuntime: Send + Sync {
    /// Returns the path to the application data directory.
    ///
    /// This is where databases, caches, and user-generated content live.
    /// On desktop: app-specific data directory
    /// On mobile: app sandbox documents directory
    fn app_data_dir(&self) -> Result<PathBuf, PlatformError>;

    /// Returns the path to the application config directory.
    ///
    /// This is where user preferences and settings live.
    fn app_config_dir(&self) -> Result<PathBuf, PlatformError>;

    /// Returns the path to the application log directory.
    fn app_log_dir(&self) -> Result<PathBuf, PlatformError>;

    /// Returns the path to a bundled resource file.
    ///
    /// Bundled resources are packaged with the app (e.g., seed vocabulary).
    fn bundled_resource_path(&self, relative_path: &str) -> Result<PathBuf, PlatformError>;

    /// Returns the current time.
    ///
    /// Abstracted for testability - mock implementations can control time.
    fn now(&self) -> chrono::DateTime<chrono::Utc>;

    /// Opens an external URL or path using platform mechanisms.
    ///
    /// On desktop: opens in default browser or file explorer
    /// On mobile: opens in browser or appropriate app
    fn open_external(&self, url: &str) -> Result<(), PlatformError>;
}

/// Errors that can occur from platform operations.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Failed to resolve path: {0}")]
    PathResolution(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Platform not supported: {0}")]
    NotSupported(String),
}
