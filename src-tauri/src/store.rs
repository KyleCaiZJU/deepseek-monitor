use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

use crate::csv_import::{AmountRow, CostRow};

pub struct Store {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayPoint {
    pub date: String,
    pub output_tokens: i64,
    pub cache_hit_tokens: i64,
    pub cache_miss_tokens: i64,
    pub request_count: i64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub output_tokens: i64,
    pub cache_hit_tokens: i64,
    pub cache_miss_tokens: i64,
    pub request_count: i64,
    pub cost: f64,
    pub cache_hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSource {
    pub api_key_name: String,
    pub request_count: i64,
    pub hit_tokens: i64,
    pub miss_tokens: i64,
    pub hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub balance: f64,
    pub available: bool,
    pub today_cost: f64,
    pub month_cost: f64,
    pub trend: Vec<DayPoint>,
    pub models: Vec<ModelUsage>,
    pub cache_overall_rate: f64,
    pub cache_hit_tokens: i64,
    pub cache_miss_tokens: i64,
    pub cache_by_model: Vec<ModelUsage>,
    pub cache_by_source: Vec<CacheSource>,
    pub last_import_ts: Option<String>,
}

impl Store {
    pub fn new(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("Failed to open DB: {e}"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS balance_history (
                ts TEXT PRIMARY KEY,
                balance TEXT NOT NULL,
                available INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS usage (
                utc_date TEXT NOT NULL,
                model TEXT NOT NULL,
                api_key_name TEXT NOT NULL,
                type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                price TEXT,
                PRIMARY KEY (utc_date, model, api_key_name, type)
            );
            CREATE TABLE IF NOT EXISTS cost (
                utc_date TEXT NOT NULL,
                model TEXT NOT NULL,
                cost TEXT NOT NULL,
                currency TEXT NOT NULL DEFAULT 'CNY',
                PRIMARY KEY (utc_date, model)
            );
            CREATE TABLE IF NOT EXISTS import_log (
                file_path TEXT PRIMARY KEY,
                imported_at TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Failed to init DB: {e}"))?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    pub fn upsert_usage(&self, rows: &[AmountRow]) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut count = 0;
        for r in rows {
            let price_str = r.price.as_ref().map(|p| p.to_string());
            let affected = conn
                .execute(
                    "INSERT OR REPLACE INTO usage (utc_date, model, api_key_name, type, amount, price)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        r.utc_date.to_string(),
                        r.model,
                        r.api_key_name,
                        r.r#type,
                        r.amount,
                        price_str,
                    ],
                )
                .map_err(|e| format!("Insert usage: {e}"))?;
            count += affected;
        }
        Ok(count)
    }

    pub fn upsert_cost(&self, rows: &[CostRow]) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut count = 0;
        for r in rows {
            let affected = conn
                .execute(
                    "INSERT OR REPLACE INTO cost (utc_date, model, cost, currency)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        r.utc_date.to_string(),
                        r.model,
                        r.cost.to_string(),
                        r.currency,
                    ],
                )
                .map_err(|e| format!("Insert cost: {e}"))?;
            count += affected;
        }
        Ok(count)
    }

    pub fn insert_balance(&self, ts: &str, balance: &str, available: bool) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO balance_history (ts, balance, available) VALUES (?1, ?2, ?3)",
            params![ts, balance, available as i32],
        )
        .map_err(|e| format!("Insert balance: {e}"))?;
        Ok(())
    }

    pub fn get_last_balance(&self) -> Result<Option<(String, String)>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT ts, balance FROM balance_history ORDER BY ts DESC LIMIT 1")
            .map_err(|e| format!("Query: {e}"))?;
        let result = stmt
            .query_row([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .optional()
            .map_err(|e| format!("Query last balance: {e}"))?;
        Ok(result)
    }

    pub fn get_dashboard(&self) -> Result<Dashboard, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;

        let (balance, available) = match conn.query_row(
            "SELECT balance, available FROM balance_history ORDER BY ts DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
        ) {
            Ok((b, a)) => (
                b.parse::<f64>().unwrap_or(0.0),
                a != 0,
            ),
            Err(_) => (0.0, false),
        };

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let month_start = chrono::Utc::now().format("%Y-%m-01").to_string();

        let today_cost = conn
            .query_row(
                "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM cost WHERE utc_date = ?1",
                params![today],
                |row| row.get::<_, f64>(0),
            )
            .unwrap_or(0.0);

        let month_cost = conn
            .query_row(
                "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM cost WHERE utc_date >= ?1",
                params![month_start],
                |row| row.get::<_, f64>(0),
            )
            .unwrap_or(0.0);

        // 7-day trend
        let mut trend = Vec::new();
        for i in (0..7).rev() {
            let d = chrono::Utc::now() - chrono::Duration::days(i);
            let date_str = d.format("%Y-%m-%d").to_string();
            let row = conn
                .query_row(
                    "SELECT
                        COALESCE(SUM(CASE WHEN type='output_tokens' THEN amount ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN type='input_cache_hit_tokens' THEN amount ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN type='input_cache_miss_tokens' THEN amount ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN type='request_count' THEN amount ELSE 0 END), 0),
                        COALESCE((SELECT CAST(cost AS REAL) FROM cost WHERE utc_date = ?1 LIMIT 1), 0)
                     FROM usage WHERE utc_date = ?1",
                    params![date_str],
                    |row| {
                        Ok(DayPoint {
                            date: date_str.clone(),
                            output_tokens: row.get(0)?,
                            cache_hit_tokens: row.get(1)?,
                            cache_miss_tokens: row.get(2)?,
                            request_count: row.get(3)?,
                            cost: row.get(4)?,
                        })
                    },
                )
                .unwrap_or(DayPoint {
                    date: date_str,
                    output_tokens: 0,
                    cache_hit_tokens: 0,
                    cache_miss_tokens: 0,
                    request_count: 0,
                    cost: 0.0,
                });
            trend.push(row);
        }

        // Model usage
        let mut stmt = conn
            .prepare(
                "SELECT
                    model,
                    COALESCE(SUM(CASE WHEN type='output_tokens' THEN amount ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN type='input_cache_hit_tokens' THEN amount ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN type='input_cache_miss_tokens' THEN amount ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN type='request_count' THEN amount ELSE 0 END), 0)
                 FROM usage
                 GROUP BY model",
            )
            .map_err(|e| format!("Query models: {e}"))?;

        let mut models: Vec<ModelUsage> = stmt
            .query_map([], |row| {
                let hit: i64 = row.get(2)?;
                let miss: i64 = row.get(3)?;
                let total = hit + miss;
                let rate = if total > 0 { hit as f64 / total as f64 } else { 0.0 };
                Ok(ModelUsage {
                    model: row.get(0)?,
                    output_tokens: row.get(1)?,
                    cache_hit_tokens: hit,
                    cache_miss_tokens: miss,
                    request_count: row.get(4)?,
                    cost: 0.0,
                    cache_hit_rate: rate,
                })
            })
            .map_err(|e| format!("Query models: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        // Add cost per model
        for m in &mut models {
            m.cost = conn
                .query_row(
                    "SELECT COALESCE(SUM(CAST(cost AS REAL)), 0) FROM cost WHERE model = ?1",
                    params![m.model],
                    |row| row.get(0),
                )
                .unwrap_or(0.0);
        }

        // Cache hit rate overall
        let (cache_hit_tokens, cache_miss_tokens) = conn
            .query_row(
                "SELECT
                    COALESCE(SUM(CASE WHEN type='input_cache_hit_tokens' THEN amount ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN type='input_cache_miss_tokens' THEN amount ELSE 0 END), 0)
                 FROM usage",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .unwrap_or((0, 0));
        let cache_overall_rate = if cache_hit_tokens + cache_miss_tokens > 0 {
            cache_hit_tokens as f64 / (cache_hit_tokens + cache_miss_tokens) as f64
        } else {
            0.0
        };

        // Cache by model
        let cache_by_model = models.clone();

        // Cache by source (api_key_name)
        let mut src_stmt = conn
            .prepare(
                "SELECT
                    api_key_name,
                    COALESCE(SUM(CASE WHEN type='request_count' THEN amount ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN type='input_cache_hit_tokens' THEN amount ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN type='input_cache_miss_tokens' THEN amount ELSE 0 END), 0)
                 FROM usage
                 GROUP BY api_key_name",
            )
            .map_err(|e| format!("Query sources: {e}"))?;

        let cache_by_source: Vec<CacheSource> = src_stmt
            .query_map([], |row| {
                let hit: i64 = row.get(2)?;
                let miss: i64 = row.get(3)?;
                let total = hit + miss;
                let rate = if total > 0 { hit as f64 / total as f64 } else { 0.0 };
                Ok(CacheSource {
                    api_key_name: row.get(0)?,
                    request_count: row.get(1)?,
                    hit_tokens: hit,
                    miss_tokens: miss,
                    hit_rate: rate,
                })
            })
            .map_err(|e| format!("Query sources: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        let last_import_ts = conn
            .query_row(
                "SELECT imported_at FROM import_log ORDER BY imported_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        Ok(Dashboard {
            balance,
            available,
            today_cost,
            month_cost,
            trend,
            models,
            cache_overall_rate,
            cache_hit_tokens,
            cache_miss_tokens,
            cache_by_model,
            cache_by_source,
            last_import_ts,
        })
    }

    pub fn log_import(&self, file_path: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        conn.execute(
            "INSERT OR REPLACE INTO import_log (file_path, imported_at) VALUES (?1, ?2)",
            params![file_path, now],
        )
        .map_err(|e| format!("Log import: {e}"))?;
        Ok(())
    }

    pub fn was_imported(&self, file_path: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM import_log WHERE file_path = ?1",
                params![file_path],
                |row| row.get(0),
            )
            .unwrap_or(false);
        Ok(exists)
    }
}
