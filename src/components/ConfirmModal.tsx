interface ConfirmModalProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  type?: "danger" | "warning" | "info";
  onConfirm: () => void | Promise<void>;
  onCancel: () => void;
  /// 处理中状态：确认按钮显示"处理中..."并禁用，取消按钮禁用，遮罩点击不关闭
  /// 用于耗时操作（如删除大目录）期间防止重复点击和误关闭
  isProcessing?: boolean;
}

export function ConfirmModal({
  isOpen,
  title,
  message,
  confirmText = "确定",
  cancelText = "取消",
  type = "info",
  onConfirm,
  onCancel,
  isProcessing = false,
}: ConfirmModalProps) {
  if (!isOpen) return null;

  const icons = {
    danger: "🗑️",
    warning: "⚠️",
    info: "ℹ️",
  };

  return (
    <div className="modal-overlay" onClick={isProcessing ? undefined : onCancel}>
      <div className={`confirm-modal confirm-${type}`} onClick={(e) => e.stopPropagation()}>
        <div className="confirm-icon">{isProcessing ? "⏳" : icons[type]}</div>
        <h3 className="confirm-title">{title}</h3>
        <p className="confirm-message">{message}</p>
        <div className="confirm-actions">
          <button className="confirm-btn cancel" onClick={onCancel} disabled={isProcessing}>
            {cancelText}
          </button>
          <button
            className={`confirm-btn ${type}`}
            onClick={onConfirm}
            disabled={isProcessing}
          >
            {isProcessing ? "处理中..." : confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
