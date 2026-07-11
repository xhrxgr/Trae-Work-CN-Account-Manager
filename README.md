# TRAE Work CN Account Manager

<div align="center">

![TRAE Work CN Account Manager](https://img.shields.io/badge/TRAE%20Work%20CN-Account%20Manager-blue?style=for-the-badge)
![Version](https://img.shields.io/badge/version-1.0.7-green?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-orange?style=for-the-badge)

**管理 TRAE Work CN 多账号一键切换 · 支持多开**

基于 [Yang-505/Trae-Account-Manager](https://github.com/Yang-505/Trae-Account-Manager) 修改，专注 TRAE Work CN 中国版

作者：[@xhrxgr](https://github.com/xhrxgr)（小黄人xgr）

</div>

---

## 核心特色

### 🚀 多开模式（v1.0.7+）

**同时运行多个 TRAE Work CN 实例，每个实例登录不同账号，互不影响。**

- 基于 VSCode `--user-data-dir` 参数，为每个账号分配独立数据目录
- 各实例的登录状态、机器码、会话缓存完全隔离
- 插件目录共享，无需重复安装
- 不杀进程、不改系统注册表，与单实例切换模式并存
- 右键账号卡片 →「🚀 多开实例」即可启动

> 适用场景：A 账号写代码、B 账号查资料，多窗口并行工作。

### 🔀 单实例切换

**一键切换账号，自动关闭当前实例、写入新登录信息、重新启动。**

- 切换速度优化：轮询替代固定等待，300-600ms 完成（原 1500ms+）
- 保留 IDE 设置（state.vscdb），切换后工作现场不丢失
- 聊天上下文云端按账号隔离，切回即自动加载

---

## 项目简介

本工具用于管理 **TRAE Work CN**（中国版）的多个账号，支持多开、一键切换、机器码管理、账号导入导出。

### 功能特性

- **多开模式**：独立 data-dir 启动多个实例，互不影响，共享插件
- **一键切换账号**：自动关闭进程、写入登录信息、重新启动
- 通过 Token 添加账号
- 浏览器登录获取 Token（incognito 模式，自动获取真实用户名和头像）
- 从本地自动读取已登录账号（自动解密本地认证信息）
- 账号导入导出（合并导入 / 替换导入）
- Token 自动刷新（单个 / 批量）
- 机器码管理（系统机器码查看/修改/重置、产品机器码、绑定账号）
- 路径配置（自动扫描注册表和常见安装位置 / 手动设置）
- 账号自定义备注（右键编辑）
- 清除登录状态

## 快速开始

### 下载

前往 [Releases](../../releases) 页面下载最新安装包（提供 EXE / MSI / NSIS 三种格式）。

### 从源码构建

```bash
git clone https://github.com/xhrxgr/Trae-Work-CN-Account-Manager.git
cd Trae-Work-CN-Account-Manager
npm install
npx tauri build
```

构建产物位于 `src-tauri/target/release/`。

## 技术栈

- **前端**: React 18 + TypeScript + Vite
- **后端**: Rust + Tauri 2

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
