# Research A01: Programmatically Fetching DeepSeek Usage Data

**Date:** 2026-06-09
**Researcher:** Research-A01
**Goal:** Find whether DeepSeek's web platform has internal API endpoints that can be called directly to get usage/amount/cost data without manual CSV download.

---

## Executive Summary

DeepSeek **does not provide a documented public API endpoint** for programmatically fetching usage/billing data. The only programmatically accessible data point is account balance via `GET https://api.deepseek.com/user/balance`. All other approaches require either (a) browser-based auth tokens extracted manually, or (b) local accumulation of token usage from API responses.

---

## 1. Endpoints Discovered

### 1.1 API-Endpoints (api.deepseek.com) -- Use API Key (Bearer Token)

| Endpoint | Method | Status | Auth | Description |
|----------|--------|--------|------|-------------|
| `/user/balance` | GET | **200 -- WORKS** | API Key | Returns balance info (CNY amounts) |
| `/v1/chat/completions` | POST | **200 -- WORKS** | API Key | Returns `usage` object with token counts per request |
| `/v1/models` | GET | **200 -- WORKS** | API Key | Lists available models (v4-flash, v4-pro) |
| `/v1/usage` | GET | **404 -- NOT FOUND** | API Key | Does not exist; contrary to multiple Chinese blog posts |
| `/v1/usage/` | GET | **404 -- NOT FOUND** | -- | Trailing slash also 404 |
| `/v1/billing/usage` | GET | **404 -- NOT FOUND** | -- | Does not exist |
| `/v1/dashboard/billing/usage` | GET | **404 -- NOT FOUND** | -- | Does not exist |
| `/v1/organization/usage` | GET | **404 -- NOT FOUND** | -- | Does not exist |
| `/v1/organization/costs` | GET | **404 -- NOT FOUND** | -- | Does not exist |
| `/v1/usage/records` | GET | **404 -- NOT FOUND** | -- | Does not exist |

Verdict: `/v1/usage` and all billing-related endpoints **do not exist** on api.deepseek.com. The Chinese blog posts that reference it are incorrect or outdated.

### 1.2 Platform Web App Endpoints (platform.deepseek.com) -- Use userToken JWT (NOT API Key)

| Endpoint | Method | Auth Required | Status | Description |
|----------|--------|---------------|--------|-------------|
| `/api/v0/usage/cost?month=MM&year=YYYY` | GET | Browser userToken JWT | **200 - EXISTS** | Returns full monthly cost breakdown by model per day |
| `/api/v0/user/usage` | GET | -- | Blocked (429 Cloudflare) | Cloudflare blocks non-browser requests |
| `/api/v0/balance` | GET | -- | Blocked (429 Cloudflare) | Cloudflare blocks non-browser requests |

The `/api/v0/usage/cost` endpoint is the SPA's internal API that powers the Usage page. Here is what we know about it:

**Request:**
```
GET https://platform.deepseek.com/api/v0/usage/cost?month=06&year=2026
Authorization: Bearer <userToken>       # JWT from browser localStorage
Referer: https://platform.deepseek.com/usage
Accept: application/json
User-Agent: Mozilla/5.0 ... (browser-like)
Cookie: <optional browser cookies>
```

**Auth test results:**
- With API Key (`sk-3b1...`): `{"code":40003,"msg":"Authorization Failed (invalid token)"}` -- API key is NOT valid for this endpoint
- Without any token: `{"code":40002,"msg":"Missing Token"}` -- endpoint exists but requires auth
- The required token is the `userToken` JWT stored in browser localStorage on `platform.deepseek.com`

**Response structure** (from koishi-plugin-deepseek-usage source code):
```json
{
  "code": 0,
  "data": {
    "biz_data": [{
      "currency": "CNY",
      "days": [
        {
          "date": "2026-06-01",
          "data": [
            {
              "model": "deepseek-v4-flash",
              "usage": [
                { "amount": "12.34" }   // cost in currency units
              ]
            }
          ]
        }
      ],
      "total": [
        {
          "model": "deepseek-v4-flash",
          "usage": [
            { "amount": "123.45" }
          ]
        }
      ]
    }]
  }
}
```

### 1.3 Response Headers from Chat API

Chat completion responses contain token usage in the response body (`usage` object), but **no billing headers**:
- `prompt_tokens`: input tokens consumed
- `completion_tokens`: output tokens generated
- `total_tokens`: sum of above
- `completion_tokens_details.reasoning_tokens`: reasoning tokens (thinking mode)
- `prompt_cache_hit_tokens`: cache hit tokens
- `prompt_cache_miss_tokens`: cache miss tokens

No `x-billed-tokens` header was found in any v4 model API response. This header was mentioned in some older posts but is absent from current v4 API responses.

---

## 2. Can Our API Key Be Used?

**API Key:** `sk-3b1955f5ab0e44f3a01e8481b720805b`

**What it CAN do:**
- `GET /user/balance` -- Returns balance (100.58 CNY, all topped-up, no granted balance)
- `POST /v1/chat/completions` -- Standard chat API
- `GET /v1/models` -- List models

**What it CANNOT do:**
- Access any usage/billing/cost API endpoint (they don't exist on api.deepseek.com)
- Access platform.deepseek.com internal endpoints (requires browser userToken JWT, not API key)

---

## 3. Current Pricing (for Cost Calculation)

Prices from api-docs.deepseek.com (in USD):

| Model | Input (cache miss) | Input (cache hit) | Output |
|-------|-------------------|-------------------|--------|
| deepseek-v4-flash | $0.14/M | $0.0028/M | $0.28/M |
| deepseek-v4-pro | $0.435/M | $0.003625/M | $0.87/M |

Note: Balance is returned in CNY while documented pricing is in USD. Exchange rate fluctuations must be accounted for, or obtain CNY pricing from the Chinese pricing page (`api-docs.deepseek.com/zh-cn/quick_start/pricing`).

---

## 4. Recommended Implementation Options

### Option A: Balance Delta Tracking (Simplest)

**How it works:** Poll `/user/balance` at regular intervals (e.g., every 60 seconds). Calculate spending as the difference between consecutive balance readings.

**Pros:**
- Works with the existing API key
- Simple to implement
- Gives real spending in CNY (no exchange rate issues)

**Cons:**
- Cannot break down cost by model
- Cannot see per-request costs
- Misses spending if balance is topped up between checks
- Balance has limited precision (2 decimal places)
- No historical data beyond 90 days of local logs

**Implementation sketch:**
```python
import requests, time

def get_balance(api_key):
    r = requests.get("https://api.deepseek.com/user/balance",
                     headers={"Authorization": f"Bearer {api_key}"})
    data = r.json()
    info = data["balance_infos"][0]
    return float(info["total_balance"])

# Track: balance_delta = previous_balance - current_balance
```

### Option B: Token Accumulation from API Responses (Recommended)

**How it works:** Intercept all chat completion API calls. For each response, read the `usage` object and multiply tokens by model-specific pricing. Accumulate over time.

**Pros:**
- Per-request granularity
- Can break down by model
- Works with existing API key
- Can track cache hits separately for cost savings analysis
- Works with OpenLIT/OpenTelemetry instrumentation

**Cons:**
- Must intercept all API calls (requires middleware/proxy)
- Need to maintain accurate pricing table
- USD-to-CNY conversion needed if comparing to platform balance
- Does not capture costs from other API keys on the same account

**Implementation sketch:**
```python
PRICING = {
    "deepseek-v4-flash":  {"input": 0.14,  "input_cache": 0.0028,  "output": 0.28},
    "deepseek-v4-pro":    {"input": 0.435, "input_cache": 0.003625, "output": 0.87},
}

def calc_cost(model, usage):
    p = PRICING.get(model, {})
    cache_hit = usage.get("prompt_cache_hit_tokens", 0)
    cache_miss = usage.get("prompt_cache_miss_tokens", 0)
    completion = usage.get("completion_tokens", 0)
    input_cost = (cache_miss * p["input"] + cache_hit * p["input_cache"]) / 1_000_000
    output_cost = completion * p["output"] / 1_000_000
    return input_cost + output_cost
```

### Option C: Browser userToken Extraction + Platform API (Most Complete)

**How it works:** Extract the `userToken` JWT from browser localStorage on `platform.deepseek.com`, then use it to call `/api/v0/usage/cost` to get the same data the web UI shows.

**Pros:**
- Exact same data as the platform UI (ground truth)
- Full cost breakdown by model and day
- Already in CNY

**Cons:**
- `userToken` must be manually extracted from browser (not automatable without browser login)
- Token expiration unknown (may refresh periodically)
- Requires different auth mechanism than API key
- Cloudflare protection may block non-browser requests (enhanced browser-like headers needed)
- Same-domain cookie may be required alongside the token

**Token extraction:**
1. Open https://platform.deepseek.com in browser (logged in)
2. F12 -> Application -> Local Storage -> platform.deepseek.com
3. Copy the `userToken` value
4. Note: the stored value might be `{"value":"<actual-jwt>"}` -- the koishi plugin tries to JSON.parse it first

### Option D: OpenLIT / OpenTelemetry (Production-Grade Monitoring)

**How it works:** Use OpenLIT's auto-instrumentation which wraps the OpenAI-compatible client and captures traces with token usage and cost calculation.

**Pros:**
- Zero code changes if using standard OpenAI client
- Full observability: traces, tokens, costs, latency, prompts
- Open source (Apache 2.0), self-hosted
- OpenTelemetry compatible (export to Grafana, Datadog, etc.)

**Cons:**
- Adds infrastructure dependency (OTLP collector, UI)
- Overkill for simple balance monitoring
- Cost calculation uses pricing table that must be kept current

---

## 5. Recommendation

**For the deepseek-monitor project, recommend a hybrid approach combining Option A and Option B:**

1. **Poll `/user/balance` at interval** (e.g., every 5 minutes) to track total spending as ground truth in CNY
2. **Parse `usage` from each API response** to accumulate model-specific token counts and estimate costs
3. **Cross-reference** the balance delta against accumulated token costs to validate
4. **Log everything** to a local file (JSON) for historical analysis and charting

This gives you:
- Real CNY spending from balance tracking (authoritative)
- Per-model cost breakdown from token accumulation (detailed)
- No dependency on browser-extracted tokens
- Works entirely with the existing API key

### Architecture

```
[API Requests] --> Middleware/Interceptor --> extracts {model, usage} --> log to file
                                                                     --> calculate cost
[Interval Timer] --> GET /user/balance --> compare deltas --> log to file
                                                          --> detect top-ups

[Dashboard] <-- reads log file --> shows charts, trends, per-model costs
```

---

## 6. Key Files & References

- **koishi-plugin-deepseek-usage source**: https://github.com/gogky/koishi-plugin-deepseek-usage/blob/main/src/index.ts
  - The only known open-source project that successfully calls platform.deepseek.com internal API
  - Shows the exact `/api/v0/usage/cost` endpoint format and response structure
- **Official DeepSeek API docs**: https://api-docs.deepseek.com/
  - Only 5 documented endpoints: Chat, Completions, Models, Balance, and Introduction
  - No usage/billing API exists in official docs
- **OpenLIT DeepSeek integration**: https://docs.openlit.io/latest/sdk/integrations/deepseek
- **DeepSeek Balance Checker**: https://github.com/tristan-mcinnis/Deepseek-API-Balance-Checker

## 7. Live Test Results (2026-06-09)

All tests run with API key `sk-3b1955f5ab0e44f3a01e8481b720805b`:

| Test | Result |
|------|--------|
| `GET /user/balance` | 200: `{"total_balance":"100.58","granted_balance":"0.00","topped_up_balance":"100.58","currency":"CNY"}` |
| `GET /v1/usage` | 404 (empty body) |
| `GET /v1/usage?start_date=2026-06-01&end_date=2026-06-09` | 404 (empty body) |
| `GET /v1/usage?start_date=2026-06-01&end_date=2026-06-09&limit=10` | 404 (empty body) |
| `GET /v1/billing/usage` | 404 |
| `GET /v1/dashboard/billing/usage` | 404 |
| `GET /v1/usage/records` | 404 |
| `GET /v1/organization/usage` | 404 |
| `GET /v1/organization/costs` | 404 |
| `GET /v1/models` | 200: `["deepseek-v4-flash", "deepseek-v4-pro"]` |
| `GET platform.deepseek.com/api/v0/usage/cost` (API key) | 200: `{"code":40003,"msg":"Authorization Failed (invalid token)"}` |
| `GET platform.deepseek.com/api/v0/usage/cost` (no auth) | 200: `{"code":40002,"msg":"Missing Token"}` |
| `POST /v1/chat/completions` response | Includes `usage` with token counts; no `x-billed-tokens` header |
