import { useEffect, useRef } from "react";

// ContextMenu 通过「槽位」语义复用同一组件渲染账号/实例两种右键菜单。
// prop 名不绑定具体动作语义，调用方按 contextType 传入对应 handler，
// 组件内根据 contextType 选择显示文本和图标。
//
// 槽位语义映射表：
// | prop             | account 模式        | instance 模式       |
// |------------------|---------------------|---------------------|
// | onPrimaryAction  | 查看详情 👁          | 启动实例 👁          |
// | onSecondaryAction| 更新 Token 🔐       | 绑定账号 🔗          |
// | onTertiaryAction | 复制 Token 🔑       | 打开数据目录 📂      |
// | onQuaternaryAction| 切换账号 🔀        | 创建快捷方式 🖥️     |
// | onExtraAction    | 多开实例 🚀         | 重命名 ✏️            |
interface ContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  onPrimaryAction: () => void;
  onSecondaryAction: () => void;
  onTertiaryAction: () => void;
  onQuaternaryAction: () => void;
  onExtraAction?: () => void;
  onDelete: () => void;
  onEditNote?: () => void;
  onSafeClean?: () => void;
  isCurrent?: boolean;
  /// 是否为默认实例（默认实例隐藏删除项）
  isDefaultInstance?: boolean;
  contextType?: "account" | "instance";
}

export function ContextMenu({
  x,
  y,
  onClose,
  onPrimaryAction,
  onSecondaryAction,
  onTertiaryAction,
  onQuaternaryAction,
  onExtraAction,
  onDelete,
  onEditNote,
  onSafeClean,
  isCurrent = false,
  isDefaultInstance = false,
  contextType = "account",
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
        <div className="context-menu-item" onClick={onPrimaryAction}>
          <span className="icon">👁</span>
          {contextType === "instance" ? "启动实例" : "查看详情"}
        </div>
        <div className="context-menu-item" onClick={onSecondaryAction}>
          <span className="icon">{contextType === "instance" ? "🔗" : "🔐"}</span>
          {contextType === "instance" ? "绑定账号" : "更新 Token"}
        </div>
        <div className="context-menu-item" onClick={onTertiaryAction}>
          <span className="icon">{contextType === "instance" ? "📂" : "🔑"}</span>
          {contextType === "instance" ? "打开数据目录" : "复制 Token"}
        </div>
        {onEditNote && (
          <div className="context-menu-item" onClick={onEditNote}>
            <span className="icon">📝</span>
            编辑备注
          </div>
        )}
        <div
          className={`context-menu-item ${isCurrent ? "disabled" : ""}`}
          onClick={isCurrent ? undefined : onQuaternaryAction}
          title={isCurrent ? "当前已是此账号" : contextType === "instance" ? "为实例创建桌面快捷方式" : "切换账号（关闭当前实例后启动）"}
        >
          <span className="icon">{isCurrent ? "✓" : contextType === "instance" ? "🖥️" : "🔀"}</span>
          {isCurrent ? "当前使用中" : contextType === "instance" ? "创建快捷方式" : "切换账号"}
        </div>
        {onExtraAction && (
          <div
            className="context-menu-item"
            onClick={onExtraAction}
            title={contextType === "instance" ? "重命名实例" : "多开：启动独立实例，不影响当前实例"}
          >
            <span className="icon">{contextType === "instance" ? "✏️" : "🚀"}</span>
            {contextType === "instance" ? "重命名" : "多开实例"}
          </div>
        )}
        {onSafeClean && contextType === "instance" && (
          <div className="context-menu-item" onClick={onSafeClean} title="清理缓存、日志、崩溃转储等可安全删除的文件">
            <span className="icon">🧹</span>
            安全清理
          </div>
        )}
        <div className="context-menu-divider" />
        {/* 默认实例不可删除：隐藏删除项 */}
        {!(contextType === "instance" && isDefaultInstance) && (
          <div className="context-menu-item danger" onClick={onDelete}>
            <span className="icon">🗑</span>
            {contextType === "instance" ? "删除实例" : "删除账号"}
          </div>
        )}
      </div>
    </>
  );
}
