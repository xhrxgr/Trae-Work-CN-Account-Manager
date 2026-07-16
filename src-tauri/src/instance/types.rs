use serde::{Deserialize, Serialize};

/// TRAE 实例（独立 data-dir 的工作环境）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraeInstance {
    /// 实例唯一 ID
    pub id: String,
    /// 用户自定义名称，如 "工作实例"
    pub name: String,
    /// 绝对路径，如 %APPDATA%\TRAE SOLO CN 或 %APPDATA%\TRAE SOLO CN_xxx
    pub data_dir: String,
    /// 是否为默认实例（指向 %APPDATA%\TRAE SOLO CN）
    #[serde(default)]
    pub is_default: bool,
    /// 当前绑定的账号 ID（None=未绑定）
    #[serde(default)]
    pub bound_account_id: Option<String>,
    /// 该实例的机器码（None=首次启动时生成）
    #[serde(default)]
    pub machine_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 实例列表存储结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstanceStore {
    pub instances: Vec<TraeInstance>,
}

/// 实例简要信息（用于列表展示，含运行时数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceBrief {
    pub id: String,
    pub name: String,
    pub data_dir: String,
    pub is_default: bool,
    pub bound_account_id: Option<String>,
    /// 绑定账号的邮箱（展示用）
    pub bound_account_email: Option<String>,
    /// 绑定账号的名称
    pub bound_account_name: Option<String>,
    /// 绑定账号的头像
    pub bound_account_avatar: Option<String>,
    /// 绑定账号的备注
    pub bound_account_note: Option<String>,
    pub machine_id: Option<String>,
    pub created_at: i64,
    /// 磁盘占用（字节）
    pub disk_usage: u64,
    /// 是否正在运行
    pub is_running: bool,
    /// code.lock 中的 PID（运行时才有）
    pub pid: Option<u32>,
}

/// 生成简单 UUID
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}

impl TraeInstance {
    pub fn new(name: String, data_dir: String, is_default: bool) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid_simple(),
            name,
            data_dir,
            is_default,
            bound_account_id: None,
            machine_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}
