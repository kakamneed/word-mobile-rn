//! Mobile platform adapter for Word.
//!
//! This crate provides the platform-specific implementation for iOS and Android,
//! including:
//! - PlatformRuntime implementation for mobile
//! - Native bridge exports for React Native
//! - Mobile-specific path resolution
//! - SQLite initialization in mobile sandbox

pub mod bridge;
pub mod paths;
pub mod runtime;

pub use bridge::*;
pub use paths::MobilePaths;
pub use runtime::MobileRuntime;
