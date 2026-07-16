use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;

use super::types::*;
use crate::account::{Account, AccountManager};
use crate::machine;

/// 实例管理器
pub struct InstanceManager {
    store: InstanceStore,
    data_path: PathBuf,
}

impl InstanceManager {
    /// 创建实例管理器，自动执行迁移
    pub fn new(account_manager: &AccountManager) -> Result<Self> {
        let data_path = Self::get_data_path()?;
        let store = if data_path.exists() {
            let content = fs::read_to_string(&data_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            // 首次启动：执行迁移
            Self::migrate_from_accounts(account_manager)?
        };

        Ok(Self { store, data_path })
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

    /// 从 accounts.json 迁移到 instances.json
    fn migrate_from_accounts(account_manager: &AccountManager) -> Result<InstanceStore> {
        println!("[INFO] 首次启动，执行实例迁移...");
        let accounts = account_manager.get_all_accounts();
        let mut instances: Vec<TraeInstance> = Vec::new();

        // 1. 创建默认实例
        #[cfg(target_os = "windows")]
        let default_data_dir = std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join("TRAE SOLO CN").to_string_lossy().to_string())
            .unwrap_or_else(|_| "TRAE SOLO CN".to_string());
        #[cfg(not(target_os = "windows"))]
        let default_data_dir = machine::get_product_data_path(machine::ProductType::TraeSoloCn)
            .to_string_lossy().to_string();

        let mut default_instance = TraeInstance::new(
            "默认".to_string(),
            default_data_dir,
            true,
        );
        // 默认实例绑定当前账号
        default_instance.bound_account_id = account_manager.get_current_account_id();
        instances.push(default_instance);

        // 2. 为有 data_dir 的账号创建多开实例
        for acc in accounts {
            if let Some(data_dir) = &acc.data_dir {
                if !data_dir.is_empty() {
                    let name = acc.note.clone()
                        .unwrap_or_else(|| format!("实例-{}", &acc.user_id));
                    let mut inst = TraeInstance::new(name.clone(), data_dir.clone(), false);
                    inst.bound_account_id = Some(acc.id.clone());
                    inst.machine_id = acc.machine_id.clone();
                    instances.push(inst);
                    println!("[INFO] 迁移实例: {} -> {}", name, data_dir);
                }
            }
        }

        println!("[INFO] 迁移完成，共 {} 个实例", instances.len());
        Ok(InstanceStore { instances })
    }

    /// 获取所有实例（含运行时信息）
    pub fn list_instances(&self, account_manager: &AccountManager) -> Vec<InstanceBrief> {
        let accounts = account_manager.get_all_accounts();
        self.store.instances.iter().map(|inst| {
            let (email, name, avatar) = inst.bound_account_id.as_ref()
                .and_then(|aid| accounts.iter().find(|a| &a.id == aid))
                .map(|a| (Some(a.email.clone()), Some(a.name.clone()), Some(a.avatar_url.clone())))
                .unwrap_or((None, None, None));

            let disk_usage = machine::get_dir_size(&inst.data_dir);
            let (is_running, pid) = machine::is_instance_running(&inst.data_dir);

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
                disk_usage,
                is_running,
                pid,
            }
        }).collect()
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

            std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_script])
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
