# 更新日志

本项目所有显著变更都会记录在此文件中。

格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

---

## [1.0.21] - 2026-07-22

### 新增
- **逆向 TRAE 源码，揭示"速通"真实含义**：`fast_request_per` 是**剩余速通次数**（对话优先队列），不是"月免费对话次数"。免费版 `fast_request_per=0` 表示无速通额度，但用户仍可使用普通对话，由服务端动态控制
- **官方 API 实时获取额度**：右键实例卡片点击 🔄 按钮，调用 `POST https://api.trae.cn/trae/api/v1/pay/user_current_entitlement_list` 获取实时账号状态（速通次数、额外礼包、重置时间等）
- **实例卡片新增「上次启动时间」显示**

### 修复
- **免费版不再误显示"月免费已用完"徽章**：免费版（Free）一律不显示状态徽章，避免 `fast_request_per=0` 被误读为"月免费对话已用完"
- **未登录实例不再误显示状态徽章**：`read_instance_account_status` 中 user_id 为空时直接返回 None
- 只对非 Free 身份（Pro 等）才显示徽章和速通次数

---

## [1.0.20] - 2026-07-21

### 新增
- **自动创建本地账号**：当实例 `storage.json` 中登录了账号但本地账号列表无匹配记录时（例如用户在 IDE 内手动登录了新账号），自动调用 `AccountManager::add_account_from_local_login` 创建新本地账号（source="local"，不调用 API，不联网）
- `parse_jwt_meta(token)` 函数从 JWT 解析 tenant_id 和 exp
- `tauri.conf.json` 显式设置 NSIS `installMode = "currentUser"` + `languages = ["SimpChinese", "English"]`，支持覆盖更新（无需先卸载旧版本）

### 修复
- **已绑定账号的实例不再误显示"未绑定"**：InstanceCard.tsx 中 `bound_account_email` 是空字符串时（CN 账号通常没有邮箱）被 JS 当作 falsy 误判。改为用 `instance.bound_account_id` 判断绑定状态
- **`.instances-page` 缺少 `flex: 1; overflow-y: auto;`** 导致实例多时页面内容溢出视口无法滚动，补充 CSS 后可垂直滚动

---

## [1.0.19] - 2026-07-19

### 新增
- **实例启动时自动发现已有 data-dir**：扫描 `%APPDATA%\TRAE SOLO CN*` 下有 `storage.json` 的文件夹，未登记的自动加入实例列表（实例名优先用 `storage.json` 中的 username，note 标记为"自动发现"）
- **自动绑定账号到实例**：扫描每个实例 `data-dir` 的 `storage.json`，按 user_id 匹配本地账号。处理场景：用户在 IDE 内手动登录账号但实例未绑定、bound_account_id 失效、默认实例从未绑定
- **实例备注功能**：右键菜单「编辑备注」，写入 `TraeInstance.note`，实例卡片名称旁显示 📝 徽章（最多 120 字符）
- **账号状态显示**：从 `storage.json` 的 `iCubeServerData://icube.cloudide` 解析 entitlementInfo，实例卡片显示状态徽章
- `read_trae_login_from_dir(data_dir)` 函数：读取任意 data-dir 的 `storage.json`，自动处理加密/明文两种 iCubeAuthInfo 格式
- `read_instance_account_status(data_dir)` 函数：优先用 iCubeServerData（含 lastSyncTime），失败回退到本地 iCubeEntitlementInfo 缓存

### 修复
- **移除 v1.0.11 引入的 `store.instances.retain(...)` 过滤逻辑**：该逻辑会删除用户手动创建但还没绑定账号的实例，导致"重启管理器后实例消失"的 bug。用户创建的实例（无论是否绑定账号）现在都会保留

---

## [1.0.15] - 2026-07-15

### 新增
- **支持跟随系统深色模式**：`color-scheme: light dark` 声明 + `@media (prefers-color-scheme: dark)` 深色 CSS 变量覆盖（背景/文字/边框/阴影/状态色/渐变至暗色系），天蓝青主色不变

---

## [1.0.12] - 2026-07-12

### 备注
- 此版本计划实现 `read_trae_login_from_dir` 函数和自动绑定，但实际代码中该函数从未实现（仅在 AGENTS.md 中记录）
- **保留**：`find_account_by_user_id` 方法（按 user_id 在 AccountManager 中查找已存储的账号）
- **保留**：右键菜单适配实例管理、实例卡片「未绑定账号 → 点击绑定」
- v1.0.19 重新实现了完整的 `read_trae_login_from_dir`

---

## [1.0.11] - 2026-07-11

### 修复
- **实例管理页自动从旧账号 data_dir 创建了用户没创建的实例**：`migrate_from_accounts` 改为只创建默认实例，不再为旧账号的 data_dir 自动创建多开实例
- **实例卡片的「启动」按钮看不见**：CSS `background: var(--primary)` 但 `--primary` 未定义，导致背景透明 + 白色文字在白底上完全不可见。改用 `var(--gradient-accent)`（已定义的青蓝渐变），同时优化 hover 效果
- **点「启动」按钮直接打开 TRAE 没确认**：`handleLaunch` 加确认弹窗，确认后再启动；用 Toast 替代 alert 显示结果

> ⚠️ 此版本引入的 `store.instances.retain(...)` 过滤逻辑在 v1.0.19 中被移除

---

## [1.0.9] - 2026-07-09

### 性能优化
- **实例管理列表加载性能优化**：解决 `list_instances` 每次调用都同步递归遍历整个 data_dir（几 GB）计算 `get_dir_size` 导致严重阻塞的问题
  - 拆分 `list_instances` 为 `list_instances_basic`（快速返回基本信息）+ `compute_runtime_info`（慢速计算运行时信息），锁持有时间从「遍历全部目录」降到「clone 列表」
  - 用 `tokio::task::spawn_blocking` 在 blocking 线程计算 disk_usage 和 is_running，不阻塞 Tauri 的 async 运行时
  - 添加 disk_usage 缓存（`disk_cache: HashMap<data_dir, (timestamp, size)>`，TTL 5 分钟），轮询时复用缓存避免重复遍历
  - 批量检查进程状态（`check_instances_running_batch`），一次 tasklist 获取所有 TRAE SOLO CN.exe 进程，替代每个实例单独启动 tasklist 子进程
  - 前端轮询频率从 5 秒改为 15 秒
- **效果**：首次加载实例列表 <100ms（仅 clone），运行时信息异步加载不阻塞 UI；后续轮询直接命中缓存 <10ms

---

## [1.0.8] - 2026-07-08

### 新增
- **实例管理系统**：实例成为一等实体（`instances.json`），账号退为"令牌仓库"（`accounts.json`）
  - `TraeInstance` 结构：id, name, data_dir, is_default, bound_account_id, machine_id, note, created_at, updated_at, last_launched_at
  - 关系：1 账号 → 多实例（同一账号可绑到多个实例）；1 实例 → 1 当前账号
  - 首次启动自动从 accounts.json 迁移：创建默认实例 + 为有 data_dir 的账号创建多开实例 + 清除 Account.data_dir
  - 实例管理页作为应用首页，卡片网格展示实例（名称、绑定账号、磁盘占用、运行状态、启动按钮）
  - 运行状态检测：读 data-dir/code.lock 拿 PID，用 tasklist 检查进程存活
  - 磁盘占用：`get_dir_size` 递归计算 data-dir 大小
  - 快捷方式：`create_instance_shortcut` 用 PowerShell WScript.Shell 创建桌面快捷方式
  - 默认实例不可删除：指向 `%APPDATA%\TRAE SOLO CN`，是单实例切换的目标
  - 启动行为：总是新开进程，运行中时 toast 提示
  - 切换账号：只改绑定 + 写 storage.json，不杀进程，toast 提示"重启实例生效"

### 变更
- 账号管理页降为次要：侧边栏顺序为 实例管理 > 账号管理 > 设置 > 关于

---

## [1.0.7] - 2026-07-07

### 新增
- **多开模式**：利用 VSCode 的 `--user-data-dir` 参数为每个账号启动独立数据目录的 TRAE 实例
  - 不同 data-dir 的实例可并存，`code.lock` 单实例锁基于 data-dir，`machineid` 文件在 data-dir 内自动隔离
  - 插件目录通过 `--extensions-dir` 共享同一份插件目录（`%APPDATA%\TRAE SOLO CN_SharedExtensions`）
  - data-dir 命名：`%APPDATA%\TRAE SOLO CN_<user_id>`，首次多开时绑定到 `Account.data_dir` 字段，后续复用
  - 不杀进程：多开模式不调用 `kill_product`，不影响已运行的实例
  - 不修改系统注册表：多开模式只写 data-dir 内的 `machineid` 文件，不动 `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography\MachineGuid`
  - 初始化策略：空目录启动（新 data-dir 初始化为全新状态，仅写入登录信息）
  - 与单实例切换并存：右键菜单同时提供「切换账号」（单实例替换）和「多开实例」（新开窗口）

### 性能优化
- **kill_product 切换速度优化**：用轮询替代固定 `sleep(500ms) + sleep(1000ms)` = 1500ms
  - 优雅关闭最多轮询 800ms（每 100ms 检查），强制关闭最多轮询 500ms（每 50ms 检查）
  - 进程快速退出时 200-400ms 即完成，正常情况 300-600ms

---

## [1.0.5] - 2026-07-05

### 新增
- 账号导入导出功能（合并导入 / 替换导入）
  - `import_accounts(json, overwrite=false)`：合并模式，按 user_id 去重，跳过已存在的账号
  - `import_accounts(json, overwrite=true)`：替换模式，清空当前所有账号后导入

---

## [1.0.4] - 2026-07-04

### 新增
- 账号自定义备注（note），通过右键菜单编辑，AccountCard 中显示备注替代默认名称

### 修复
- UI 中所有"trae.ai"文案和链接改为"trae.cn"（AddAccountModal、UpdateTokenModal 的 Token 获取指引链接改为 https://www.trae.cn/）

---

## [1.0.3] - 2026-07-03

### 新增
- 浏览器登录使用 incognito 模式，每次打开都是全新会话，避免上次登录状态影响
- 浏览器登录 init script 会拦截 GetUserInfo 响应，并在捕获 token 后主动调用 GetUserInfo 获取真实用户名和头像
- AddAccountModal 用 `useRef` 保存最新的 onClose，避免 listen 闭包捕获陈旧引用（浏览器登录获取到账号后会自动关闭添加账号弹窗）

---

## [1.0.2] - 2026-07-02

### 新增
- 浏览器登录流程（使用 `https://work.trae.cn/` 页面）
- 通过 Token 获取用户信息时，优先调用 `GetUserInfo` 接口获取真实用户名（screen_name），失败时回退到 entitlement 接口（仅能获取 user_id 数字）
- 账号来源（source）字段: "browser"(浏览器登录), "local"(本地读取), "manual"(手动输入Token)

---

## [1.0.1] - 2026-07-01

### 修复
- 切换账号操作会自动关闭目标产品进程、清除旧 Cookies/会话缓存、写入新登录信息、启动产品
  - 保留 `state.vscdb` 和 `state.vscdb.backup`（用户 IDE 设置，删除会导致"命令运行方式"等设置被重置为默认值）
- 切换账号后自动打开 TRAE 失败会返回明确错误（不再静默吞掉），登录信息已写入，用户可手动打开
- `open_product` 在路径未设置时会自动调用 `scan_solo_cn_path` 扫描注册表和常见安装位置

### 已知问题
- TRAE Work CN 聊天会话存储在云端按 user_id 隔离，本地 `chat.ChatSessionStore.index` 只是空索引缓存；切换账号时保留 state.vscdb，聊天上下文切回账号时自动加载

---

## [1.0.0] - 2026-06-30

### 首次发布
- 基于 [Yang-505/Trae-Account-Manager](https://github.com/Yang-505/Trae-Account-Manager) 修改，专注 TRAE Work CN 中国版
- 核心功能：
  - 一键切换账号（关闭进程 → 写入登录信息 → 启动）
  - 通过 Token 添加账号
  - 从本地自动读取已登录账号（自动解密 `storage.json` 中的 AES-128-CBC + HMAC-SHA512 加密认证信息）
  - 机器码管理（系统机器码查看/修改/重置、产品机器码、绑定账号）
  - 路径配置（自动扫描注册表和常见安装位置 / 手动设置）
  - 清除登录状态
  - 关于页面（GitHub 仓库链接和作者信息）

### 技术细节
- 加密方案：`storage.json` 中的 `iCubeAuthInfo` 使用 AES-128-CBC + HMAC-SHA512 加密
  - Header (38 bytes)：`"tc"` (2 bytes) + `version=5` (1 byte) + `0x10/0x00/0x00` (3 bytes) + `embedded_key` (32 bytes)
  - Body：AES-128-CBC 加密的 `HMAC-SHA512(64 bytes) || plaintext + PKCS7 padding`
  - 密钥派生：从嵌入的 32 字节随机密钥派生 AES-128 密钥和 IV（`AES_CONSTANT = jQ XOR WQ`，`derived = SHA-512(SHA-512(embedded_key) || AES_CONSTANT)`，`aes_key = derived[0..16]`，`iv = derived[16..32]`）
- API 端点：`https://api.trae.cn`（中国版）
- 浏览器登录：`https://work.trae.cn/`

---

[1.0.21]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.21
[1.0.20]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.20
[1.0.19]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.19
[1.0.15]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.15
[1.0.12]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.12
[1.0.11]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.11
[1.0.9]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.9
[1.0.8]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.8
[1.0.7]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.7
[1.0.5]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.5
[1.0.4]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.4
[1.0.3]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.3
[1.0.2]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.2
[1.0.1]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.1
[1.0.0]: https://github.com/xhrxgr/Trae-Work-CN-Account-Manager/releases/v1.0.0
