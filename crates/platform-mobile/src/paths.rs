//! Mobile path resolution for sandbox-safe storage.

use std::path::PathBuf;

/// Mobile-specific path resolution.
///
/// Provides sandbox-safe paths for database, config, logs, and bundled resources
/// on both iOS and Android.
pub struct MobilePaths {
    app_data_dir: PathBuf,
    app_config_dir: PathBuf,
    app_cache_dir: PathBuf,
    bundle_resource_dir: PathBuf,
}

impl MobilePaths {
    /// Create a new MobilePaths instance.
    ///
    /// On iOS: Uses NSHomeDirectory-based paths
    /// On Android: Uses Context.getFilesDir() equivalent
    pub fn new(
        app_data_dir: PathBuf,
        app_config_dir: PathBuf,
        app_cache_dir: PathBuf,
        bundle_resource_dir: PathBuf,
    ) -> Self {
        Self {
            app_data_dir,
            app_config_dir,
            app_cache_dir,
            bundle_resource_dir,
        }
    }

    /// Get the database file path.
    pub fn database_path(&self) -> PathBuf {
        self.app_data_dir.join("word.db")
    }

    /// Get the app data directory.
    pub fn app_data_dir(&self) -> &PathBuf {
        &self.app_data_dir
    }

    /// Get the app config directory.
    pub fn app_config_dir(&self) -> &PathBuf {
        &self.app_config_dir
    }

    /// Get the app cache directory.
    pub fn app_cache_dir(&self) -> &PathBuf {
        &self.app_cache_dir
    }

    /// Get the logs directory.
    pub fn logs_dir(&self) -> PathBuf {
        self.app_cache_dir.join("logs")
    }

    /// Get the vocabulary cache directory.
    pub fn vocab_cache_dir(&self) -> PathBuf {
        self.app_cache_dir.join("vocab")
    }

    /// Get a bundled resource path.
    pub fn bundled_resource_path(&self, relative_path: &str) -> PathBuf {
        self.bundle_resource_dir.join(relative_path)
    }

    /// Get the bundled snapshot path.
    pub fn bundled_snapshot_path(&self) -> PathBuf {
        self.bundle_resource_dir
            .join("vocab-snapshot/vocab-snapshot.jsonl")
    }
}

/// Get default mobile paths from environment.
///
/// This function should be called from the native side with the correct
/// platform-specific directories.
pub fn default_mobile_paths() -> Option<MobilePaths> {
    // On actual devices, these would come from:
    // - iOS: NSHomeDirectory, NSBundle.mainBundle.resourcePath
    // - Android: Context.getFilesDir(), Context.getCacheDir(), AssetManager
    //
    // For now, this returns None to indicate paths must be provided by the native layer.
    None
}
