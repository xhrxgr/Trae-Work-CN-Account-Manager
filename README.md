<div align="center">

# TRAE Work CN Account Manager

**一款用于管理 TRAE Work CN 多账号与多实例的桌面工具**

[![Version](https://img.shields.io/badge/version-1.0.22-00B4D8?style=flat-square)](../../releases)
[![Platform](https://img.shields.io/badge/platform-Windows-0078D4?style=flat-square&logo=windows&logoColor=white)](../../releases)
[![License](https://img.shields.io/badge/license-MIT-FF6B35?style=flat-square)](./LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2-FFC131?style=flat-square&logo=tauri&logoColor=black)](https://tauri.app/)

</div>

---

## 项目简介

TRAE Work CN Account Manager 是一款基于 Tauri 2 的桌面应用，专为 TRAE Work CN（原 TRAE SOLO CN）中国版用户设计。通过为每个账号分配独立的数据目录，实现多账号同时在线、互不干扰，彻底解决频繁切换登录的痛点。

> **适用人群**：需要在多个 TRAE Work CN 账号之间并行工作的开发者、研究人员。

---

## 核心功能

### 🚀 多开实例（主要特性）

同时运行多个 TRAE Work CN 实例，每个实例独立登录不同账号，多窗口并行工作。

- **数据隔离**：基于 VSCode `--user-data-dir` 参数，每个实例拥有独立的登录状态、机器码与会话缓存
- **插件共享**：通过 `--extensions-dir` 共享插件目录，无需重复安装
- **零侵入**：不杀进程、不修改系统注册表，新开窗口不影响已运行实例
- **自动发现**：启动时自动扫描 `%APPDATA%\TRAE SOLO CN*` 下已有 data-dir 并登记为实例
- **自动绑定**：按 user_id 自动匹配账号并绑定到对应实例；IDE 内手动登录的新账号会自动创建本地记录
- **快捷启动**：支持创建桌面快捷方式，双击直接启动指定实例

### 🔀 单实例切换

一键切换默认实例的账号：自动关闭当前实例 → 写入新登录信息 → 重新启动。

- 切换速度优化：轮询替代固定等待，300–600ms 完成
- 保留 IDE 设置（`state.vscdb`），工作现场不丢失
- 聊天上下文云端按账号隔离，切回时自动加载

### 📋 实例管理

| 能力 | 说明 |
|------|------|
| 实例卡片 | 展示名称、绑定账号、磁盘占用、运行状态、上次启动时间 |
| 实例备注 | 右键菜单编辑备注，卡片上显示 📝 徽章 |
| 账号状态徽章 | 非 Free 身份（Pro 等）显示速通次数和额外礼包 |
| 自动同步 | 启动时扫描 `storage.json`，按 user_id 自动绑定账号 |
| 实例为一等实体 | 实例（`instances.json`）与账号（`accounts.json`）分离，一个账号可绑到多个实例 |

### 🔄 账号管理

- 通过 Token 添加账号
- 浏览器登录自动获取 Token（incognito 模式，自动获取真实用户名和头像）
- 从本地自动读取已登录账号（自动解密 AES-128-CBC + HMAC-SHA512 加密的认证信息）
- 账号导入导出（合并导入 / 替换导入）
- Token 自动刷新（单个 / 批量）
- 机器码管理（系统机器码查看 / 修改 / 重置、产品机器码、绑定账号）
- 账号自定义备注

---

## 快速开始

### 下载安装

前往 [Releases](../../releases) 页面下载最新安装包：

| 格式 | 说明 |
|------|------|
| `.exe` | 免安装绿色版，直接运行 |
| `.msi` | Windows Installer 标准安装包 |
| `.exe` (NSIS Setup) | 推荐方式，支持覆盖更新，无需先卸载旧版本 |

### 从源码构建

```bash
git clone https://github.com/xhrxgr/Trae-Work-CN-Account-Manager.git
cd Trae-Work-CN-Account-Manager
npm install
npx tauri build
```

构建产物位于 `src-tauri/target/release/`。

> **前置依赖**：[Node.js](https://nodejs.org/)、[Rust](https://www.rust-lang.org/)、Tauri 2 CLI

### 基本使用

1. **首次启动** — 管理器自动扫描 TRAE Work CN 安装路径，并创建默认实例（指向 `%APPDATA%\TRAE SOLO CN`）
2. **创建实例** — 点击「+ 创建实例」，输入名称即可（data-dir 自动生成，也可自定义路径）
3. **绑定账号** — 右键实例 →「切换账号」，从账号列表中选择（没有账号时先去「账号管理」添加）
4. **启动实例** — 点击实例卡片上的「▶ 启动」按钮，确认后即开新窗口
5. **自动发现** — 之前手动用 `--user-data-dir` 启动过的 TRAE 会被自动加入实例列表

---

## 磁盘空间说明

每个 TRAE 实例的 data-dir 约占 **3–8 GB**，主要构成：

| 目录 | 大小 | 说明 | 可否清理 |
|------|------|------|---------|
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

---

## 技术栈

| 层 | 技术 |
|----|------|
| 前端 | React 18 + TypeScript + Vite |
| 后端 | Rust + Tauri 2 |
| 加密 | AES-128-CBC + HMAC-SHA512（`storage.json` 中的 `iCubeAuthInfo`） |
| API | `https://api.trae.cn`（中国版） |

---

## 更新日志

详见 [CHANGELOG.md](./CHANGELOG.md)。

---

## 项目结构

```
src-tauri/src/
├── main.rs                  # 应用入口
├── lib.rs                   # Tauri 命令注册
├── machine.rs               # 产品操作（进程、路径、机器码、登录状态、加解密）
├── login.rs                 # 浏览器登录流程
├── account/                 # 账号管理模块
│   ├── types.rs             # 账号类型定义
│   └── account_manager.rs   # 账号管理器
├── instance/                # 实例管理模块
│   ├── types.rs             # 实例类型定义
│   └── instance_manager.rs  # 实例管理器（CRUD + 启动 + 快捷方式）
└── api/                     # TRAE API 客户端
    ├── trae_api.rs          # API 调用
    └── types.rs             # API 类型定义

src/
├── App.tsx                  # 主应用组件
├── api.ts                   # 前端 API 封装
├── pages/                   # 页面（实例管理 / 账号管理 / 设置 / 关于）
└── components/              # 通用组件（卡片、弹窗、右键菜单等）
```

---

## 免责声明

> **本工具仅供学习和技术研究使用。使用者需自行承担所有风险。**
>
> 使用本工具可能违反 TRAE Work CN 的服务条款。作者不对因使用本工具而导致的任何后果负责，包括但不限于账号封禁、数据丢失等。

---

## 致谢

- 原项目 [Yang-505/Trae-Account-Manager](https://github.com/Yang-505/Trae-Account-Manager)
- [Tauri](https://tauri.app/) — 桌面应用框架
- [React](https://react.dev/) — UI 框架

---

## License

[MIT License](./LICENSE)
