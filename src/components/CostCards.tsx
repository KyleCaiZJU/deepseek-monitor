interface Props {
  todayCost: number;
  monthCost: number;
}

export default function CostCards({ todayCost, monthCost }: Props) {
  return (
    <div className="cost-row">
      <div className="card cost-card">
        <div className="cost-label">Today (UTC)</div>
        <div className="cost-value">¥{todayCost.toFixed(2)}</div>
      </div>
      <div className="card cost-card">
        <div className="cost-label">This Month (UTC)</div>
        <div className="cost-value">¥{monthCost.toFixed(2)}</div>
      </div>
    </div>
  );
}
