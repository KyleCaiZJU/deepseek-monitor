use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountRow {
    pub user_id: String,
    pub utc_date: NaiveDate,
    pub model: String,
    pub api_key_name: String,
    pub api_key: String,
    pub r#type: String,
    pub price: Option<Decimal>,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRow {
    pub user_id: String,
    pub utc_date: NaiveDate,
    pub model: String,
    pub wallet_type: String,
    pub cost: Decimal,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub amount_rows: usize,
    pub cost_rows: usize,
    pub skipped_amount: usize,
    pub skipped_cost: usize,
}

fn strip_bom(content: &str) -> &str {
    content.strip_prefix('\u{FEFF}').unwrap_or(content)
}

pub fn parse_amount_csv(path: &Path) -> Result<Vec<AmountRow>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("Failed to read amount CSV: {e}"))?;
    let content = strip_bom(&raw);

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut rows = Vec::new();
    for (line_num, result) in reader.records().enumerate() {
        let record = result.map_err(|e| format!("Line {}: {e}", line_num + 1))?;
        if record.is_empty() || record.iter().all(|f| f.trim().is_empty()) {
            continue;
        }

        let utc_date_str = record.get(1).unwrap_or("");
        let utc_date = NaiveDate::parse_from_str(utc_date_str.trim(), "%Y-%m-%d")
            .map_err(|e| format!("Line {}: invalid date '{}': {e}", line_num + 1, utc_date_str))?;

        let price_str = record.get(6).unwrap_or("").trim();
        let price = if price_str.is_empty() {
            None
        } else {
            Some(
                price_str
                    .parse::<Decimal>()
                    .map_err(|e| format!("Line {}: invalid price '{}': {e}", line_num + 1, price_str))?,
            )
        };

        let amount_str = record.get(7).unwrap_or("0").trim();
        let amount = amount_str
            .parse::<i64>()
            .map_err(|e| format!("Line {}: invalid amount '{}': {e}", line_num + 1, amount_str))?;

        rows.push(AmountRow {
            user_id: record.get(0).unwrap_or("").trim().to_string(),
            utc_date,
            model: record.get(2).unwrap_or("").trim().to_string(),
            api_key_name: record.get(3).unwrap_or("").trim().to_string(),
            api_key: record.get(4).unwrap_or("").trim().to_string(),
            r#type: record.get(5).unwrap_or("").trim().to_string(),
            price,
            amount,
        });
    }
    Ok(rows)
}

pub fn parse_cost_csv(path: &Path) -> Result<Vec<CostRow>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("Failed to read cost CSV: {e}"))?;
    let content = strip_bom(&raw);

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut rows = Vec::new();
    for (line_num, result) in reader.records().enumerate() {
        let record = result.map_err(|e| format!("Line {}: {e}", line_num + 1))?;
        if record.is_empty() || record.iter().all(|f| f.trim().is_empty()) {
            continue;
        }

        let utc_date_str = record.get(1).unwrap_or("");
        let utc_date = NaiveDate::parse_from_str(utc_date_str.trim(), "%Y-%m-%d")
            .map_err(|e| format!("Line {}: invalid date '{}': {e}", line_num + 1, utc_date_str))?;

        let cost_str = record.get(4).unwrap_or("0").trim();
        let cost = cost_str
            .parse::<Decimal>()
            .map_err(|e| format!("Line {}: invalid cost '{}': {e}", line_num + 1, cost_str))?;

        rows.push(CostRow {
            user_id: record.get(0).unwrap_or("").trim().to_string(),
            utc_date,
            model: record.get(2).unwrap_or("").trim().to_string(),
            wallet_type: record.get(3).unwrap_or("").trim().to_string(),
            cost,
            currency: record.get(5).unwrap_or("").trim().to_string(),
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_price() {
        let csv = "user_id,utc_date,model,api_key_name,api_key,type,price,amount\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-xxx,request_count,,2066";
        let dir = std::env::temp_dir();
        let path = dir.join("test_empty_price.csv");
        fs::write(&path, csv).unwrap();
        let rows = parse_amount_csv(&path).unwrap();
        fs::remove_file(&path).ok();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].r#type, "request_count");
        assert!(rows[0].price.is_none());
        assert_eq!(rows[0].amount, 2066);
    }

    #[test]
    fn test_parse_with_price() {
        let csv = "user_id,utc_date,model,api_key_name,api_key,type,price,amount\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-xxx,output_tokens,0.000006,1310166";
        let dir = std::env::temp_dir();
        let path = dir.join("test_with_price.csv");
        fs::write(&path, csv).unwrap();
        let rows = parse_amount_csv(&path).unwrap();
        fs::remove_file(&path).ok();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].price.unwrap(), Decimal::new(6, 6)); // 0.000006
    }

    #[test]
    fn test_parse_bom() {
        let csv_bytes = vec![
            0xEF, 0xBB, 0xBF, // BOM
        ];
        let csv_content = String::from_utf8(csv_bytes.clone()).unwrap();
        let full_csv = format!(
            "{}user_id,utc_date,model,api_key_name,api_key,type,price,amount\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-xxx,output_tokens,0.000006,1310166",
            csv_content
        );
        let dir = std::env::temp_dir();
        let path = dir.join("test_bom.csv");
        fs::write(&path, &full_csv).unwrap();
        let rows = parse_amount_csv(&path).unwrap();
        fs::remove_file(&path).ok();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_parse_crlf() {
        let csv = "user_id,utc_date,model,api_key_name,api_key,type,price,amount\r\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-xxx,output_tokens,0.000006,1310166";
        let dir = std::env::temp_dir();
        let path = dir.join("test_crlf.csv");
        fs::write(&path, csv).unwrap();
        let rows = parse_amount_csv(&path).unwrap();
        fs::remove_file(&path).ok();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_cache_hit_rate_v4_pro_june1() {
        let csv = "user_id,utc_date,model,api_key_name,api_key,type,price,amount\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-1181a...,input_cache_hit_tokens,0.000000025,237281408\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-1181a...,input_cache_miss_tokens,0.000003,9459428\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-3b195...,input_cache_hit_tokens,0.000000025,58469248\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-3b195...,input_cache_miss_tokens,0.000003,4743424\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-45e26...,input_cache_hit_tokens,0.000000025,129664\n\
                        X,2026-06-01,deepseek-v4-pro,ccc,sk-45e26...,input_cache_miss_tokens,0.000003,256722\n\
                        X,2026-06-01,deepseek-v4-pro,vael,sk-55d82...,input_cache_miss_tokens,0.000003,75933";
        let dir = std::env::temp_dir();
        let path = dir.join("test_hit_rate.csv");
        fs::write(&path, csv).unwrap();
        let rows = parse_amount_csv(&path).unwrap();
        fs::remove_file(&path).ok();

        let mut hit: i64 = 0;
        let mut miss: i64 = 0;
        for r in &rows {
            if r.model == "deepseek-v4-pro" && r.utc_date == NaiveDate::from_ymd_opt(2026, 6, 1).unwrap() {
                if r.r#type == "input_cache_hit_tokens" {
                    hit += r.amount;
                } else if r.r#type == "input_cache_miss_tokens" {
                    miss += r.amount;
                }
            }
        }
        let total = hit + miss;
        let rate = hit as f64 / total as f64;
        assert!(rate > 0.95 && rate < 0.96, "Expected ~95.3%, got {:.1}%", rate * 100.0);
    }

    #[test]
    fn test_parse_real_csv() {
        let amount_path = Path::new(r"E:\Download\usage_data_2026_6\amount-2026-6.csv");
        if !amount_path.exists() {
            eprintln!("Skipping: real CSV not found");
            return;
        }
        let rows = parse_amount_csv(amount_path).unwrap();
        assert!(!rows.is_empty(), "Should parse real CSV");

        // Verify cache hit rate for v4-pro on June 1
        let mut hit: i64 = 0;
        let mut miss: i64 = 0;
        for r in &rows {
            if r.model == "deepseek-v4-pro" && r.utc_date == NaiveDate::from_ymd_opt(2026, 6, 1).unwrap() {
                if r.r#type == "input_cache_hit_tokens" {
                    hit += r.amount;
                } else if r.r#type == "input_cache_miss_tokens" {
                    miss += r.amount;
                }
            }
        }
        let total = hit + miss;
        let rate = hit as f64 / total as f64;
        println!("v4-pro 6/1: hit={hit} miss={miss} total={total} rate={:.2}%", rate * 100.0);
        assert!(rate > 0.90, "Cache hit rate too low: {:.2}%", rate * 100.0);
    }

    #[test]
    fn test_parse_cost_csv_real() {
        let cost_path = Path::new(r"E:\Download\usage_data_2026_6\cost-2026-6.csv");
        if !cost_path.exists() {
            eprintln!("Skipping: real CSV not found");
            return;
        }
        let rows = parse_cost_csv(cost_path).unwrap();
        assert!(!rows.is_empty(), "Should parse real cost CSV");
        // Calculate total cost for June 1
        let total: Decimal = rows
            .iter()
            .filter(|r| r.utc_date == NaiveDate::from_ymd_opt(2026, 6, 1).unwrap())
            .map(|r| r.cost)
            .sum();
        println!("June 1 total cost: {total}");
        assert!(total > Decimal::ZERO);
    }
}
