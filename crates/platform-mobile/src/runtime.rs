//! Mobile runtime implementation.

use std::path::PathBuf;

use word_app_core::platform::{PlatformError, PlatformRuntime};

use crate::paths::MobilePaths;

/// Mobile platform runtime implementation.
pub struct MobileRuntime {
    paths: MobilePaths,
}

impl MobileRuntime {
    /// Create a new mobile runtime with the given paths.
    pub fn new(paths: MobilePaths) -> Self {
        Self { paths }
    }

    /// Get a reference to the paths.
    pub fn paths(&self) -> &MobilePaths {
        &self.paths
    }
}

impl PlatformRuntime for MobileRuntime {
    fn app_data_dir(&self) -> Result<PathBuf, PlatformError> {
        Ok(self.paths.app_data_dir().clone())
    }

    fn app_config_dir(&self) -> Result<PathBuf, PlatformError> {
        Ok(self.paths.app_config_dir().clone())
    }

    fn app_log_dir(&self) -> Result<PathBuf, PlatformError> {
        Ok(self.paths.logs_dir())
    }

    fn bundled_resource_path(&self, relative_path: &str) -> Result<PathBuf, PlatformError> {
        Ok(self.paths.bundled_resource_path(relative_path))
    }

    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }

    fn open_external(&self, url: &str) -> Result<(), PlatformError> {
        // On mobile, this would use platform-specific APIs:
        // - iOS: UIApplication.shared.open()
        // - Android: Intent.ACTION_VIEW
        //
        // For now, log the attempt
        eprintln!("Open external: {}", url);
        Ok(())
    }
}
