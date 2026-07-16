import type { InstanceBrief } from "../types";

interface InstanceCardProps {
  instance: InstanceBrief;
  onLaunch: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}

function formatDiskUsage(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let size = bytes;
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024;
    i++;
  }
  return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function InstanceCard({ instance, onLaunch, onContextMenu }: InstanceCardProps) {
  return (
    <div
      className={`instance-card ${instance.is_default ? "default" : ""} ${instance.is_running ? "running" : ""}`}
      onContextMenu={onContextMenu}
    >
      <div className="instance-header">
        <div className="instance-name">
          {instance.name}
          {instance.is_default && <span className="default-badge">默认</span>}
        </div>
        <div className={`instance-status ${instance.is_running ? "running" : "stopped"}`}>
          {instance.is_running ? "运行中" : "已停止"}
        </div>
      </div>

      <div className="instance-body">
        {instance.bound_account_email ? (
          <div className="instance-account">
            {instance.bound_account_avatar && (
              <img src={instance.bound_account_avatar} alt="" className="avatar" />
            )}
            <div>
              <div>{instance.bound_account_name}</div>
              <div className="muted">{instance.bound_account_email}</div>
            </div>
          </div>
        ) : (
          <div className="instance-account muted">未绑定账号</div>
        )}

        <div className="instance-disk">
          磁盘: {formatDiskUsage(instance.disk_usage)}
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
