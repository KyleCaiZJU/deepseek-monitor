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
      <div className="section-title">Cache Hit Rate</div>

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
          Hit: {fmt(cache_hit_tokens)} / Miss: {fmt(cache_miss_tokens)}
        </div>
      </div>

      {/* By model */}
      {cache_by_model.length > 0 && (
        <>
          <div style={{ marginTop: 10 }}>
            <div className="section-title">By Model</div>
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

      {/* By source */}
      {cache_by_source.length > 0 && (
        <>
          <div style={{ marginTop: 10 }}>
            <div className="section-title">By Source</div>
          </div>
          <table className="source-table">
            <thead>
              <tr>
                <th>Source</th>
                <th>Requests</th>
                <th>Hit Rate</th>
              </tr>
            </thead>
            <tbody>
              {cache_by_source.map((s) => (
                <tr key={s.api_key_name}>
                  <td>{s.api_key_name}</td>
                  <td>{s.request_count}</td>
                  <td className={`source-rate ${rateClass(s.hit_rate)}`}>
                    {(s.hit_rate * 100).toFixed(1)}%
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
