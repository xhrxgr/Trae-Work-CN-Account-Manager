import { useState } from "react";
import * as api from "../api";
import type { AccountBrief } from "../types";

interface CreateInstanceModalProps {
  accounts: AccountBrief[];
  onClose: () => void;
  onCreated: () => void;
}

export function CreateInstanceModal({ accounts, onClose, onCreated }: CreateInstanceModalProps) {
  const [name, setName] = useState("");
  const [accountId, setAccountId] = useState("");
  const [creating, setCreating] = useState(false);

  const handleSubmit = async () => {
    if (!name.trim()) {
      alert("请输入实例名称");
      return;
    }
    setCreating(true);
    try {
      await api.createInstance(
        name.trim(),
        undefined,
        accountId || undefined
      );
      onCreated();
    } catch (err: any) {
      alert(err.message || "创建失败");
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <h2>创建实例</h2>

        <div className="form-group">
          <label>实例名称</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="如：工作实例、临时账号"
            autoFocus
          />
        </div>

        <div className="form-group">
          <label>绑定账号（可选）</label>
          <select
            value={accountId}
            onChange={(e) => setAccountId(e.target.value)}
          >
            <option value="">不绑定（首次启动手动登录）</option>
            {accounts.map((acc) => (
              <option key={acc.id} value={acc.id}>
                {acc.name} ({acc.email})
              </option>
            ))}
          </select>
        </div>

        <div className="form-info">
          <p>· 数据目录将自动生成（空目录启动）</p>
          <p>· 插件目录与其他实例共享</p>
        </div>

        <div className="modal-actions">
          <button className="btn-secondary" onClick={onClose} disabled={creating}>
            取消
          </button>
          <button className="btn-primary" onClick={handleSubmit} disabled={creating}>
            {creating ? "创建中..." : "创建"}
          </button>
        </div>
      </div>
    </div>
  );
}
