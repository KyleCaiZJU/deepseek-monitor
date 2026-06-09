import type { ModelUsage as MU } from "../store";

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(0);
}

interface Props {
  models: MU[];
}

export default function ModelUsage({ models }: Props) {
  if (models.length === 0) return null;

  return (
    <div className="card">
      <div className="section-title">Model Usage</div>
      <div className="model-usage-row">
        {models.map((m) => (
          <div key={m.model} className="model-usage-card">
            <div className="model-name">
              {m.model.replace("deepseek-", "")}
            </div>
            <div className="model-stat">
              <span className="model-stat-label">Output</span>
              <span className="model-stat-value">{fmt(m.output_tokens)}</span>
            </div>
            <div className="model-stat">
              <span className="model-stat-label">Req</span>
              <span className="model-stat-value">{m.request_count}</span>
            </div>
            <div className="model-stat">
              <span className="model-stat-label">Cost</span>
              <span className="model-stat-value">
                ¥{m.cost.toFixed(2)}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
