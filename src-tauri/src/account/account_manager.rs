use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;

use super::types::*;
use crate::api::{TraeApiClient, UsageSummary};

/// 账号管理器
pub struct AccountManager {
    store: AccountStore,
    data_path: PathBuf,
}

impl AccountManager {
    /// 创建账号管理器
    pub fn new() -> Result<Self> {
        let data_path = Self::get_data_path()?;
        let store = Self::load_store(&data_path)?;

        Ok(Self { store, data_path })
    }

    /// 获取数据存储路径
    fn get_data_path() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "trae", "work-cn-manager")
            .ok_or_else(|| anyhow!("无法获取应用数据目录"))?;

        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir)?;

        Ok(data_dir.join("accounts.json"))
    }

    /// 加载账号存储
    fn load_store(path: &PathBuf) -> Result<AccountStore> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let store: AccountStore = serde_json::from_str(&content)?;
            Ok(store)
        } else {
            Ok(AccountStore::default())
        }
    }

    /// 保存账号存储
    fn save_store(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.store)?;
        fs::write(&self.data_path, content)?;
        Ok(())
    }

    /// 添加账号（通过 cookies）
    pub async fn add_account(&mut self, cookies: String) -> Result<Account> {
        let mut client = TraeApiClient::new(&cookies)?;

        // 获取 token
        let token_result = client.get_user_token().await?;

        // 获取用户信息
        let user_info = client.get_user_info().await?;

        // 检查是否已存在
        if self
            .store
            .accounts
            .iter()
            .any(|a| a.user_id == token_result.user_id)
        {
            return Err(anyhow!("该账号已存在"));
        }

        let mut account = Account::new(
            user_info.screen_name.clone(),
            user_info.non_plain_text_email.unwrap_or_default(),
            cookies,
            token_result.user_id,
            token_result.tenant_id,
        );

        account.avatar_url = user_info.avatar_url;
        account.region = user_info.region;
        account.jwt_token = Some(token_result.token);
        account.token_expired_at = Some(token_result.expired_at);

        self.store.accounts.push(account.clone());

        // 如果是第一个账号，设为活跃账号
        if self.store.active_account_id.is_none() {
            self.store.active_account_id = Some(account.id.clone());
        }

        self.save_store()?;
        Ok(account)
    }

    /// 添加账号（通过 Token，可选 Cookies）
    /// source: "browser"(浏览器登录), "local"(本地读取), "manual"(手动输入)
    /// browser_user_info: 浏览器登录时从前端拦截到的用户信息（screen_name, avatar_url, email）
    pub async fn add_account_by_token(
        &mut self,
        token: String,
        cookies: Option<String>,
        source: String,
        browser_user_info: Option<BrowserUserInfo>,
    ) -> Result<Account> {
        let client = TraeApiClient::new_with_token(&token)?;

        // 通过 Token 获取用户信息
        let user_info = client.get_user_info_by_token().await?;

        // 检查是否已存在
        if self
            .store
            .accounts
            .iter()
            .any(|a| a.user_id == user_info.user_id)
        {
            return Err(anyhow!("该账号已存在"));
        }

        // 确定用户名、邮箱、头像
        // 优先级: 浏览器拦截到的 > API获取的 > Cookies获取的
        let (name, email, avatar_url) = if let Some(ref browser_info) = browser_user_info {
            // 浏览器登录时，优先使用前端拦截到的用户信息
            (
                if browser_info.screen_name.is_empty() {
                    user_info.screen_name.unwrap_or_else(|| format!("User_{}", &user_info.user_id[..8.min(user_info.user_id.len())]))
                } else {
                    browser_info.screen_name.clone()
                },
                if browser_info.email.is_empty() {
                    user_info.email.unwrap_or_default()
                } else {
                    browser_info.email.clone()
                },
                if browser_info.avatar_url.is_empty() {
                    user_info.avatar_url.unwrap_or_default()
                } else {
                    browser_info.avatar_url.clone()
                },
            )
        } else if let Some(ref cookies_str) = cookies {
            // 如果提供了 Cookies，尝试获取更详细的用户信息
            match self.get_user_info_with_cookies(cookies_str).await {
                Ok(info) => (
                    info.screen_name,
                    info.non_plain_text_email.unwrap_or_default(),
                    info.avatar_url,
                ),
                Err(_) => (
                    user_info.screen_name.unwrap_or_else(|| format!("User_{}", &user_info.user_id[..8.min(user_info.user_id.len())])),
                    user_info.email.unwrap_or_default(),
                    user_info.avatar_url.unwrap_or_default(),
                ),
            }
        } else {
            (
                user_info.screen_name.unwrap_or_else(|| format!("User_{}", &user_info.user_id[..8.min(user_info.user_id.len())])),
                user_info.email.unwrap_or_default(),
                user_info.avatar_url.unwrap_or_default(),
            )
        };

        let mut account = Account::new(
            name,
            email,
            cookies.unwrap_or_default(),
            user_info.user_id.clone(),
            user_info.tenant_id.clone(),
        );

        account.avatar_url = avatar_url;
        account.jwt_token = Some(token);
        account.token_expired_at = None;
        account.source = source;

        self.store.accounts.push(account.clone());

        // 如果是第一个账号，设为活跃账号
        if self.store.active_account_id.is_none() {
            self.store.active_account_id = Some(account.id.clone());
        }

        self.save_store()?;
        Ok(account)
    }

    /// 使用 Cookies 获取用户信息
    async fn get_user_info_with_cookies(&self, cookies: &str) -> Result<crate::api::UserInfoResult> {
        let client = TraeApiClient::new(cookies)?;
        client.get_user_info().await
    }

    /// 删除账号
    pub fn remove_account(&mut self, account_id: &str) -> Result<()> {
        let index = self
            .store
            .accounts
            .iter()
            .position(|a| a.id == account_id)
            .ok_or_else(|| anyhow!("账号不存在"))?;

        self.store.accounts.remove(index);

        // 如果删除的是活跃账号，重置活跃账号
        if self.store.active_account_id.as_deref() == Some(account_id) {
            self.store.active_account_id = self.store.accounts.first().map(|a| a.id.clone());
        }

        self.save_store()?;
        Ok(())
    }

    /// 设置活跃账号
    pub fn set_active_account(&mut self, account_id: &str) -> Result<()> {
        if !self.store.accounts.iter().any(|a| a.id == account_id) {
            return Err(anyhow!("账号不存在"));
        }

        self.store.active_account_id = Some(account_id.to_string());
        self.save_store()?;
        Ok(())
    }

    /// 切换账号到 TRAE Work CN
    pub fn switch_account(&mut self, account_id: &str) -> Result<()> {
        let account = self.store.accounts.iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| anyhow!("账号不存在"))?
            .clone();

        // 检查账号是否有有效的 Token
        let token = account.jwt_token.as_ref()
            .ok_or_else(|| anyhow!("账号没有有效的 Token，无法切换"))?;

        // 构建登录信息
        let login_info = crate::machine::TraeLoginInfo {
            token: token.clone(),
            refresh_token: None,
            user_id: account.user_id.clone(),
            email: account.email.clone(),
            username: account.name.clone(),
            avatar_url: account.avatar_url.clone(),
            host: String::new(),
            region: if account.region.is_empty() { "CN".to_string() } else { account.region.clone() },
        };

        // 切换 Trae Solo CN 到该账号
        crate::machine::switch_solo_cn_account(&login_info, account.machine_id.as_deref())?;

        // 如果账号有绑定的机器码，也更新系统机器码
        if let Some(machine_id) = &account.machine_id {
            match crate::machine::set_machine_guid(machine_id) {
                Ok(_) => println!("[INFO] 已切换系统机器码: {}", machine_id),
                Err(e) => println!("[WARN] 切换系统机器码失败（可能需要管理员权限）: {}", e),
            }
        }

        // 设置活跃账号
        self.store.active_account_id = Some(account_id.to_string());
        self.store.current_account_id = Some(account_id.to_string());
        self.save_store()?;

        println!("[INFO] 已切换 TRAE Work CN 到账号: {}", account.email);
        Ok(())
    }

    /// 多开模式：为指定账号启动一个独立的 TRAE Work CN 实例
    /// 不杀进程、不影响其他实例，使用账号绑定的独立 data-dir
    /// 首次多开时自动创建 data-dir 并绑定到账号，后续复用
    pub fn launch_account_multi(&mut self, account_id: &str) -> Result<()> {
        let account = self.store.accounts.iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| anyhow!("账号不存在"))?
            .clone();

        let token = account.jwt_token.as_ref()
            .ok_or_else(|| anyhow!("账号没有有效的 Token，无法多开"))?;

        // 构建登录信息
        let login_info = crate::machine::TraeLoginInfo {
            token: token.clone(),
            refresh_token: None,
            user_id: account.user_id.clone(),
            email: account.email.clone(),
            username: account.name.clone(),
            avatar_url: account.avatar_url.clone(),
            host: String::new(),
            region: if account.region.is_empty() { "CN".to_string() } else { account.region.clone() },
        };

        // 多开 data-dir：按 user_id 命名，放在 APPDATA 下与默认目录平级
        #[cfg(target_os = "windows")]
        let appdata = std::env::var("APPDATA")
            .map_err(|_| anyhow!("无法获取 APPDATA 环境变量"))?;
        #[cfg(target_os = "macos")]
        let appdata = std::env::var("HOME")
            .map_err(|_| anyhow!("无法获取 HOME 环境变量"))?;

        #[cfg(target_os = "windows")]
        let base_dir = PathBuf::from(&appdata);
        #[cfg(target_os = "macos")]
        let base_dir = PathBuf::from(&appdata).join("Library").join("Application Support");

        let multi_dir = base_dir.join(format!("TRAE SOLO CN_{}", &account.user_id));
        let multi_dir_str = multi_dir.to_string_lossy().to_string();

        // 共享插件目录（所有多开实例共用一份插件）
        #[cfg(target_os = "windows")]
        let shared_ext_dir = PathBuf::from(&appdata).join("TRAE SOLO CN_SharedExtensions");
        #[cfg(target_os = "macos")]
        let shared_ext_dir = PathBuf::from(&appdata).join("Library").join("Application Support")
            .join("TRAE SOLO CN_SharedExtensions");
        let shared_ext_str = shared_ext_dir.to_string_lossy().to_string();

        // 调用多开启动
        crate::machine::launch_product_multi(
            &login_info,
            account.machine_id.as_deref(),
            &multi_dir_str,
            Some(&shared_ext_str),
        )?;

        // 绑定 data-dir 到账号（首次多开时记录，后续复用）
        let need_save = account.data_dir.is_none();
        if need_save {
            if let Some(acc) = self.store.accounts.iter_mut().find(|a| a.id == account_id) {
                acc.data_dir = Some(multi_dir_str.clone());
                acc.updated_at = chrono::Utc::now().timestamp();
            }
            self.save_store()?;
        }

        println!("[INFO] 已多开 TRAE Work CN 账号: {} (data-dir: {})", account.email, multi_dir_str);
        Ok(())
    }

    /// 绑定当前系统机器码到账号
    pub fn bind_machine_id(&mut self, account_id: &str) -> Result<String> {
        // 获取当前系统机器码
        let current_machine_id = crate::machine::get_machine_guid()?;

        // 更新账号的机器码
        let account = self.store.accounts.iter_mut()
            .find(|a| a.id == account_id)
            .ok_or_else(|| anyhow!("账号不存在"))?;

        account.machine_id = Some(current_machine_id.clone());
        account.updated_at = chrono::Utc::now().timestamp();
        let email = account.email.clone();

        self.save_store()?;
        println!("[INFO] 已绑定机器码 {} 到账号 {}", current_machine_id, email);

        Ok(current_machine_id)
    }

    /// 获取所有账号列表
    pub fn get_accounts(&self) -> Vec<AccountBrief> {
        let current_id = self.store.current_account_id.as_deref();
        self.store.accounts.iter().map(|account| {
            let is_current = current_id == Some(account.id.as_str());
            AccountBrief::from_account(account, is_current)
        }).collect()
    }

    /// 获取活跃账号
    pub fn get_active_account(&self) -> Option<&Account> {
        self.store
            .active_account_id
            .as_ref()
            .and_then(|id| self.store.accounts.iter().find(|a| &a.id == id))
    }

    /// 获取指定账号
    pub fn get_account(&self, account_id: &str) -> Result<Account> {
        self.store
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .cloned()
            .ok_or_else(|| anyhow!("账号不存在"))
    }

    /// 更新账号备注
    pub fn update_account_note(&mut self, account_id: &str, note: Option<String>) -> Result<()> {
        let account = self.store
            .accounts
            .iter_mut()
            .find(|a| a.id == account_id)
            .ok_or_else(|| anyhow!("账号不存在"))?;
        account.note = note;
        account.updated_at = chrono::Utc::now().timestamp();
        self.save_store()?;
        Ok(())
    }

    /// 导出所有账号为 JSON 字符串
    /// 格式: { "version": "1.0", "exported_at": "...", "accounts": [...] }
    pub fn export_accounts(&self) -> Result<String> {
        let export_data = serde_json::json!({
            "version": "1.0",
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "accounts": self.store.accounts,
        });
        serde_json::to_string_pretty(&export_data)
            .map_err(|e| anyhow!("序列化导出数据失败: {}", e))
    }

    /// 从 JSON 字符串导入账号
    /// overwrite=true: 替换所有账号; overwrite=false: 合并（跳过已存在的 user_id）
    /// 返回成功导入的账号数量
    pub fn import_accounts(&mut self, json_str: &str, overwrite: bool) -> Result<usize> {
        let data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("解析导入文件失败: {}", e))?;

        // 兼容两种格式：
        // 1. { "accounts": [...] }（导出格式）
        // 2. [...]（直接是账号数组）
        let accounts_array = data
            .get("accounts")
            .and_then(|v| v.as_array())
            .or_else(|| data.as_array())
            .ok_or_else(|| anyhow!("导入文件格式错误: 缺少 accounts 数组"))?;

        // 反序列化为 Account 列表
        let accounts: Vec<Account> = accounts_array
            .iter()
            .filter_map(|v| serde_json::from_value::<Account>(v.clone()).ok())
            .collect();

        if accounts.is_empty() {
            return Err(anyhow!("导入文件中没有有效的账号数据"));
        }

        let count = if overwrite {
            // 替换模式：清空现有账号，直接导入
            self.store.accounts = accounts.clone();
            // 重置活跃账号为第一个
            self.store.active_account_id = accounts.first().map(|a| a.id.clone());
            self.store.current_account_id = None;
            accounts.len()
        } else {
            // 合并模式：跳过已存在的 user_id
            // 注意：必须使用 owned String 集合，避免不可变借用阻止后续 push
            let existing_user_ids: std::collections::HashSet<String> = self
                .store
                .accounts
                .iter()
                .map(|a| a.user_id.clone())
                .collect();

            let mut added = 0;
            for account in accounts {
                if existing_user_ids.contains(&account.user_id) {
                    // 跳过已存在的账号
                    continue;
                }
                self.store.accounts.push(account);
                added += 1;
            }

            // 如果之前没有活跃账号，设置第一个为活跃
            if self.store.active_account_id.is_none() && !self.store.accounts.is_empty() {
                self.store.active_account_id = Some(self.store.accounts[0].id.clone());
            }

            added
        };

        self.save_store()?;
        println!("[INFO] 已导入 {} 个账号 (overwrite={})", count, overwrite);
        Ok(count)
    }

    /// 获取账号使用量
    pub async fn get_account_usage(&mut self, account_id: &str) -> Result<UsageSummary> {
        let account = self
            .store
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| anyhow!("账号不存在"))?
            .clone();

        // 根据账号类型选择不同的方式获取使用量
        let summary = if let Some(token) = &account.jwt_token {
            // 优先使用 Token，根据区域选择 API 端点
            let client = TraeApiClient::new_with_token(token)?;
            match client.get_usage_summary_by_token().await {
                Ok(summary) => summary,
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("401") {
                        return Err(anyhow!("TRAE Work CN Token 已过期，请重新登录获取新 Token"));
                    } else {
                        return Err(e);
                    }
                }
            }
        } else if !account.cookies.is_empty() {
            // 使用 Cookies
            let mut client = TraeApiClient::new(&account.cookies)?;
            client.get_usage_summary().await?
        } else {
            return Err(anyhow!("账号没有有效的 Token 或 Cookies"));
        };

        // 更新账号的 plan_type
        if let Some(acc) = self.store.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.plan_type = summary.plan_type.clone();
            acc.updated_at = chrono::Utc::now().timestamp();
        }
        self.save_store()?;

        Ok(summary)
    }

    /// 刷新账号 Token
    pub async fn refresh_token(&mut self, account_id: &str) -> Result<()> {
        let account = self
            .store
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .ok_or_else(|| anyhow!("账号不存在"))?
            .clone();

        let mut client = TraeApiClient::new(&account.cookies)?;
        let token_result = client.get_user_token().await?;

        if let Some(acc) = self.store.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.jwt_token = Some(token_result.token);
            acc.token_expired_at = Some(token_result.expired_at);
            acc.updated_at = chrono::Utc::now().timestamp();
        }

        self.save_store()?;
        Ok(())
    }

    /// 更新账号 Token
    pub async fn update_account_token(&mut self, account_id: &str, token: String) -> Result<UsageSummary> {
        let client = TraeApiClient::new_with_token(&token)?;

        // 验证 Token 并获取用户信息
        let user_info = client.get_user_info_by_token().await?;

        // 查找账号
        let acc = self.store.accounts.iter_mut()
            .find(|a| a.id == account_id)
            .ok_or_else(|| anyhow!("账号不存在"))?;

        // 确保是同一个用户
        if acc.user_id != user_info.user_id {
            return Err(anyhow!("Token 对应的用户与当前账号不匹配"));
        }

        // 更新 Token
        acc.jwt_token = Some(token.clone());
        acc.updated_at = chrono::Utc::now().timestamp();

        // 获取最新使用量
        let summary = client.get_usage_summary_by_token().await?;
        acc.plan_type = summary.plan_type.clone();

        self.save_store()?;
        Ok(summary)
    }

    /// 更新账号 Cookies
    pub async fn update_cookies(&mut self, account_id: &str, cookies: String) -> Result<()> {
        // 验证新 cookies 是否有效
        let mut client = TraeApiClient::new(&cookies)?;
        let token_result = client.get_user_token().await?;

        if let Some(acc) = self.store.accounts.iter_mut().find(|a| a.id == account_id) {
            // 确保是同一个用户
            if acc.user_id != token_result.user_id {
                return Err(anyhow!("Cookies 对应的用户与当前账号不匹配"));
            }

            acc.cookies = cookies;
            acc.jwt_token = Some(token_result.token);
            acc.token_expired_at = Some(token_result.expired_at);
            acc.updated_at = chrono::Utc::now().timestamp();
        } else {
            return Err(anyhow!("账号不存在"));
        }

        self.save_store()?;
        Ok(())
    }

    /// 从 TRAE Work CN 读取当前登录账号
    pub async fn read_solo_cn_account(&mut self) -> Result<Option<Account>> {
        #[cfg(target_os = "windows")]
        let solo_cn_data_path = {
            let appdata = std::env::var("APPDATA")
                .map_err(|_| anyhow!("无法获取 APPDATA 环境变量"))?;
            PathBuf::from(appdata).join("TRAE SOLO CN")
        };

        #[cfg(target_os = "macos")]
        let solo_cn_data_path = {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow!("无法获取 HOME 环境变量"))?;
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("TRAE SOLO CN")
        };

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let solo_cn_data_path: PathBuf = {
            return Err(anyhow!("此功能仅支持 Windows 和 macOS 系统"));
        };

        let storage_path = solo_cn_data_path
            .join("User")
            .join("globalStorage")
            .join("storage.json");

        if !storage_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&storage_path)
            .map_err(|e| anyhow!("读取 TRAE Work CN 配置文件失败: {}", e))?;

        let storage: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow!("解析 TRAE Work CN 配置文件失败: {}", e))?;

        let auth_info_str = storage
            .get("iCubeAuthInfo://icube.cloudide")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("未找到 TRAE Work CN 登录信息"))?;

        let auth_info: serde_json::Value = match serde_json::from_str(auth_info_str) {
            Ok(json) => json,
            Err(_) => {
                // JSON 解析失败，尝试解密（加密数据）
                println!("[INFO] TRAE Work CN 认证信息已加密，尝试解密...");
                let decrypted = crate::machine::decrypt_solo_cn_auth_info(auth_info_str)?;
                serde_json::from_str(&decrypted)
                    .map_err(|e| anyhow!("解密后 JSON 解析失败: {}", e))?
            }
        };

        let token = auth_info
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("未找到 Token"))?
            .to_string();

        let user_id = auth_info
            .get("userId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("未找到 User ID"))?
            .to_string();

        let email = auth_info
            .get("account")
            .and_then(|acc| acc.get("email"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let avatar_url = auth_info
            .get("account")
            .and_then(|acc| acc.get("avatar_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let username = auth_info
            .get("account")
            .and_then(|acc| acc.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // 检查账号是否已存在
        if self.store.accounts.iter().any(|a| a.user_id == user_id) {
            println!("[INFO] TRAE Work CN 账号已存在于账号管理中");
            return Ok(None);
        }

        let client = TraeApiClient::new_with_token(&token)?;
        let user_info = client.get_user_info_by_token().await?;

        let mut account = Account::new(
            if username.is_empty() {
                user_info.screen_name.unwrap_or_else(|| format!("User_{}", &user_id[..8.min(user_id.len())]))
            } else {
                username
            },
            if email.is_empty() {
                user_info.email.unwrap_or_default()
            } else {
                email
            },
            String::new(),
            user_id,
            user_info.tenant_id,
        );

        account.avatar_url = if avatar_url.is_empty() {
            user_info.avatar_url.unwrap_or_default()
        } else {
            avatar_url
        };
        account.jwt_token = Some(token);
        account.source = "local".to_string();
        account.region = auth_info
            .get("storeRegion")
            .and_then(|v| v.as_str())
            .unwrap_or("CN")
            .to_string();

        self.store.accounts.push(account.clone());

        if self.store.active_account_id.is_none() {
            self.store.active_account_id = Some(account.id.clone());
        }

        self.save_store()?;

        println!("[INFO] 成功从 TRAE Work CN 读取并添加账号: {}", account.email);
        Ok(Some(account))
    }

    /// 判断账号的 Token 是否即将过期（< 1小时）或已过期
    fn is_token_expiring_soon(account: &Account) -> bool {
        match &account.token_expired_at {
            None => true, // 无过期时间信息，需要刷新
            Some(expired_at) => {
                match chrono::DateTime::parse_from_rfc3339(expired_at) {
                    Ok(expiry) => {
                        let now = chrono::Utc::now();
                        let one_hour = chrono::Duration::hours(1);
                        expiry.with_timezone(&chrono::Utc) < now + one_hour
                    }
                    Err(_) => {
                        // 尝试解析为时间戳（秒）
                        if let Ok(ts) = expired_at.parse::<i64>() {
                            let now = chrono::Utc::now().timestamp();
                            ts < now + 3600
                        } else {
                            true // 无法解析，需要刷新
                        }
                    }
                }
            }
        }
    }

    /// 批量刷新所有即将过期的 Token
    pub async fn refresh_all_tokens(&mut self) -> Result<Vec<String>> {
        let mut refreshed = Vec::new();
        let account_ids: Vec<String> = self.store.accounts.iter()
            .filter(|a| !a.cookies.is_empty())
            .filter(|a| Self::is_token_expiring_soon(a))
            .map(|a| a.id.clone())
            .collect();

        for id in account_ids {
            match self.refresh_token(&id).await {
                Ok(_) => {
                    println!("[INFO] 自动刷新 Token 成功: {}", id);
                    refreshed.push(id);
                }
                Err(e) => {
                    println!("[WARN] 自动刷新 Token 失败 {}: {}", id, e);
                }
            }
        }
        Ok(refreshed)
    }
}
