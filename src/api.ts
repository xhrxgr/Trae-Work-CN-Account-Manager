import { invoke } from "@tauri-apps/api/core";
import type { Account, AccountBrief } from "./types";

// 添加账号（通过 Cookies）
export async function addAccount(cookies: string): Promise<Account> {
  return invoke("add_account", { cookies });
}

// 添加账号（通过 Token，可选 Cookies）
export async function addAccountByToken(token: string, cookies?: string, source?: string): Promise<Account> {
  return invoke("add_account_by_token", { token, cookies, source });
}

// 删除账号
export async function removeAccount(accountId: string): Promise<void> {
  return invoke("remove_account", { accountId });
}

// 获取所有账号
export async function getAccounts(): Promise<AccountBrief[]> {
  return invoke("get_accounts");
}

// 获取单个账号详情（包含 token）
export async function getAccount(accountId: string): Promise<Account> {
  return invoke("get_account", { accountId });
}

// 更新账号备注
export async function updateAccountNote(accountId: string, note: string | null): Promise<void> {
  return invoke("update_account_note", { accountId, note });
}

// 设置活跃账号
export async function setActiveAccount(accountId: string): Promise<void> {
  return invoke("switch_account", { accountId });
}

// 切换账号
export async function switchAccount(accountId: string): Promise<void> {
  return invoke("switch_account", { accountId });
}

// 多开模式：启动独立的 TRAE Work CN 实例
export async function launchAccountMulti(accountId: string): Promise<void> {
  return invoke("launch_account_multi", { accountId });
}

// 更新账号 Token
export async function updateAccountToken(accountId: string, token: string): Promise<void> {
  return invoke("update_account_token", { accountId, token });
}

// 刷新 Token
export async function refreshToken(accountId: string): Promise<void> {
  return invoke("refresh_token", { accountId });
}

// 更新 Cookies
export async function updateCookies(accountId: string, cookies: string): Promise<void> {
  return invoke("update_cookies", { accountId, cookies });
}

// 从 TRAE Work CN 读取当前登录账号
export async function readLocalAccount(): Promise<Account | null> {
  return invoke("read_solo_cn_account");
}

// ============ 机器码相关 API ============

// 获取当前系统机器码
export async function getMachineId(): Promise<string> {
  return invoke("get_machine_id");
}

// 重置系统机器码（生成新的随机机器码）
export async function resetMachineId(): Promise<string> {
  return invoke("reset_machine_id");
}

// 设置系统机器码为指定值
export async function setMachineId(machineId: string): Promise<void> {
  return invoke("set_machine_id", { machineId });
}

// 绑定账号机器码（保存当前系统机器码到账号）
export async function bindAccountMachineId(accountId: string): Promise<string> {
  return invoke("bind_account_machine_id", { accountId });
}

// ============ TRAE Work CN 机器码相关 API ============

// 获取 TRAE Work CN 的机器码
export async function getProductMachineId(): Promise<string> {
  return invoke("get_solo_cn_machine_id");
}

// 设置 TRAE Work CN 的机器码
export async function setProductMachineId(machineId: string): Promise<void> {
  return invoke("set_solo_cn_machine_id", { machineId });
}

// 清除 TRAE Work CN 登录状态
export async function clearProductLoginState(): Promise<void> {
  return invoke("clear_solo_cn_login_state");
}

// 获取保存的 TRAE Work CN 路径
export async function getProductPath(): Promise<string> {
  return invoke("get_solo_cn_path");
}

// 设置 TRAE Work CN 路径
export async function setProductPath(path: string): Promise<void> {
  return invoke("set_solo_cn_path", { path });
}

// 自动扫描 TRAE Work CN 路径
export async function scanProductPath(): Promise<string> {
  return invoke("scan_solo_cn_path");
}

// ============ Token 刷新相关 API ============

// 批量刷新所有即将过期的 Token
export async function refreshAllTokens(): Promise<string[]> {
  return invoke("refresh_all_tokens");
}

// ============ 账号导入导出 ============

// 导出所有账号为 JSON 字符串
export async function exportAccounts(): Promise<string> {
  return invoke("export_accounts");
}

// 导出所有账号到指定文件
export async function exportAccountsToFile(filePath: string): Promise<void> {
  return invoke("export_accounts_to_file", { filePath });
}

// 从 JSON 字符串导入账号
// overwrite=true: 替换所有账号; overwrite=false: 合并（跳过已存在的 user_id）
export async function importAccounts(jsonStr: string, overwrite: boolean): Promise<number> {
  return invoke("import_accounts", { jsonStr, overwrite });
}

// 从指定文件导入账号
export async function importAccountsFromFile(filePath: string, overwrite: boolean): Promise<number> {
  return invoke("import_accounts_from_file", { filePath, overwrite });
}

// ============ 浏览器登录 ============

// 打开浏览器登录窗口
export async function startBrowserLogin(): Promise<void> {
  return invoke("start_browser_login");
}
