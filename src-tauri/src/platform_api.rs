use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::str::FromStr;

use crate::csv_import::{self, AmountRow, CostRow};

/// A single cost entry extracted from the platform API response.
pub struct CostEntry {
    pub utc_date: String,
    pub model: String,
    pub cost: String,
    pub currency: String,
}

/// A single usage entry extracted from the platform amount API response.
/// Contains token counts and request counts mapped to DB-compatible type names.
pub struct UsageEntry {
    pub utc_date: String,
    pub model: String,
    pub db_type: String,
    pub amount: i64,
}

/// Call the DeepSeek platform internal API to fetch daily cost breakdown.
///
/// Endpoint: GET https://platform.deepseek.com/api/v0/usage/cost?month=MM&year=YYYY
/// Auth: Bearer <userToken> (JWT from browser localStorage on platform.deepseek.com)
///
/// The response shape:
///   { "code": 0, "data": { "biz_data": [{ "currency": "CNY", "days": [
///     { "date": "2026-06-01", "data": [{ "model": "...", "usage": [
///       {"type":"PROMPT_TOKEN","amount":"0"},
///       {"type":"PROMPT_CACHE_HIT_TOKEN","amount":"37.16..."},
///       ...
///     ] }] }
///   ] }] } }
///
/// Amounts are CNY costs (not token counts).  Sum all usage amounts per day+model.
/// A User-Agent header is required — Cloudflare blocks requests without one.
pub async fn fetch_platform_usage(
    token: &str,
    month: u32,
    year: i32,
) -> Result<Vec<CostEntry>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Build client: {e}"))?;

    let url = format!(
        "https://platform.deepseek.com/api/v0/usage/cost?month={:02}&year={}",
        month, year
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        )
        .header("Referer", "https://platform.deepseek.com/usage")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    let raw_text = resp.text().await.map_err(|e| format!("Read body: {e}"))?;

    log::info!(
        "Platform API status={}, body={}",
        status,
        &raw_text[..raw_text.len().min(800)]
    );

    if !status.is_success() {
        return Err(format!("HTTP {}", status));
    }

    let root: serde_json::Value =
        serde_json::from_str(&raw_text).map_err(|e| format!("Parse JSON: {e}"))?;

    // Check API-level success indicator (code=0 means success)
    let code = root.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        let msg = root.get("msg").and_then(|v| v.as_str()).unwrap_or("");
        log::warn!("Platform API returned code={}, msg=\"{}\"", code, msg);
        // Continue parsing — data may still be present despite non-zero code
    }

    let mut entries: Vec<CostEntry> = Vec::new();

    // ── Primary format: data.biz_data[].days[].data[].model + usage[].amount ──
    // Each day has one or more models; each model has a usage array.
    // Sum all usage[].amount values for a (date, model) pair to get the
    // total CNY cost for that day+model.
    if let Some(biz_data) = root
        .get("data")
        .and_then(|d| d.get("biz_data"))
        .and_then(|b| b.as_array())
    {
        for biz in biz_data {
            let currency = biz
                .get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or("CNY");
            if let Some(days) = biz.get("days").and_then(|d| d.as_array()) {
                for day in days {
                    let date = day
                        .get("date")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if date.is_empty() {
                        continue;
                    }
                    if let Some(data) = day.get("data").and_then(|d| d.as_array()) {
                        for model_entry in data {
                            let model = model_entry
                                .get("model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            // Sum all usage amounts for this (date, model) pair
                            let total: Decimal = model_entry
                                .get("usage")
                                .and_then(|u| u.as_array())
                                .map(|usage| {
                                    usage
                                        .iter()
                                        .filter_map(|u| {
                                            u.get("amount")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| Decimal::from_str(s).ok())
                                        })
                                        .sum()
                                })
                                .unwrap_or_default();
                            entries.push(CostEntry {
                                utc_date: date.to_string(),
                                model: model.to_string(),
                                cost: total.to_string(),
                                currency: currency.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // ── Fallback: simple flat data.costs[] ──
    if entries.is_empty() {
        if let Some(costs) = root
            .get("data")
            .and_then(|d| d.get("costs"))
            .and_then(|c| c.as_array())
        {
            for item in costs {
                let date = item
                    .get("utc_date")
                    .or_else(|| item.get("date"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let model = item
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let cost = item
                    .get("cost")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                if !date.is_empty() {
                    entries.push(CostEntry {
                        utc_date: date.to_string(),
                        model: model.to_string(),
                        cost: cost.to_string(),
                        currency: "CNY".to_string(),
                    });
                }
            }
        }
    }

    if entries.is_empty() {
        log::warn!(
            "Platform API: no cost entries parsed from response. Raw: {}",
            &raw_text[..raw_text.len().min(500)]
        );
    }

    Ok(entries)
}

/// Map a platform API usage type string to the corresponding DB `usage.type` column value.
/// PROMPT_TOKEN and RESPONSE_TOKEN both map to `output_tokens`; amounts for the same
/// (date, model, db_type) key are summed together in `fetch_amount`.
fn map_api_type(api_type: &str) -> Option<&'static str> {
    match api_type {
        "RESPONSE_TOKEN" => Some("output_tokens"),
        "PROMPT_TOKEN" => Some("output_tokens"),
        "PROMPT_CACHE_HIT_TOKEN" => Some("input_cache_hit_tokens"),
        "PROMPT_CACHE_MISS_TOKEN" => Some("input_cache_miss_tokens"),
        "REQUEST" => Some("request_count"),
        _ => None,
    }
}

/// Call the DeepSeek platform internal API to fetch daily token / request counts.
///
/// Endpoint: GET https://platform.deepseek.com/api/v0/usage/amount?month=MM&year=YYYY
/// Auth: Bearer <userToken> (same JWT as the cost endpoint).
///
/// The response shape:
///   { "code": 0, "data": { "biz_code": 0, "biz_data": {
///     "total": [{"model": "...", "usage": [{"type": "...", "amount": "..."}]}],
///     "days": [
///       { "date": "2026-06-01", "data": [
///         { "model": "deepseek-v4-pro", "usage": [
///           {"type": "RESPONSE_TOKEN", "amount": "295880320"},
///           {"type": "PROMPT_CACHE_HIT_TOKEN", "amount": "295880320"},
///           ...
///         ] }
///       ] }
///     ]
///   } } }
///
/// PROMPT_TOKEN and RESPONSE_TOKEN are summed into a single `output_tokens` row per
/// (date, model) because the usage table uses a composite primary key.
pub async fn fetch_amount(
    token: &str,
    month: u32,
    year: i32,
) -> Result<Vec<UsageEntry>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Build client: {e}"))?;

    let url = format!(
        "https://platform.deepseek.com/api/v0/usage/amount?month={:02}&year={}",
        month, year
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        )
        .header("Referer", "https://platform.deepseek.com/usage")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Amount request failed: {e}"))?;

    let status = resp.status();
    let raw_text = resp.text().await.map_err(|e| format!("Read body: {e}"))?;

    log::info!(
        "Platform amount API status={}, body len={}",
        status,
        raw_text.len()
    );

    if !status.is_success() {
        return Err(format!("Amount HTTP {}", status));
    }

    let root: serde_json::Value =
        serde_json::from_str(&raw_text).map_err(|e| format!("Parse amount JSON: {e}"))?;

    let code = root.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        let msg = root.get("msg").and_then(|v| v.as_str()).unwrap_or("");
        log::warn!("Platform amount API returned code={}, msg=\"{}\"", code, msg);
    }

    // biz_data is an object (not an array) with a "days" array
    let days = root
        .get("data")
        .and_then(|d| d.get("biz_data"))
        .and_then(|b| b.get("days"))
        .and_then(|d| d.as_array());

    let Some(days_array) = days else {
        log::warn!(
            "Platform amount API: no days array found. Raw: {}",
            &raw_text[..raw_text.len().min(500)]
        );
        return Ok(Vec::new());
    };

    // Accumulate amounts keyed by (date, model, db_type) so that entries that
    // map to the same DB type (e.g. PROMPT_TOKEN + RESPONSE_TOKEN → output_tokens)
    // are summed into a single row.
    let mut amounts_by_key: BTreeMap<(String, String, String), i64> = BTreeMap::new();

    for day in days_array {
        let date = day
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if date.is_empty() {
            continue;
        }
        if let Some(data) = day.get("data").and_then(|d| d.as_array()) {
            for model_entry in data {
                let model = model_entry
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                if let Some(usage) = model_entry.get("usage").and_then(|u| u.as_array()) {
                    for u in usage {
                        let api_type = u.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let db_type = match map_api_type(api_type) {
                            Some(t) => t,
                            None => continue,
                        };
                        let amount: i64 = u
                            .get("amount")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                        if amount > 0 {
                            let key = (date.to_string(), model.to_string(), db_type.to_string());
                            *amounts_by_key.entry(key).or_insert(0) += amount;
                        }
                    }
                }
            }
        }
    }

    let entries: Vec<UsageEntry> = amounts_by_key
        .into_iter()
        .map(|((utc_date, model, db_type), amount)| UsageEntry {
            utc_date,
            model,
            db_type,
            amount,
        })
        .collect();

    if entries.is_empty() {
        log::warn!(
            "Platform amount API: no entries parsed. Raw: {}",
            &raw_text[..raw_text.len().min(500)]
        );
    } else {
        log::info!("Platform amount API: parsed {} usage entries", entries.len());
    }

    Ok(entries)
}

/// Download the platform usage export ZIP, extract both CSV files, and parse them.
///
/// Endpoint: GET https://platform.deepseek.com/api/v0/usage/export?month=MM&year=YYYY
/// Returns: ZIP containing `cost-YYYY-M.csv` and `amount-YYYY-M.csv`
///
/// The amount CSV has a per-key `api_key_name` column, giving us the data needed
/// for the "by source" cache-hit breakdown that the separate amount API lacks.
pub async fn fetch_export_zip(
    token: &str,
    month: u32,
    year: i32,
) -> Result<(Vec<AmountRow>, Vec<CostRow>), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Build client: {e}"))?;

    let url = format!(
        "https://platform.deepseek.com/api/v0/usage/export?month={:02}&year={}",
        month, year
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        )
        .header("Referer", "https://platform.deepseek.com/usage")
        .send()
        .await
        .map_err(|e| format!("Export request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Export HTTP {}", status));
    }

    let zip_bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Read ZIP body: {e}"))?;

    log::info!(
        "Export ZIP downloaded: {} bytes for {}-{:02}",
        zip_bytes.len(),
        year,
        month
    );

    let cursor = Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Open ZIP: {e}"))?;

    // The ZIP file names use month without leading zero: cost-2026-6.csv
    let amount_name = format!("amount-{}-{}.csv", year, month);
    let cost_name = format!("cost-{}-{}.csv", year, month);

    let mut amount_rows: Vec<AmountRow> = Vec::new();
    let mut cost_rows: Vec<CostRow> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Read ZIP entry {i}: {e}"))?;
        let fname = file.name().to_string();

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| format!("Read {fname} from ZIP: {e}"))?;

        // CSV files from the platform have UTF-8 BOM + CRLF line endings
        let content = String::from_utf8(buf).map_err(|e| format!("{fname} not UTF-8: {e}"))?;

        if fname == amount_name || fname.contains("amount-") {
            match csv_import::parse_amount_csv_content(&content) {
                Ok(rows) => {
                    log::info!("Parsed {} amount rows from {}", rows.len(), fname);
                    amount_rows = rows;
                }
                Err(e) => {
                    log::warn!("Failed to parse {}: {}", fname, e);
                }
            }
        } else if fname == cost_name || fname.contains("cost-") {
            match csv_import::parse_cost_csv_content(&content) {
                Ok(rows) => {
                    log::info!("Parsed {} cost rows from {}", rows.len(), fname);
                    cost_rows = rows;
                }
                Err(e) => {
                    log::warn!("Failed to parse {}: {}", fname, e);
                }
            }
        }
    }

    Ok((amount_rows, cost_rows))
}
