import type { Dashboard } from "../store";

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(0);
}

function rateClass(rate: number): string {
  return rate >= 0.8 ? "good" : "warn";
}

interface Props {
  dashboard: Dashboard;
}

function rateClassSource(rate: number): string {
  if (rate >= 0.8) return "good";
  if (rate >= 0.4) return "warn";
  return "bad";
}

export default function CacheHitPanel({ dashboard }: Props) {
  const {
    cache_overall_rate,
    cache_hit_tokens,
    cache_miss_tokens,
    cache_by_model,
    cache_by_source,
  } = dashboard;

  return (
    <div className="card">
      <div className="section-title">{'缓存命中率'}</div>

      {/* Overall */}
      <div className="cache-overall">
        <div className={`cache-rate ${rateClass(cache_overall_rate)}`}>
          {(cache_overall_rate * 100).toFixed(1)}%
        </div>
        <div className="progress-bar">
          <div
            className={`progress-fill ${rateClass(cache_overall_rate)}`}
            style={{ width: `${Math.min(cache_overall_rate * 100, 100)}%` }}
          />
        </div>
        <div className="cache-tokens">
          {'命中'}: {fmt(cache_hit_tokens)} / {'未命中'}: {fmt(cache_miss_tokens)}
        </div>
      </div>

      {/* By model */}
      {cache_by_model.length > 0 && (
        <>
          <div style={{ marginTop: 10 }}>
            <div className="section-title">{'按模型'}</div>
          </div>
          <div className="cache-models">
            {cache_by_model.map((m) => (
              <div key={m.model} className="cache-model-card">
                <div className="cache-model-name">
                  {m.model.replace("deepseek-", "")}
                </div>
                <div className={`cache-model-rate ${rateClass(m.cache_hit_rate)}`}>
                  {(m.cache_hit_rate * 100).toFixed(1)}%
                </div>
                <div className="progress-bar" style={{ marginTop: 6 }}>
                  <div
                    className={`progress-fill ${rateClass(m.cache_hit_rate)}`}
                    style={{ width: `${Math.min(m.cache_hit_rate * 100, 100)}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {/* By source (api key) */}
      {cache_by_source.length > 0 && (
        <>
          <div style={{ marginTop: 10 }}>
            <div className="section-title">{'按来源'}</div>
          </div>
          <table className="source-table">
            <thead>
              <tr>
                <th>{'密钥'}</th>
                <th style={{ textAlign: "right" }}>{'请求'}</th>
                <th style={{ textAlign: "right" }}>{'花费'}</th>
                <th style={{ textAlign: "right" }}>{'占比'}</th>
                <th style={{ textAlign: "right" }}>{'命中率'}</th>
              </tr>
            </thead>
            <tbody>
              {cache_by_source.map((s) => (
                <tr key={s.api_key_name}>
                  <td>{s.api_key_name}</td>
                  <td style={{ textAlign: "right" }}>{fmt(s.request_count)}</td>
                  <td style={{ textAlign: "right" }}>¥{s.cost.toFixed(2)}</td>
                  <td style={{ textAlign: "right" }}>{s.cost_pct.toFixed(1)}%</td>
                  <td style={{ textAlign: "right" }}>
                    <span className={`source-rate ${rateClassSource(s.hit_rate)}`}>
                      {(s.hit_rate * 100).toFixed(1)}%
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}

    </div>
  );
}
