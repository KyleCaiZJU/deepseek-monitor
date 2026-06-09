interface Props {
  balance: number;
  available: boolean;
}

export default function BalanceCard({ balance, available }: Props) {
  return (
    <div className="card balance-card">
      <div className="balance-status">
        <span className={`balance-dot ${available ? "online" : "offline"}`} />
        <span>{available ? "Available" : "Offline"}</span>
      </div>
      <div className="balance-amount">
        <span className="balance-currency">¥</span>
        {balance.toFixed(2)}
      </div>
    </div>
  );
}
