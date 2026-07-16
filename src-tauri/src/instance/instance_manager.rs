use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::types::*;
use crate::account::AccountManager;
use crate::machine;

/// 磁盘占用缓存条目: (时间戳秒, 大小字节)
/// 缓存有效期 5 分钟，避免每次轮询都递归遍历整个 data_dir
const DISK_CACHE_TTL_SECS: i64 = 300;

/// 实例管理器
pub struct InstanceManager {
    store: InstanceStore,
    data_path: PathBuf,
    /// data_dir -> (计算时间戳, 大小) 缓存
    disk_cache: HashMap<String, (i64, u64)>,
}

impl InstanceManager {
    /// 创建实例管理器，自动执行迁移
    pub fn new(account_manager: &AccountManager) -> Result<Self> {
        let data_path = Self::get_data_path()?;
        let mut store = if data_path.exists() {
            let content = fs::read_to_string(&data_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            // 首次启动：执行迁移
            Self::migrate_from_accounts(account_manager)?
        };

        // v1.0.10+ 清理：移除未绑定账号的非默认实例（旧的自动迁移脏数据）
        store.instances.retain(|inst| {
            inst.is_default || inst.bound_account_id.is_some()
        });

        // 尝试自动绑定：对每个未绑定账号的实例，检测其 data-dir 是否已登录
        let mut need_save = false;
        for inst in store.instances.iter_mut() {
            if inst.bound_account_id.is_some() {
                continue;
            }
            if let Ok(Some((user_id, _))) = machine::read_trae_login_from_dir(&inst.data_dir) {
                if let Some(account) = account_manager.find_account_by_user_id(&user_id) {
                    inst.bound_account_id = Some(account.id.clone());
                    inst.updated_at = chrono::Utc::now().timestamp();
                    need_save = true;
                    println!("[INFO] 自动绑定实例 '{}' -> 账号 '{}'", inst.name, account.name);
                }
            }
        }
        if need_save {
            let content = serde_json::to_string_pretty(&store)?;
            fs::write(&data_path, content)?;
        }

        Ok(Self { store, data_path, disk_cache: HashMap::new() })
    }

    fn get_data_path() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "trae", "work-cn-manager")
            .ok_or_else(|| anyhow!("无法获取应用数据目录"))?;
        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir)?;
        Ok(data_dir.join("instances.json"))
    }

    fn save_store(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.store)?;
        fs::write(&self.data_path, content)?;
        Ok(())
    }

    /// 从 accounts.json 迁移到 instances.json（首次启动 v1.0.8 时调用）
    /// 只创建默认实例，不自动从旧账号的 data_dir 创建多开实例（避免用户困惑）
    /// 用户如果想保留某个旧多开实例，可以手动用「创建实例」功能
    fn migrate_from_accounts(account_manager: &AccountManager) -> Result<InstanceStore> {
        use super::types::TraeInstance;
        let _accounts = account_manager.get_all_accounts();
        let mut instances: Vec<TraeInstance> = Vec::new();

        // 1. 创建默认实例（指向 %APPDATA%\TRAE SOLO CN）
        #[cfg(target_os = "windows")]
        let default_data_dir = std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join("TRAE SOLO CN").to_string_lossy().to_string())
            .unwrap_or_else(|_| "TRAE SOLO CN".to_string());

        let default_instance = TraeInstance::new(
            "默认".to_string(),
            default_data_dir,
            true,
        );
        // 默认实例不自动绑定账号，由用户自己在「账号管理」页切换
        instances.push(default_instance);

        // 不再自动从旧账号的 data_dir 创建多开实例，避免用户困惑（用户没创建却跑出来）
        // 用户如果想保留某个旧多开实例，可以手动用「创建实例」功能
        println!("[INFO] 迁移完成，创建 1 个默认实例（其他实例由用户手动创建）");
        Ok(InstanceStore { instances })
    }

    /// 获取所有实例的基本信息（不含 disk_usage 和 is_running，快速）
    /// 调用方应在锁外用 compute_runtime_info 计算运行时信息
    pub fn list_instances_basic(&self, account_manager: &AccountManager) -> Vec<InstanceBrief> {
        let accounts = account_manager.get_all_accounts();
        self.store.instances.iter().map(|inst| {
            let (email, name, avatar) = inst.bound_account_id.as_ref()
                .and_then(|aid| accounts.iter().find(|a| &a.id == aid))
                .map(|a| (Some(a.email.clone()), Some(a.name.clone()), Some(a.avatar_url.clone())))
                .unwrap_or((None, None, None));

            InstanceBrief {
                id: inst.id.clone(),
                name: inst.name.clone(),
                data_dir: inst.data_dir.clone(),
                is_default: inst.is_default,
                bound_account_id: inst.bound_account_id.clone(),
                bound_account_email: email,
                bound_account_name: name,
                bound_account_avatar: avatar,
                machine_id: inst.machine_id.clone(),
                created_at: inst.created_at,
                disk_usage: 0,
                is_running: false,
                pid: None,
            }
        }).collect()
    }

    /// 计算实例的运行时信息（is_running 批量检查 + disk_usage 从缓存读）
    /// disk_usage 缓存未命中时返回 0，由后台任务异步计算填充
    /// 应在 spawn_blocking 中调用，避免阻塞 async 运行时
    pub fn compute_runtime_info(&mut self, briefs: &mut [InstanceBrief]) {
        if briefs.is_empty() {
            return;
        }

        // 1. 批量检查所有实例的运行状态（一次 tasklist 加 IMAGENAME 过滤，很快）
        let data_dirs: Vec<String> = briefs.iter().map(|b| b.data_dir.clone()).collect();
        let running_info = machine::check_instances_running_batch(&data_dirs);
        for brief in briefs.iter_mut() {
            if let Some((_, is_running, pid)) = running_info.iter().find(|(dir, _, _)| *dir == brief.data_dir) {
                brief.is_running = *is_running;
                brief.pid = *pid;
            }
        }

        // 2. disk_usage 只从缓存读（未命中返回 0，由后台任务异步填充）
        // 这样首次加载不阻塞，下次轮询时缓存已填好
        for brief in briefs.iter_mut() {
            if let Some((_, cached_size)) = self.disk_cache.get(&brief.data_dir) {
                brief.disk_usage = *cached_size;
            }
            // 缓存未命中，disk_usage 保持 0
        }
    }

    /// 获取缓存未命中或已过期的实例 data_dir 列表（用于后台异步计算）
    pub fn get_uncached_data_dirs(&self, briefs: &[InstanceBrief]) -> Vec<String> {
        let now = chrono::Utc::now().timestamp();
        briefs.iter()
            .filter(|b| {
                match self.disk_cache.get(&b.data_dir) {
                    Some((cached_at, _)) => now - cached_at >= DISK_CACHE_TTL_SECS,
                    None => true,
                }
            })
            .map(|b| b.data_dir.clone())
            .collect()
    }

    /// 同步默认实例的账号绑定（检测 storage.json 的实际登录状态）
    /// 每次 list_instances 和 switch_account 后调用，确保绑定与实际登录一致
    pub fn sync_default_instance_binding(&mut self, account_manager: &crate::account::AccountManager) {
        let default = match self.store.instances.iter_mut()
            .find(|i| i.is_default)
        {
            Some(inst) => inst,
            None => return,
        };

        match crate::machine::read_trae_login_from_dir(&default.data_dir) {
            Ok(Some((user_id, _))) => {
                if let Some(account) = account_manager.find_account_by_user_id(&user_id) {
                    if default.bound_account_id.as_deref() != Some(&account.id) {
                        default.bound_account_id = Some(account.id.clone());
                        default.updated_at = chrono::Utc::now().timestamp();
                        if let Err(e) = self.save_store() {
                            eprintln!("[WARN] 保存默认实例绑定失败: {}", e);
                        } else {
                            println!("[INFO] 同步默认实例绑定: 账号 '{}'", account.name);
                        }
                    }
                } else {
                    if default.bound_account_id.is_some() {
                        default.bound_account_id = None;
                        default.updated_at = chrono::Utc::now().timestamp();
                        let _ = self.save_store();
                        println!("[INFO] 清除默认实例绑定（账号不在管理器中）");
                    }
                }
            }
            Ok(None) => {
                if default.bound_account_id.is_some() {
                    default.bound_account_id = None;
                    default.updated_at = chrono::Utc::now().timestamp();
                    let _ = self.save_store();
                    println!("[INFO] 清除默认实例绑定（storage.json 无登录信息）");
                }
            }
            Err(e) => {
                eprintln!("[WARN] 读取默认实例登录信息失败: {}", e);
            }
        }
    }

    /// 后台计算 disk_usage 并填充缓存（不阻塞，由 list_instances spawn 调用）
    pub fn compute_disk_usage_for_dirs(&mut self, data_dirs: &[String]) {
        let now = chrono::Utc::now().timestamp();
        for dir in data_dirs {
            let size = machine::get_dir_size(dir);
            self.disk_cache.insert(dir.clone(), (now, size));
            println!("[INFO] 磁盘占用计算完成: {} = {} bytes", dir, size);
        }
    }

    /// 强制刷新指定实例的磁盘占用缓存（在删除/创建实例后调用）
    pub fn invalidate_disk_cache(&mut self, data_dir: &str) {
        self.disk_cache.remove(data_dir);
    }

    /// 创建实例
    pub fn create_instance(&mut self, name: String, data_dir: Option<String>, account_id: Option<String>) -> Result<TraeInstance> {
        // 检查名称非空
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("实例名称不能为空"));
        }

        // 确定 data_dir
        let data_dir = match data_dir {
            Some(dir) if !dir.trim().is_empty() => dir.trim().to_string(),
            _ => {
                // 自动生成
                let id = uuid_simple_str();
                #[cfg(target_os = "windows")]
                {
                    let appdata = std::env::var("APPDATA")
                        .map_err(|_| anyhow!("无法获取 APPDATA"))?;
                    PathBuf::from(appdata)
                        .join(format!("TRAE SOLO CN_{}", &id[..8.min(id.len())]))
                        .to_string_lossy().to_string()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    format!("./trae_data_{}", &id[..8.min(id.len())])
                }
            }
        };

        // 检查路径冲突
        if self.store.instances.iter().any(|i| i.data_dir == data_dir) {
            return Err(anyhow!("数据目录已被其他实例占用: {}", data_dir));
        }

        let mut inst = TraeInstance::new(name.to_string(), data_dir, false);
        inst.bound_account_id = account_id;
        self.store.instances.push(inst.clone());
        self.save_store()?;
        println!("[INFO] 已创建实例: {} ({})", inst.name, inst.data_dir);
        Ok(inst)
    }

    /// 删除实例
    pub fn delete_instance(&mut self, id: &str, delete_data: bool) -> Result<()> {
        let pos = self.store.instances.iter().position(|i| i.id == id)
            .ok_or_else(|| anyhow!("实例不存在"))?;

        // 先取出需要的字段，避免借用冲突
        let inst = self.store.instances[pos].clone();

        if inst.is_default {
            return Err(anyhow!("默认实例不可删除"));
        }

        if delete_data {
            let _ = fs::remove_dir_all(&inst.data_dir);
            println!("[INFO] 已删除实例数据目录: {}", inst.data_dir);
        }

        // 失效磁盘缓存
        self.invalidate_disk_cache(&inst.data_dir);

        self.store.instances.remove(pos);
        self.save_store()?;
        println!("[INFO] 已删除实例: {}", inst.name);
        Ok(())
    }

    /// 重命名实例
    pub fn rename_instance(&mut self, id: &str, new_name: &str) -> Result<()> {
        let name = new_name.trim();
        if name.is_empty() {
            return Err(anyhow!("名称不能为空"));
        }
        let inst = self.store.instances.iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow!("实例不存在"))?;
        inst.name = name.to_string();
        inst.updated_at = chrono::Utc::now().timestamp();
        self.save_store()?;
        Ok(())
    }

    /// 绑定账号到实例
    pub fn bind_account(&mut self, instance_id: &str, account_id: Option<&str>, account_manager: &AccountManager) -> Result<()> {
        let inst = self.store.instances.iter_mut()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| anyhow!("实例不存在"))?;

        inst.bound_account_id = account_id.map(|s| s.to_string());
        inst.updated_at = chrono::Utc::now().timestamp();

        // 先 clone data_dir 和 name，避免后续借用冲突
        let data_dir = inst.data_dir.clone();
        let inst_name = inst.name.clone();

        // 如果有绑定账号，写入登录信息到 data-dir
        if let Some(aid) = account_id {
            let account = account_manager.get_account_ref(aid)
                .ok_or_else(|| anyhow!("账号不存在"))?;
            let token = account.jwt_token.as_ref()
                .ok_or_else(|| anyhow!("账号没有有效 Token"))?;

            let login_info = machine::TraeLoginInfo {
                token: token.clone(),
                refresh_token: None,
                user_id: account.user_id.clone(),
                email: account.email.clone(),
                username: account.name.clone(),
                avatar_url: account.avatar_url.clone(),
                host: String::new(),
                region: if account.region.is_empty() { "CN".to_string() } else { account.region.clone() },
            };

            // 共享插件目录
            #[cfg(target_os = "windows")]
            let shared_ext = std::env::var("APPDATA")
                .ok()
                .map(|p| PathBuf::from(p).join("TRAE SOLO CN_SharedExtensions").to_string_lossy().to_string());
            #[cfg(not(target_os = "windows"))]
            let shared_ext = None;

            machine::launch_product_multi(
                &login_info,
                account.machine_id.as_deref(),
                &data_dir,
                shared_ext.as_deref(),
            )?;
        }

        self.save_store()?;
        println!("[INFO] 已绑定账号到实例: {}", inst_name);
        Ok(())
    }

    /// 启动实例（不写入登录信息，仅启动）
    pub fn launch_instance(&self, id: &str) -> Result<bool> {
        let inst = self.store.instances.iter()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow!("实例不存在"))?;

        let (is_running, _pid) = machine::is_instance_running(&inst.data_dir);

        // 共享插件目录
        #[cfg(target_os = "windows")]
        let shared_ext = std::env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("TRAE SOLO CN_SharedExtensions").to_string_lossy().to_string());
        #[cfg(not(target_os = "windows"))]
        let shared_ext = None;

        machine::open_product_with_data_dir(
            machine::ProductType::TraeSoloCn,
            &inst.data_dir,
            shared_ext.as_deref(),
        )?;

        Ok(is_running) // 返回启动前是否已在运行
    }

    /// 打开实例数据目录
    pub fn open_instance_data_dir(&self, id: &str) -> Result<()> {
        let inst = self.store.instances.iter()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow!("实例不存在"))?;

        fs::create_dir_all(&inst.data_dir)?;

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(&inst.data_dir)
                .spawn()?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new("open")
                .arg(&inst.data_dir)
                .spawn()?;
        }
        Ok(())
    }

    /// 创建桌面快捷方式
    pub fn create_instance_shortcut(&self, id: &str) -> Result<String> {
        let inst = self.store.instances.iter()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow!("实例不存在"))?;

        let exe_path = machine::get_saved_product_path(machine::ProductType::TraeSoloCn)
            .map_err(|_| anyhow!("未设置 TRAE 路径，请先在设置中扫描"))?;

        #[cfg(target_os = "windows")]
        {
            let desktop = std::env::var("USERPROFILE")
                .map(|p| PathBuf::from(p).join("Desktop").to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());
            let shortcut_name = format!("TRAE Work CN - {}.lnk", inst.name);
            let shortcut_path = PathBuf::from(&desktop).join(&shortcut_name);

            // 用 PowerShell 创建快捷方式
            let arg_str = format!("--user-data-dir=\"{}\"", inst.data_dir);
            let ps_script = format!(
                "$ws = New-Object -ComObject WScript.Shell; \
                 $s = $ws.CreateShortcut('{}'); \
                 $s.TargetPath = '{}'; \
                 $s.Arguments = '{}'; \
                 $s.IconLocation = '{}'; \
                 $s.WorkingDirectory = '{}'; \
                 $s.Save()",
                shortcut_path.to_string_lossy(),
                exe_path.replace('\'', "''"),
                arg_str.replace('\'', "''"),
                exe_path.replace('\'', "''"),
                PathBuf::from(&exe_path).parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default().replace('\'', "''")
            );

            machine::hide_window(std::process::Command::new("powershell"))
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
                .output()?;

            println!("[INFO] 已创建快捷方式: {}", shortcut_path.display());
            return Ok(shortcut_path.to_string_lossy().to_string());
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("此功能仅支持 Windows"))
        }
    }
}

fn uuid_simple_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}
