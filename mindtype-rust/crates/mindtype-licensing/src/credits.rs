//! Credits management for MindType Cloud

use crate::error::LicenseError;
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

/// API response for credits balance
#[derive(Debug, Deserialize)]
struct BalanceResponse {
    balance: u32,
}

/// API response for credit deduction
#[derive(Debug, Deserialize)]
struct DeductResponse {
    success: bool,
    remaining: u32,
    #[serde(default)]
    error: Option<String>,
}

/// Credits manager for MindType Cloud
pub struct CreditsManager {
    api_base: String,
    license_key: String,
    cached_balance: Arc<AtomicU32>,
    client: reqwest::Client,
}

impl CreditsManager {
    /// Create a new credits manager
    pub fn new(api_base: &str, license_key: &str) -> Self {
        Self {
            api_base: api_base.to_string(),
            license_key: license_key.to_string(),
            cached_balance: Arc::new(AtomicU32::new(0)),
            client: reqwest::Client::new(),
        }
    }

    /// Get current credit balance
    pub fn cached_balance(&self) -> u32 {
        self.cached_balance.load(Ordering::SeqCst)
    }

    /// Check if there are credits available
    pub fn has_credits(&self) -> bool {
        self.cached_balance() > 0
    }

    /// Fetch balance from API
    pub async fn fetch_balance(&self) -> Result<u32, LicenseError> {
        let url = format!("{}/api/credits/balance", self.api_base);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.license_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LicenseError::ApiError(format!(
                "Failed to fetch balance: {} - {}",
                status, body
            )));
        }

        let body: BalanceResponse = response.json().await?;
        self.cached_balance.store(body.balance, Ordering::SeqCst);

        info!("Credits balance: {}", body.balance);
        Ok(body.balance)
    }

    /// Deduct credits for a document
    pub async fn deduct(&self, amount: u32, reason: &str) -> Result<u32, LicenseError> {
        let current = self.cached_balance();
        if current < amount {
            return Err(LicenseError::InsufficientCredits);
        }

        let url = format!("{}/api/credits/deduct", self.api_base);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.license_key))
            .json(&serde_json::json!({
                "amount": amount,
                "reason": reason,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LicenseError::ApiError(format!(
                "Failed to deduct credits: {} - {}",
                status, body
            )));
        }

        let body: DeductResponse = response.json().await?;

        if !body.success {
            return Err(LicenseError::ApiError(
                body.error.unwrap_or_else(|| "Deduction failed".to_string()),
            ));
        }

        self.cached_balance.store(body.remaining, Ordering::SeqCst);
        debug!(
            "Deducted {} credits, remaining: {}",
            amount, body.remaining
        );

        Ok(body.remaining)
    }

    /// Try to deduct credits, returning whether successful
    pub async fn try_deduct(&self, amount: u32, reason: &str) -> bool {
        match self.deduct(amount, reason).await {
            Ok(_) => true,
            Err(e) => {
                debug!("Credit deduction failed: {}", e);
                false
            }
        }
    }
}
