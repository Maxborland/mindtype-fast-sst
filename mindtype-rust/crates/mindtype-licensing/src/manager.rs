//! License manager implementation

use crate::cache::LicenseCache;
use crate::error::LicenseError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// License plan type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    /// Personal plan (1 device)
    Personal,
    /// Pro plan (3 devices)
    Pro,
    /// Team plan (10 devices)
    Team,
}

impl Plan {
    /// Maximum number of device activations allowed
    pub fn max_devices(&self) -> u32 {
        match self {
            Plan::Personal => 1,
            Plan::Pro => 3,
            Plan::Team => 10,
        }
    }
}

impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Plan::Personal => write!(f, "Personal"),
            Plan::Pro => write!(f, "Pro"),
            Plan::Team => write!(f, "Team"),
        }
    }
}

/// Current license status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LicenseStatus {
    /// Valid license with optional expiration
    Valid {
        plan: Plan,
        expires_at: Option<DateTime<Utc>>,
    },
    /// Trial mode active
    Trial {
        days_left: u32,
        minutes_left: f64,
    },
    /// Trial has expired
    TrialExpired,
    /// License key is invalid
    Invalid,
    /// License has expired
    Expired,
    /// Device limit reached for this license
    DeviceLimitReached,
    /// No license configured
    NotConfigured,
}

impl LicenseStatus {
    /// Check if the license allows using the app
    pub fn is_usable(&self) -> bool {
        matches!(
            self,
            LicenseStatus::Valid { .. } | LicenseStatus::Trial { .. }
        )
    }

    /// Check if this is a valid paid license
    pub fn is_valid(&self) -> bool {
        matches!(self, LicenseStatus::Valid { .. })
    }

    /// Check if this is trial mode
    pub fn is_trial(&self) -> bool {
        matches!(self, LicenseStatus::Trial { .. })
    }
}

/// API response for license validation
#[derive(Debug, Deserialize)]
struct ValidateResponse {
    valid: bool,
    #[serde(default)]
    plan: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_code: Option<String>,
}

/// License manager handles validation and caching
pub struct LicenseManager {
    api_base: String,
    device_id: String,
    license_key: Option<String>,
    cache: LicenseCache,
    client: reqwest::Client,
}

impl LicenseManager {
    /// Create a new license manager
    pub fn new(api_base: &str, device_id: &str, cache_dir: &std::path::Path) -> Self {
        Self {
            api_base: api_base.to_string(),
            device_id: device_id.to_string(),
            license_key: None,
            cache: LicenseCache::new(cache_dir),
            client: reqwest::Client::new(),
        }
    }

    /// Set the license key
    pub fn set_license_key(&mut self, key: &str) {
        self.license_key = Some(key.to_string());
    }

    /// Get the current license key
    pub fn license_key(&self) -> Option<&str> {
        self.license_key.as_deref()
    }

    /// Validate the license with the API
    pub async fn validate(&mut self) -> Result<LicenseStatus, LicenseError> {
        let key = match &self.license_key {
            Some(k) => k.clone(),
            None => {
                // Check for cached trial status
                if let Some(status) = self.cache.get_status()? {
                    return Ok(status);
                }
                return Ok(LicenseStatus::NotConfigured);
            }
        };

        info!("Validating license key");

        // Try online validation first
        match self.validate_online(&key).await {
            Ok(status) => {
                // Cache the result
                self.cache.set_status(&status)?;
                Ok(status)
            }
            Err(LicenseError::NetworkError(_) | LicenseError::HttpError(_)) => {
                // Fall back to cache on network error
                warn!("Network error, checking cache");
                if let Some(status) = self.cache.get_status()? {
                    if self.cache.is_valid_offline()? {
                        return Ok(status);
                    }
                }
                Err(LicenseError::NetworkError(
                    "Cannot validate license offline".to_string(),
                ))
            }
            Err(e) => Err(e),
        }
    }

    /// Validate license online
    async fn validate_online(&self, key: &str) -> Result<LicenseStatus, LicenseError> {
        let url = format!("{}/api/license/validate", self.api_base);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "license_key": key,
                "device_id": self.device_id,
                "device_name": hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "Unknown".to_string()),
                "app_version": env!("CARGO_PKG_VERSION"),
            }))
            .send()
            .await?;

        let status_code = response.status();
        let body: ValidateResponse = response.json().await?;

        debug!("Validation response: {:?}", body);

        if !body.valid {
            return match body.error_code.as_deref() {
                Some("EXPIRED") => Ok(LicenseStatus::Expired),
                Some("DEVICE_LIMIT") => Ok(LicenseStatus::DeviceLimitReached),
                Some("NOT_FOUND") => Ok(LicenseStatus::Invalid),
                _ => Ok(LicenseStatus::Invalid),
            };
        }

        let plan = match body.plan.as_deref() {
            Some("personal") => Plan::Personal,
            Some("pro") => Plan::Pro,
            Some("team") => Plan::Team,
            _ => Plan::Personal,
        };

        let expires_at = body
            .expires_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(LicenseStatus::Valid { plan, expires_at })
    }

    /// Deactivate this device from the license
    pub async fn deactivate(&self) -> Result<(), LicenseError> {
        let key = self
            .license_key
            .as_ref()
            .ok_or(LicenseError::KeyNotFound)?;

        let url = format!("{}/api/license/deactivate", self.api_base);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "license_key": key,
                "device_id": self.device_id,
            }))
            .send()
            .await?;

        if response.status().is_success() {
            info!("Device deactivated successfully");
            self.cache.clear()?;
            Ok(())
        } else {
            let body = response.text().await?;
            Err(LicenseError::ApiError(body))
        }
    }

    /// Start a trial period
    pub fn start_trial(&mut self) -> Result<LicenseStatus, LicenseError> {
        let status = LicenseStatus::Trial {
            days_left: 7,
            minutes_left: 30.0,
        };
        self.cache.set_status(&status)?;
        self.cache.set_trial_start(Utc::now())?;
        Ok(status)
    }

    /// Check and update trial status
    pub fn check_trial(&mut self) -> Result<LicenseStatus, LicenseError> {
        let trial_start = self.cache.get_trial_start()?;

        if let Some(start) = trial_start {
            let days_elapsed = (Utc::now() - start).num_days() as u32;
            let minutes_used = self.cache.get_trial_minutes_used()?;

            if days_elapsed >= 7 {
                return Ok(LicenseStatus::TrialExpired);
            }

            let minutes_left = (30.0 - minutes_used).max(0.0);
            if minutes_left <= 0.0 {
                return Ok(LicenseStatus::TrialExpired);
            }

            Ok(LicenseStatus::Trial {
                days_left: 7 - days_elapsed,
                minutes_left,
            })
        } else {
            Ok(LicenseStatus::NotConfigured)
        }
    }

    /// Record trial usage
    pub fn record_trial_usage(&mut self, minutes: f64) -> Result<(), LicenseError> {
        let current = self.cache.get_trial_minutes_used()?;
        self.cache.set_trial_minutes_used(current + minutes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_plan_max_devices() {
        assert_eq!(Plan::Personal.max_devices(), 1);
        assert_eq!(Plan::Pro.max_devices(), 3);
        assert_eq!(Plan::Team.max_devices(), 10);
    }

    #[test]
    fn test_plan_display() {
        assert_eq!(Plan::Personal.to_string(), "Personal");
        assert_eq!(Plan::Pro.to_string(), "Pro");
        assert_eq!(Plan::Team.to_string(), "Team");
    }

    #[test]
    fn test_license_status_is_usable() {
        let valid = LicenseStatus::Valid {
            plan: Plan::Pro,
            expires_at: None,
        };
        assert!(valid.is_usable());
        assert!(valid.is_valid());
        assert!(!valid.is_trial());

        let trial = LicenseStatus::Trial {
            days_left: 5,
            minutes_left: 20.0,
        };
        assert!(trial.is_usable());
        assert!(!trial.is_valid());
        assert!(trial.is_trial());

        let expired = LicenseStatus::TrialExpired;
        assert!(!expired.is_usable());

        let invalid = LicenseStatus::Invalid;
        assert!(!invalid.is_usable());

        let not_configured = LicenseStatus::NotConfigured;
        assert!(!not_configured.is_usable());
    }

    #[test]
    fn test_manager_create() {
        let dir = tempdir().unwrap();
        let manager = LicenseManager::new(
            "https://api.example.com",
            "test-device-123",
            dir.path(),
        );

        assert!(manager.license_key().is_none());
    }

    #[test]
    fn test_manager_set_license_key() {
        let dir = tempdir().unwrap();
        let mut manager = LicenseManager::new(
            "https://api.example.com",
            "test-device-123",
            dir.path(),
        );

        manager.set_license_key("MTAB-1234-5678-90AB");
        assert_eq!(manager.license_key(), Some("MTAB-1234-5678-90AB"));
    }

    #[test]
    fn test_manager_start_trial() {
        let dir = tempdir().unwrap();
        let mut manager = LicenseManager::new(
            "https://api.example.com",
            "test-device-123",
            dir.path(),
        );

        let status = manager.start_trial().unwrap();
        assert!(matches!(status, LicenseStatus::Trial { days_left: 7, minutes_left: 30.0 }));
    }

    #[test]
    fn test_manager_check_trial() {
        let dir = tempdir().unwrap();
        let mut manager = LicenseManager::new(
            "https://api.example.com",
            "test-device-123",
            dir.path(),
        );

        // Start trial
        manager.start_trial().unwrap();

        // Check trial status
        let status = manager.check_trial().unwrap();
        assert!(matches!(status, LicenseStatus::Trial { .. }));

        if let LicenseStatus::Trial { days_left, minutes_left } = status {
            assert_eq!(days_left, 7);
            assert!((minutes_left - 30.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_manager_record_trial_usage() {
        let dir = tempdir().unwrap();
        let mut manager = LicenseManager::new(
            "https://api.example.com",
            "test-device-123",
            dir.path(),
        );

        manager.start_trial().unwrap();

        // Record some usage
        manager.record_trial_usage(5.0).unwrap();
        manager.record_trial_usage(3.0).unwrap();

        // Check updated trial
        let status = manager.check_trial().unwrap();
        if let LicenseStatus::Trial { minutes_left, .. } = status {
            assert!((minutes_left - 22.0).abs() < 0.001);
        } else {
            panic!("Expected Trial status");
        }
    }

    #[test]
    fn test_manager_check_trial_not_started() {
        let dir = tempdir().unwrap();
        let mut manager = LicenseManager::new(
            "https://api.example.com",
            "test-device-123",
            dir.path(),
        );

        // Check trial without starting
        let status = manager.check_trial().unwrap();
        assert!(matches!(status, LicenseStatus::NotConfigured));
    }
}
