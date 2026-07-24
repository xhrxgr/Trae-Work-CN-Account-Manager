use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::types::*;
use crate::account::AccountManager;
use crate::machine;

/// 磁盘占用缓存条目: (时间戳秒, 大小字节)
/// 缓存有效期 5 分钟，避免每次轮询都递归遍历整个 data_dir
const DISK_CACHE_TTL_SECS: i64 = 300;

/// 安全清理候选项：仅包含 100% 删除无害的文件/目录（均为缓存、日志、崩溃转储，可自动重建）
/// 用户可在弹窗中勾选要删除的项
#[derive(Serialize, Clone)]
pub struct SafeCleanItem {
    /// 唯一标识，传回后端用于指定删除目标
    pub key: String,
    /// 显示名称
    pub label: String,
    /// 相对于 data-dir 的路径
    pub path: String,
    /// 分类: "cache"(缓存) | "crash"(崩溃转储) | "logs"(日志)
    pub category: String,
    /// 当前占用字节数
    pub size_bytes: u64,
    /// 说明：删什么、为何安全
    pub description: String,
}

/// 实例管理器
pub struct InstanceManager {
    store: InstanceStore,
    data_path: PathBuf,
    /// data_dir -> (计算时间戳, 大小) 缓存
    disk_cache: HashMap<String, (i64, u64)>,
    /// 上一次轮询时各实例的运行状态（instance_id -> was_running）
    /// 用于检测「运行→停止」转换，记录 last_closed_at
    /// 应用刚启动时此 map 为空，首次轮询只初始化状态不记录关闭
    prev_running: HashMap<String, bool>,
}

impl InstanceManager {
    /// 创建实例管理器，自动执行迁移 + 自动发现 + 自动绑定
    pub fn new(account_manager: &mut AccountManager) -> Result<Self> {
        let data_path = Self::get_data_path()?;
        let mut store = if data_path.exists() {
            let content = fs::read_to_string(&data_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            // 首次启动：执行迁移
            Self::migrate_from_accounts(account_manager)?
        };

        // v1.0.16+ 修复：移除 v1.0.11 引入的 retain 过滤逻辑
        // 该逻辑会删除用户手动创建但还没绑定账号的实例，导致"重启后实例消失"的 bug
        // 用户创建的实例（无论是否绑定账号）都应保留

        // 自动绑定：扫描每个实例的 data-dir，根据 storage.json 中的 user_id 匹配账号
        // 如果 storage.json 中登录的账号不在本地账号列表，自动创建新账号（source="local"）
        Self::auto_bind_accounts(&mut store, account_manager);

        // 自动发现：扫描 %APPDATA% 下的 TRAE SOLO CN* 文件夹，把不在列表中的加入
        let discovered = Self::auto_discover_instances(&mut store);
        if discovered > 0 {
            println!("[INFO] 自动发现并添加了 {} 个新实例", discovered);
            // 保存到磁盘
            let _ = fs::write(&data_path, serde_json::to_string_pretty(&store)?);
        }

        Ok(Self { store, data_path, disk_cache: HashMap::new(), prev_running: HashMap::new() })
    }

    /// 自动绑定账号：扫描每个实例的 data-dir，根据 storage.json 中的 user_id 匹配本地账号
    /// 处理场景：
    /// - 用户在 IDE 内手动登录了账号，但实例未绑定
    /// - bound_account_id 失效（账号被重新添加导致 ID 变化）
    /// - 默认实例从未绑定
    /// - **本地账号列表中没有匹配账号 → 自动从 storage.json 创建新账号（v1.0.20+）**
    fn auto_bind_accounts(store: &mut InstanceStore, account_manager: &mut AccountManager) {
        let mut changed = false;

        // 收集所有需要处理的实例（避免在循环中借用 store 又调用 account_manager）
        // 每个元素: (instance_id, data_dir, current_bound_id, login_info)
        let mut to_process = Vec::new();
        for inst in store.instances.iter() {
            let login_info = match machine::read_trae_login_from_dir(&inst.data_dir) {
                Ok(Some(info)) if !info.user_id.is_empty() => info,
                _ => continue,
            };
            to_process.push((inst.id.clone(), inst.name.clone(), inst.bound_account_id.clone(), login_info));
        }

        for (inst_id, inst_name, current_bound_id, login_info) in to_process {
            // 检查当前 bound_account_id 是否仍然有效
            let current_valid = current_bound_id
                .as_ref()
                .and_then(|aid| account_manager.get_account_ref(aid))
                .map(|a| a.user_id == login_info.user_id)
                .unwrap_or(false);

            if current_valid {
                continue; // 已正确绑定，跳过
            }

            // 在 accounts 中找匹配的 user_id
            let matched_account_id = account_manager
                .find_account_by_user_id(&login_info.user_id)
                .map(|a| a.id.clone());

            let new_account_id = match matched_account_id {
                Some(id) => {
                    // 已存在匹配账号，直接绑定
                    if current_bound_id.as_deref() == Some(&id) {
                        continue;
                    }
                    println!(
                        "[INFO] 实例 '{}' 自动绑定到已有账号 (user_id={})",
                        inst_name, login_info.user_id
                    );
                    id
                }
                None => {
                    // 本地没有匹配账号，自动从 storage.json 创建新账号
                    match account_manager.add_account_from_local_login(
                        login_info.user_id.clone(),
                        login_info.token.clone(),
                        login_info.refresh_token.clone(),
                        login_info.email.clone(),
                        login_info.username.clone(),
                        login_info.avatar_url.clone(),
                        login_info.region.clone(),
                    ) {
                        Ok(new_account) => {
                            println!(
                                "[INFO] 实例 '{}' 自动创建新账号: {} (user_id={})",
                                inst_name, new_account.name, login_info.user_id
                            );
                            new_account.id
                        }
                        Err(e) => {
                            println!(
                                "[WARN] 实例 '{}' 自动创建账号失败: {}",
                                inst_name, e
                            );
                            continue;
                        }
                    }
                }
            };

            // 找到实例并更新 bound_account_id
            if let Some(inst) = store.instances.iter_mut().find(|i| i.id == inst_id) {
                inst.bound_account_id = Some(new_account_id);
                inst.updated_at = chrono::Utc::now().timestamp();
                changed = true;
            }
        }

        if changed {
            let _ = fs::write(
                Self::get_data_path().unwrap_or_else(|_| PathBuf::from("instances.json")),
                serde_json::to_string_pretty(store).unwrap_or_default(),
            );
        }
    }

    /// 自动发现：扫描 %APPDATA% 下的 TRAE SOLO CN* 文件夹
    /// 把有 storage.json 但不在 instances 列表中的文件夹加入为新实例
    /// 返回新增实例数量
    fn auto_discover_instances(store: &mut InstanceStore) -> usize {
        #[cfg(target_os = "windows")]
        let appdata = match std::env::var("APPDATA") {
            Ok(v) => PathBuf::from(v),
            Err(_) => return 0,
        };
        #[cfg(not(target_os = "windows"))]
        let appdata = match std::env::var("HOME") {
            Ok(v) => PathBuf::from(v).join("Library").join("Application Support"),
            Err(_) => return 0,
        };

        let entries = match fs::read_dir(&appdata) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        let default_data_dir = appdata.join("TRAE SOLO CN");
        let mut added = 0;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            // 匹配 TRAE SOLO CN 或 TRAE SOLO CN_xxx
            if name != "TRAE SOLO CN" && !name.starts_with("TRAE SOLO CN_") {
                continue;
            }
            // 必须有 storage.json 才认为是有效实例
            let storage_path = path.join("User").join("globalStorage").join("storage.json");
            if !storage_path.exists() {
                continue;
            }
            // 排除共享插件目录
            if name == "TRAE SOLO CN_SharedExtensions" {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();

            // 已存在则跳过
            if store.instances.iter().any(|i| i.data_dir == path_str) {
                continue;
            }

            // 必须有有效登录账号（user_id 非空）才自动发现
            // 避免空测试目录、无登录的 data-dir 被误加入实例列表
            let login_info = machine::read_trae_login_from_dir(&path_str)
                .ok()
                .flatten();

            let login_info = match login_info {
                Some(ref info) if !info.user_id.is_empty() => login_info,
                _ => {
                    // 没有有效登录账号，跳过此目录
                    continue;
                }
            };

            // 推断实例名：优先用 username，回退到 user_id 前缀
            let inferred_name = login_info
                .and_then(|info| {
                    if !info.username.is_empty() {
                        Some(info.username.clone())
                    } else {
                        Some(format!("用户_{}", &info.user_id[..6.min(info.user_id.len())]))
                    }
                })
                .unwrap_or_else(|| {
                    // 用目录后缀作为名称（理论上不会走到这里，因为上面已过滤）
                    if let Some(suffix) = name.strip_prefix("TRAE SOLO CN_") {
                        format!("实例_{}", &suffix[..8.min(suffix.len())])
                    } else {
                        "默认".to_string()
                    }
                });

            let is_default = path == default_data_dir;
            let mut new_inst = crate::instance::types::TraeInstance::new(inferred_name, path_str, is_default);
            new_inst.note = Some("自动发现".to_string());
            store.instances.push(new_inst);
            added += 1;
        }

        added
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
            let (email, name, avatar, account_note) = inst.bound_account_id.as_ref()
                .and_then(|aid| accounts.iter().find(|a| &a.id == aid))
                .map(|a| (Some(a.email.clone()), Some(a.name.clone()), Some(a.avatar_url.clone()), a.note.clone()))
                .unwrap_or((None, None, None, None));

            InstanceBrief {
                id: inst.id.clone(),
                name: inst.name.clone(),
                data_dir: inst.data_dir.clone(),
                is_default: inst.is_default,
                bound_account_id: inst.bound_account_id.clone(),
                bound_account_email: email,
                bound_account_name: name,
                bound_account_avatar: avatar,
                bound_account_note: account_note,
                note: inst.note.clone(),
                machine_id: inst.machine_id.clone(),
                created_at: inst.created_at,
                last_launched_at: inst.last_launched_at,
                last_closed_at: inst.last_closed_at,
                disk_usage: 0,
                is_running: false,
                pid: None,
                account_status: None,
            }
        }).collect()
    }

    /// 计算实例的运行时信息（is_running 批量检查 + disk_usage 从缓存读 + account_status 从 storage.json 读）
    /// disk_usage 缓存未命中时返回 0，由后台任务异步计算填充
    /// 应在 spawn_blocking 中调用，避免阻塞 async 运行时
    /// 同时检测「运行→停止」转换，记录 last_closed_at 并持久化
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

        // 1.5 检测「运行→停止」转换，记录 last_closed_at
        // 应用刚启动时 prev_running 为空，首次轮询只初始化状态不记录关闭（避免误把"启动前就停止"的实例当作刚关闭）
        // 仅当 prev_running 中有该实例且值为 true、当前为 false 时才记录关闭
        let now = chrono::Utc::now().timestamp();
        // 第一遍：只读，收集发生关闭的实例 id，并更新 prev_running
        let mut closed_ids: Vec<String> = Vec::new();
        for brief in briefs.iter() {
            let was_running = self.prev_running.get(&brief.id).copied();
            if matches!(was_running, Some(true)) && !brief.is_running {
                closed_ids.push(brief.id.clone());
            }
            // 更新 prev_running 为当前状态
            self.prev_running.insert(brief.id.clone(), brief.is_running);
        }
        // 第二遍：可变更新 briefs 的 last_closed_at，让前端本次轮询就能看到
        if !closed_ids.is_empty() {
            for b in briefs.iter_mut() {
                if closed_ids.iter().any(|id| id == &b.id) {
                    b.last_closed_at = now;
                }
            }
            // 同步写入 store.instances 并持久化
            let mut changed = false;
            for id in &closed_ids {
                if let Some(inst) = self.store.instances.iter_mut().find(|i| &i.id == id) {
                    inst.last_closed_at = now;
                    inst.updated_at = now;
                    changed = true;
                }
            }
            if changed {
                if let Err(e) = self.save_store() {
                    println!("[WARN] 保存 last_closed_at 失败: {}", e);
                } else {
                    println!("[INFO] 检测到 {} 个实例关闭，已记录 last_closed_at", closed_ids.len());
                }
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

        // 3. account_status：从 data-dir 的 storage.json 读取 iCubeServerData
        // 这是同步文件 IO，但只读取单个 JSON 文件，速度可接受（每个文件几 KB）
        for brief in briefs.iter_mut() {
            brief.account_status = match machine::read_instance_account_status(&brief.data_dir) {
                Ok(s) => s,
                Err(_) => None,
            };
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

    /// 获取指定实例的 data_dir（用于 API 调用等外部操作）
    pub fn get_instance_data_dir(&self, id: &str) -> Option<String> {
        self.store.instances.iter()
            .find(|i| i.id == id)
            .map(|i| i.data_dir.clone())
    }

    /// 获取实例可安全清理的候选项列表（仅 100% 无害的缓存/日志/崩溃转储）
    /// 每项包含当前占用大小，供前端弹窗展示并让用户勾选
    pub fn get_safe_clean_items(&self, id: &str) -> Result<Vec<SafeCleanItem>> {
        let data_dir = self.store.instances.iter()
            .find(|i| i.id == id)
            .map(|i| i.data_dir.clone())
            .ok_or_else(|| anyhow!("实例不存在"))?;

        // 候选清单：(key, label, 相对路径, 分类, 说明)
        // 以下均为 TRAE/VSCode/Chromium 运行时自动重建的缓存、日志、崩溃转储，
        // 删除后下次启动会重新生成，不影响登录信息、用户设置、插件数据、聊天记录
        let candidates: &[(&str, &str, &str, &str, &str)] = &[
            ("logs", "运行日志", "logs", "logs",
             "每次启动 TRAE 时重新生成。包含渲染进程、扩展宿主、网络、MCP 等运行日志"),
            ("cache_cache", "网络缓存", "Cache", "cache",
             "Chromium 网络磁盘缓存，启动时自动重建"),
            ("cache_code_cache", "代码缓存", "Code Cache", "cache",
             "V8 编译代码缓存，启动时自动重建"),
            ("cache_cached_data", "缓存数据", "CachedData", "cache",
             "VSCode 缓存的 GPU 加速数据，启动时自动重建（通常占空间最大）"),
            ("cache_gpu", "GPU 缓存", "GPUCache", "cache",
             "Chromium GPU 缓存，启动时自动重建"),
            ("cache_dawn_graphite", "Dawn Graphite 缓存", "DawnGraphiteCache", "cache",
             "Dawn GPU 渲染缓存，启动时自动重建"),
            ("cache_dawn_webgpu", "WebGPU 缓存", "DawnWebGPUCache", "cache",
             "WebGPU 缓存，启动时自动重建"),
            ("cache_video_decode", "视频解码统计", "VideoDecodeStats", "cache",
             "视频解码统计信息，启动时自动重建"),
            ("crash_reports", "崩溃转储", "Crashpad/reports", "crash",
             "已收集的崩溃转储文件 (.dmp)，仅供排查问题，删除无任何影响"),
        ];

        let base = PathBuf::from(&data_dir);
        let mut items = Vec::new();
        for (key, label, rel, category, desc) in candidates {
            let p = base.join(rel);
            let size = if p.exists() { machine::get_dir_size(p.to_str().unwrap_or("")) } else { 0 };
            items.push(SafeCleanItem {
                key: key.to_string(),
                label: label.to_string(),
                path: rel.to_string(),
                category: category.to_string(),
                size_bytes: size,
                description: desc.to_string(),
            });
        }
        Ok(items)
    }

    /// 执行安全清理：删除用户勾选的候选项，返回释放的总字节数
    /// 若实例正在运行则拒绝（文件可能被占用），要求用户先关闭实例
    pub fn safe_clean_instance(&mut self, id: &str, keys: &[String]) -> Result<u64> {
        let data_dir = self.store.instances.iter()
            .find(|i| i.id == id)
            .map(|i| i.data_dir.clone())
            .ok_or_else(|| anyhow!("实例不存在"))?;

        // 检查实例是否运行中（运行时缓存文件可能被锁，拒绝清理）
        let (is_running, _) = machine::is_instance_running(&data_dir);
        if is_running {
            return Err(anyhow!("实例正在运行中，请先关闭实例再清理（运行时缓存文件可能被占用）"));
        }

        // 候选 key → 相对路径映射（与 get_safe_clean_items 保持一致）
        let key_to_path: &[(&str, &str)] = &[
            ("logs", "logs"),
            ("cache_cache", "Cache"),
            ("cache_code_cache", "Code Cache"),
            ("cache_cached_data", "CachedData"),
            ("cache_gpu", "GPUCache"),
            ("cache_dawn_graphite", "DawnGraphiteCache"),
            ("cache_dawn_webgpu", "DawnWebGPUCache"),
            ("cache_video_decode", "VideoDecodeStats"),
            ("crash_reports", "Crashpad/reports"),
        ];
        let base = PathBuf::from(&data_dir);
        let mut freed: u64 = 0;
        let mut errors: Vec<String> = Vec::new();

        for key in keys {
            let rel = match key_to_path.iter().find(|(k, _)| k == key) {
                Some((_, r)) => *r,
                None => {
                    errors.push(format!("未知清理项: {}", key));
                    continue;
                }
            };
            let p = base.join(rel);
            if !p.exists() {
                continue; // 不存在则跳过
            }
            // 先计算大小（删除后就拿不到了）
            let size = machine::get_dir_size(p.to_str().unwrap_or(""));
            match fs::remove_dir_all(&p) {
                Ok(_) => {
                    freed += size;
                    println!("[INFO] 安全清理: 已删除 {} ({} 字节)", p.display(), size);
                }
                Err(e) => {
                    errors.push(format!("{}: {}", rel, e));
                }
            }
        }

        // 失效该实例的磁盘缓存，让下次轮询重新计算真实大小
        self.invalidate_disk_cache(&data_dir);

        if !errors.is_empty() {
            // 部分失败也返回已释放的大小，但附带错误信息
            return Err(anyhow!("已释放 {} 字节，但部分项删除失败:\n{}", freed, errors.join("\n")));
        }
        Ok(freed)
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
            match fs::remove_dir_all(&inst.data_dir) {
                Ok(_) => println!("[INFO] 已删除实例数据目录: {}", inst.data_dir),
                Err(e) => {
                    // 目录可能被占用（实例运行中）或部分文件被锁
                    let kind = e.kind();
                    let msg = if kind == std::io::ErrorKind::PermissionDenied || kind == std::io::ErrorKind::Other {
                        format!("删除数据目录失败（可能实例正在运行中，请先关闭实例再删除）。路径: {}\n错误: {}", inst.data_dir, e)
                    } else {
                        format!("删除数据目录失败。路径: {}\n错误: {}", inst.data_dir, e)
                    };
                    return Err(anyhow!(msg));
                }
            }
        }

        // 失效磁盘缓存
        self.invalidate_disk_cache(&inst.data_dir);

        self.store.instances.remove(pos);
        self.save_store()?;
        println!("[INFO] 已删除实例: {}", inst.name);
        Ok(())
    }

    /// 重命名实例（同步更新 window.title 设置）
    pub fn rename_instance(&mut self, id: &str, new_name: &str) -> Result<()> {
        let name = new_name.trim();
        if name.is_empty() {
            return Err(anyhow!("名称不能为空"));
        }
        let inst = self.store.instances.iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow!("实例不存在"))?;
        let old_name = inst.name.clone();
        inst.name = name.to_string();
        inst.updated_at = chrono::Utc::now().timestamp();
        let data_dir = inst.data_dir.clone();
        let new_inst_name = inst.name.clone();
        // 释放可变借用，后续需要不可变借用（save_store + write_window_title）
        let _ = inst;

        // 同步更新 data-dir 的 window.title
        if data_dir.as_str() != "TRAE SOLO CN" {
            let title = format!("{} - TRAE Work CN", new_inst_name);
            let _ = machine::write_window_title_to_dir(&data_dir, &title);
        }

        self.save_store()?;
        println!("[INFO] 已重命名实例: {} -> {}", old_name, new_inst_name);
        Ok(())
    }

    /// 更新实例备注（仅写入 instances.json，不影响 data-dir）
    pub fn update_instance_note(&mut self, id: &str, note: Option<String>) -> Result<()> {
        let inst = self.store.instances.iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow!("实例不存在"))?;
        inst.note = note;
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
                refresh_token: account.refresh_token.clone(),
                user_id: account.user_id.clone(),
                email: account.email.clone(),
                username: account.name.clone(),
                avatar_url: account.avatar_url.clone(),
                host: String::new(),
                region: if account.region.is_empty() { "CN".to_string() } else { account.region.clone() },
            };

            machine::write_login_info_to_dir(
                &login_info,
                account.machine_id.as_deref(),
                &data_dir,
            )?;

            // 设置窗口标题（通过 settings.json 的 window.title，持久化且跨重启）
            let title = format!("{} - TRAE Work CN", inst_name);
            let _ = machine::write_window_title_to_dir(&data_dir, &title);

            // 绑定后自动启动 TRAE
            #[cfg(target_os = "windows")]
            let shared_ext = std::env::var("APPDATA")
                .ok()
                .map(|p| PathBuf::from(p).join("TRAE SOLO CN_SharedExtensions").to_string_lossy().to_string());
            #[cfg(not(target_os = "windows"))]
            let shared_ext = None;

            machine::open_product_with_data_dir(
                machine::ProductType::TraeSoloCn,
                &data_dir,
                shared_ext.as_deref(),
            )?;
        }

        self.save_store()?;
        println!("[INFO] 已绑定账号到实例: {}", inst_name);
        Ok(())
    }

    /// 启动实例（若绑定了账号则先写入登录信息 + 窗口标题，再启动）
    /// 启动后更新 last_launched_at 时间戳并保存
    pub fn launch_instance(&mut self, id: &str, account_manager: &AccountManager) -> Result<bool> {
        // 先取出需要的字段，避免借用冲突（后面要 &mut self.store）
        let (data_dir, inst_name, bound_account_id) = {
            let inst = self.store.instances.iter()
                .find(|i| i.id == id)
                .ok_or_else(|| anyhow!("实例不存在"))?;
            (inst.data_dir.clone(), inst.name.clone(), inst.bound_account_id.clone())
        };

        let (is_running, _pid) = machine::is_instance_running(&data_dir);

        // 如果绑定了账号，先写入登录信息（确保第一次启动时有凭证）
        if let Some(ref aid) = bound_account_id {
            if let Some(account) = account_manager.get_account_ref(aid) {
                if let Some(token) = account.jwt_token.as_ref() {
                    let login_info = machine::TraeLoginInfo {
                        token: token.clone(),
                        refresh_token: account.refresh_token.clone(),
                        user_id: account.user_id.clone(),
                        email: account.email.clone(),
                        username: account.name.clone(),
                        avatar_url: account.avatar_url.clone(),
                        host: String::new(),
                        region: if account.region.is_empty() { "CN".to_string() } else { account.region.clone() },
                    };
                    let _ = machine::write_login_info_to_dir(
                        &login_info,
                        account.machine_id.as_deref(),
                        &data_dir,
                    );
                }
            }
        }

        // 设置窗口标题（通过 settings.json 的 window.title，持久化且跨重启）
        let title = format!("{} - TRAE Work CN", inst_name);
        let _ = machine::write_window_title_to_dir(&data_dir, &title);

        // 共享插件目录
        #[cfg(target_os = "windows")]
        let shared_ext = std::env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("TRAE SOLO CN_SharedExtensions").to_string_lossy().to_string());
        #[cfg(not(target_os = "windows"))]
        let shared_ext = None;

        machine::open_product_with_data_dir(
            machine::ProductType::TraeSoloCn,
            &data_dir,
            shared_ext.as_deref(),
        )?;

        // 启动成功，更新 last_launched_at
        let now = chrono::Utc::now().timestamp();
        if let Some(inst) = self.store.instances.iter_mut().find(|i| i.id == id) {
            inst.last_launched_at = now;
            inst.updated_at = now;
        }
        // 保存到磁盘（失败不阻断主流程）
        if let Err(e) = self.save_store() {
            println!("[WARN] 保存 last_launched_at 失败: {}", e);
        }

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
            // 注意：窗口标题通过 settings.json 的 window.title 设置，--title CLI 参数对 TRAE 无效
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
