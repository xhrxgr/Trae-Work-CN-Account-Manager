mod api;
mod account;
mod instance;
mod machine;
mod login;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

use account::{AccountBrief, AccountManager, Account, BrowserUserInfo};
use api::UsageSummary;
use instance::{InstanceBrief, InstanceManager, TraeInstance};

/// 应用状态
pub struct AppState {
    pub account_manager: Arc<Mutex<AccountManager>>,
    pub instance_manager: Arc<Mutex<InstanceManager>>,
}

/// 错误类型
#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    pub message: String,
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

type Result<T> = std::result::Result<T, ApiError>;

// ============ Tauri 命令 ============

/// 添加账号（通过 Token，可选 Cookies）
/// source: "browser"(浏览器登录), "local"(本地读取), "manual"(手动输入)
#[tauri::command]
async fn add_account_by_token(
    token: String,
    cookies: Option<String>,
    source: Option<String>,
    browser_user_info: Option<BrowserUserInfo>,
    state: State<'_, AppState>,
) -> Result<Account> {
    let mut manager = state.account_manager.lock().await;
    let src = source.unwrap_or_else(|| "manual".to_string());
    manager.add_account_by_token(token, cookies, src, browser_user_info).await.map_err(Into::into)
}

/// 删除账号
#[tauri::command]
async fn remove_account(account_id: String, state: State<'_, AppState>) -> Result<()> {
    let mut manager = state.account_manager.lock().await;
    manager.remove_account(&account_id).map_err(Into::into)
}

/// 获取所有账号
#[tauri::command]
async fn get_accounts(state: State<'_, AppState>) -> Result<Vec<AccountBrief>> {
    let manager = state.account_manager.lock().await;
    Ok(manager.get_accounts())
}

/// 获取单个账号详情
#[tauri::command]
async fn get_account(account_id: String, state: State<'_, AppState>) -> Result<Account> {
    let manager = state.account_manager.lock().await;
    manager.get_account(&account_id).map_err(Into::into)
}

/// 更新账号备注
#[tauri::command]
async fn update_account_note(account_id: String, note: Option<String>, state: State<'_, AppState>) -> Result<()> {
    let mut manager = state.account_manager.lock().await;
    manager.update_account_note(&account_id, note).map_err(Into::into)
}

/// 切换账号（设置活跃账号并写入 TRAE Work CN 登录信息）
#[tauri::command]
async fn switch_account(account_id: String, state: State<'_, AppState>) -> Result<()> {
    let mut manager = state.account_manager.lock().await;
    manager.switch_account(&account_id).map_err(Into::into)
}

/// 多开模式：为指定账号启动独立的 TRAE Work CN 实例
#[tauri::command]
async fn launch_account_multi(account_id: String, state: State<'_, AppState>) -> Result<()> {
    let mut manager = state.account_manager.lock().await;
    manager.launch_account_multi(&account_id).map_err(Into::into)
}

// ============ 实例管理命令 ============

/// 获取所有实例（快速返回基本信息 + is_running 批量检查 + disk_usage 异步后台计算）
#[tauri::command]
async fn list_instances(state: State<'_, AppState>) -> Result<Vec<InstanceBrief>> {
    // 1. 快速获取基本信息 + is_running + 缓存的 disk_usage（持有锁时间短）
    let (briefs, uncached_dirs) = {
        let account_manager = state.account_manager.lock().await;
        let mut instance_manager = state.instance_manager.lock().await;
        let briefs = instance_manager.list_instances_basic(&account_manager);
        let mut briefs = briefs;
        // compute_runtime_info 只做 is_running 批量检查 + 读缓存 disk_usage（不阻塞）
        instance_manager.compute_runtime_info(&mut briefs);
        let uncached = instance_manager.get_uncached_data_dirs(&briefs);
        (briefs, uncached)
    };

    // 2. 对缓存未命中的实例，spawn 后台任务计算 disk_usage（不阻塞当前请求）
    //    下次轮询时缓存已填好，用户看到磁盘占用
    if !uncached_dirs.is_empty() {
        let instance_manager = state.instance_manager.clone();
        tokio::task::spawn_blocking(move || {
            let mut manager = instance_manager.blocking_lock();
            manager.compute_disk_usage_for_dirs(&uncached_dirs);
        });
        // 故意不 await，让它在后台运行
    }

    Ok(briefs)
}

/// 创建实例
#[tauri::command]
async fn create_instance(
    name: String,
    data_dir: Option<String>,
    account_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<TraeInstance> {
    let mut manager = state.instance_manager.lock().await;
    manager.create_instance(name, data_dir, account_id).map_err(Into::into)
}

/// 删除实例
#[tauri::command]
async fn delete_instance(
    instance_id: String,
    delete_data: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    let mut manager = state.instance_manager.lock().await;
    manager.delete_instance(&instance_id, delete_data).map_err(Into::into)
}

/// 重命名实例
#[tauri::command]
async fn rename_instance(
    instance_id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let mut manager = state.instance_manager.lock().await;
    manager.rename_instance(&instance_id, &new_name).map_err(Into::into)
}

/// 绑定账号到实例（写入登录信息）
#[tauri::command]
async fn bind_account_to_instance(
    instance_id: String,
    account_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    let account_manager = state.account_manager.lock().await;
    let mut manager = state.instance_manager.lock().await;
    manager.bind_account(&instance_id, account_id.as_deref(), &account_manager).map_err(Into::into)
}

/// 启动实例
#[tauri::command]
async fn launch_instance(instance_id: String, state: State<'_, AppState>) -> Result<bool> {
    let manager = state.instance_manager.lock().await;
    manager.launch_instance(&instance_id).map_err(Into::into)
}

/// 打开实例数据目录
#[tauri::command]
async fn open_instance_data_dir(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    let manager = state.instance_manager.lock().await;
    manager.open_instance_data_dir(&instance_id).map_err(Into::into)
}

/// 创建实例桌面快捷方式
#[tauri::command]
async fn create_instance_shortcut(instance_id: String, state: State<'_, AppState>) -> Result<String> {
    let manager = state.instance_manager.lock().await;
    manager.create_instance_shortcut(&instance_id).map_err(Into::into)
}

/// 导出所有账号为 JSON 字符串
#[tauri::command]
async fn export_accounts(state: State<'_, AppState>) -> Result<String> {
    let manager = state.account_manager.lock().await;
    manager.export_accounts().map_err(Into::into)
}

/// 导出所有账号到指定文件
#[tauri::command]
async fn export_accounts_to_file(file_path: String, state: State<'_, AppState>) -> Result<()> {
    let manager = state.account_manager.lock().await;
    let json = manager.export_accounts()?;
    std::fs::write(&file_path, json)
        .map_err(|e| anyhow::anyhow!("写入导出文件失败: {}", e))?;
    println!("[INFO] 账号已导出到: {}", file_path);
    Ok(())
}

/// 从 JSON 字符串导入账号
/// overwrite=true: 替换所有账号; overwrite=false: 合并（跳过已存在的 user_id）
#[tauri::command]
async fn import_accounts(json_str: String, overwrite: bool, state: State<'_, AppState>) -> Result<usize> {
    let mut manager = state.account_manager.lock().await;
    manager.import_accounts(&json_str, overwrite).map_err(Into::into)
}

/// 从指定文件导入账号
#[tauri::command]
async fn import_accounts_from_file(file_path: String, overwrite: bool, state: State<'_, AppState>) -> Result<usize> {
    let json = std::fs::read_to_string(&file_path)
        .map_err(|e| anyhow::anyhow!("读取导入文件失败: {}", e))?;
    let mut manager = state.account_manager.lock().await;
    manager.import_accounts(&json, overwrite).map_err(Into::into)
}

/// 获取账号使用量
#[tauri::command]
async fn get_account_usage(account_id: String, state: State<'_, AppState>) -> Result<UsageSummary> {
    let mut manager = state.account_manager.lock().await;
    manager.get_account_usage(&account_id).await.map_err(Into::into)
}

/// 更新账号 Token
#[tauri::command]
async fn update_account_token(account_id: String, token: String, state: State<'_, AppState>) -> Result<UsageSummary> {
    let mut manager = state.account_manager.lock().await;
    manager.update_account_token(&account_id, token).await.map_err(Into::into)
}

/// 获取当前系统机器码
#[tauri::command]
async fn get_machine_id() -> Result<String> {
    machine::get_machine_guid().map_err(Into::into)
}

/// 重置系统机器码（生成新的随机机器码）
#[tauri::command]
async fn reset_machine_id() -> Result<String> {
    machine::reset_machine_guid().map_err(Into::into)
}

/// 设置系统机器码为指定值
#[tauri::command]
async fn set_machine_id(machine_id: String) -> Result<()> {
    machine::set_machine_guid(&machine_id).map_err(Into::into)
}

/// 绑定账号机器码（保存当前系统机器码到账号）
#[tauri::command]
async fn bind_account_machine_id(account_id: String, state: State<'_, AppState>) -> Result<String> {
    let mut manager = state.account_manager.lock().await;
    manager.bind_machine_id(&account_id).map_err(Into::into)
}

// ============ TRAE Work CN 相关命令 ============

/// 从 Trae Solo CN 读取当前登录账号
#[tauri::command]
async fn read_solo_cn_account(state: State<'_, AppState>) -> Result<Option<Account>> {
    let mut manager = state.account_manager.lock().await;
    manager.read_solo_cn_account().await.map_err(Into::into)
}

/// 获取 Trae Solo CN 的机器码
#[tauri::command]
async fn get_solo_cn_machine_id() -> Result<String> {
    machine::get_solo_cn_machine_id().map_err(Into::into)
}

/// 设置 Trae Solo CN 的机器码
#[tauri::command]
async fn set_solo_cn_machine_id(machine_id: String) -> Result<()> {
    machine::set_solo_cn_machine_id(&machine_id).map_err(Into::into)
}

/// 清除 Trae Solo CN 登录状态
#[tauri::command]
async fn clear_solo_cn_login_state() -> Result<()> {
    machine::clear_solo_cn_login_state().map_err(Into::into)
}

/// 获取保存的 Trae Solo CN 路径
#[tauri::command]
async fn get_solo_cn_path() -> Result<String> {
    machine::get_saved_solo_cn_path().map_err(Into::into)
}

/// 设置 Trae Solo CN 路径
#[tauri::command]
async fn set_solo_cn_path(path: String) -> Result<()> {
    machine::save_solo_cn_path(&path).map_err(Into::into)
}

/// 自动扫描 TRAE Work CN 路径
#[tauri::command]
async fn scan_solo_cn_path() -> Result<String> {
    machine::scan_solo_cn_path().map_err(Into::into)
}

/// 刷新单个账号 Token
#[tauri::command]
async fn refresh_token(account_id: String, state: State<'_, AppState>) -> Result<()> {
    let mut manager = state.account_manager.lock().await;
    manager.refresh_token(&account_id).await.map_err(Into::into)
}

/// 批量刷新所有即将过期的 Token
#[tauri::command]
async fn refresh_all_tokens(state: State<'_, AppState>) -> Result<Vec<String>> {
    let mut manager = state.account_manager.lock().await;
    manager.refresh_all_tokens().await.map_err(Into::into)
}

/// 浏览器登录
#[tauri::command]
async fn start_browser_login(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<()> {
    let manager = state.account_manager.clone();
    login::start_login_flow(app, manager).await.map_err(|e| ApiError { message: e })?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let account_manager = AccountManager::new().expect("无法初始化账号管理器");
    let instance_manager = InstanceManager::new(&account_manager).expect("无法初始化实例管理器");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            account_manager: Arc::new(Mutex::new(account_manager)),
            instance_manager: Arc::new(Mutex::new(instance_manager)),
        })
        .invoke_handler(tauri::generate_handler![
            add_account_by_token,
            remove_account,
            get_accounts,
            get_account,
            update_account_note,
            switch_account,
            launch_account_multi,
            // 实例管理
            list_instances,
            create_instance,
            delete_instance,
            rename_instance,
            bind_account_to_instance,
            launch_instance,
            open_instance_data_dir,
            create_instance_shortcut,
            export_accounts,
            export_accounts_to_file,
            import_accounts,
            import_accounts_from_file,
            get_account_usage,
            update_account_token,
            get_machine_id,
            reset_machine_id,
            set_machine_id,
            bind_account_machine_id,
            refresh_token,
            refresh_all_tokens,
            start_browser_login,
            read_solo_cn_account,
            get_solo_cn_machine_id,
            set_solo_cn_machine_id,
            clear_solo_cn_login_state,
            get_solo_cn_path,
            set_solo_cn_path,
            scan_solo_cn_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
