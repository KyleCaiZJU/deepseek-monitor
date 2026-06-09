use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub currency: String,
    pub total_balance: String,
    pub granted_balance: String,
    pub topped_up_balance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub is_available: bool,
    pub balance_infos: Vec<BalanceInfo>,
}

pub async fn fetch_balance(api_key: &str) -> Result<BalanceResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Build client: {e}"))?;

    let resp = client
        .get("https://api.deepseek.com/user/balance")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body: BalanceResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse balance: {e}"))?;

    Ok(body)
}
