import { useState, useEffect, useCallback } from "react";
import * as api from "../api";
import type { InstanceBrief, AccountBrief } from "../types";
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
    onConfirm: () => void;
  } | null>(null);

  const [renameModal, setRenameModal] = useState<{
    instanceId: string;
    name: string;
  } | null>(null);

  const [accountSelectModal, setAccountSelectModal] = useState<{
    instanceId: string;
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
      message: `确定删除实例 "${inst.name}" 吗？\n\n点击"确定"将删除实例配置和数据目录。`,
      deleteData: true,
      type: "danger",
      onConfirm: () => {
        const delData = confirmModal?.deleteData ?? false;
        api.deleteInstance(instanceId, delData)
          .then(() => loadInstances())
          .catch((err) => alert(err.message || "删除失败"));
        setConfirmModal(null);
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
        <button className="btn-primary" onClick={() => setShowCreateModal(true)}>
          + 创建实例
        </button>
      </div>

      {instances.length === 0 ? (
        <div className="empty-state">
          <p>暂无实例</p>
          <button className="btn-primary" onClick={() => setShowCreateModal(true)}>
            创建第一个实例
          </button>
        </div>
      ) : (
        <div className="instance-grid">
          {instances.map((inst) => (
            <InstanceCard
              key={inst.id}
              instance={inst}
              onLaunch={() => handleLaunch(inst.id)}
              onBindAccount={() => handleSwitchAccount(inst.id)}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, instanceId: inst.id });
              }}
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
          onDelete={() => {
            handleDelete(contextMenu.instanceId);
            setContextMenu(null);
          }}
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
                    <div>{acc.name}</div>
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
    </div>
  );
}
