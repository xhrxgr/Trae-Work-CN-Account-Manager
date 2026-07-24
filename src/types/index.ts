// 账号简要信息
export interface AccountBrief {
  id: string;
  name: string;
  email: string;
  avatar_url: string;
  plan_type: string;
  is_active: boolean;
  created_at: number;
  machine_id: string | null;
  is_current: boolean; // 是否是当前 Trae IDE 正在使用的账号
  token_expired_at: string | null; // Token 过期时间
  source: string; // 账号来源: "browser", "local", "manual"
  note: string | null; // 用户自定义备注
  data_dir: string | null; // 多开模式绑定的独立数据目录
}

// 完整账号信息
export interface Account {
  id: string;
  name: string;
  email: string;
  avatar_url: string;
  cookies: string;
  jwt_token: string | null;
  token_expired_at: string | null;
  user_id: string;
  tenant_id: string;
  region: string;
  plan_type: string;
  created_at: number;
  updated_at: number;
  is_active: boolean;
  machine_id: string | null;
  source: string;
  note: string | null;
  data_dir: string | null;
}

// 实例账号状态（来自 data-dir 的 storage.json 或官方 API 实时获取）
export interface InstanceAccountStatus {
  user_id: string;
  identity_str: string;       // "Free" / "Pro" 等
  fast_request_per: number;    // 月免费剩余次数（0=已用完）
  can_gen_solo_code: boolean;
  is_pay_freshman: boolean;
  last_sync_ms: number;        // iCubeServerData 的 lastSyncTime（毫秒），0=无同步时间
  // ===== API 实时字段（v1.0.21+）=====
  is_from_api: boolean;        // true=官方API实时获取，false=本地缓存推测
  fast_request_limit: number;  // 总额度（API），0=免费版无配额
  fast_request_used: number;   // 已用次数（API）
  fast_request_left: number;  // 剩余次数（API，= limit - used）
  extra_fast_request_left: number;  // 额外礼包剩余（如周年礼包）
  extra_package_name: string;  // 额外礼包名称
  reset_time: number;          // 额度重置时间（UTC 秒）
  is_free_plan: boolean;       // 是否免费版
}

// 实例简要信息
export interface InstanceBrief {
  id: string;
  name: string;
  data_dir: string;
  is_default: boolean;
  bound_account_id: string | null;
  bound_account_email: string | null;
  bound_account_name: string | null;
  bound_account_avatar: string | null;
  bound_account_note: string | null;
  note: string | null;        // 实例自定义备注
  machine_id: string | null;
  created_at: number;
  last_launched_at: number;  // 上次启动时间（UTC 秒），0=从未启动
  last_closed_at: number;    // 上次关闭时间（UTC 秒），0=从未检测到关闭
  disk_usage: number;
  is_running: boolean;
  pid: number | null;
  account_status: InstanceAccountStatus | null;
}

// API 错误
export interface ApiError {
  message: string;
}

// 安全清理候选项（v1.0.26+）
export interface SafeCleanItem {
  key: string;          // 唯一标识
  label: string;        // 显示名称
  path: string;         // 相对 data-dir 的路径
  category: string;     // "cache" | "crash" | "logs"
  size_bytes: number;   // 当前占用字节数
  description: string;  // 说明：删什么、为何安全
}
