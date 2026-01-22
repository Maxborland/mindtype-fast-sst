//! MindType Licensing System
//!
//! Handles license validation, device activation, and credits management.

mod cache;
mod credits;
mod error;
mod manager;

pub use cache::LicenseCache;
pub use credits::CreditsManager;
pub use error::LicenseError;
pub use manager::{LicenseManager, LicenseStatus, Plan};
