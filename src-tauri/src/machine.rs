use anyhow::{anyhow, Result};
use uuid::Uuid;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

/// 产品类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProductType {
    #[serde(rename = "trae_cn")]
    TraeSoloCn,
}

impl ProductType {
    pub fn data_dir_name(&self) -> &str {
        "TRAE SOLO CN"
    }

    /// 主进程名
    pub fn process_name(&self) -> &str {
        "TRAE SOLO CN.exe"
    }

    /// 所有可能的进程名候选（用于进程检查和关闭）
    /// 注意：trae-auto.exe 是另一个独立项目 "Trae Auto"，不是 TRAE Work CN，不能包含
    pub fn process_names(&self) -> &'static [&'static str] {
        &["TRAE SOLO CN.exe"]
    }

    /// 主 exe 候选文件名（用于路径扫描）
    pub fn main_exe_candidates(&self) -> &'static [&'static str] {
        &["TRAE SOLO CN.exe"]
    }

    pub fn display_name(&self) -> &str {
        "TRAE Work CN"
    }
}

/// Windows 注册表中 MachineGuid 的路径
#[cfg(target_os = "windows")]
const MACHINE_GUID_PATH: &str = r"SOFTWARE\Microsoft\Cryptography";
#[cfg(target_os = "windows")]
const MACHINE_GUID_KEY: &str = "MachineGuid";

/// 读取当前系统的 MachineGuid
#[cfg(target_os = "windows")]
pub fn get_machine_guid() -> Result<String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(MACHINE_GUID_PATH)
        .map_err(|e| anyhow!("无法打开注册表: {}", e))?;

    let guid: String = key.get_value(MACHINE_GUID_KEY)
        .map_err(|e| anyhow!("无法读取 MachineGuid: {}", e))?;

    Ok(guid)
}

/// 设置系统的 MachineGuid（需要管理员权限）
#[cfg(target_os = "windows")]
pub fn set_machine_guid(new_guid: &str) -> Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey_with_flags(MACHINE_GUID_PATH, KEY_SET_VALUE)
        .map_err(|e| anyhow!("无法打开注册表（需要管理员权限）: {}", e))?;

    key.set_value(MACHINE_GUID_KEY, &new_guid)
        .map_err(|e| anyhow!("无法设置 MachineGuid: {}", e))?;

    Ok(())
}

/// 生成新的 MachineGuid
pub fn generate_machine_guid() -> String {
    Uuid::new_v4().to_string()
}

/// 重置 MachineGuid 为新的随机值
#[cfg(target_os = "windows")]
pub fn reset_machine_guid() -> Result<String> {
    let new_guid = generate_machine_guid();
    set_machine_guid(&new_guid)?;
    Ok(new_guid)
}

/// 获取产品数据目录路径
#[cfg(target_os = "windows")]
fn get_product_data_path(product_type: ProductType) -> Result<PathBuf> {
    let appdata = std::env::var("APPDATA")
        .map_err(|_| anyhow!("无法获取 APPDATA 环境变量"))?;
    Ok(PathBuf::from(appdata).join(product_type.data_dir_name()))
}

#[cfg(target_os = "macos")]
fn get_product_data_path(product_type: ProductType) -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow!("无法获取 HOME 环境变量"))?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join(product_type.data_dir_name()))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn get_product_data_path(_product_type: ProductType) -> Result<PathBuf> {
    Err(anyhow!("此功能仅支持 Windows 和 macOS 系统"))
}

/// 检查指定产品是否正在运行
#[cfg(target_os = "windows")]
pub fn is_product_running(product_type: ProductType) -> bool {
    // 检查所有候选进程名（新版本 trae-auto.exe + 旧版本 TRAE SOLO CN.exe）
    for process_name in product_type.process_names() {
        let output = Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {}", process_name), "/NH"])
            .output();

        if let Ok(out) = output {
            let result = String::from_utf8_lossy(&out.stdout);
            if result.contains(process_name) {
                return true;
            }
        }
    }
    false
}

#[cfg(target_os = "macos")]
pub fn is_product_running(_product_type: ProductType) -> bool {
    let app_name = "TRAE SOLO CN.app";
    Command::new("pgrep")
        .args(["-f", &format!("{}/Contents/MacOS", app_name)])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn is_product_running(_product_type: ProductType) -> bool {
    false
}

/// 关闭指定产品进程
/// 优化版：用轮询替代固定 sleep，通常 200-600ms 即可完成（原 1500ms+）
#[cfg(target_os = "windows")]
pub fn kill_product(product_type: ProductType) -> Result<()> {
    let display_name = product_type.display_name();

    if !is_product_running(product_type) {
        println!("[INFO] {} 未运行", display_name);
        return Ok(());
    }

    println!("[INFO] 正在关闭 {}...", display_name);

    // 遍历所有候选进程名（新版本 trae-auto.exe + 旧版本 TRAE SOLO CN.exe）
    for process_name in product_type.process_names() {
        // 先尝试优雅关闭
        let _ = Command::new("taskkill")
            .args(["/IM", process_name])
            .output();
    }

    // 轮询等待进程退出（最多 800ms，每 100ms 检查一次）
    let mut waited = 0;
    while waited < 800 {
        if !is_product_running(product_type) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        waited += 100;
    }

    // 如果还在运行，强制关闭所有候选进程
    if is_product_running(product_type) {
        for process_name in product_type.process_names() {
            let output = Command::new("taskkill")
                .args(["/F", "/IM", process_name])
                .output()
                .map_err(|e| anyhow!("关闭 {} 失败: {}", display_name, e))?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                // 忽略"进程未找到"错误，只报真正的失败
                if !err.is_empty() && !err.contains("not found") && !err.contains("找不到") {
                    println!("[WARN] 关闭 {} 时出错: {}", process_name, err);
                }
            }
        }
    }

    // 轮询等待强制关闭完成（最多 500ms）
    let mut waited = 0;
    while waited < 500 {
        if !is_product_running(product_type) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        waited += 50;
    }

    if is_product_running(product_type) {
        return Err(anyhow!("无法关闭 {}，请手动关闭后重试", display_name));
    }

    println!("[INFO] {} 已关闭 (耗时 {}ms)", display_name, waited);
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn kill_product(_product_type: ProductType) -> Result<()> {
    let display_name = "TRAE Work CN";
    let app_name = "TRAE SOLO CN";
    let app_bundle = "TRAE SOLO CN.app";

    if !is_product_running(product_type) {
        println!("[INFO] {} 未运行", display_name);
        return Ok(());
    }

    println!("[INFO] 正在关闭 {}...", display_name);

    let _ = Command::new("osascript")
        .args(["-e", &format!("tell application \"{}\" to quit", app_name)])
        .output();

    std::thread::sleep(std::time::Duration::from_millis(1500));

    if is_product_running(product_type) {
        println!("[INFO] 优雅关闭失败，正在强制关闭...");
        let _ = Command::new("pkill")
            .args(["-9", "-f", &format!("{}/Contents/MacOS", app_bundle)])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }

    if is_product_running(product_type) {
        return Err(anyhow!("无法关闭 {}，请手动关闭后重试", display_name));
    }

    println!("[INFO] {} 已关闭", display_name);
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn kill_product(_product_type: ProductType) -> Result<()> {
    Err(anyhow!("此功能仅支持 Windows 和 macOS 系统"))
}

/// 获取产品配置文件路径
fn get_product_config_path(_product_type: ProductType) -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "trae", "work-cn-manager")
        .ok_or_else(|| anyhow!("无法获取应用数据目录"))?;
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir)?;
    let filename = "trae_solo_cn_path.txt";
    Ok(config_dir.join(filename))
}

/// 获取保存的产品路径
pub fn get_saved_product_path(product_type: ProductType) -> Result<String> {
    let config_path = get_product_config_path(product_type)?;
    if config_path.exists() {
        let path = fs::read_to_string(&config_path)?;
        let path = path.trim().to_string();
        if !path.is_empty() && PathBuf::from(&path).exists() {
            return Ok(path);
        }
    }
    Err(anyhow!("未设置 {} 路径", product_type.display_name()))
}

/// 保存产品路径
#[cfg(target_os = "windows")]
pub fn save_product_path(product_type: ProductType, path: &str) -> Result<()> {
    let exe_path = PathBuf::from(path);
    if !exe_path.exists() {
        return Err(anyhow!("指定的路径不存在"));
    }
    if !path.to_lowercase().ends_with(".exe") {
        return Err(anyhow!("请选择 .exe 文件"));
    }
    let config_path = get_product_config_path(product_type)?;
    fs::write(&config_path, path)?;
    println!("[INFO] 已保存 {} 路径: {}", product_type.display_name(), path);
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn save_product_path(product_type: ProductType, path: &str) -> Result<()> {
    let app_path = PathBuf::from(path);
    if !app_path.exists() {
        return Err(anyhow!("指定的路径不存在"));
    }
    if !path.to_lowercase().ends_with(".app") {
        return Err(anyhow!("请选择 .app 应用程序"));
    }
    let config_path = get_product_config_path(product_type)?;
    fs::write(&config_path, path)?;
    println!("[INFO] 已保存 {} 路径: {}", product_type.display_name(), path);
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn save_product_path(_product_type: ProductType, _path: &str) -> Result<()> {
    Err(anyhow!("此功能仅支持 Windows 和 macOS 系统"))
}

/// 打开产品
#[cfg(target_os = "windows")]
pub fn open_product(product_type: ProductType) -> Result<()> {
    // 如果路径未设置或无效，尝试自动扫描
    let exe_path = match get_saved_product_path(product_type) {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            println!("[INFO] {} 路径未设置，尝试自动扫描...", product_type.display_name());
            match scan_solo_cn_path() {
                Ok(path) => PathBuf::from(path),
                Err(_) => return Err(anyhow!(
                    "未设置 {} 路径且自动扫描失败，请在设置中手动配置 TRAE SOLO CN.exe 的路径",
                    product_type.display_name()
                )),
            }
        }
    };

    if !exe_path.exists() {
        return Err(anyhow!("{} 路径无效: {}，请在设置中重新配置", product_type.display_name(), exe_path.display()));
    }

    println!("[INFO] 正在启动 {}: {}", product_type.display_name(), exe_path.display());

    Command::new(&exe_path)
        .spawn()
        .map_err(|e| anyhow!("启动 {} 失败: {}", product_type.display_name(), e))?;

    println!("[INFO] {} 已启动", product_type.display_name());
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn open_product(product_type: ProductType) -> Result<()> {
    let app_path = match get_saved_product_path(product_type) {
        Ok(path) => PathBuf::from(path),
        Err(_) => return Err(anyhow!("未设置 {} 路径，请在设置中配置", product_type.display_name())),
    };

    if !app_path.exists() {
        return Err(anyhow!("{} 路径无效，请在设置中重新配置", product_type.display_name()));
    }

    println!("[INFO] 正在启动 {}: {}", product_type.display_name(), app_path.display());

    Command::new("open")
        .arg("-a")
        .arg(&app_path)
        .spawn()
        .map_err(|e| anyhow!("启动 {} 失败: {}", product_type.display_name(), e))?;

    println!("[INFO] {} 已启动", product_type.display_name());
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn open_product(_product_type: ProductType) -> Result<()> {
    Err(anyhow!("此功能仅支持 Windows 和 macOS 系统"))
}

// ========== Trae Solo CN 路径相关函数 ==========

/// 获取保存的 Trae Solo CN 路径
pub fn get_saved_solo_cn_path() -> Result<String> {
    get_saved_product_path(ProductType::TraeSoloCn)
}

/// 保存 Trae Solo CN 路径
pub fn save_solo_cn_path(path: &str) -> Result<()> {
    save_product_path(ProductType::TraeSoloCn, path)
}

/// 自动扫描 TRAE Work CN 安装路径
/// 优先级：常见安装位置 > 注册表卸载信息
/// 注意：Trae Auto (trae-auto.exe) 是另一个独立项目，不是 TRAE Work CN，必须排除
#[cfg(target_os = "windows")]
pub fn scan_solo_cn_path() -> Result<String> {
    // 1. 检查常见安装位置
    let appdata_local = std::env::var("LOCALAPPDATA")
        .map_err(|_| anyhow!("无法获取 LOCALAPPDATA 环境变量"))?;

    // 候选路径：TRAE Work CN 主程序是 TRAE SOLO CN.exe（不是 trae-auto.exe）
    let candidates = [
        PathBuf::from(&appdata_local).join("Programs").join("TRAE SOLO CN").join("TRAE SOLO CN.exe"),
        PathBuf::from(&appdata_local).join("Programs").join("Trae").join("TRAE SOLO CN.exe"),
        PathBuf::from(&appdata_local).join("TRAE SOLO CN").join("TRAE SOLO CN.exe"),
        // 自定义安装位置（用户可能装在 D 盘等）
        PathBuf::from(r"C:\Program Files\TRAE SOLO CN\TRAE SOLO CN.exe"),
        PathBuf::from(r"D:\TRAE SOLO CN\TRAE SOLO CN.exe"),
        PathBuf::from(r"E:\TRAE SOLO CN\TRAE SOLO CN.exe"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            let path = candidate.to_string_lossy().to_string();
            // 自动保存扫描到的路径
            let _ = save_solo_cn_path(&path);
            println!("[INFO] 扫描到 TRAE Work CN 路径: {}", path);
            return Ok(path);
        }
    }

    // 2. 检查注册表卸载信息
    if let Ok(exe_path) = scan_from_registry() {
        let _ = save_solo_cn_path(&exe_path);
        println!("[INFO] 从注册表扫描到 TRAE Work CN 路径: {}", exe_path);
        return Ok(exe_path);
    }

    Err(anyhow!("未找到 TRAE Work CN 安装路径，请手动设置（选择 TRAE SOLO CN.exe 文件）"))
}

/// 从注册表扫描 TRAE Work CN 安装路径
/// 排除：
/// - 本应用本身（"TRAE Work CN Manager"）
/// - "Trae Auto"（另一个独立项目，主程序是 trae-auto.exe，不是 TRAE Work CN）
#[cfg(target_os = "windows")]
fn scan_from_registry() -> Result<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 检查卸载信息
    let uninstall_paths = [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    // 必须排除的关键词
    // - "manager" → 本应用 "TRAE Work CN Manager"
    // - "trae auto" → 另一个独立项目，主程序是 trae-auto.exe，不是 TRAE Work CN
    const EXCLUDE_KEYWORDS: &[&str] = &["manager", "trae auto"];

    // 匹配关键词（任一即可）
    const MATCH_KEYWORDS: &[&str] = &["TRAE SOLO CN", "TRAE Work CN"];

    // 主 exe 文件名（与 ProductType::main_exe_candidates 一致）
    const MAIN_EXES: &[&str] = &["TRAE SOLO CN.exe"];

    for root in [hklm, hkcu] {
        for uninstall_path in &uninstall_paths {
            let uninstall_key = match root.open_subkey(uninstall_path) {
                Ok(k) => k,
                Err(_) => continue,
            };

            for subkey_name in uninstall_key.enum_keys().filter_map(Result::ok) {
                let subkey = match uninstall_key.open_subkey(&subkey_name) {
                    Ok(k) => k,
                    Err(_) => continue,
                };

                let display_name: String = subkey.get_value("DisplayName").unwrap_or_default();
                let install_location: String = subkey.get_value("InstallLocation").unwrap_or_default();
                let uninstall_string: String = subkey.get_value("UninstallString").unwrap_or_default();
                let display_icon: String = subkey.get_value("DisplayIcon").unwrap_or_default();

                let display_name_lower = display_name.to_lowercase();

                // 排除本应用本身 和 Trae Auto
                if EXCLUDE_KEYWORDS.iter().any(|kw| display_name_lower.contains(kw)) {
                    continue;
                }

                // 必须匹配任一关键词
                if !MATCH_KEYWORDS.iter().any(|kw| display_name_lower.contains(&kw.to_lowercase())) {
                    continue;
                }

                // 收集可能的安装目录（从 InstallLocation 和卸载路径推断）
                let mut possible_dirs: Vec<String> = Vec::new();

                if !install_location.is_empty() {
                    possible_dirs.push(install_location.trim_end_matches('\\').trim_matches('"').to_string());
                }

                // 从 DisplayIcon 推断父目录（DisplayIcon 可能指向主 exe 或卸载 exe）
                if !display_icon.is_empty() {
                    let icon_path = display_icon.split(',').next().unwrap_or("").trim().trim_matches('"').to_string();
                    if let Some(parent) = PathBuf::from(&icon_path).parent() {
                        possible_dirs.push(parent.to_string_lossy().to_string());
                    }
                }

                // 从 UninstallString 推断父目录
                if !uninstall_string.is_empty() {
                    let uninst_path = uninstall_string
                        .split('"').nth(1)
                        .unwrap_or_else(|| uninstall_string.split_whitespace().next().unwrap_or(""))
                        .trim()
                        .to_string();
                    if !uninst_path.is_empty() {
                        if let Some(parent) = PathBuf::from(&uninst_path).parent() {
                            possible_dirs.push(parent.to_string_lossy().to_string());
                        }
                    }
                }

                // 在每个候选目录中查找主 exe
                for dir in &possible_dirs {
                    for exe_name in MAIN_EXES {
                        let exe_path = PathBuf::from(dir).join(exe_name);
                        if exe_path.exists() {
                            return Ok(exe_path.to_string_lossy().to_string());
                        }
                    }
                }

                // 如果以上都没找到，但 DisplayIcon 直接指向 TRAE SOLO CN.exe，使用它
                if !display_icon.is_empty() {
                    let icon_path = display_icon.split(',').next().unwrap_or("").trim().trim_matches('"').to_string();
                    let p = PathBuf::from(&icon_path);
                    if p.exists() && p.extension().and_then(|e| e.to_str()) == Some("exe") {
                        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        // 只接受 TRAE SOLO CN.exe，排除 uninstall.exe / unins000.exe
                        if file_name == "TRAE SOLO CN.exe" {
                            return Ok(icon_path);
                        }
                    }
                }
            }
        }
    }

    Err(anyhow!("注册表中未找到 TRAE Work CN 安装信息"))
}

#[cfg(not(target_os = "windows"))]
pub fn scan_solo_cn_path() -> Result<String> {
    Err(anyhow!("此功能仅支持 Windows 系统"))
}

/// 打开 Trae Solo CN
pub fn open_solo_cn() -> Result<()> {
    open_product(ProductType::TraeSoloCn)
}

/// 多开模式：用指定 data-dir 启动产品（不影响已运行的实例）
/// data_dir: 独立数据目录路径；extensions_dir: 共享插件目录路径
#[cfg(target_os = "windows")]
pub fn open_product_with_data_dir(
    product_type: ProductType,
    data_dir: &str,
    extensions_dir: Option<&str>,
) -> Result<()> {
    let exe_path = match get_saved_product_path(product_type) {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            println!("[INFO] {} 路径未设置，尝试自动扫描...", product_type.display_name());
            match scan_solo_cn_path() {
                Ok(path) => PathBuf::from(path),
                Err(_) => return Err(anyhow!(
                    "未设置 {} 路径且自动扫描失败，请在设置中手动配置 TRAE SOLO CN.exe 的路径",
                    product_type.display_name()
                )),
            }
        }
    };

    if !exe_path.exists() {
        return Err(anyhow!("{} 路径无效: {}", product_type.display_name(), exe_path.display()));
    }

    // 确保目标 data-dir 存在
    fs::create_dir_all(data_dir)
        .map_err(|e| anyhow!("创建多开数据目录失败: {}", e))?;

    println!("[INFO] 多开启动 {}: data-dir={}", product_type.display_name(), data_dir);

    let mut cmd = Command::new(&exe_path);
    cmd.arg("--user-data-dir").arg(data_dir);
    if let Some(ext_dir) = extensions_dir {
        fs::create_dir_all(ext_dir).ok();
        cmd.arg("--extensions-dir").arg(ext_dir);
    }

    cmd.spawn()
        .map_err(|e| anyhow!("多开启动 {} 失败: {}", product_type.display_name(), e))?;

    println!("[INFO] {} 多开实例已启动", product_type.display_name());
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn open_product_with_data_dir(
    _product_type: ProductType,
    _data_dir: &str,
    _extensions_dir: Option<&str>,
) -> Result<()> {
    Err(anyhow!("此功能仅支持 Windows 系统"))
}

/// 账号登录信息结构（用于写入 Trae IDE）
#[derive(Debug, Clone)]
pub struct TraeLoginInfo {
    pub token: String,
    pub refresh_token: Option<String>,
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub avatar_url: String,
    pub host: String,
    pub region: String,
}

/// 将账号登录信息写入产品
pub fn write_product_login_info(info: &TraeLoginInfo, product_type: ProductType) -> Result<()> {
    let data_path = get_product_data_path(product_type)?;

    // 确保目录存在
    let storage_dir = data_path.join("User").join("globalStorage");
    fs::create_dir_all(&storage_dir)
        .map_err(|e| anyhow!("创建目录失败: {}", e))?;

    let storage_path = storage_dir.join("storage.json");

    // 读取现有配置或创建新的
    let mut json: serde_json::Value = if storage_path.exists() {
        let content = fs::read_to_string(&storage_path)
            .map_err(|e| anyhow!("读取 storage.json 失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let obj = json.as_object_mut()
        .ok_or_else(|| anyhow!("storage.json 格式错误"))?;

    // 计算过期时间（14天后）
    let now = chrono::Utc::now();
    let expired_at = now + chrono::Duration::days(14);
    let refresh_expired_at = now + chrono::Duration::days(180);

    // 构建 host URL
    let host = if info.host.is_empty() {
        match info.region.to_uppercase().as_str() {
            "SG" => "https://api-sg-central.trae.ai",
            "CN" => "https://api.trae.cn",
            _ => "https://api-sg-central.trae.ai",
        }
    } else {
        &info.host
    };

    // 构建 iCubeAuthInfo
    let auth_info = serde_json::json!({
        "token": info.token,
        "refreshToken": info.refresh_token.clone().unwrap_or_default(),
        "expiredAt": expired_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        "refreshExpiredAt": refresh_expired_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        "tokenReleaseAt": now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        "userId": info.user_id,
        "host": host,
        "userRegion": {
            "region": info.region.to_uppercase(),
            "_aiRegion": info.region.to_uppercase()
        },
        "account": {
            "username": info.username,
            "iss": "",
            "iat": 0,
            "organization": "",
            "work_country": "",
            "email": info.email,
            "avatar_url": info.avatar_url,
            "description": "",
            "scope": "marscode",
            "loginScope": "trae",
            "storeCountryCode": "cn",
            "storeCountrySrc": "uid",
            "storeRegion": info.region.to_uppercase(),
            "userTag": "row"
        }
    });

    // 构建 iCubeEntitlementInfo
    let entitlement_info = serde_json::json!({
        "identityStr": "Free",
        "identity": 0,
        "isPayFreshman": false,
        "isSupportCommercialization": true,
        "hasPackage": false,
        "enableEntitlement": true,
        "detail": {
            "can_gen_solo_code": false,
            "fast_request_per": 1,
            "in_wait": false,
            "permission": 1,
            "toast_read": false,
            "toastRead": false,
            "canGenSoloCode": false,
            "fastRequestPer": 1,
            "inWaitlist": false
        }
    });

    // 写入登录信息（iCubeAuthInfo 需要 AES-128-CBC + HMAC-SHA512 加密）
    let auth_info_plain = serde_json::to_string(&auth_info)
        .map_err(|e| anyhow!("序列化 auth_info 失败: {}", e))?;
    let auth_info_encrypted = encrypt_solo_cn_auth_info(&auth_info_plain)?;
    obj.insert(
        "iCubeAuthInfo://icube.cloudide".to_string(),
        serde_json::Value::String(auth_info_encrypted)
    );
    obj.insert(
        "iCubeEntitlementInfo://icube.cloudide".to_string(),
        serde_json::Value::String(serde_json::to_string(&entitlement_info).unwrap())
    );

    // 写回文件
    let new_content = serde_json::to_string_pretty(&json)
        .map_err(|e| anyhow!("序列化 JSON 失败: {}", e))?;
    fs::write(&storage_path, new_content)
        .map_err(|e| anyhow!("写入 storage.json 失败: {}", e))?;

    println!("[INFO] 已写入 {} 登录信息: {}", product_type.display_name(), info.email);
    Ok(())
}

/// 切换产品到指定账号（清除旧登录状态并写入新账号信息）
pub fn switch_product_account(info: &TraeLoginInfo, machine_id: Option<&str>, product_type: ProductType) -> Result<()> {
    let display_name = product_type.display_name();

    // 0. 先关闭产品
    kill_product(product_type)?;

    let data_path = get_product_data_path(product_type)?;

    // 1. 设置机器码（如果提供则使用，否则生成新的）
    let new_machine_id = match machine_id {
        Some(mid) => mid.to_string(),
        None => generate_machine_guid(),
    };
    let machine_id_path = data_path.join("machineid");
    fs::write(&machine_id_path, &new_machine_id)
        .map_err(|e| anyhow!("写入 {} 机器码失败: {}", display_name, e))?;
    println!("[INFO] 已设置 {} 机器码: {}", display_name, new_machine_id);

    // 2. 保留 state.vscdb 和 state.vscdb.backup（用户 IDE 设置，删除会导致"命令运行方式"等设置被重置）
    //    登录信息在 storage.json 中，不需要通过删除 state.vscdb 来清除

    // 3. 清除 Local State（Chromium 本地状态，含旧会话密钥）
    let local_state_path = data_path.join("Local State");
    if local_state_path.exists() {
        let _ = fs::remove_file(&local_state_path);
    }

    // 4. 清除 IndexedDB（Web 应用数据库，可能含旧登录会话）
    let indexed_db_path = data_path.join("IndexedDB");
    if indexed_db_path.exists() {
        let _ = fs::remove_dir_all(&indexed_db_path);
    }

    // 5. 清除 Local Storage（Web 本地存储，可能含旧登录会话）
    let local_storage_path = data_path.join("Local Storage");
    if local_storage_path.exists() {
        let _ = fs::remove_dir_all(&local_storage_path);
    }

    // 6. 清除 Session Storage（Web 会话存储）
    let session_storage_path = data_path.join("Session Storage");
    if session_storage_path.exists() {
        let _ = fs::remove_dir_all(&session_storage_path);
    }

    // 7. 清除 Cookies（登录会话，必须清除以防止旧账号会话干扰）
    let cookies_path = data_path.join("Network").join("Cookies");
    if cookies_path.exists() {
        let _ = fs::remove_file(&cookies_path);
        println!("[INFO] 已清除 Cookies");
    }

    // 8. 清除 Cookies-journal
    let cookies_journal_path = data_path.join("Network").join("Cookies-journal");
    if cookies_journal_path.exists() {
        let _ = fs::remove_file(&cookies_journal_path);
    }

    // 10. 更新 storage.json 中的 telemetry ID 并写入登录信息
    let storage_dir = data_path.join("User").join("globalStorage");
    fs::create_dir_all(&storage_dir)
        .map_err(|e| anyhow!("创建目录失败: {}", e))?;
    let storage_path = storage_dir.join("storage.json");

    // 读取现有配置或创建新的
    let mut json: serde_json::Value = if storage_path.exists() {
        let content = fs::read_to_string(&storage_path)
            .map_err(|e| anyhow!("读取 storage.json 失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let obj = json.as_object_mut()
        .ok_or_else(|| anyhow!("storage.json 格式错误"))?;

    // 移除旧的登录信息
    obj.remove("iCubeAuthInfo://icube.cloudide");
    obj.remove("iCubeEntitlementInfo://icube.cloudide");
    obj.remove("iCubeServerData://icube.cloudide");
    obj.remove("iCubeAuthInfo://usertag");

    // 更新 telemetry ID
    let new_telemetry_id = format!("{:x}", md5_hash(&new_machine_id));
    obj.insert("telemetry.machineId".to_string(), serde_json::Value::String(new_telemetry_id));
    obj.insert("telemetry.sqmId".to_string(), serde_json::Value::String(format!("{{{}}}", Uuid::new_v4().to_string().to_uppercase())));
    obj.insert("telemetry.devDeviceId".to_string(), serde_json::Value::String(Uuid::new_v4().to_string()));

    // 写回文件
    let new_content = serde_json::to_string_pretty(&json)
        .map_err(|e| anyhow!("序列化 JSON 失败: {}", e))?;
    fs::write(&storage_path, new_content)
        .map_err(|e| anyhow!("写入 storage.json 失败: {}", e))?;

    // 11. 写入新的登录信息
    write_product_login_info(info, product_type)?;

    println!("[INFO] 已切换 {} 到账号: {}", display_name, info.email);

    // 12. 自动打开产品（失败时返回明确错误，登录信息已写入，用户也可手动打开）
    if let Err(e) = open_product(product_type) {
        return Err(anyhow!(
            "账号登录信息已写入成功，但自动打开 {} 失败: {}。请手动打开 TRAE Work CN。",
            display_name, e
        ));
    }

    Ok(())
}

/// 清除产品登录状态（让产品变成全新安装状态）
pub fn clear_product_login_state(product_type: ProductType) -> Result<()> {
    let data_path = get_product_data_path(product_type)?;
    let display_name = product_type.display_name();

    // 1. 生成新的机器码
    let new_machine_id = generate_machine_guid();
    let machine_id_path = data_path.join("machineid");
    fs::write(&machine_id_path, &new_machine_id)
        .map_err(|e| anyhow!("重置 {} 机器码失败: {}", display_name, e))?;
    println!("[INFO] 已重置 {} 机器码: {}", display_name, new_machine_id);

    // 2. 清除 storage.json 中的登录信息
    let storage_path = data_path.join("User").join("globalStorage").join("storage.json");
    if storage_path.exists() {
        let content = fs::read_to_string(&storage_path)
            .map_err(|e| anyhow!("读取 storage.json 失败: {}", e))?;

        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.as_object_mut() {
                obj.remove("iCubeAuthInfo://icube.cloudide");
                obj.remove("iCubeEntitlementInfo://icube.cloudide");
                obj.remove("iCubeServerData://icube.cloudide");
                obj.remove("iCubeAuthInfo://usertag");

                let new_telemetry_id = format!("{:x}", md5_hash(&new_machine_id));
                obj.insert("telemetry.machineId".to_string(), serde_json::Value::String(new_telemetry_id));
                obj.insert("telemetry.sqmId".to_string(), serde_json::Value::String(format!("{{{}}}", Uuid::new_v4().to_string().to_uppercase())));
                obj.insert("telemetry.devDeviceId".to_string(), serde_json::Value::String(Uuid::new_v4().to_string()));

                let new_content = serde_json::to_string_pretty(&json)
                    .map_err(|e| anyhow!("序列化 JSON 失败: {}", e))?;
                fs::write(&storage_path, new_content)
                    .map_err(|e| anyhow!("写入 storage.json 失败: {}", e))?;
                println!("[INFO] 已清除 storage.json 中的登录信息");
            }
        }
    }

    // 3-9. 清除各种缓存数据（保留 state.vscdb 用户 IDE 设置）
    // 注意：state.vscdb 和 state.vscdb.backup 存储用户 IDE 设置（如"命令运行方式"），
    //       删除会导致设置被重置为默认值，登录信息在 storage.json 中，无需删除 state.vscdb
    let paths_to_clear = [
        ("Local State", false),
        ("IndexedDB", true),
        ("Local Storage", true),
        ("Session Storage", true),
        ("Network/Cookies", false),
        ("Network/Cookies-journal", false),
    ];

    for (path, is_dir) in paths_to_clear.iter() {
        let full_path = data_path.join(path);
        if full_path.exists() {
            if *is_dir {
                let _ = fs::remove_dir_all(&full_path);
            } else {
                let _ = fs::remove_file(&full_path);
            }
            println!("[INFO] 已清除 {}", path);
        }
    }

    Ok(())
}

/// 多开模式：为指定账号在独立 data-dir 中写入登录信息并启动新实例
/// 不杀进程、不动默认数据目录、不影响其他运行中的实例
///
/// - info: 账号登录信息
/// - machine_id: 绑定的机器码（None 则生成新的）
/// - data_dir: 独立数据目录路径（由调用方提供，通常按 user_id 命名）
/// - extensions_dir: 共享插件目录（None 则不指定，使用 data-dir 内默认位置）
pub fn launch_product_multi(
    info: &TraeLoginInfo,
    machine_id: Option<&str>,
    data_dir: &str,
    extensions_dir: Option<&str>,
) -> Result<()> {
    let product_type = ProductType::TraeSoloCn;
    let display_name = product_type.display_name();
    let data_path = PathBuf::from(data_dir);

    // 确保目录存在
    fs::create_dir_all(&data_path)
        .map_err(|e| anyhow!("创建多开数据目录失败: {}", e))?;

    // 1. 写入机器码（data-dir 内的 machineid，不动系统注册表）
    let new_machine_id = match machine_id {
        Some(mid) => mid.to_string(),
        None => generate_machine_guid(),
    };
    let machine_id_path = data_path.join("machineid");
    fs::write(&machine_id_path, &new_machine_id)
        .map_err(|e| anyhow!("写入 {} 多开机器码失败: {}", display_name, e))?;
    println!("[INFO] 已设置 {} 多开机器码: {}", display_name, new_machine_id);

    // 2. 写入登录信息到 storage.json（复用加密逻辑）
    let storage_dir = data_path.join("User").join("globalStorage");
    fs::create_dir_all(&storage_dir)
        .map_err(|e| anyhow!("创建目录失败: {}", e))?;
    let storage_path = storage_dir.join("storage.json");

    let mut json: serde_json::Value = if storage_path.exists() {
        let content = fs::read_to_string(&storage_path)
            .map_err(|e| anyhow!("读取 storage.json 失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let obj = json.as_object_mut()
        .ok_or_else(|| anyhow!("storage.json 格式错误"))?;

    // 移除旧的登录信息
    obj.remove("iCubeAuthInfo://icube.cloudide");
    obj.remove("iCubeEntitlementInfo://icube.cloudide");
    obj.remove("iCubeServerData://icube.cloudide");
    obj.remove("iCubeAuthInfo://usertag");

    // 更新 telemetry ID
    let new_telemetry_id = format!("{:x}", md5_hash(&new_machine_id));
    obj.insert("telemetry.machineId".to_string(), serde_json::Value::String(new_telemetry_id));
    obj.insert("telemetry.sqmId".to_string(), serde_json::Value::String(format!("{{{}}}", Uuid::new_v4().to_string().to_uppercase())));
    obj.insert("telemetry.devDeviceId".to_string(), serde_json::Value::String(Uuid::new_v4().to_string()));

    // 3. 构建 iCubeAuthInfo（加密写入）
    let now = chrono::Utc::now();
    let expired_at = now + chrono::Duration::days(14);
    let refresh_expired_at = now + chrono::Duration::days(180);

    let host = if info.host.is_empty() {
        match info.region.to_uppercase().as_str() {
            "SG" => "https://api-sg-central.trae.ai",
            "CN" => "https://api.trae.cn",
            _ => "https://api-sg-central.trae.ai",
        }
    } else {
        &info.host
    };

    let auth_info = serde_json::json!({
        "token": info.token,
        "refreshToken": info.refresh_token.clone().unwrap_or_default(),
        "expiredAt": expired_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        "refreshExpiredAt": refresh_expired_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        "tokenReleaseAt": now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        "userId": info.user_id,
        "host": host,
        "userRegion": {
            "region": info.region.to_uppercase(),
            "_aiRegion": info.region.to_uppercase()
        },
        "account": {
            "username": info.username,
            "iss": "",
            "iat": 0,
            "organization": "",
            "work_country": "",
            "email": info.email,
            "avatar_url": info.avatar_url,
            "description": "",
            "scope": "marscode",
            "loginScope": "trae",
            "storeCountryCode": "cn",
            "storeCountrySrc": "uid",
            "storeRegion": info.region.to_uppercase(),
            "userTag": "row"
        }
    });

    let entitlement_info = serde_json::json!({
        "identityStr": "Free",
        "identity": 0,
        "isPayFreshman": false,
        "isSupportCommercialization": true,
        "hasPackage": false,
        "enableEntitlement": true,
        "detail": {
            "can_gen_solo_code": false,
            "fast_request_per": 1,
            "in_wait": false,
            "permission": 1,
            "toast_read": false,
            "toastRead": false,
            "canGenSoloCode": false,
            "fastRequestPer": 1,
            "inWaitlist": false
        }
    });

    let auth_info_plain = serde_json::to_string(&auth_info)
        .map_err(|e| anyhow!("序列化 auth_info 失败: {}", e))?;
    let auth_info_encrypted = encrypt_solo_cn_auth_info(&auth_info_plain)?;
    obj.insert(
        "iCubeAuthInfo://icube.cloudide".to_string(),
        serde_json::Value::String(auth_info_encrypted)
    );
    obj.insert(
        "iCubeEntitlementInfo://icube.cloudide".to_string(),
        serde_json::Value::String(serde_json::to_string(&entitlement_info).unwrap())
    );

    // 写回（最后一次性写入，包含 telemetry + 登录信息）
    let new_content = serde_json::to_string_pretty(&json)
        .map_err(|e| anyhow!("序列化 JSON 失败: {}", e))?;
    fs::write(&storage_path, new_content)
        .map_err(|e| anyhow!("写入 storage.json 失败: {}", e))?;

    println!("[INFO] 已写入 {} 多开登录信息: {}", display_name, info.email);

    // 4. 启动多开实例
    open_product_with_data_dir(product_type, data_dir, extensions_dir)?;

    println!("[INFO] {} 多开已启动: {}", display_name, info.email);
    Ok(())
}

// ========== Trae Solo CN 专属函数 ==========

/// 读取 Trae Solo CN 的机器码
pub fn get_solo_cn_machine_id() -> Result<String> {
    let data_path = get_product_data_path(ProductType::TraeSoloCn)?;
    let machine_id_path = data_path.join("machineid");

    if !machine_id_path.exists() {
        return Err(anyhow!("TRAE SOLO CN 机器码文件不存在"));
    }

    let content = fs::read_to_string(&machine_id_path)
        .map_err(|e| anyhow!("读取 TRAE SOLO CN 机器码失败: {}", e))?;

    Ok(content.trim().to_string())
}

/// 设置 Trae Solo CN 的机器码
pub fn set_solo_cn_machine_id(new_id: &str) -> Result<()> {
    let data_path = get_product_data_path(ProductType::TraeSoloCn)?;
    let machine_id_path = data_path.join("machineid");

    fs::write(&machine_id_path, new_id)
        .map_err(|e| anyhow!("写入 TRAE SOLO CN 机器码失败: {}", e))?;

    Ok(())
}

/// 将账号登录信息写入 Trae Solo CN
pub fn write_solo_cn_login_info(info: &TraeLoginInfo) -> Result<()> {
    write_product_login_info(info, ProductType::TraeSoloCn)
}

/// 切换 Trae Solo CN 到指定账号
pub fn switch_solo_cn_account(info: &TraeLoginInfo, machine_id: Option<&str>) -> Result<()> {
    switch_product_account(info, machine_id, ProductType::TraeSoloCn)
}

/// 清除 Trae Solo CN 的登录状态
pub fn clear_solo_cn_login_state() -> Result<()> {
    clear_product_login_state(ProductType::TraeSoloCn)
}

/// TRAE Work CN 加密数据格式 (AES-128-CBC + HMAC-SHA512):
/// Header (38 bytes): "tc" (2) + version/5 (1) + 0x10/0x00/0x00 (3) + embedded_key (32)
/// Body: AES-128-CBC encrypted (HMAC_SHA512(64) || plaintext + PKCS7 padding)
/// Key derivation: AES-128 key = SHA512(SHA512(embedded_key) || (jQ XOR WQ))[0..16]
///                 IV = SHA512(SHA512(embedded_key) || (jQ XOR WQ))[16..32]
#[cfg(target_os = "windows")]
pub fn decrypt_solo_cn_auth_info(encrypted_b64: &str) -> Result<String> {
    use base64::Engine;
    use sha2::{Sha512, Digest};

    // 硬编码常量 (来自 Trae 源码)
    const JQ: [u8; 64] = [
        82, 9, 106, 213, 48, 54, 165, 56, 191, 64, 163, 158, 129, 243, 215, 251,
        124, 227, 57, 130, 155, 47, 255, 135, 52, 142, 67, 68, 196, 222, 233, 203,
        84, 123, 148, 50, 166, 194, 35, 61, 238, 76, 149, 11, 66, 250, 195, 78,
        8, 46, 161, 102, 40, 217, 36, 178, 118, 91, 162, 73, 109, 139, 209, 37,
    ];
    const WQ: [u8; 64] = [
        31, 221, 168, 51, 136, 7, 199, 49, 177, 18, 16, 89, 39, 128, 236, 95,
        96, 81, 127, 169, 25, 181, 74, 13, 45, 229, 122, 159, 147, 201, 156, 239,
        160, 224, 59, 77, 174, 42, 245, 176, 200, 235, 187, 60, 131, 83, 153, 97,
        23, 43, 4, 126, 186, 119, 214, 38, 225, 105, 20, 99, 85, 33, 12, 125,
    ];

    const HEADER_MAGIC_T: u8 = 116; // 't'
    const HEADER_MAGIC_C: u8 = 99;  // 'c'
    const HEADER_VERSION: u8 = 5;
    const HEADER_SIZE: usize = 6;   // magic(2) + version(1) + reserved(3)
    const EMBEDDED_KEY_SIZE: usize = 32;
    const HMAC_SIZE: usize = 64;    // SHA-512

    // 1. Base64 解码
    let encrypted_data = base64::engine::general_purpose::STANDARD
        .decode(encrypted_b64)
        .map_err(|e| anyhow!("Base64 解码失败: {}", e))?;

    let total_header = HEADER_SIZE + EMBEDDED_KEY_SIZE; // 38
    if encrypted_data.len() < total_header + 16 {
        return Err(anyhow!("加密数据太短: {} bytes", encrypted_data.len()));
    }

    // 2. 验证头部
    if encrypted_data[0] != HEADER_MAGIC_T || encrypted_data[1] != HEADER_MAGIC_C {
        return Err(anyhow!("加密数据头部魔数不正确"));
    }
    if encrypted_data[2] != HEADER_VERSION {
        return Err(anyhow!("不支持的加密版本: {}", encrypted_data[2]));
    }

    // 3. 提取嵌入的密钥
    let embedded_key = &encrypted_data[HEADER_SIZE..total_header];

    // 4. 计算 AES_CONSTANT = jQ XOR WQ
    let mut aes_constant = [0u8; 64];
    for i in 0..64 {
        aes_constant[i] = JQ[i] ^ WQ[i];
    }

    // 5. 派生 AES-128 密钥和 IV
    // $Q 函数:
    //   o = SHA-512(embedded_key)
    //   n = o || aes_constant (128 bytes)
    //   c = SHA-512(n)
    //   aes_key = c[0..16], iv = c[16..32]
    let mut hasher = Sha512::new();
    hasher.update(embedded_key);
    let o = hasher.finalize(); // 64 bytes

    let mut n = [0u8; 128];
    n[0..64].copy_from_slice(&o);
    n[64..128].copy_from_slice(&aes_constant);

    let mut hasher = Sha512::new();
    hasher.update(&n);
    let c = hasher.finalize(); // 64 bytes

    let aes_key: [u8; 16] = c[0..16].try_into().unwrap();
    let iv: [u8; 16] = c[16..32].try_into().unwrap();

    // 6. AES-128-CBC 解密
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    use aes::Aes128;
    use cbc::Decryptor;

    let ciphertext = &encrypted_data[total_header..];
    let mut decryptor = Decryptor::<Aes128>::new(&aes_key.into(), &iv.into());

    let mut padded = vec![0u8; ciphertext.len()];
    // CBC 解密逐块进行
    for (chunk, out_chunk) in ciphertext.chunks(16).zip(padded.chunks_mut(16)) {
        let mut block = aes::Block::default();
        block.copy_from_slice(chunk);
        let mut out_block = aes::Block::default();
        decryptor.decrypt_block_b2b_mut(&block, &mut out_block);
        out_chunk.copy_from_slice(&out_block);
    }

    // 7. 移除 PKCS7 填充
    let pad_len = *padded.last().ok_or_else(|| anyhow!("解密数据为空"))? as usize;
    if pad_len == 0 || pad_len > 16 {
        return Err(anyhow!("无效的 PKCS7 填充: {}", pad_len));
    }
    // 验证填充
    for i in 0..pad_len {
        if padded[padded.len() - 1 - i] != pad_len as u8 {
            return Err(anyhow!("PKCS7 填充验证失败"));
        }
    }
    let decrypted = &padded[..padded.len() - pad_len];

    if decrypted.len() < HMAC_SIZE {
        return Err(anyhow!("解密数据太短: {} bytes", decrypted.len()));
    }

    // 8. 验证 HMAC
    let hmac_expected = &decrypted[..HMAC_SIZE];
    let plaintext = &decrypted[HMAC_SIZE..];

    let mut hasher = Sha512::new();
    hasher.update(plaintext);
    let hmac_computed = hasher.finalize();

    if hmac_expected != hmac_computed.as_slice() {
        return Err(anyhow!("HMAC 验证失败"));
    }

    // 9. 返回解密后的 JSON
    String::from_utf8(plaintext.to_vec())
        .map_err(|e| anyhow!("UTF-8 解码失败: {}", e))
}

/// 加密 TRAE Work CN 认证信息（decrypt_solo_cn_auth_info 的逆运算）
/// 输入: 明文 JSON 字符串
/// 输出: Base64 编码的加密数据（带 "tc" 头 + embedded_key + AES-128-CBC 密文）
#[cfg(target_os = "windows")]
pub fn encrypt_solo_cn_auth_info(plaintext: &str) -> Result<String> {
    use base64::Engine;
    use sha2::{Sha512, Digest};
    use rand::RngCore;

    // 硬编码常量（与 decrypt 一致）
    const JQ: [u8; 64] = [
        82, 9, 106, 213, 48, 54, 165, 56, 191, 64, 163, 158, 129, 243, 215, 251,
        124, 227, 57, 130, 155, 47, 255, 135, 52, 142, 67, 68, 196, 222, 233, 203,
        84, 123, 148, 50, 166, 194, 35, 61, 238, 76, 149, 11, 66, 250, 195, 78,
        8, 46, 161, 102, 40, 217, 36, 178, 118, 91, 162, 73, 109, 139, 209, 37,
    ];
    const WQ: [u8; 64] = [
        31, 221, 168, 51, 136, 7, 199, 49, 177, 18, 16, 89, 39, 128, 236, 95,
        96, 81, 127, 169, 25, 181, 74, 13, 45, 229, 122, 159, 147, 201, 156, 239,
        160, 224, 59, 77, 174, 42, 245, 176, 200, 235, 187, 60, 131, 83, 153, 97,
        23, 43, 4, 126, 186, 119, 214, 38, 225, 105, 20, 99, 85, 33, 12, 125,
    ];

    const HEADER_MAGIC_T: u8 = 116; // 't'
    const HEADER_MAGIC_C: u8 = 99;  // 'c'
    const HEADER_VERSION: u8 = 5;
    const HEADER_SIZE: usize = 6;   // magic(2) + version(1) + reserved(3)
    const EMBEDDED_KEY_SIZE: usize = 32;
    const HMAC_SIZE: usize = 64;    // SHA-512

    // 1. 生成随机 32 字节 embedded_key
    let mut embedded_key = [0u8; EMBEDDED_KEY_SIZE];
    rand::thread_rng().fill_bytes(&mut embedded_key);

    // 2. 计算 AES_CONSTANT = jQ XOR WQ
    let mut aes_constant = [0u8; 64];
    for i in 0..64 {
        aes_constant[i] = JQ[i] ^ WQ[i];
    }

    // 3. 派生 AES-128 密钥和 IV
    //    o = SHA-512(embedded_key)
    //    n = o || aes_constant (128 bytes)
    //    c = SHA-512(n)
    //    aes_key = c[0..16], iv = c[16..32]
    let mut hasher = Sha512::new();
    hasher.update(&embedded_key);
    let o = hasher.finalize(); // 64 bytes

    let mut n = [0u8; 128];
    n[0..64].copy_from_slice(&o);
    n[64..128].copy_from_slice(&aes_constant);

    let mut hasher = Sha512::new();
    hasher.update(&n);
    let c = hasher.finalize(); // 64 bytes

    let aes_key: [u8; 16] = c[0..16].try_into().unwrap();
    let iv: [u8; 16] = c[16..32].try_into().unwrap();

    // 4. 计算 HMAC-SHA512(plaintext)
    let plaintext_bytes = plaintext.as_bytes();
    let mut hasher = Sha512::new();
    hasher.update(plaintext_bytes);
    let hmac = hasher.finalize(); // 64 bytes

    // 5. 拼接 HMAC || plaintext
    let mut data = Vec::with_capacity(HMAC_SIZE + plaintext_bytes.len());
    data.extend_from_slice(&hmac);
    data.extend_from_slice(plaintext_bytes);

    // 6. PKCS7 填充到 16 字节的倍数
    let pad_len = 16 - (data.len() % 16);
    let pad_byte = pad_len as u8;
    for _ in 0..pad_len {
        data.push(pad_byte);
    }

    // 7. AES-128-CBC 加密
    use aes::cipher::{BlockEncryptMut, KeyIvInit};
    use aes::Aes128;
    use cbc::Encryptor;

    let mut ciphertext = vec![0u8; data.len()];
    let mut encryptor = Encryptor::<Aes128>::new(&aes_key.into(), &iv.into());
    for (chunk, out_chunk) in data.chunks(16).zip(ciphertext.chunks_mut(16)) {
        let mut block = aes::Block::default();
        block.copy_from_slice(chunk);
        let mut out_block = aes::Block::default();
        encryptor.encrypt_block_b2b_mut(&block, &mut out_block);
        out_chunk.copy_from_slice(&out_block);
    }

    // 8. 构造完整数据: header(6) + embedded_key(32) + ciphertext
    let mut output = Vec::with_capacity(HEADER_SIZE + EMBEDDED_KEY_SIZE + ciphertext.len());
    output.push(HEADER_MAGIC_T);
    output.push(HEADER_MAGIC_C);
    output.push(HEADER_VERSION);
    output.push(0x10); // reserved
    output.push(0x00);
    output.push(0x00);
    output.extend_from_slice(&embedded_key);
    output.extend_from_slice(&ciphertext);

    // 9. Base64 编码
    Ok(base64::engine::general_purpose::STANDARD.encode(&output))
}

#[cfg(not(target_os = "windows"))]
pub fn encrypt_solo_cn_auth_info(_plaintext: &str) -> Result<String> {
    Err(anyhow!("此功能仅支持 Windows 系统"))
}

#[cfg(not(target_os = "windows"))]
pub fn decrypt_solo_cn_auth_info(_encrypted_b64: &str) -> Result<String> {
    Err(anyhow!("此功能仅支持 Windows 系统"))
}

/// 简单的 MD5 哈希（用于生成 telemetry.machineId 格式）
fn md5_hash(input: &str) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let h1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    format!("{}{}", input, h1).hash(&mut hasher2);
    let h2 = hasher2.finish();

    ((h1 as u128) << 64) | (h2 as u128)
}

// macOS 平台实现
#[cfg(target_os = "macos")]
pub fn get_machine_guid() -> Result<String> {
    // 使用 ioreg 命令读取 IOPlatformUUID
    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .map_err(|e| anyhow!("执行 ioreg 失败: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // 解析 IOPlatformUUID
    for line in stdout.lines() {
        if line.contains("IOPlatformUUID") {
            // 格式: "IOPlatformUUID" = "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
            if let Some(uuid) = line.split('"').nth(3) {
                return Ok(uuid.to_string());
            }
        }
    }
    
    Err(anyhow!("无法获取 IOPlatformUUID"))
}

#[cfg(target_os = "macos")]
pub fn set_machine_guid(_new_guid: &str) -> Result<()> {
    // macOS 无法修改系统 UUID
    Err(anyhow!("macOS 不支持修改系统机器码"))
}

#[cfg(target_os = "macos")]
pub fn reset_machine_guid() -> Result<String> {
    // macOS 无法重置系统 UUID
    Err(anyhow!("macOS 不支持重置系统机器码"))
}

// 非 Windows/macOS 平台的占位实现
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn get_machine_guid() -> Result<String> {
    Err(anyhow!("此功能仅支持 Windows 和 macOS 系统"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn set_machine_guid(_new_guid: &str) -> Result<()> {
    Err(anyhow!("此功能仅支持 Windows 和 macOS 系统"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn reset_machine_guid() -> Result<String> {
    Err(anyhow!("此功能仅支持 Windows 和 macOS 系统"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_solo_cn_auth_info() {
        let appdata = std::env::var("APPDATA").expect("APPDATA");
        let storage_path = PathBuf::from(&appdata)
            .join("TRAE SOLO CN")
            .join("User")
            .join("globalStorage")
            .join("storage.json");

        if !storage_path.exists() {
            eprintln!("storage.json 不存在，跳过测试");
            return;
        }

        let content = fs::read_to_string(&storage_path).expect("读取 storage.json");
        let storage: serde_json::Value = serde_json::from_str(&content).expect("解析 storage.json");

        // 测试所有加密值
        let keys = [
            "iCubeAuthInfo://usertag",
            "iCubeAuthInfo://icube.cloudide",
        ];
        for key in &keys {
            let encrypted = match storage.get(key).and_then(|v| v.as_str()) {
                Some(v) => v,
                None => continue,
            };
            match decrypt_solo_cn_auth_info(encrypted) {
                Ok(decrypted) => {
                    let auth: serde_json::Value = serde_json::from_str(&decrypted).unwrap();
                    if *key == "iCubeAuthInfo://usertag" {
                        println!("[TEST] {} 解密成功: {:?}", key, auth);
                    } else {
                        let email = auth
                            .get("account")
                            .and_then(|a| a.get("email"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("N/A");
                        let user_id = auth.get("userId").and_then(|v| v.as_str()).unwrap_or("N/A");
                        println!("[TEST] {} 解密成功! UserID: {}, Email: {}", key, user_id, email);
                    }
                }
                Err(e) => {
                    panic!("解密 {} 失败: {}", key, e);
                }
            }
        }
    }

    /// 测试加密 → 解密往返（round-trip）
    #[cfg(target_os = "windows")]
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = r#"{"token":"abc123","userId":"u-123","email":"test@example.com"}"#;

        // 加密
        let encrypted = encrypt_solo_cn_auth_info(plaintext)
            .expect("加密失败");

        // 加密结果应为 Base64 字符串，且解码后以 "tc" 开头
        use base64::Engine;
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&encrypted)
            .expect("Base64 解码失败");
        assert_eq!(&raw[0..2], b"tc", "加密数据头部魔数不正确");
        assert_eq!(raw[2], 5, "版本号不正确");

        // 解密
        let decrypted = decrypt_solo_cn_auth_info(&encrypted)
            .expect("解密失败");

        // 验证往返一致
        assert_eq!(plaintext, decrypted, "加密-解密往返数据不一致");
        println!("[TEST] 加密-解密往返测试通过!");
    }

    /// 测试多次加密产生不同密文（验证随机 embedded_key 生效）
    #[cfg(target_os = "windows")]
    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        let plaintext = r#"{"token":"abc"}"#;
        let enc1 = encrypt_solo_cn_auth_info(plaintext).expect("加密失败");
        let enc2 = encrypt_solo_cn_auth_info(plaintext).expect("加密失败");
        assert_ne!(enc1, enc2, "两次加密应产生不同密文（随机 embedded_key）");

        // 但都能解密回原文
        assert_eq!(plaintext, decrypt_solo_cn_auth_info(&enc1).unwrap());
        assert_eq!(plaintext, decrypt_solo_cn_auth_info(&enc2).unwrap());
        println!("[TEST] 随机 embedded_key 测试通过!");
    }
}
