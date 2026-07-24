import { useState, useEffect, useCallback } from "react";
import * as api from "../api";
import type { InstanceBrief, AccountBrief, SafeCleanItem } from "../types";
import { InstanceCard } from "../components/InstanceCard";
import { CreateInstanceModal } from "../components/CreateInstanceModal";
import { ContextMenu } from "../components/ContextMenu";
import { ConfirmModal } from "../components/ConfirmModal";
import { useToast } from "../hooks/useToast";

interface InstancesProps {
  accounts: AccountBrief[];
  onRefreshAccounts: () => void;
}

export function Instances({ accounts, onRefreshAccounts }: InstancesProps) {
  const [instances, setInstances] = useState<InstanceBrief[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const { addToast } = useToast();

  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    instanceId: string;
  } | null>(null);

  const [confirmModal, setConfirmModal] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    deleteData: boolean;
    type: "danger" | "warning" | "info";
    onConfirm: () => void | Promise<void>;
    isProcessing?: boolean;
  } | null>(null);

  const [renameModal, setRenameModal] = useState<{
    instanceId: string;
    name: string;
  } | null>(null);

  const [noteModal, setNoteModal] = useState<{
    instanceId: string;
    instanceName: string;
    note: string;
  } | null>(null);

  const [accountSelectModal, setAccountSelectModal] = useState<{
    instanceId: string;
  } | null>(null);

  // 安全清理弹窗（v1.0.26+）
  const [safeCleanModal, setSafeCleanModal] = useState<{
    instanceId: string;
    instanceName: string;
    items: SafeCleanItem[];
    selected: Set<string>;
    loading: boolean;
    cleaning: boolean;
  } | null>(null);

  const loadInstances = useCallback(async () => {
    try {
      const list = await api.listInstances();
      setInstances(list);
    } catch (err) {
      console.error("加载实例失败:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadInstances();
    // 每 15 秒刷新运行状态（后端有 5 分钟磁盘占用缓存，不会重复计算）
    const interval = setInterval(loadInstances, 15000);
    return () => clearInterval(interval);
  }, [loadInstances]);

  // 启动实例（先弹确认框，避免误触直接打开 TRAE）
  const handleLaunch = (instanceId: string) => {
    const inst = instances.find((i) => i.id === instanceId);
    if (!inst) return;
    const actionDesc = inst.is_running ? "新开窗口" : "启动";
    setConfirmModal({
      isOpen: true,
      title: `${actionDesc}实例`,
      message: `确定要${actionDesc}实例 "${inst.name}" 吗？`,
      deleteData: false,
      type: "info",
      onConfirm: async () => {
        setConfirmModal(null);
        try {
          const wasRunning = await api.launchInstance(instanceId);
          if (wasRunning) {
            addToast("info", "该实例已在运行，已启动新进程");
          } else {
            addToast("success", "实例已启动");
          }
          await loadInstances();
        } catch (err: any) {
          addToast("error", err.message || "启动失败");
        }
      },
    });
  };

  // 刷新单个实例的账号状态（调用官方 API 获取实时额度）
  const handleRefreshStatus = async (instanceId: string) => {
    try {
      const status = await api.refreshInstanceStatus(instanceId);
      if (status) {
        // 更新对应实例的 account_status（不重新加载整个列表）
        setInstances((prev) =>
          prev.map((inst) =>
            inst.id === instanceId ? { ...inst, account_status: status } : inst
          )
        );
        addToast("success", "已从官方 API 获取实时额度");
      } else {
        addToast("info", "该实例未登录或实例不存在");
      }
    } catch (err: any) {
      addToast("error", err.message || "刷新失败");
    }
  };

  // 打开数据目录
  const handleOpenDataDir = async (instanceId: string) => {
    try {
      await api.openInstanceDataDir(instanceId);
    } catch (err: any) {
      alert(err.message || "打开失败");
    }
  };

  // 创建快捷方式
  const handleCreateShortcut = async (instanceId: string) => {
    try {
      const path = await api.createInstanceShortcut(instanceId);
      alert(`快捷方式已创建: ${path}`);
    } catch (err: any) {
      alert(err.message || "创建失败");
    }
  };

  // 删除实例
  const handleDelete = (instanceId: string) => {
    const inst = instances.find((i) => i.id === instanceId);
    if (!inst) return;

    setConfirmModal({
      isOpen: true,
      title: "删除实例",
      message: `确定删除实例 "${inst.name}" 吗？\n\n将永久删除实例配置和全部数据目录（无法恢复）。\n如果实例正在运行，请先关闭再删除，否则会因文件占用导致删除失败。\n\n删除大目录可能需要几十秒，期间按钮会显示"处理中..."，请耐心等待。`,
      deleteData: true,
      type: "danger",
      isProcessing: false,
      onConfirm: async () => {
        // 标记处理中：按钮变灰 + 显示"处理中..." + 禁用取消/遮罩关闭
        setConfirmModal((prev) => (prev ? { ...prev, isProcessing: true } : prev));
        try {
          await api.deleteInstance(instanceId, true);
          addToast("success", `实例 "${inst.name}" 已删除`);
          setConfirmModal(null);
          await loadInstances();
        } catch (err: any) {
          addToast("error", err.message || "删除失败");
          // 恢复按钮，让用户可以重试或取消
          setConfirmModal((prev) => (prev ? { ...prev, isProcessing: false } : prev));
        }
      },
    });
  };

  // 重命名
  const handleRename = (instanceId: string) => {
    const inst = instances.find((i) => i.id === instanceId);
    if (!inst) return;
    setRenameModal({ instanceId, name: inst.name });
  };

  const handleRenameSubmit = async () => {
    if (!renameModal) return;
    try {
      await api.renameInstance(renameModal.instanceId, renameModal.name);
      setRenameModal(null);
      await loadInstances();
    } catch (err: any) {
      alert(err.message || "重命名失败");
    }
  };

  // 编辑备注
  const handleEditNote = (instanceId: string) => {
    const inst = instances.find((i) => i.id === instanceId);
    if (!inst) return;
    setNoteModal({ instanceId, instanceName: inst.name, note: inst.note ?? "" });
  };

  const handleNoteSubmit = async () => {
    if (!noteModal) return;
    try {
      const trimmed = noteModal.note.trim();
      const note = trimmed.length > 0 ? trimmed : null;
      await api.updateInstanceNote(noteModal.instanceId, note);
      // 局部更新，不重新加载整个列表
      setInstances((prev) =>
        prev.map((i) =>
          i.id === noteModal.instanceId ? { ...i, note: note ?? null } : i
        )
      );
      addToast("success", "备注已保存");
      setNoteModal(null);
    } catch (err: any) {
      addToast("error", err.message || "备注更新失败");
    }
  };

  // 安全清理（v1.0.26+）
  const handleSafeClean = async (instanceId: string) => {
    const inst = instances.find((i) => i.id === instanceId);
    if (!inst) return;
    setSafeCleanModal({
      instanceId,
      instanceName: inst.name,
      items: [],
      selected: new Set(),
      loading: true,
      cleaning: false,
    });
    try {
      const items = await api.getSafeCleanItems(instanceId);
      // 默认勾选全部项（全部为 100% 安全可删）
      const selected = new Set(items.map((it) => it.key));
      setSafeCleanModal({
        instanceId,
        instanceName: inst.name,
        items,
        selected,
        loading: false,
        cleaning: false,
      });
    } catch (err: any) {
      addToast("error", err.message || "获取清理项失败");
      setSafeCleanModal(null);
    }
  };

  const handleSafeCleanToggle = (key: string) => {
    setSafeCleanModal((prev) => {
      if (!prev) return prev;
      const next = new Set(prev.selected);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return { ...prev, selected: next };
    });
  };

  const handleSafeCleanSubmit = async () => {
    if (!safeCleanModal) return;
    const keys = Array.from(safeCleanModal.selected);
    if (keys.length === 0) {
      addToast("warning", "请至少勾选一项");
      return;
    }
    setSafeCleanModal((prev) => (prev ? { ...prev, cleaning: true } : prev));
    try {
      const freed = await api.safeCleanInstance(safeCleanModal.instanceId, keys);
      addToast("success", `已清理，释放 ${(freed / 1024 / 1024).toFixed(1)} MB`);
      setSafeCleanModal(null);
      await loadInstances();
    } catch (err: any) {
      addToast("error", err.message || "清理失败");
      setSafeCleanModal((prev) => (prev ? { ...prev, cleaning: false } : prev));
    }
  };

  // 切换账号
  const handleSwitchAccount = (instanceId: string) => {
    setAccountSelectModal({ instanceId });
  };

  const handleBindAccount = async (accountId: string | null) => {
    if (!accountSelectModal) return;
    try {
      await api.bindAccountToInstance(accountSelectModal.instanceId, accountId || undefined);
      setAccountSelectModal(null);
      await loadInstances();
      onRefreshAccounts();
    } catch (err: any) {
      alert(err.message || "绑定失败");
    }
  };

  if (loading) {
    return <div className="loading">加载中...</div>;
  }

  return (
    <div className="instances-page">
      <div className="page-header">
        <h1>实例管理</h1>
        <button className="add-btn" onClick={() => setShowCreateModal(true)}>
          <span>+</span> 创建实例
        </button>
      </div>

      {instances.length === 0 ? (
        <div className="empty-state">
          <p>暂无实例</p>
          <button className="add-btn" onClick={() => setShowCreateModal(true)}>
            <span>+</span> 创建第一个实例
          </button>
        </div>
      ) : (
        <div className="instance-grid">
          {instances.map((inst) => (
            <InstanceCard
              key={inst.id}
              instance={inst}
              onLaunch={() => handleLaunch(inst.id)}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, instanceId: inst.id });
              }}
              onRefreshStatus={handleRefreshStatus}
            />
          ))}
        </div>
      )}

      {showCreateModal && (
        <CreateInstanceModal
          accounts={accounts}
          onClose={() => setShowCreateModal(false)}
          onCreated={() => {
            setShowCreateModal(false);
            loadInstances();
          }}
        />
      )}

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          contextType="instance"
          onClose={() => setContextMenu(null)}
          onViewDetail={() => {
            handleLaunch(contextMenu.instanceId);
            setContextMenu(null);
          }}
          onUpdateToken={() => {
            handleSwitchAccount(contextMenu.instanceId);
            setContextMenu(null);
          }}
          onCopyToken={() => {
            handleOpenDataDir(contextMenu.instanceId);
            setContextMenu(null);
          }}
          onSwitchAccount={() => {
            handleCreateShortcut(contextMenu.instanceId);
            setContextMenu(null);
          }}
          onLaunchMulti={() => {
            handleRename(contextMenu.instanceId);
            setContextMenu(null);
          }}
          onEditNote={() => {
            handleEditNote(contextMenu.instanceId);
            setContextMenu(null);
          }}
          onDelete={() => {
            handleDelete(contextMenu.instanceId);
            setContextMenu(null);
          }}
          onSafeClean={() => {
            handleSafeClean(contextMenu.instanceId);
            setContextMenu(null);
          }}
          isDefaultInstance={
            instances.find((i) => i.id === contextMenu.instanceId)?.is_default ?? false
          }
          isCurrent={false}
        />
      )}

      {confirmModal && (
        <ConfirmModal
          isOpen={confirmModal.isOpen}
          title={confirmModal.title}
          message={confirmModal.message}
          type={confirmModal.type}
          onConfirm={confirmModal.onConfirm}
          onCancel={() => setConfirmModal(null)}
          isProcessing={confirmModal.isProcessing}
        />
      )}

      {renameModal && (
        <div className="modal-overlay" onClick={() => setRenameModal(null)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <h2>重命名实例</h2>
            <input
              type="text"
              value={renameModal.name}
              onChange={(e) => setRenameModal({ ...renameModal, name: e.target.value })}
              autoFocus
            />
            <div className="modal-actions">
              <button className="btn-secondary" onClick={() => setRenameModal(null)}>
                取消
              </button>
              <button className="btn-primary" onClick={handleRenameSubmit}>
                确定
              </button>
            </div>
          </div>
        </div>
      )}

      {noteModal && (
        <div className="modal-overlay" onClick={() => setNoteModal(null)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header-fixed">
              <h2>编辑备注</h2>
              <button className="modal-close-btn" onClick={() => setNoteModal(null)}>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="20" height="20">
                  <line x1="18" y1="6" x2="6" y2="18"/>
                  <line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            </div>
            <div className="modal-body-scrollable">
              <div className="form-section">
                <label className="form-label">
                  备注 <span className="optional">（可选）</span>
                </label>
                <textarea
                  value={noteModal.note}
                  onChange={(e) => setNoteModal({ ...noteModal, note: e.target.value })}
                  placeholder="输入备注内容（留空则清除备注）..."
                  rows={4}
                  autoFocus
                />
              </div>
            </div>
            <div className="modal-actions-fixed">
              <button type="button" onClick={() => setNoteModal(null)}>取消</button>
              <button type="button" className="primary" onClick={handleNoteSubmit}>保存备注</button>
            </div>
          </div>
        </div>
      )}

      {accountSelectModal && (
        <div className="modal-overlay" onClick={() => setAccountSelectModal(null)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <h2>选择账号</h2>
            <div className="account-list">
              {accounts.map((acc) => (
                <div
                  key={acc.id}
                  className="account-select-item"
                  onClick={() => handleBindAccount(acc.id)}
                >
                  {acc.avatar_url && (
                    <img src={acc.avatar_url} alt="" className="avatar" />
                  )}
                  <div>
                    <div>
                      {acc.note
                        ? `${acc.name} · ${acc.note}`
                        : acc.name}
                    </div>
                    <div className="muted">{acc.email}</div>
                  </div>
                </div>
              ))}
              <div
                className="account-select-item"
                onClick={() => handleBindAccount(null)}
              >
                <div className="muted">不绑定（首次启动手动登录）</div>
              </div>
            </div>
          </div>
        </div>
      )}

      {safeCleanModal && (
        <div className="modal-overlay" onClick={safeCleanModal.cleaning ? undefined : () => setSafeCleanModal(null)}>
          <div className="modal-content safe-clean-modal" onClick={(e) => e.stopPropagation()}>
            <h2>安全清理 - {safeCleanModal.instanceName}</h2>
            <p className="safe-clean-hint">
              以下均为可安全删除的缓存、日志、崩溃转储，删除后下次启动自动重建。
              不影响登录信息、用户设置、插件数据、聊天记录。
              <br />
              <strong>实例运行中时无法清理，请先关闭实例。</strong>
            </p>
            {safeCleanModal.loading ? (
              <p>正在扫描可清理项...</p>
            ) : (
              <>
                <div className="safe-clean-list">
                  {safeCleanModal.items.map((item) => (
                    <label key={item.key} className="safe-clean-item" title={item.description}>
                      <div className="safe-clean-item-main">
                        <input
                          type="checkbox"
                          checked={safeCleanModal.selected.has(item.key)}
                          onChange={() => handleSafeCleanToggle(item.key)}
                          disabled={safeCleanModal.cleaning}
                        />
                        <div className="safe-clean-item-info">
                          <div className="safe-clean-item-label">
                            {item.label}
                            <span className="safe-clean-item-path">{item.path}</span>
                          </div>
                          <div className="safe-clean-item-desc">{item.description}</div>
                        </div>
                      </div>
                      <div className="safe-clean-item-size">
                        {item.size_bytes > 0
                          ? `${(item.size_bytes / 1024 / 1024).toFixed(1)} MB`
                          : "0 B"}
                      </div>
                    </label>
                  ))}
                </div>
                <div className="safe-clean-total">
                  预计释放：
                  {(() => {
                    const total = safeCleanModal.items
                      .filter((it) => safeCleanModal.selected.has(it.key))
                      .reduce((sum, it) => sum + it.size_bytes, 0);
                    return `${(total / 1024 / 1024).toFixed(1)} MB`;
                  })()}
                </div>
              </>
            )}
            <div className="modal-actions">
              <button
                className="btn-secondary"
                onClick={() => setSafeCleanModal(null)}
                disabled={safeCleanModal.cleaning}
              >
                取消
              </button>
              <button
                className="btn-primary"
                onClick={handleSafeCleanSubmit}
                disabled={safeCleanModal.loading || safeCleanModal.cleaning || safeCleanModal.selected.size === 0}
              >
                {safeCleanModal.cleaning ? "清理中..." : "删除选中项"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
