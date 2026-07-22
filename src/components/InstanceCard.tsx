import { useState } from "react";
import type { InstanceBrief, InstanceAccountStatus } from "../types";

interface InstanceCardProps {
  instance: InstanceBrief;
  onLaunch: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  onRefreshStatus: (instanceId: string) => Promise<void>;
}

function formatDiskUsage(bytes: number): string {
  if (bytes === 0) return "计算中...";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let size = bytes;
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024;
    i++;
  }
  return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

/// 数据陈旧度阈值：超过 24 小时视为可能过期
const STALE_THRESHOLD_MS = 24 * 60 * 60 * 1000;

function formatSyncTime(ms: number): string {
  if (ms <= 0) return "未知";
  const date = new Date(ms);
  const now = Date.now();
  const diff = now - ms;
  if (diff < 60 * 1000) return "刚刚";
  if (diff < 60 * 60 * 1000) return `${Math.floor(diff / 60000)} 分钟前`;
  if (diff < 24 * 60 * 60 * 1000) return `${Math.floor(diff / 3600000)} 小时前`;
  return `${date.getMonth() + 1}/${date.getDate()} ${date.getHours()}:${String(date.getMinutes()).padStart(2, "0")}`;
}

function formatLaunchTime(seconds: number): string {
  if (seconds <= 0) return "从未启动";
  const date = new Date(seconds * 1000);
  const now = Date.now();
  const diff = now - seconds * 1000;
  if (diff < 60 * 1000) return "刚刚启动";
  if (diff < 60 * 60 * 1000) return `${Math.floor(diff / 60000)} 分钟前启动`;
  if (diff < 24 * 60 * 60 * 1000) return `${Math.floor(diff / 3600000)} 小时前启动`;
  return `${date.getMonth() + 1}/${date.getDate()} ${date.getHours()}:${String(date.getMinutes()).padStart(2, "0")} 启动`;
}

function formatResetTime(seconds: number): string {
  if (seconds <= 0) return "";
  const date = new Date(seconds * 1000);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")} ${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}

function AccountStatusBadge({
  status,
  isRefreshing,
  onRefresh,
}: {
  status: InstanceAccountStatus;
  isRefreshing: boolean;
  onRefresh: () => void;
}) {
  const isFree = status.identity_str === "Free" || status.identity_str === "";

  // 逆向 TRAE 源码发现（v1.0.21+）：
  // - "速通" = Express/Fast Pass，是对话的优先队列，有次数限制
  // - fast_request_per = 剩余速通次数（不是"月免费对话次数"）
  // - 免费版 fast_request_per=0 = 无速通额度（但用户仍可普通对话，由服务端控制）
  // - "本月对话额度已耗尽"提示来自服务端，前端无法提前预知

  // 用户反馈：免费版一律不显示 badge（不管速通次数），避免误读
  // 只对非 Free 身份（Pro 等）才显示状态徽章
  if (isFree) {
    return null;
  }

  let label: string;
  let cls: string = "account-status paid";
  let tooltip: string;

  if (status.is_from_api) {
    // 官方 API 实时数据
    tooltip = `官方API实时获取 · ${formatSyncTime(status.last_sync_ms)}`;
    const left = Math.floor(status.fast_request_left);
    const used = Math.floor(status.fast_request_used);
    if (status.fast_request_limit > 0) {
      // 有速通额度
      if (left <= 0) {
        label = `${status.identity_str} · 速通已用完`;
        cls = "account-status exhausted";
      } else {
        label = `${status.identity_str} · 速通 ${left}`;
      }
      tooltip += ` · 速通已用 ${used}/${status.fast_request_limit}`;
      if (status.reset_time > 0) {
        tooltip += ` · 重置: ${formatResetTime(status.reset_time)}`;
      }
    } else {
      // 非 Free 身份但 limit=0（如 Pro 无速通额度）
      label = status.identity_str;
    }
    // 额外礼包
    if (status.extra_fast_request_left > 0) {
      label += ` +${Math.floor(status.extra_fast_request_left)}`;
      tooltip += ` · 额外礼包: ${status.extra_package_name || "未知"} 剩余 ${Math.floor(status.extra_fast_request_left)}`;
    }
  } else {
    // 本地缓存（来自 storage.json 的 iCubeEntitlementInfo/iCubeServerData）
    const isStale = status.last_sync_ms === 0 || (Date.now() - status.last_sync_ms) > STALE_THRESHOLD_MS;
    tooltip = `本地缓存 · 上次同步: ${formatSyncTime(status.last_sync_ms)}${isStale ? "（数据可能过期）" : ""}`;
    label = status.identity_str;
    tooltip += " · 点击🔄从官方API获取实时数据";
  }

  return (
    <>
      <span className={cls} title={tooltip}>{label}</span>
      <button
        className="refresh-status-btn"
        onClick={onRefresh}
        disabled={isRefreshing}
        title={isRefreshing ? "正在刷新..." : "从官方API获取实时速通次数"}
      >
        {isRefreshing ? "⏳" : "🔄"}
      </button>
    </>
  );
}

export function InstanceCard({ instance, onLaunch, onContextMenu, onRefreshStatus }: InstanceCardProps) {
  const [refreshing, setRefreshing] = useState(false);

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await onRefreshStatus(instance.id);
    } finally {
      setRefreshing(false);
    }
  };

  return (
    <div
      className={`instance-card ${instance.is_default ? "default" : ""} ${instance.is_running ? "running" : ""}`}
      onContextMenu={onContextMenu}
    >
      <div className="instance-header">
        <div className="instance-name">
          {instance.name}
          {instance.is_default && <span className="default-badge">默认</span>}
          {instance.note && <span className="instance-note-badge" title={instance.note}>📝 {instance.note}</span>}
        </div>
        <div className={`instance-status ${instance.is_running ? "running" : "stopped"}`}>
          {instance.is_running ? "运行中" : "已停止"}
        </div>
      </div>

      <div className="instance-body">
        {instance.bound_account_id && instance.bound_account_name ? (
          <div className="instance-account">
            {instance.bound_account_avatar && (
              <img src={instance.bound_account_avatar} alt="" className="avatar" />
            )}
            <div>
              <div>
                {instance.bound_account_note
                  ? `${instance.bound_account_name} · ${instance.bound_account_note}`
                  : instance.bound_account_name}
              </div>
              <div className="muted">
                {instance.bound_account_email || "无邮箱"}
                {instance.account_status && (
                  <>
                    {" · "}
                    <AccountStatusBadge
                      status={instance.account_status}
                      isRefreshing={refreshing}
                      onRefresh={handleRefresh}
                    />
                  </>
                )}
              </div>
            </div>
          </div>
        ) : instance.account_status?.user_id ? (
          // 未绑定到本地账号，但 data-dir 的 storage.json 中有登录信息
          <div className="instance-account muted">
            未绑定（IDE 已登录 #{instance.account_status.user_id.slice(-6)}）
            {" · "}
            <AccountStatusBadge
              status={instance.account_status}
              isRefreshing={refreshing}
              onRefresh={handleRefresh}
            />
          </div>
        ) : (
          <div className="instance-account muted">
            未绑定账号
            {instance.account_status && (
              <>
                {" · "}
                <AccountStatusBadge
                  status={instance.account_status}
                  isRefreshing={refreshing}
                  onRefresh={handleRefresh}
                />
              </>
            )}
          </div>
        )}

        <div className="instance-disk">
          磁盘: {formatDiskUsage(instance.disk_usage)} · {formatLaunchTime(instance.last_launched_at)}
        </div>
      </div>

      <div className="instance-footer">
        <button className="btn-launch" onClick={onLaunch}>
          {instance.is_running ? "▶ 新开窗口" : "▶ 启动"}
        </button>
      </div>
    </div>
  );
}
