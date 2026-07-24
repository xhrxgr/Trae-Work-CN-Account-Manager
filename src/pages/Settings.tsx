import { useState, useEffect } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as api from "../api";

interface SettingsProps {
  onToast?: (type: "success" | "error" | "warning" | "info", message: string) => void;
  onAccountsChanged?: () => void;
}

export function Settings({ onToast, onAccountsChanged }: SettingsProps) {
  // 机器码状态（v1.0.28+：展示所有实例的机器码）
  const [instanceMachineIds, setInstanceMachineIds] = useState<api.InstanceMachineIdInfo[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [clearingId, setClearingId] = useState<string | null>(null);

  // 路径状态
  const [appPath, setAppPath] = useState<string>("");
  const [pathLoading, setPathLoading] = useState(false);
  const [scanning, setScanning] = useState(false);

  // 导入导出状态
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [hasBackup, setHasBackup] = useState(false);

  // 加载所有实例的机器码
  const loadInstanceMachineIds = async () => {
    setRefreshing(true);
    try {
      const list = await api.getInstanceMachineIds();
      setInstanceMachineIds(list);
    } catch (err: any) {
      console.error("获取实例机器码失败:", err);
    } finally {
      setRefreshing(false);
    }
  };

  // 加载路径
  const loadPath = async () => {
    setPathLoading(true);
    try {
      const path = await api.getProductPath();
      setAppPath(path);
    } catch (err: any) {
      console.error("获取路径失败:", err);
      setAppPath("");
    } finally {
      setPathLoading(false);
    }
  };

  useEffect(() => {
    loadInstanceMachineIds();
    loadPath();
    checkBackupStatus();
  }, []);

  // 复制机器码
  const handleCopyMachineId = async (machineId: string) => {
    try {
      await navigator.clipboard.writeText(machineId);
      onToast?.("success", "机器码已复制到剪贴板");
    } catch {
      onToast?.("error", "复制失败");
    }
  };

  // 清除指定实例的登录状态
  const handleClearInstanceLogin = async (instanceId: string, instanceName: string) => {
    if (!confirm(`确定要清除实例「${instanceName}」的登录状态吗？\n\n这将：\n• 重置该实例的机器码\n• 清除该实例的登录信息\n• 删除本地缓存数据\n\n操作后该实例将变成全新状态，需要重新登录。\n\n请确保该实例已关闭！`)) {
      return;
    }

    setClearingId(instanceId);
    try {
      const newMachineId = await api.clearInstanceLoginState(instanceId);
      setInstanceMachineIds(prev => prev.map(item =>
        item.id === instanceId ? { ...item, machine_id: newMachineId } : item
      ));
      onToast?.("success", `已清除「${instanceName}」的登录状态`);
    } catch (err: any) {
      onToast?.("error", err.message || "清除失败");
    } finally {
      setClearingId(null);
    }
  };

  // 自动扫描路径
  const handleScanPath = async () => {
    setScanning(true);
    try {
      const path = await api.scanProductPath();
      setAppPath(path);
      onToast?.("success", "已找到 TRAE Work CN: " + path);
    } catch (err: any) {
      onToast?.("error", err.message || "未找到 TRAE Work CN，请手动设置路径");
    } finally {
      setScanning(false);
    }
  };

  // 手动设置路径
  const handleSetPath = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: "TRAE Work CN",
          extensions: ["exe"]
        }],
        title: "选择 TRAE SOLO CN.exe 文件"
      });

      if (selected) {
        const path = selected as string;
        await api.setProductPath(path);
        setAppPath(path);
        onToast?.("success", "路径已保存");
      }
    } catch (err: any) {
      onToast?.("error", err.message || "选择文件失败");
    }
  };

  // 导出账号到文件
  const handleExport = async () => {
    setExporting(true);
    try {
      const filePath = await save({
        filters: [{
          name: "JSON 文件",
          extensions: ["json"]
        }],
        defaultPath: `trae-accounts-${new Date().toISOString().slice(0, 10)}.json`,
        title: "选择导出文件保存位置"
      });

      if (!filePath) {
        setExporting(false);
        return;
      }

      await api.exportAccountsToFile(filePath);
      onToast?.("success", "账号已导出到: " + filePath);
    } catch (err: any) {
      onToast?.("error", err.message || "导出失败");
    } finally {
      setExporting(false);
    }
  };

  // 从文件导入账号
  const handleImport = async (overwrite: boolean) => {
    setImporting(true);
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: "JSON 文件",
          extensions: ["json"]
        }],
        title: "选择要导入的账号文件"
      });

      if (!selected) {
        setImporting(false);
        return;
      }

      const filePath = selected as string;
      const count = await api.importAccountsFromFile(filePath, overwrite);
      onToast?.("success", `成功导入 ${count} 个账号`);
      onAccountsChanged?.();
      // 替换导入会自动备份，刷新备份状态
      if (overwrite) {
        checkBackupStatus();
      }
    } catch (err: any) {
      onToast?.("error", err.message || "导入失败");
    } finally {
      setImporting(false);
    }
  };

  // 检查备份状态
  const checkBackupStatus = async () => {
    try {
      const has = await api.hasAccountBackup();
      setHasBackup(has);
    } catch (err) {
      console.error("检查备份状态失败:", err);
    }
  };

  // 从备份恢复账号数据
  const handleRestoreBackup = async () => {
    if (!confirm("确定要从备份恢复账号数据吗？\n\n这将覆盖当前所有账号，恢复到上次替换导入前的状态。")) {
      return;
    }
    try {
      const count = await api.restoreAccountBackup();
      onToast?.("success", `已恢复 ${count} 个账号，备份已清理`);
      onAccountsChanged?.();
      checkBackupStatus();
    } catch (err: any) {
      onToast?.("error", err.message || "恢复失败");
    }
  };

  // 删除备份文件
  const handleDeleteBackup = async () => {
    if (!confirm("确定要删除账号备份文件吗？\n\n删除后将无法恢复到替换导入前的状态。")) {
      return;
    }
    try {
      await api.deleteAccountBackup();
      onToast?.("success", "备份文件已删除");
      setHasBackup(false);
    } catch (err: any) {
      onToast?.("error", err.message || "删除失败");
    }
  };

  return (
    <div className="settings-page">
      {/* 机器码 */}
      <div className="settings-section">
        <h3>机器码</h3>
        <div className="machine-id-card solo-cn-card">
          <div className="machine-id-header">
            <div className="machine-id-icon solo-cn-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M12 2L2 7l10 5 10-5-10-5z"/>
                <path d="M2 17l10 5 10-5"/>
                <path d="M2 12l10 5 10-5"/>
              </svg>
            </div>
            <div className="machine-id-title">
              <span>MachineId</span>
              <span className="machine-id-subtitle">客户端唯一标识符（每个实例独立）</span>
            </div>
            <button
              className="machine-id-btn"
              onClick={loadInstanceMachineIds}
              disabled={refreshing}
              title="刷新全部"
              style={{ marginLeft: "auto" }}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
              </svg>
              {refreshing ? "刷新中..." : "刷新"}
            </button>
          </div>

          {instanceMachineIds.length === 0 && (
            <div className="machine-id-value">
              <code>{refreshing ? "加载中..." : "暂无实例"}</code>
            </div>
          )}

          {instanceMachineIds.map(item => (
            <div className="machine-id-instance" key={item.id}>
              <div className="machine-id-instance-name">
                {item.name}
                {item.is_default && <span className="default-badge">默认</span>}
              </div>
              <div className="machine-id-value">
                <code>{item.machine_id || "未生成"}</code>
              </div>
              <div className="machine-id-actions">
                <button
                  className="machine-id-btn"
                  onClick={() => handleCopyMachineId(item.machine_id)}
                  disabled={!item.machine_id}
                  title="复制"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                  </svg>
                  复制
                </button>
                <button
                  className="machine-id-btn danger"
                  onClick={() => handleClearInstanceLogin(item.id, item.name)}
                  disabled={clearingId === item.id}
                  title="清除登录状态"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                    <line x1="10" y1="11" x2="10" y2="17"/>
                    <line x1="14" y1="11" x2="14" y2="17"/>
                  </svg>
                  {clearingId === item.id ? "清除中..." : "清除登录状态"}
                </button>
              </div>
            </div>
          ))}

          <div className="machine-id-tip warning">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
              <line x1="12" y1="9" x2="12" y2="13"/>
              <line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
            <span>清除登录状态会重置该实例的机器码并删除登录信息，该实例将需要重新登录。请先关闭对应实例。</span>
          </div>
        </div>
      </div>

      {/* 路径设置 */}
      <div className="settings-section">
        <h3>TRAE Work CN 路径</h3>
        <div className="machine-id-card solo-cn-card">
          <div className="machine-id-header">
            <div className="machine-id-icon solo-cn-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
              </svg>
            </div>
            <div className="machine-id-title">
              <span>安装路径</span>
              <span className="machine-id-subtitle">用于自动打开 TRAE Work CN</span>
            </div>
          </div>
          <div className="machine-id-value">
            <code>{pathLoading ? "加载中..." : (appPath || "未设置")}</code>
          </div>
          <div className="machine-id-actions">
            <button
              className="machine-id-btn"
              onClick={handleScanPath}
              disabled={scanning}
              title="自动扫描"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="11" cy="11" r="8"/>
                <path d="M21 21l-4.35-4.35"/>
              </svg>
              {scanning ? "扫描中..." : "自动扫描"}
            </button>
            <button
              className="machine-id-btn"
              onClick={handleSetPath}
              title="手动设置"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
                <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
              </svg>
              手动设置
            </button>
          </div>
          <div className="machine-id-tip">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="10"/>
              <path d="M12 16v-4"/>
              <path d="M12 8h.01"/>
            </svg>
            <span>切换账号后会自动打开 TRAE Work CN。如果自动扫描找不到，请手动设置 TRAE SOLO CN.exe 的完整路径。</span>
          </div>
        </div>
      </div>

      {/* 账号导入导出 */}
      <div className="settings-section">
        <h3>账号导入导出</h3>
        <div className="machine-id-card solo-cn-card">
          <div className="machine-id-header">
            <div className="machine-id-icon solo-cn-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            </div>
            <div className="machine-id-title">
              <span>账号备份与恢复</span>
              <span className="machine-id-subtitle">导出账号到文件 / 从文件导入账号</span>
            </div>
          </div>
          <div className="machine-id-actions">
            <button
              className="machine-id-btn"
              onClick={handleExport}
              disabled={exporting || importing}
              title="导出所有账号到 JSON 文件"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="17 8 12 3 7 8"/>
                <line x1="12" y1="3" x2="12" y2="15"/>
              </svg>
              {exporting ? "导出中..." : "导出账号"}
            </button>
            <button
              className="machine-id-btn"
              onClick={() => handleImport(false)}
              disabled={exporting || importing}
              title="合并导入：跳过已存在的账号"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
              {importing ? "导入中..." : "合并导入"}
            </button>
            <button
              className="machine-id-btn danger"
              onClick={() => {
                if (confirm("替换导入会清除当前所有账号并替换为文件中的账号，确定继续吗？\n\n（替换前会自动备份当前账号到 accounts.json.bak，可在下方恢复）")) {
                  handleImport(true);
                }
              }}
              disabled={exporting || importing}
              title="替换导入：清除当前所有账号（自动备份）"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="17 1 21 5 17 9"/>
                <path d="M3 11V9a4 4 0 0 1 4-4h14"/>
                <polyline points="7 23 3 19 7 15"/>
                <path d="M21 13v2a4 4 0 0 1-4 4H3"/>
              </svg>
              {importing ? "导入中..." : "替换导入"}
            </button>
          </div>
          {hasBackup && (
            <div className="machine-id-instance">
              <div className="machine-id-instance-name">
                上次替换前的备份
                <span className="default-badge" style={{ background: "#f59e0b" }}>未使用</span>
              </div>
              <div className="machine-id-value">
                <code>accounts.json.bak</code>
              </div>
              <div className="machine-id-actions">
                <button
                  className="machine-id-btn"
                  onClick={handleRestoreBackup}
                  disabled={exporting || importing}
                  title="从备份恢复账号（覆盖当前数据）"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 10"/>
                    <path d="M3 10v5a5 5 0 0 0 5 5h11"/>
                  </svg>
                  恢复备份
                </button>
                <button
                  className="machine-id-btn danger"
                  onClick={handleDeleteBackup}
                  disabled={exporting || importing}
                  title="删除备份文件"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                  </svg>
                  删除备份
                </button>
              </div>
            </div>
          )}
          <div className="machine-id-tip">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="10"/>
              <path d="M12 16v-4"/>
              <path d="M12 8h.01"/>
            </svg>
            <span>
              导出：将所有账号（含 Token）保存为 JSON 文件。合并导入：跳过已存在的账号。替换导入：清除当前所有账号后导入（自动备份旧数据到 .bak，可恢复）。
            </span>
          </div>
        </div>
      </div>

    </div>
  );
}