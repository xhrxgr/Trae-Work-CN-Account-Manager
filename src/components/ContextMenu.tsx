import { useEffect, useRef } from "react";

interface ContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  onViewDetail: () => void;
  onUpdateToken: () => void;
  onCopyToken: () => void;
  onSwitchAccount: () => void;
  onLaunchMulti?: () => void;
  onDelete: () => void;
  onEditNote?: () => void;
  isCurrent?: boolean;
}

export function ContextMenu({
  x,
  y,
  onClose,
  onViewDetail,
  onUpdateToken,
  onCopyToken,
  onSwitchAccount,
  onLaunchMulti,
  onDelete,
  onEditNote,
  isCurrent = false,
}: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // 调整菜单位置，防止超出屏幕
    if (menuRef.current) {
      const menu = menuRef.current;
      const rect = menu.getBoundingClientRect();

      if (rect.right > window.innerWidth) {
        menu.style.left = `${x - rect.width}px`;
      }
      if (rect.bottom > window.innerHeight) {
        menu.style.top = `${y - rect.height}px`;
      }
    }
  }, [x, y]);

  return (
    <>
      <div className="context-menu-overlay" onClick={onClose} />
      <div
        ref={menuRef}
        className="context-menu"
        style={{ left: x, top: y }}
      >
        <div className="context-menu-item" onClick={onViewDetail}>
          <span className="icon">👁</span>
          查看详情
        </div>
        <div className="context-menu-item" onClick={onUpdateToken}>
          <span className="icon">🔐</span>
          更新 Token
        </div>
        <div className="context-menu-item" onClick={onCopyToken}>
          <span className="icon">🔑</span>
          复制 Token
        </div>
        {onEditNote && (
          <div className="context-menu-item" onClick={onEditNote}>
            <span className="icon">📝</span>
            编辑备注
          </div>
        )}
        <div
          className={`context-menu-item ${isCurrent ? "disabled" : ""}`}
          onClick={isCurrent ? undefined : onSwitchAccount}
          title={isCurrent ? "当前已是此账号" : "切换账号（关闭当前实例后启动）"}
        >
          <span className="icon">{isCurrent ? "✓" : "🔀"}</span>
          {isCurrent ? "当前使用中" : "切换账号"}
        </div>
        {onLaunchMulti && (
          <div
            className="context-menu-item"
            onClick={onLaunchMulti}
            title="多开：启动独立实例，不影响当前实例"
          >
            <span className="icon">🚀</span>
            多开实例
          </div>
        )}
        <div className="context-menu-divider" />
        <div className="context-menu-item danger" onClick={onDelete}>
          <span className="icon">🗑</span>
          删除账号
        </div>
      </div>
    </>
  );
}
