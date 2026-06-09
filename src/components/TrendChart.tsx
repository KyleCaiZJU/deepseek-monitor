import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import type { DayPoint } from "../store";

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(0);
}

interface Props {
  trend: DayPoint[];
}

const CHART_COLORS = {
  miss: "#fca5a5",
  output: "#93c5fd",
  cache: "#2563eb",
};

function CustomTooltip({ active, payload, trend }: any) {
  if (!active || !payload?.length) return null;

  const dateKey = payload[0]?.payload?.date;
  const day: DayPoint | undefined = trend.find(
    (d: DayPoint) => d.date.slice(5) === dateKey
  );

  if (!day) return null;

  const total = day.cache_hit_tokens + day.cache_miss_tokens;
  const hitRate = total > 0 ? (day.cache_hit_tokens / total) * 100 : 0;

  return (
    <div
      style={{
        background: "#ffffff",
        border: "1px solid rgba(0,0,0,0.08)",
        borderRadius: "8px",
        padding: "10px 12px",
        fontSize: "12px",
        color: "#1a2332",
        fontFamily: "var(--font-mono), monospace",
        boxShadow: "0 4px 12px rgba(0,0,0,0.06)",
        lineHeight: 1.8,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4, color: "#2563eb" }}>
        {day.date}
      </div>
      <div>输出: {formatTokens(day.output_tokens)}</div>
      <div>缓存命中: {formatTokens(day.cache_hit_tokens)}</div>
      <div>缓存未命中: {formatTokens(day.cache_miss_tokens)}</div>
      <div style={{ color: "#2563eb", fontWeight: 500 }}>
        命中率: {hitRate.toFixed(1)}%
      </div>
      {day.cost > 0 && (
        <div style={{ color: "#ea580c", fontWeight: 500 }}>
          花费: ¥{day.cost.toFixed(2)}
        </div>
      )}
    </div>
  );
}

export default function TrendChart({ trend }: Props) {
  const data = trend.map((d) => ({
    date: d.date.slice(5),
    output: d.output_tokens,
    cache: d.cache_hit_tokens,
    miss: d.cache_miss_tokens,
  }));

  const totalTokens = trend.reduce(
    (s, d) =>
      s + d.output_tokens + d.cache_hit_tokens + d.cache_miss_tokens,
    0
  );

  return (
    <div className="card trend-chart">
      <div className="section-title">{'近7天消耗趋势'}</div>
      <ResponsiveContainer width="100%" height={140}>
        <BarChart data={data}>
          <XAxis
            dataKey="date"
            tick={{ fontSize: 10, fill: "#5c6f82", fontFamily: "var(--font-mono)" }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            tick={{ fontSize: 10, fill: "#5c6f82", fontFamily: "var(--font-mono)" }}
            axisLine={false}
            tickLine={false}
            tickFormatter={formatTokens}
            width={40}
          />
          <Tooltip content={<CustomTooltip trend={trend} />} />
          <Bar dataKey="miss" stackId="a" fill={CHART_COLORS.miss} radius={[0, 0, 0, 0]} />
          <Bar dataKey="output" stackId="a" fill={CHART_COLORS.output} radius={[0, 0, 0, 0]} />
          <Bar dataKey="cache" stackId="a" fill={CHART_COLORS.cache} radius={[3, 3, 0, 0]} />
        </BarChart>
      </ResponsiveContainer>
      <div className="trend-total">{'合计'} {formatTokens(totalTokens)} tokens</div>
    </div>
  );
}
