import { useState } from "react";
import * as api from "../api";
import type { AccountBrief } from "../types";

interface CreateInstanceModalProps {
  accounts: AccountBrief[];
  onClose: () => void;
  onCreated: () => void;
  onToast: (type: "success" | "error" | "warning" | "info", message: string) => void;
}

export function CreateInstanceModal({ accounts, onClose, onCreated, onToast }: CreateInstanceModalProps) {
  const [name, setName] = useState("");
  const [accountId, setAccountId] = useState("");
  const [creating, setCreating] = useState(false);

  const handleSubmit = async () => {
    if (!name.trim()) {
      onToast("warning", "请输入实例名称");
      return;
    }
    setCreating(true);
    try {
      await api.createInstance(
        name.trim(),
        undefined,
        accountId || undefined
      );
      onToast("success", "实例已创建");
      onCreated();
    } catch (err: any) {
      onToast("error", err.message || "创建失败");
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header-fixed">
          <h2>创建实例</h2>
          <button className="modal-close-btn" onClick={onClose}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="20" height="20">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
        <div className="modal-body-scrollable">
          <div className="form-section">
            <label className="form-label">实例名称</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="如：工作实例、临时账号"
              autoFocus
            />
          </div>
          <div className="form-section">
            <label className="form-label">
              绑定账号 <span className="optional">（可选）</span>
            </label>
            <select
              value={accountId}
              onChange={(e) => setAccountId(e.target.value)}
            >
              <option value="">不绑定（首次启动手动登录）</option>
              {accounts.map((acc) => (
                <option key={acc.id} value={acc.id}>
                  {acc.note ? `${acc.name} · ${acc.note}` : acc.name} ({acc.email})
                </option>
              ))}
            </select>
          </div>
          <div className="form-info">
            <p>· 数据目录将自动生成（空目录启动）</p>
            <p>· 插件目录与其他实例共享</p>
          </div>
        </div>
        <div className="modal-actions-fixed">
          <button type="button" onClick={onClose} disabled={creating}>取消</button>
          <button type="button" className="primary" onClick={handleSubmit} disabled={creating}>
            {creating ? "创建中..." : "创建"}
          </button>
        </div>
      </div>
    </div>
  );
}
