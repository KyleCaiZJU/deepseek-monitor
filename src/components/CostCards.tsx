interface Props {
  todayCost: number;
  monthCost: number;
}

export default function CostCards({ todayCost, monthCost }: Props) {
  return (
    <div className="cost-row">
      <div className="card cost-card">
        <div className="cost-label">{'今日 (UTC)'}</div>
        <div className="cost-value">¥{todayCost.toFixed(2)}</div>
      </div>
      <div className="card cost-card">
        <div className="cost-label">{'本月 (UTC)'}</div>
        <div className="cost-value">¥{monthCost.toFixed(2)}</div>
      </div>
    </div>
  );
}
