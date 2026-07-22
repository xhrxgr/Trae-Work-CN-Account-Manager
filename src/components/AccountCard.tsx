interface AccountCardProps {
  account: {
    id: string;
    name: string;
    email: string;
    avatar_url: string;
    plan_type: string;
    created_at: number;
    is_current?: boolean;
    token_expired_at?: string | null;
    source?: string;
    note?: string | null;
  };
  selected: boolean;
  onSelect: (id: string) => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
}

export function AccountCard({ account, selected, onSelect, onContextMenu }: AccountCardProps) {
  const formatCreatedDate = (timestamp: number) => {
    if (!timestamp) return "-";
    const date = new Date(timestamp * 1000);
    const year = date.getFullYear();
    const month = date.getMonth() + 1;
    const day = date.getDate();
    return `${year}/${month}/${day}`;
  };

  const getTokenStatus = (): "normal" | "expiring" | "expired" | "unknown" => {
    if (!account.token_expired_at) return "unknown";
    const expiry = new Date(account.token_expired_at).getTime();
    if (isNaN(expiry)) return "unknown";
    const now = Date.now();
    if (expiry < now) return "expired";
    if (expiry - now < 3600000) return "expiring"; // < 1小时
    return "normal";
  };

  const tokenStatus = getTokenStatus();

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(account.email || account.name);
  };

  return (
    <div
      className={`account-card ${selected ? "selected" : ""} ${account.is_current ? "current" : ""}`}
      onClick={() => onSelect(account.id)}
      onContextMenu={(e) => onContextMenu(e, account.id)}
    >
      <div className="card-header">
        <div className="card-checkbox" onClick={(e) => e.stopPropagation()}>
          <input
            type="checkbox"
            checked={selected}
            onChange={() => onSelect(account.id)}
          />
        </div>

        <div className="card-avatar">
          {account.avatar_url ? (
            <img src={account.avatar_url} alt={account.name} />
          ) : (
            <div className="avatar-placeholder">
              {(account.email || account.name).charAt(0).toUpperCase()}
            </div>
          )}
        </div>

        <div className="card-info">
          <div className="card-email">
            <span className="email-text">{account.email || account.name}</span>
            <button
              className="copy-btn"
              onClick={handleCopy}
              title="复制邮箱"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
              </svg>
            </button>
          </div>
          <div className="card-name">
            {account.note ? (
              <span className="account-note" title={account.note}>{account.note}</span>
            ) : (
              <span>{account.name}</span>
            )}
          </div>
        </div>

        {tokenStatus !== "unknown" && (
          <div className={`card-status ${tokenStatus === "expired" ? "expired" : tokenStatus === "expiring" ? "expiring" : "normal"}`}>
            <span className="status-indicator"></span>
            {tokenStatus === "expired" ? "已过期" : tokenStatus === "expiring" ? "即将过期" : "正常"}
          </div>
        )}
      </div>

      <div className="card-tags">
        <span className="tag plan">{account.plan_type || "Free"}</span>
        {account.source === "browser" && (
          <span className="tag source-browser" title="通过浏览器登录添加">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="10"/>
              <line x1="2" y1="12" x2="22" y2="12"/>
              <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
            </svg>
            浏览器
          </span>
        )}
        {account.source === "local" && (
          <span className="tag source-local" title="从本地 TRAE Work CN 读取">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M12 2L2 7l10 5 10-5-10-5z"/>
              <path d="M2 17l10 5 10-5"/>
            </svg>
            本地
          </span>
        )}
        {account.source === "manual" && (
          <span className="tag source-manual" title="手动输入 Token 添加">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
              <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
            </svg>
            手动
          </span>
        )}
        {account.is_current && (
          <span className="tag current">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
              <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
            </svg>
            当前使用
          </span>
        )}
      </div>

      <div className="card-meta">
        <span className="meta-item">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="3" y="4" width="18" height="18" rx="2" ry="2"/>
            <line x1="16" y1="2" x2="16" y2="6"/>
            <line x1="8" y1="2" x2="8" y2="6"/>
            <line x1="3" y1="10" x2="21" y2="10"/>
          </svg>
          添加于 {formatCreatedDate(account.created_at)}
        </span>
      </div>

      <div className="card-footer">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>
        </svg>
        右键查看更多操作
      </div>
    </div>
  );
}
