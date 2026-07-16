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
  machine_id: string | null;
  created_at: number;
  disk_usage: number;
  is_running: boolean;
  pid: number | null;
}

// API 错误
export interface ApiError {
  message: string;
}
