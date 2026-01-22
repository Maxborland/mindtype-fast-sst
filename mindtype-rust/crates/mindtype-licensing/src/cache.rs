//! License cache for offline validation

use crate::error::LicenseError;
use crate::manager::LicenseStatus;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::PathBuf;
use tracing::debug;

type HmacSha256 = Hmac<Sha256>;

const CACHE_SECRET: &[u8] = b"mindtype-license-cache-v1";
const OFFLINE_GRACE_DAYS: i64 = 7;

/// Cached license data
#[derive(Debug, Serialize, Deserialize)]
struct CacheData {
    status: LicenseStatus,
    cached_at: DateTime<Utc>,
    signature: String,
    trial_start: Option<DateTime<Utc>>,
    trial_minutes_used: f64,
}

/// License cache manager
pub struct LicenseCache {
    cache_file: PathBuf,
}

impl LicenseCache {
    /// Create a new cache instance
    pub fn new(cache_dir: &std::path::Path) -> Self {
        Self {
            cache_file: cache_dir.join("license_cache.json"),
        }
    }

    /// Get cached license status
    pub fn get_status(&self) -> Result<Option<LicenseStatus>, LicenseError> {
        let data = self.read_cache()?;
        match data {
            Some(d) if self.verify_signature(&d) => Ok(Some(d.status)),
            _ => Ok(None),
        }
    }

    /// Set cached license status
    pub fn set_status(&self, status: &LicenseStatus) -> Result<(), LicenseError> {
        let mut data = self.read_cache()?.unwrap_or_else(|| CacheData {
            status: status.clone(),
            cached_at: Utc::now(),
            signature: String::new(),
            trial_start: None,
            trial_minutes_used: 0.0,
        });

        data.status = status.clone();
        data.cached_at = Utc::now();
        data.signature = self.compute_signature(&data);

        self.write_cache(&data)
    }

    /// Check if offline validation is still valid
    pub fn is_valid_offline(&self) -> Result<bool, LicenseError> {
        let data = self.read_cache()?;
        match data {
            Some(d) => {
                let days_offline = (Utc::now() - d.cached_at).num_days();
                Ok(days_offline <= OFFLINE_GRACE_DAYS && self.verify_signature(&d))
            }
            None => Ok(false),
        }
    }

    /// Set trial start time
    pub fn set_trial_start(&self, start: DateTime<Utc>) -> Result<(), LicenseError> {
        let mut data = self.read_cache()?.unwrap_or_else(|| CacheData {
            status: LicenseStatus::NotConfigured,
            cached_at: Utc::now(),
            signature: String::new(),
            trial_start: None,
            trial_minutes_used: 0.0,
        });

        data.trial_start = Some(start);
        data.signature = self.compute_signature(&data);

        self.write_cache(&data)
    }

    /// Get trial start time
    pub fn get_trial_start(&self) -> Result<Option<DateTime<Utc>>, LicenseError> {
        let data = self.read_cache()?;
        Ok(data.and_then(|d| d.trial_start))
    }

    /// Get trial minutes used
    pub fn get_trial_minutes_used(&self) -> Result<f64, LicenseError> {
        let data = self.read_cache()?;
        Ok(data.map(|d| d.trial_minutes_used).unwrap_or(0.0))
    }

    /// Set trial minutes used
    pub fn set_trial_minutes_used(&self, minutes: f64) -> Result<(), LicenseError> {
        let mut data = self.read_cache()?.ok_or_else(|| {
            LicenseError::CacheError("No cache data for trial".to_string())
        })?;

        data.trial_minutes_used = minutes;
        data.signature = self.compute_signature(&data);

        self.write_cache(&data)
    }

    /// Clear the cache
    pub fn clear(&self) -> Result<(), LicenseError> {
        if self.cache_file.exists() {
            std::fs::remove_file(&self.cache_file)?;
        }
        Ok(())
    }

    /// Read cache from disk
    fn read_cache(&self) -> Result<Option<CacheData>, LicenseError> {
        if !self.cache_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&self.cache_file)?;
        let data: CacheData = serde_json::from_str(&content)?;
        Ok(Some(data))
    }

    /// Write cache to disk
    fn write_cache(&self, data: &CacheData) -> Result<(), LicenseError> {
        if let Some(parent) = self.cache_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(data)?;
        std::fs::write(&self.cache_file, content)?;
        debug!("Cache written to {:?}", self.cache_file);
        Ok(())
    }

    /// Compute HMAC signature for cache data
    fn compute_signature(&self, data: &CacheData) -> String {
        let payload = format!(
            "{}:{}:{}:{}",
            serde_json::to_string(&data.status).unwrap_or_default(),
            data.cached_at.timestamp(),
            data.trial_start.map(|t| t.timestamp()).unwrap_or(0),
            data.trial_minutes_used as u64,
        );

        let mut mac =
            HmacSha256::new_from_slice(CACHE_SECRET).expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        let result = mac.finalize();

        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, result.into_bytes())
    }

    /// Verify signature of cache data
    fn verify_signature(&self, data: &CacheData) -> bool {
        let expected = self.compute_signature(data);
        expected == data.signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::Plan;
    use tempfile::tempdir;

    #[test]
    fn test_cache_create() {
        let dir = tempdir().unwrap();
        let cache = LicenseCache::new(dir.path());
        assert!(cache.cache_file.ends_with("license_cache.json"));
    }

    #[test]
    fn test_cache_status_roundtrip() {
        let dir = tempdir().unwrap();
        let cache = LicenseCache::new(dir.path());

        let status = LicenseStatus::Valid {
            plan: Plan::Pro,
            expires_at: Some(Utc::now() + chrono::Duration::days(30)),
        };

        cache.set_status(&status).unwrap();
        let retrieved = cache.get_status().unwrap();

        assert!(retrieved.is_some());
        assert!(matches!(retrieved.unwrap(), LicenseStatus::Valid { plan: Plan::Pro, .. }));
    }

    #[test]
    fn test_cache_empty_returns_none() {
        let dir = tempdir().unwrap();
        let cache = LicenseCache::new(dir.path());

        let status = cache.get_status().unwrap();
        assert!(status.is_none());
    }

    #[test]
    fn test_cache_trial_tracking() {
        let dir = tempdir().unwrap();
        let cache = LicenseCache::new(dir.path());

        // Initialize cache with a status first
        let status = LicenseStatus::Trial {
            days_left: 7,
            minutes_left: 30.0,
        };
        cache.set_status(&status).unwrap();

        // Set and get trial start
        let now = Utc::now();
        cache.set_trial_start(now).unwrap();
        let retrieved = cache.get_trial_start().unwrap();
        assert!(retrieved.is_some());

        // Track trial minutes
        cache.set_trial_minutes_used(5.5).unwrap();
        let minutes = cache.get_trial_minutes_used().unwrap();
        assert!((minutes - 5.5).abs() < 0.001);
    }

    #[test]
    fn test_cache_offline_validation() {
        let dir = tempdir().unwrap();
        let cache = LicenseCache::new(dir.path());

        let status = LicenseStatus::Valid {
            plan: Plan::Personal,
            expires_at: None,
        };
        cache.set_status(&status).unwrap();

        // Should be valid immediately after caching
        assert!(cache.is_valid_offline().unwrap());
    }

    #[test]
    fn test_cache_clear() {
        let dir = tempdir().unwrap();
        let cache = LicenseCache::new(dir.path());

        let status = LicenseStatus::Valid {
            plan: Plan::Team,
            expires_at: None,
        };
        cache.set_status(&status).unwrap();
        assert!(cache.get_status().unwrap().is_some());

        cache.clear().unwrap();
        assert!(cache.get_status().unwrap().is_none());
    }

    #[test]
    fn test_cache_signature_tamper_detection() {
        let dir = tempdir().unwrap();
        let cache = LicenseCache::new(dir.path());

        let status = LicenseStatus::Valid {
            plan: Plan::Personal,
            expires_at: None,
        };
        cache.set_status(&status).unwrap();

        // Read the content and parse it to modify the signature
        let content = std::fs::read_to_string(&cache.cache_file).unwrap();
        let mut data: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Corrupt the signature
        data["signature"] = serde_json::Value::String("tampered_signature".to_string());

        std::fs::write(&cache.cache_file, serde_json::to_string_pretty(&data).unwrap()).unwrap();

        // Should return None due to signature mismatch
        let status = cache.get_status().unwrap();
        assert!(status.is_none());
    }
}
