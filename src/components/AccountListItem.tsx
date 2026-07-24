interface AccountListItemProps {
  account: {
    id: string;
    name: string;
    email: string;
    avatar_url: string;
    plan_type: string;
    created_at: number;
    token_expired_at?: string | null;
    source?: string;
    note?: string | null;
  };
  selected: boolean;
  onSelect: (id: string) => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
}

/// 判断 name 是否是 "用户xxx" 占位形式（无可读性）
function isListUserIdPlaceholder(name: string): boolean {
  return /^用户\d+$/.test(name);
}

/// 主标题（与 AccountCard 策略保持一致，v1.0.30 修复空标题）：
/// 1) 人类可读的 name → 2) note → 3) email → 4) 兜底用占位 name
function getListDisplayName(acc: AccountListItemProps["account"]): string {
  if (acc.name && !isListUserIdPlaceholder(acc.name)) {
    return acc.name;
  }
  if (acc.note) {
    return acc.note;
  }
  if (acc.email) {
    return acc.email;
  }
  // 兜底：用占位 name，总比空标题好
  return acc.name || "";
}

/// 副标题：与主标题**不同**才显示，否则用账号来源标签
/// v1.0.30: 去掉 #hex-id 后缀（UUID 后 6 位像颜色值，用户反馈看不懂）
function getListSubtitle(acc: AccountListItemProps["account"]): string {
  const main = getListDisplayName(acc);
  // 1) email
  if (acc.email && acc.email !== main) {
    return acc.email;
  }
  // 2) name 是占位形式时显示 "用户xxx"
  if (acc.name && isListUserIdPlaceholder(acc.name) && acc.name !== main) {
    return acc.name;
  }
  // 3) 账号来源标签
  if (acc.source === "browser") return "浏览器登录";
  if (acc.source === "local") return "本地读取";
  if (acc.source === "manual") return "手动添加";
  return "";
}

export function AccountListItem({ account, selected, onSelect, onContextMenu }: AccountListItemProps) {
  const formatCreatedDate = (timestamp: number) => {
    if (!timestamp) return "-";
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diffTime = Math.abs(now.getTime() - date.getTime());
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return "今天";
    if (diffDays === 1) return "昨天";
    if (diffDays < 7) return `${diffDays}天前`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)}周前`;
    if (diffDays < 365) return `${Math.floor(diffDays / 30)}个月前`;
    return `${Math.floor(diffDays / 365)}年前`;
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

  return (
    <div
      className={`account-list-item ${selected ? "selected" : ""}`}
      onClick={() => onSelect(account.id)}
      onContextMenu={(e) => onContextMenu(e, account.id)}
    >
      <div className="list-item-checkbox" onClick={(e) => e.stopPropagation()}>
        <input
          type="checkbox"
          checked={selected}
          onChange={() => onSelect(account.id)}
        />
      </div>

      <div className="list-item-avatar">
        {account.avatar_url ? (
          <img src={account.avatar_url} alt={account.name} />
        ) : (
          <div className="avatar-placeholder">
            {(getListDisplayName(account) || account.name).charAt(0).toUpperCase()}
          </div>
        )}
      </div>

      <div className="list-item-info">
        <span className="list-item-email">{getListDisplayName(account)}</span>
        <span className="list-item-id">{getListSubtitle(account)}</span>
      </div>

      <div className="list-item-plan">
        <span className="plan-badge">{account.plan_type || "Free"}</span>
        {account.source === "browser" && <span className="extra-badge">浏览器</span>}
        {account.source === "local" && <span className="extra-badge">本地</span>}
      </div>

      <div className="list-item-reset">
        <span className="reset-label">添加时间</span>
        <span className="reset-date">{formatCreatedDate(account.created_at)}</span>
      </div>

      <div className="list-item-status">
        <span className={`status-dot ${tokenStatus === "expired" ? "expired" : tokenStatus === "expiring" ? "expiring" : tokenStatus === "unknown" ? "unknown" : "normal"}`}></span>
        <span>{tokenStatus === "expired" ? "过期" : tokenStatus === "expiring" ? "即将过期" : tokenStatus === "unknown" ? "未知" : "正常"}</span>
      </div>

      <div className="list-item-actions">
        <button
          className="action-btn"
          title="更多操作"
          onClick={(e) => {
            e.stopPropagation();
            onContextMenu(e, account.id);
          }}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <circle cx="12" cy="5" r="2"/>
            <circle cx="12" cy="12" r="2"/>
            <circle cx="12" cy="19" r="2"/>
          </svg>
        </button>
      </div>
    </div>
  );
}
