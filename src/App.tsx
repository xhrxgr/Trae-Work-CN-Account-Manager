import { useState, useEffect, useCallback } from "react";
import { Sidebar } from "./components/Sidebar";
import { AccountCard } from "./components/AccountCard";
import { AccountListItem } from "./components/AccountListItem";
import { AddAccountModal } from "./components/AddAccountModal";
import { ContextMenu } from "./components/ContextMenu";
import { DetailModal } from "./components/DetailModal";
import { Toast } from "./components/Toast";
import { ConfirmModal } from "./components/ConfirmModal";
import { InfoModal } from "./components/InfoModal";
import { UpdateTokenModal } from "./components/UpdateTokenModal";
import { Settings } from "./pages/Settings";
import { About } from "./pages/About";
import { useToast } from "./hooks/useToast";
import * as api from "./api";
import type { AccountBrief } from "./types";
import { Instances } from "./pages/Instances";
import "./App.css";

type ViewMode = "grid" | "list";

function App() {
  const [accounts, setAccounts] = useState<AccountBrief[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [showAddModal, setShowAddModal] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [currentPage, setCurrentPage] = useState("instances");
  const [viewMode, setViewMode] = useState<ViewMode>("grid");

  // 使用自定义 Toast hook
  const { toasts, addToast, removeToast } = useToast();

  // 确认弹窗状态
  const [confirmModal, setConfirmModal] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    type: "danger" | "warning" | "info";
    onConfirm: () => void;
  } | null>(null);

  // 右键菜单状态
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    accountId: string;
  } | null>(null);

  // 详情弹窗状态
  const [detailAccount, setDetailAccount] = useState<AccountBrief | null>(null);

  // 更新 Token 弹窗状态
  const [updateTokenModal, setUpdateTokenModal] = useState<{
    accountId: string;
    accountName: string;
  } | null>(null);

  // 编辑备注弹窗状态
  const [noteModal, setNoteModal] = useState<{
    accountId: string;
    accountName: string;
    note: string;
  } | null>(null);

  // 信息展示弹窗状态
  const [infoModal, setInfoModal] = useState<{
    isOpen: boolean;
    title: string;
    icon: string;
    sections: Array<{
      title?: string;
      content: string;
      type?: "text" | "code" | "list";
    }>;
    confirmText: string;
    onConfirm: () => void;
  } | null>(null);

  // 加载账号列表
  const loadAccounts = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.getAccounts();
      setAccounts(list);
    } catch (err: any) {
      setError(err.message || "加载账号失败");
    } finally {
      setLoading(false);
    }
  }, []);

  // 初始加载
  useEffect(() => {
    loadAccounts();

    // 检测系统深色模式并切换 html.dark class
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const updateTheme = (e: MediaQueryListEvent | MediaQueryList) => {
      document.documentElement.classList.toggle('dark', e.matches);
    };
    updateTheme(mq);
    mq.addEventListener('change', updateTheme);
    return () => mq.removeEventListener('change', updateTheme);
  }, [loadAccounts]);

  // 自动刷新即将过期的 Token
  useEffect(() => {
    // 启动时刷新
    api.refreshAllTokens().then((refreshed) => {
      if (refreshed.length > 0) {
        console.log(`[INFO] 启动时自动刷新了 ${refreshed.length} 个 Token`);
        loadAccounts();
      }
    }).catch(console.error);

    // 每30分钟刷新一次
    const interval = setInterval(() => {
      api.refreshAllTokens().then((refreshed) => {
        if (refreshed.length > 0) {
          console.log(`[INFO] 定时自动刷新了 ${refreshed.length} 个 Token`);
          loadAccounts();
        }
      }).catch(console.error);
    }, 30 * 60 * 1000);

    return () => clearInterval(interval);
  }, [loadAccounts]);

  // 添加账号
  const handleAddAccount = async (token: string, cookies?: string) => {
    await api.addAccountByToken(token, cookies, "manual");
    addToast("success", "账号添加成功");
    await loadAccounts();
  };

  // 删除账号
  const handleDeleteAccount = async (accountId: string) => {
    setConfirmModal({
      isOpen: true,
      title: "删除账号",
      message: "确定要删除此账号吗？删除后无法恢复。",
      type: "danger",
      onConfirm: async () => {
        try {
          await api.removeAccount(accountId);
          setSelectedIds((prev) => {
            const next = new Set(prev);
            next.delete(accountId);
            return next;
          });
          addToast("success", "账号已删除");
          await loadAccounts();
        } catch (err: any) {
          addToast("error", err.message || "删除账号失败");
        }
        setConfirmModal(null);
      },
    });
  };

  // 选择账号
  const handleSelectAccount = (accountId: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(accountId)) {
        next.delete(accountId);
      } else {
        next.add(accountId);
      }
      return next;
    });
  };

  // 全选/取消全选
  const handleSelectAll = () => {
    if (selectedIds.size === accounts.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(accounts.map((a) => a.id)));
    }
  };

  // 右键菜单
  const handleContextMenu = (e: React.MouseEvent, accountId: string) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, accountId });
  };

  // 复制 Token
  const handleCopyToken = async (accountId: string) => {
    try {
      const account = await api.getAccount(accountId);
      if (account.jwt_token) {
        await navigator.clipboard.writeText(account.jwt_token);
        addToast("success", "Token 已复制到剪贴板");
      } else {
        addToast("warning", "该账号没有有效的 Token");
      }
    } catch (err: any) {
      addToast("error", err.message || "获取 Token 失败");
    }
  };

  // 切换账号
  const handleSwitchAccount = async (accountId: string) => {
    const account = accounts.find((a) => a.id === accountId);
    if (!account) return;

    setConfirmModal({
      isOpen: true,
      title: "切换账号",
      message: `确定要切换到账号 "${account.email || account.name}" 吗？\n\n切换后该账号将绑定到「默认」实例。\n\n系统将自动关闭 TRAE Work CN 并切换登录信息。`,
      type: "warning",
      onConfirm: async () => {
        setConfirmModal(null);
        addToast("info", "正在切换账号，请稍候...");
        try {
          await api.switchAccount(accountId);
          await loadAccounts();
          addToast("success", "账号切换成功，请重新打开 TRAE Work CN");
        } catch (err: any) {
          addToast("error", err.message || "切换账号失败");
        }
      },
    });
  };

  // 多开实例
  const handleLaunchMulti = async (accountId: string) => {
    const account = accounts.find((a) => a.id === accountId);
    if (!account) return;

    addToast("info", `正在多开账号 "${account.email || account.name}"，请稍候...`);
    try {
      await api.launchAccountMulti(accountId);
      await loadAccounts();
      addToast("success", "多开实例已启动");
    } catch (err: any) {
      addToast("error", err.message || "多开失败");
    }
  };

  // 查看详情
  const handleViewDetail = async (accountId: string) => {
    const account = accounts.find((a) => a.id === accountId);
    if (account) {
      try {
        // 获取完整的账号信息（包含 token 和 cookies）
        const fullAccount = await api.getAccount(accountId);
        setDetailAccount({ ...account, ...fullAccount });
      } catch (err: any) {
        addToast("error", "获取账号详情失败");
        console.error("获取账号详情失败:", err);
      }
    }
  };

  // 更新 Token
  const handleUpdateToken = async (accountId: string, token: string) => {
    try {
      await api.updateAccountToken(accountId, token);
      addToast("success", "Token 更新成功");
      await loadAccounts();
    } catch (err: any) {
      throw err; // 让弹窗显示错误
    }
  };

  // 打开更新 Token 弹窗
  const handleOpenUpdateToken = (accountId: string) => {
    const account = accounts.find((a) => a.id === accountId);
    if (account) {
      setUpdateTokenModal({
        accountId,
        accountName: account.email || account.name,
      });
    }
  };

  // 打开编辑备注弹窗
  const handleOpenEditNote = (accountId: string) => {
    const account = accounts.find((a) => a.id === accountId);
    if (account) {
      setNoteModal({
        accountId,
        accountName: account.email || account.name,
        note: account.note || "",
      });
    }
  };

  // 保存备注
  const handleSaveNote = async () => {
    if (!noteModal) return;
    try {
      const note = noteModal.note.trim() || null;
      await api.updateAccountNote(noteModal.accountId, note);
      setAccounts((prev) =>
        prev.map((a) =>
          a.id === noteModal.accountId ? { ...a, note } : a
        )
      );
      addToast("success", "备注已保存");
      setNoteModal(null);
    } catch (err: any) {
      addToast("error", err.message || "保存备注失败");
    }
  };

  // 批量删除选中账号（优化：改进错误处理和反馈）
  const handleBatchDelete = () => {
    if (selectedIds.size === 0) {
      addToast("warning", "请先选择要删除的账号");
      return;
    }

    const ids = Array.from(selectedIds);
    setConfirmModal({
      isOpen: true,
      title: "批量删除",
      message: `确定要删除选中的 ${ids.length} 个账号吗？此操作无法撤销。`,
      type: "danger",
      onConfirm: async () => {
        setConfirmModal(null);
        addToast("info", `正在删除 ${ids.length} 个账号...`);

        // 并行删除所有选中的账号
        const results = await Promise.allSettled(
          ids.map((id) => api.removeAccount(id))
        );

        // 统计结果
        const successCount = results.filter((r) => r.status === 'fulfilled').length;
        const failCount = ids.length - successCount;

        setSelectedIds(new Set());
        await loadAccounts();

        if (failCount === 0) {
          addToast("success", `成功删除 ${successCount} 个账号`);
        } else {
          addToast("warning", `删除完成：${successCount} 成功，${failCount} 失败`);
        }
      },
    });
  };

  // 删除过期/失效账号
  const handleDeleteExpiredAccounts = () => {
    // 筛选出过期或失效的账号
    const expiredAccounts = accounts.filter((account) => {
      if (!account.token_expired_at) return false;
      const expiry = new Date(account.token_expired_at).getTime();
      if (isNaN(expiry)) return false;
      return expiry < Date.now(); // Token 已过期
    });

    if (expiredAccounts.length === 0) {
      addToast("info", "没有找到过期或失效的账号");
      return;
    }

    setConfirmModal({
      isOpen: true,
      title: "删除过期账号",
      message: `检测到 ${expiredAccounts.length} 个过期账号，确定要删除吗？此操作无法撤销。`,
      type: "warning",
      onConfirm: async () => {
        setConfirmModal(null);
        addToast("info", `正在删除 ${expiredAccounts.length} 个过期账号...`);

        // 并行删除所有过期账号
        const results = await Promise.allSettled(
          expiredAccounts.map((account) => api.removeAccount(account.id))
        );

        // 统计结果
        const successCount = results.filter((r) => r.status === 'fulfilled').length;
        const failCount = expiredAccounts.length - successCount;

        setSelectedIds(new Set());
        await loadAccounts();

        if (failCount === 0) {
          addToast("success", `成功删除 ${successCount} 个过期账号`);
        } else {
          addToast("warning", `删除完成：${successCount} 成功，${failCount} 失败`);
        }
      },
    });
  };

  return (
    <div className="app">
      <Sidebar currentPage={currentPage} onNavigate={setCurrentPage} />

      <div className="app-content">
        {error && (
          <div className="error-banner">
            {error}
            <button onClick={() => setError(null)}>×</button>
          </div>
        )}

        <div className={`page-content ${currentPage === "instances" ? "active" : "hidden"}`}>
          <Instances accounts={accounts} onRefreshAccounts={loadAccounts} />
        </div>

        <div className={`page-content ${currentPage === "accounts" ? "active" : "hidden"}`}>
          <>
            <header className="page-header">
              <div className="header-left">
                <h2 className="page-title">账号管理</h2>
                <p>管理您的账号 · 切换账号即绑定到默认实例</p>
              </div>
              <div className="header-right">
                <span className="account-count">共 {accounts.length} 个账号</span>
                <button
                  className="header-btn danger"
                  onClick={handleDeleteExpiredAccounts}
                  title="删除所有过期账号"
                  disabled={accounts.length === 0}
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
                    <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                    <line x1="10" y1="11" x2="10" y2="17"/>
                    <line x1="14" y1="11" x2="14" y2="17"/>
                  </svg>
                  删除过期
                  {(() => {
                    const expiredCount = accounts.filter((account) => {
                      if (!account.token_expired_at) return false;
                      const expiry = new Date(account.token_expired_at).getTime();
                      if (isNaN(expiry)) return false;
                      return expiry < Date.now();
                    }).length;
                    return expiredCount > 0 ? <span className="badge-count">{expiredCount}</span> : null;
                  })()}
                </button>
                <button className="add-btn" onClick={() => setShowAddModal(true)}>
                  <span>+</span> 添加账号
                </button>
              </div>
            </header>

            <main className="app-main">
              {accounts.length > 0 && (
                <div className="toolbar">
                  <div className="toolbar-left">
                    <label className="select-all">
                      <input
                        type="checkbox"
                        checked={selectedIds.size === accounts.length && accounts.length > 0}
                        onChange={handleSelectAll}
                      />
                      全选 ({selectedIds.size}/{accounts.length})
                    </label>
                    {selectedIds.size > 0 && (
                      <div className="batch-actions">
                        <button className="batch-btn danger" onClick={handleBatchDelete}>
                          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="14" height="14">
                            <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                          </svg>
                          删除
                        </button>
                      </div>
                    )}
                  </div>
                  <div className="toolbar-right">
                    <div className="view-toggle">
                      <button
                        className={`view-btn ${viewMode === "grid" ? "active" : ""}`}
                        onClick={() => setViewMode("grid")}
                        title="卡片视图"
                      >
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
                          <rect x="3" y="3" width="7" height="7"/>
                          <rect x="14" y="3" width="7" height="7"/>
                          <rect x="3" y="14" width="7" height="7"/>
                          <rect x="14" y="14" width="7" height="7"/>
                        </svg>
                      </button>
                      <button
                        className={`view-btn ${viewMode === "list" ? "active" : ""}`}
                        onClick={() => setViewMode("list")}
                        title="列表视图"
                      >
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
                          <line x1="8" y1="6" x2="21" y2="6"/>
                          <line x1="8" y1="12" x2="21" y2="12"/>
                          <line x1="8" y1="18" x2="21" y2="18"/>
                          <line x1="3" y1="6" x2="3.01" y2="6"/>
                          <line x1="3" y1="12" x2="3.01" y2="12"/>
                          <line x1="3" y1="18" x2="3.01" y2="18"/>
                        </svg>
                      </button>
                    </div>
                  </div>
                </div>
              )}

              {loading ? (
                <div className="loading">
                  <div className="spinner"></div>
                  <p>加载中...</p>
                </div>
              ) : accounts.length === 0 ? (
                <div className="empty-state">
                  <div className="empty-icon">📋</div>
                  <h3>暂无账号</h3>
                  <p>点击上方按钮添加账号</p>
                  <div className="empty-actions">
                    <button className="empty-btn primary" onClick={() => setShowAddModal(true)}>
                      添加账号
                    </button>
                  </div>
                </div>
              ) : viewMode === "grid" ? (
                <div className="account-grid">
                  {accounts.map((account) => (
                    <AccountCard
                      key={account.id}
                      account={account}
                      selected={selectedIds.has(account.id)}
                      onSelect={handleSelectAccount}
                      onContextMenu={handleContextMenu}
                    />
                  ))}
                </div>
              ) : (
                <div className="account-list">
                  <div className="list-header">
                    <div className="list-col checkbox"></div>
                    <div className="list-col avatar"></div>
                    <div className="list-col info">账号信息</div>
                    <div className="list-col plan">套餐</div>
                    <div className="list-col reset">添加时间</div>
                    <div className="list-col status">状态</div>
                    <div className="list-col actions"></div>
                  </div>
                  {accounts.map((account) => (
                    <AccountListItem
                      key={account.id}
                      account={account}
                      selected={selectedIds.has(account.id)}
                      onSelect={handleSelectAccount}
                      onContextMenu={handleContextMenu}
                    />
                  ))}
                </div>
              )}
            </main>
          </>
        </div>

        <div className={`page-content ${currentPage === "settings" ? "active" : "hidden"}`}>
          <>
            <header className="page-header">
              <div className="header-left">
                <h2 className="page-title">设置</h2>
                <p>配置应用程序选项</p>
              </div>
            </header>
            <Settings onToast={addToast} onAccountsChanged={loadAccounts} />
          </>
        </div>

        <div className={`page-content ${currentPage === "about" ? "active" : "hidden"}`}>
          <>
            <header className="page-header">
              <div className="header-left">
                <h2 className="page-title">关于</h2>
                <p>应用程序信息</p>
              </div>
            </header>
            <About />
          </>
        </div>
      </div>

      {/* Toast 通知 */}
      <Toast messages={toasts} onRemove={removeToast} />

      {/* 确认弹窗 */}
      {confirmModal && (
        <ConfirmModal
          isOpen={confirmModal.isOpen}
          title={confirmModal.title}
          message={confirmModal.message}
          type={confirmModal.type}
          confirmText="确定"
          cancelText="取消"
          onConfirm={confirmModal.onConfirm}
          onCancel={() => setConfirmModal(null)}
        />
      )}

      {/* 信息展示弹窗 */}
      {infoModal && (
        <InfoModal
          isOpen={infoModal.isOpen}
          title={infoModal.title}
          icon={infoModal.icon}
          sections={infoModal.sections}
          confirmText={infoModal.confirmText}
          onConfirm={infoModal.onConfirm}
          onCancel={() => setInfoModal(null)}
        />
      )}

      {/* 右键菜单 */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
          onViewDetail={() => {
            handleViewDetail(contextMenu.accountId);
            setContextMenu(null);
          }}
          onUpdateToken={() => {
            handleOpenUpdateToken(contextMenu.accountId);
            setContextMenu(null);
          }}
          onCopyToken={() => {
            handleCopyToken(contextMenu.accountId);
            setContextMenu(null);
          }}
          onEditNote={() => {
            handleOpenEditNote(contextMenu.accountId);
            setContextMenu(null);
          }}
          onSwitchAccount={() => {
            handleSwitchAccount(contextMenu.accountId);
            setContextMenu(null);
          }}
          onLaunchMulti={() => {
            handleLaunchMulti(contextMenu.accountId);
            setContextMenu(null);
          }}
          onDelete={() => {
            handleDeleteAccount(contextMenu.accountId);
            setContextMenu(null);
          }}
          isCurrent={accounts.find(a => a.id === contextMenu.accountId)?.is_current || false}
        />
      )}

      {/* 添加账号弹窗 */}
      <AddAccountModal
        isOpen={showAddModal}
        onClose={() => setShowAddModal(false)}
        onAdd={handleAddAccount}
        onToast={addToast}
        onAccountAdded={loadAccounts}
      />

      {/* 详情弹窗 */}
      <DetailModal
        isOpen={!!detailAccount}
        onClose={() => setDetailAccount(null)}
        account={detailAccount}
      />

      {/* 更新 Token 弹窗 */}
      <UpdateTokenModal
        isOpen={!!updateTokenModal}
        accountId={updateTokenModal?.accountId || ""}
        accountName={updateTokenModal?.accountName || ""}
        onClose={() => setUpdateTokenModal(null)}
        onUpdate={handleUpdateToken}
      />

      {/* 编辑备注弹窗 */}
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
              <button type="button" className="primary" onClick={handleSaveNote}>保存备注</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
