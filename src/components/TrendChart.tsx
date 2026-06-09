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
            tick={{ fontSize: 10, fill: "#a1a1aa" }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            tick={{ fontSize: 10, fill: "#a1a1aa" }}
            axisLine={false}
            tickLine={false}
            tickFormatter={formatTokens}
          />
          <Tooltip
            contentStyle={{
              background: "rgba(24,24,30,0.95)",
              border: "1px solid rgba(255,255,255,0.1)",
              borderRadius: "8px",
              fontSize: "12px",
              color: "#e4e4e7",
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
          <Bar dataKey="miss" stackId="a" fill="rgba(224,85,106,0.5)" />
          <Bar dataKey="output" stackId="a" fill="rgba(255,255,255,0.3)" />
          <Bar dataKey="cache" stackId="a" fill="rgba(29,158,117,0.5)" />
        </BarChart>
      </ResponsiveContainer>
      <div className="trend-total">Total: {formatTokens(totalTokens)} tokens</div>
    </div>
  );
}
