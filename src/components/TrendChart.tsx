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
  output: "#d6d3d1",
  cache: "#0d9488",
};

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
      <div className="section-title">7-Day Token Trend</div>
      <ResponsiveContainer width="100%" height={140}>
        <BarChart data={data}>
          <XAxis
            dataKey="date"
            tick={{ fontSize: 10, fill: "#78716c", fontFamily: "var(--font-mono)" }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            tick={{ fontSize: 10, fill: "#78716c", fontFamily: "var(--font-mono)" }}
            axisLine={false}
            tickLine={false}
            tickFormatter={formatTokens}
            width={40}
          />
          <Tooltip
            contentStyle={{
              background: "#ffffff",
              border: "1px solid rgba(0,0,0,0.08)",
              borderRadius: "8px",
              fontSize: "12px",
              color: "#1a1a1a",
              fontFamily: "var(--font-mono)",
              boxShadow: "0 4px 12px rgba(0,0,0,0.06)",
            }}
            formatter={(value: any, name: any) => [
              formatTokens(Number(value) || 0),
              String(name) === "output"
                ? "Output"
                : String(name) === "cache"
                  ? "Cache Hit"
                  : "Cache Miss",
            ]}
          />
          <Bar dataKey="miss" stackId="a" fill={CHART_COLORS.miss} radius={[0, 0, 0, 0]} />
          <Bar dataKey="output" stackId="a" fill={CHART_COLORS.output} radius={[0, 0, 0, 0]} />
          <Bar dataKey="cache" stackId="a" fill={CHART_COLORS.cache} radius={[3, 3, 0, 0]} />
        </BarChart>
      </ResponsiveContainer>
      <div className="trend-total">Total {formatTokens(totalTokens)} tokens</div>
    </div>
  );
}
