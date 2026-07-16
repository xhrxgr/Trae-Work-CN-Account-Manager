import { openUrl } from "@tauri-apps/plugin-opener";
import logoImage from "../assets/logo.png";

const REPO_URL = "https://github.com/xhrxgr/Trae-Work-CN-Account-Manager";
const AUTHOR_GITHUB_URL = "https://github.com/xhrxgr";

function GitHubIcon({ size = 20 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M12 .5C5.73.5.5 5.73.5 12c0 5.08 3.29 9.39 7.86 10.91.58.11.79-.25.79-.56 0-.28-.01-1.02-.02-2-3.2.7-3.88-1.54-3.88-1.54-.53-1.34-1.3-1.7-1.3-1.7-1.06-.72.08-.71.08-.71 1.17.08 1.78 1.2 1.78 1.2 1.04 1.78 2.73 1.27 3.4.97.11-.75.41-1.27.74-1.56-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.19-3.1-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.18 1.18a11.1 11.1 0 0 1 2.9-.39c.98 0 1.97.13 2.9.39 2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.84 1.19 3.1 0 4.42-2.69 5.39-5.25 5.68.42.36.79 1.08.79 2.18 0 1.58-.01 2.85-.01 3.24 0 .31.21.68.8.56A11.51 11.51 0 0 0 23.5 12C23.5 5.73 18.27.5 12 .5z"/>
    </svg>
  );
}

export function About() {
  const openRepo = () => openUrl(REPO_URL);
  const openAuthor = () => openUrl(AUTHOR_GITHUB_URL);

  return (
    <div className="about-page">
      <div className="about-card">
        <div className="about-logo">
          <img src={logoImage} alt="Logo" className="about-logo-image" />
        </div>
        <h3>TRAE Work CN Account Manager</h3>
        <p className="about-version">版本 1.0.17</p>
        <p className="about-desc">
          TRAE Work CN 账号管理工具，帮助您轻松管理多个 TRAE Work CN 账号并一键切换。
        </p>

        <div className="about-links">
          <button
            className="about-link-btn"
            onClick={openRepo}
            title="在 GitHub 查看项目仓库"
          >
            <GitHubIcon size={20} />
            <span>项目仓库</span>
          </button>
        </div>
      </div>

      <div className="about-section">
        <h3>功能特性</h3>
        <ul className="feature-list">
          <li>🔄 一键切换 TRAE Work CN 账号</li>
          <li>📋 账号导入导出与批量管理</li>
          <li>🔑 Token 自动刷新与机器码管理</li>
          <li>🎨 简洁美观的界面</li>
        </ul>
      </div>

      <div className="about-section">
        <h3>技术栈</h3>
        <div className="tech-tags">
          <span className="tech-tag">Tauri</span>
          <span className="tech-tag">React</span>
          <span className="tech-tag">TypeScript</span>
          <span className="tech-tag">Rust</span>
        </div>
      </div>

      <div className="about-section about-author">
        <h3>作者</h3>
        <div className="author-info">
          <button
            className="author-link"
            onClick={openAuthor}
            title="在 GitHub 查看作者主页"
          >
            <GitHubIcon size={18} />
            <span className="author-name">小黄人xgr</span>
            <span className="author-handle">@xhrxgr</span>
          </button>
        </div>
      </div>
    </div>
  );
}
