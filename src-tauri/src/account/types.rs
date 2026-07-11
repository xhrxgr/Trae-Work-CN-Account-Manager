use serde::{Deserialize, Serialize};

/// 浏览器登录时从前端拦截到的用户信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserUserInfo {
    pub screen_name: String,
    pub avatar_url: String,
    pub email: String,
}

/// 账号信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub email: String,
    pub avatar_url: String,
    pub cookies: String,
    pub jwt_token: Option<String>,
    pub token_expired_at: Option<String>,
    pub user_id: String,
    pub tenant_id: String,
    pub region: String,
    pub plan_type: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_active: bool,
    /// 账号关联的机器码
    #[serde(default)]
    pub machine_id: Option<String>,
    /// 账号来源: "browser"(浏览器登录), "local"(本地读取), "manual"(手动输入Token)
    #[serde(default)]
    pub source: String,
    /// 用户自定义备注
    #[serde(default)]
    pub note: Option<String>,
    /// 多开模式绑定的独立数据目录路径
    /// None=使用默认目录(单实例切换模式), Some=多开模式专属目录
    #[serde(default)]
    pub data_dir: Option<String>,
}

impl Account {
    pub fn new(
        name: String,
        email: String,
        cookies: String,
        user_id: String,
        tenant_id: String,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid_simple(),
            name,
            email,
            avatar_url: String::new(),
            cookies,
            jwt_token: None,
            token_expired_at: None,
            user_id,
            tenant_id,
            region: String::new(),
            plan_type: "Free".to_string(),
            created_at: now,
            updated_at: now,
            is_active: true,
            machine_id: None,
            source: String::new(),
            note: None,
            data_dir: None,
        }
    }
}

/// 账号列表存储结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountStore {
    pub accounts: Vec<Account>,
    pub active_account_id: Option<String>,
    /// 当前 Trae IDE 正在使用的账号 ID
    #[serde(default)]
    pub current_account_id: Option<String>,
}

/// 简单的 UUID 生成
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}

/// 账号简要信息（用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBrief {
    pub id: String,
    pub name: String,
    pub email: String,
    pub avatar_url: String,
    pub plan_type: String,
    pub is_active: bool,
    pub created_at: i64,
    /// 账号关联的机器码
    pub machine_id: Option<String>,
    /// 是否是当前 Trae IDE 正在使用的账号
    pub is_current: bool,
    /// Token 过期时间
    pub token_expired_at: Option<String>,
    /// 账号来源: "browser", "local", "manual"
    #[serde(default)]
    pub source: String,
    /// 用户自定义备注
    #[serde(default)]
    pub note: Option<String>,
    /// 多开模式绑定的独立数据目录路径
    #[serde(default)]
    pub data_dir: Option<String>,
}

impl From<&Account> for AccountBrief {
    fn from(account: &Account) -> Self {
        Self {
            id: account.id.clone(),
            name: account.name.clone(),
            email: account.email.clone(),
            avatar_url: account.avatar_url.clone(),
            plan_type: account.plan_type.clone(),
            is_active: account.is_active,
            created_at: account.created_at,
            machine_id: account.machine_id.clone(),
            is_current: false, // 默认为 false，由 AccountManager 设置
            token_expired_at: account.token_expired_at.clone(),
            source: account.source.clone(),
            note: account.note.clone(),
            data_dir: account.data_dir.clone(),
        }
    }
}

impl AccountBrief {
    /// 从 Account 创建 AccountBrief，并设置 is_current 标记
    pub fn from_account(account: &Account, is_current: bool) -> Self {
        Self {
            id: account.id.clone(),
            name: account.name.clone(),
            email: account.email.clone(),
            avatar_url: account.avatar_url.clone(),
            plan_type: account.plan_type.clone(),
            is_active: account.is_active,
            created_at: account.created_at,
            machine_id: account.machine_id.clone(),
            is_current,
            token_expired_at: account.token_expired_at.clone(),
            source: account.source.clone(),
            note: account.note.clone(),
            data_dir: account.data_dir.clone(),
        }
    }
}
