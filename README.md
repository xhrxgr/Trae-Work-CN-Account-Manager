# TRAE Work CN Account Manager

<div align="center">

![TRAE Work CN Account Manager](https://img.shields.io/badge/TRAE%20Work%20CN-Account%20Manager-blue?style=for-the-badge)
![Version](https://img.shields.io/badge/version-1.0.21-green?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-orange?style=for-the-badge)

**多开 TRAE Work CN，每个实例独立登录不同账号，互不影响**

基于 [Yang-505/Trae-Account-Manager](https://github.com/Yang-505/Trae-Account-Manager) 修改，专注 TRAE Work CN 中国版

作者：[@xhrxgr](https://github.com/xhrxgr)（小黄人xgr）

</div>

---

## 核心功能

### 🚀 多开实例（主推）

**同时运行多个 TRAE Work CN 实例，每个实例登录不同账号，多窗口并行工作。**

- 基于 VSCode `--user-data-dir` 参数，为每个实例分配独立数据目录
- 各实例的登录状态、机器码、会话缓存完全隔离
- 插件目录共享（`--extensions-dir` 指向同一目录），无需重复安装
- 不杀进程、不改系统注册表，新开窗口不影响已运行实例
- 启动时自动发现 `%APPDATA%\TRAE SOLO CN*` 下已有 data-dir 并加入实例列表
- 启动时自动按 user_id 匹配账号并绑定到对应实例
- 支持创建桌面快捷方式，双击直接启动指定实例

> **适用场景**：A 账号写代码、B 账号查资料、C 账号跑任务，多窗口并行工作，无需反复切换登录。

### 🔀 单实例切换（保留）

**一键切换默认实例的账号：自动关闭当前实例、写入新登录信息、重新启动。**

- 切换速度优化：轮询替代固定等待，300-600ms 完成（原 1500ms+）
- 保留 IDE 设置（`state.vscdb`），切换后工作现场不丢失
- 聊天上下文云端按账号隔离，切回账号时自动加载

### 📋 实例管理

- **实例为一等实体**：实例（`instances.json`）与账号（`accounts.json`）分离，一个账号可绑到多个实例
- **实例卡片**：展示名称、绑定账号、磁盘占用、运行状态、上次启动时间
- **自动同步**：启动时扫描每个实例的 `storage.json`，按 user_id 自动绑定本地账号；IDE 内手动登录的新账号会自动创建本地记录
- **实例备注**：右键菜单编辑备注，卡片上显示 📝 徽章
- **账号状态徽章**：非 Free 身份（Pro 等）显示速通次数和额外礼包，Free 版不显示徽章避免误读

### 🔄 账号管理

- 通过 Token 添加账号
- 浏览器登录获取 Token（incognito 模式，自动获取真实用户名和头像）
- 从本地自动读取已登录账号（自动解密 AES-128-CBC + HMAC-SHA512 加密的认证信息）
- 账号导入导出（合并导入 / 替换导入）
- Token 自动刷新（单个 / 批量）
- 机器码管理（系统机器码查看/修改/重置、产品机器码、绑定账号）
- 账号自定义备注

---

## 快速开始

### 下载

前往 [Releases](../../releases) 页面下载最新安装包（提供 EXE / MSI / NSIS 三种格式，NSIS 支持覆盖更新）。

### 从源码构建

```bash
git clone https://github.com/xhrxgr/Trae-Work-CN-Account-Manager.git
cd Trae-Work-CN-Account-Manager
npm install
npx tauri build
```

构建产物位于 `src-tauri/target/release/`。

### 基本使用

1. **首次启动**：管理器会自动扫描 TRAE Work CN 安装路径，并创建默认实例（指向 `%APPDATA%\TRAE SOLO CN`）
2. **创建实例**：点击「+ 创建实例」，输入名称即可（data-dir 自动生成，也可自定义路径）
3. **绑定账号**：右键实例 →「切换账号」，从账号列表中选择（没有账号时先去「账号管理」添加）
4. **启动实例**：点击实例卡片上的「▶ 启动」按钮，确认后即开新窗口
5. **自动发现**：如果你之前手动用 `--user-data-dir` 启动过 TRAE，管理器会自动把这些 data-dir 加入实例列表

---

## 磁盘空间说明

每个 TRAE 实例的 data-dir 约占 **3-8 GB**，主要构成：

| 目录 | 大小 | 说明 | 是否可清理 |
|------|------|------|-----------|
| `ModularData` | ~4.9 GB | AI 模型数据 | ❌ 实例运行必需 |
| `logs` | ~1.3 GB | 日志文件 | ✅ 可安全清理 |
| `CachedData` | ~0.9 GB | V8 编译缓存 | ✅ 清理后首次启动略慢 |
| `Crashpad` | ~0.4 GB | 崩溃报告 | ✅ 可安全清理 |
| `User` | ~65 MB | 用户设置 + `storage.json`（登录信息） | ❌ 实例运行必需 |

### 已有优化

- **插件目录共享**：所有多开实例通过 `--extensions-dir` 共享 `%APPDATA%\TRAE SOLO CN_SharedExtensions`，插件只存一份
- **延迟创建 data-dir**：创建实例时只记录元数据，不预先创建目录；首次启动时由 TRAE 按需创建

### 节省空间的建议

- **定期清理日志和缓存**：可手动删除各实例 data-dir 下的 `logs`、`Crashpad`、`Cache`、`CachedData`、`GPUCache`、`DawnGraphiteCache`、`DawnWebGPUCache`、`Code Cache`、`VideoDecodeStats` 目录，TRAE 会自动重建
- **删除不用的实例**：右键实例 →「删除」并勾选「删除数据目录」
- **未来改进方向**：考虑用 Windows junction 让各实例的 `ModularData` 指向同一份（需验证是否包含实例特有数据）

---

## 技术栈

- **前端**: React 18 + TypeScript + Vite
- **后端**: Rust + Tauri 2
- **加密**: AES-128-CBC + HMAC-SHA512（`storage.json` 中的 `iCubeAuthInfo`）
- **API**: `https://api.trae.cn`（中国版）

---

## 更新日志

详见 [CHANGELOG.md](./CHANGELOG.md)。

---

## 免责声明

> **本工具仅供学习和技术研究使用。使用者需自行承担所有风险。**

---

## 致谢

- 原项目 [Yang-505/Trae-Account-Manager](https://github.com/Yang-505/Trae-Account-Manager)
- [Tauri](https://tauri.app/) - 桌面应用框架
- [React](https://react.dev/) - UI 框架

---

## License

MIT License
