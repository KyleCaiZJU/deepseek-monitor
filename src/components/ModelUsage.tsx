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
      <div className="section-title">{'模型用量'}</div>
      <div className="model-usage-row">
        {models.map((m) => (
          <div key={m.model} className="model-usage-card">
            <div className="model-name">
              {m.model.replace("deepseek-", "")}
            </div>
            <div className="model-stat">
              <span className="model-stat-label">{'输出'}</span>
              <span className="model-stat-value">{fmt(m.output_tokens)}</span>
            </div>
            <div className="model-stat">
              <span className="model-stat-label">{'请求'}</span>
              <span className="model-stat-value">{m.request_count}</span>
            </div>
            <div className="model-stat">
              <span className="model-stat-label">{'费用'}</span>
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
