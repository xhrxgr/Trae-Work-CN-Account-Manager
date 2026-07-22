use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::{oneshot, Mutex};
use warp::Filter;

use crate::account::{AccountManager, BrowserUserInfo};

pub async fn start_login_flow(
    app: AppHandle,
    state: Arc<Mutex<AccountManager>>,
) -> Result<(), String> {
    // 如果已有登录窗口，聚焦它
    if let Some(win) = app.get_webview_window("trae-login") {
        let _ = win.set_focus();
        return Ok(());
    }

    // 创建 oneshot channel 用于通知 warp 服务停止
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));

    let app_clone = app.clone();
    let state_clone = state.clone();

    // POST /callback — 接收 token、cookies 和用户信息
    let callback = warp::post()
        .and(warp::path("callback"))
        .and(warp::body::json())
        .and_then(move |body: serde_json::Value| {
            let app = app_clone.clone();
            let state = state_clone.clone();
            async move {
                let token = body["token"].as_str().unwrap_or("");
                if token.is_empty() {
                    return Ok::<_, warp::Rejection>(warp::reply::json(
                        &serde_json::json!({"status": "waiting"}),
                    ));
                }

                // 提取 cookies（如果有）
                let cookies = body["cookies"].as_str().map(|s| s.to_string());

                // 提取浏览器拦截到的 refreshToken（v1.0.22+ 关键修复：避免几小时后 token 失效）
                let refresh_token = body["refresh_token"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());

                // 提取浏览器拦截到的用户信息
                let browser_user_info = {
                    let screen_name = body["screen_name"].as_str().unwrap_or("");
                    let avatar_url = body["avatar_url"].as_str().unwrap_or("");
                    let email = body["email"].as_str().unwrap_or("");
                    if screen_name.is_empty() && avatar_url.is_empty() && email.is_empty() {
                        None
                    } else {
                        Some(BrowserUserInfo {
                            screen_name: screen_name.to_string(),
                            avatar_url: avatar_url.to_string(),
                            email: email.to_string(),
                        })
                    }
                };

                let mut manager = state.lock().await;
                match manager
                    .add_account_by_token(
                        token.to_string(),
                        cookies,
                        "browser".to_string(),
                        browser_user_info,
                        refresh_token,
                    )
                    .await
                {
                    Ok(account) => {
                        let _ = app.emit("login-success", &account.email);
                        // 延迟关闭窗口，让 warp 先返回响应
                        let app2 = app.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                            if let Some(win) = app2.get_webview_window("trae-login") {
                                let _ = win.close();
                            }
                        });
                        Ok(warp::reply::json(&serde_json::json!({"status": "ok"})))
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("已存在") {
                            let _ = app.emit("login-failed", "该账号已存在");
                            let app2 = app.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                if let Some(win) = app2.get_webview_window("trae-login") {
                                    let _ = win.close();
                                }
                            });
                        }
                        Ok(warp::reply::json(
                            &serde_json::json!({"status": "error", "message": msg}),
                        ))
                    }
                }
            }
        });

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["POST"])
        .allow_headers(vec!["content-type"]);

    let routes = callback.with(cors);

    let (addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async {
            let _ = shutdown_rx.await;
        });
    let port = addr.port();

    tokio::spawn(server);

    // 注入 JS：
    // 1. Hook fetch/XHR 拦截 GetUserToken 响应，提取 token
    // 2. 拦截 GetUserInfo 响应，提取 screen_name、avatar_url
    // 3. 捕获到 token 后，主动调用 GetUserInfo 获取真实用户名和头像
    let init_script = format!(
        r#"
        (function() {{
            var __sent = false;
            var __callbackUrl = "http://127.0.0.1:{port}/callback";
            var __userInfo = {{ screen_name: "", avatar_url: "", email: "" }};
            // 关键修复（v1.0.22）：必须保存 refreshToken，否则几小时后 access token 过期无法续期
            var __refreshToken = "";

            function tryExtractUserInfo(text) {{
                try {{
                    var data = typeof text === "string" ? JSON.parse(text) : text;
                    if (data && data.Result) {{
                        var r = data.Result;
                        if (r.ScreenName) __userInfo.screen_name = r.ScreenName;
                        if (r.AvatarUrl) __userInfo.avatar_url = r.AvatarUrl;
                        if (r.NonPlainTextEmail) __userInfo.email = r.NonPlainTextEmail;
                    }}
                }} catch(e) {{}}
            }}

            function sendToBackend(token) {{
                if (__sent || !token || token.length < 50) return;
                __sent = true;

                var cookies = document.cookie || "";

                console.log("[Trae Auto] 捕获到 Token，长度:", token.length);
                console.log("[Trae Auto] 已拦截用户信息:", JSON.stringify(__userInfo));
                console.log("[Trae Auto] 已拦截 refreshToken:", __refreshToken ? "是（长度 " + __refreshToken.length + "）" : "否");

                // 用 token 主动调用 GetUserInfo 获取真实用户名和头像
                // 这样即使页面没有调用 GetUserInfo，也能获取到用户信息
                fetch("https://api.trae.cn/cloudide/api/v3/trae/GetUserInfo", {{
                    method: "POST",
                    headers: {{
                        "Content-Type": "application/json",
                        "Authorization": "Cloud-IDE-JWT " + token,
                        "Origin": "https://www.trae.cn",
                        "Referer": "https://www.trae.cn/"
                    }},
                    body: JSON.stringify({{"IfWebPage": true}})
                }}).then(function(resp) {{
                    return resp.json();
                }}).then(function(data) {{
                    tryExtractUserInfo(JSON.stringify(data));
                    doSend(token, cookies);
                }}).catch(function(e) {{
                    console.log("[Trae Auto] 主动调用 GetUserInfo 失败，使用已拦截的信息");
                    doSend(token, cookies);
                }});
            }}

            function doSend(token, cookies) {{
                var xhr = new XMLHttpRequest();
                xhr.open("POST", __callbackUrl, true);
                xhr.setRequestHeader("Content-Type", "application/json");
                xhr.send(JSON.stringify({{
                    token: token,
                    refresh_token: __refreshToken || "",
                    cookies: cookies,
                    screen_name: __userInfo.screen_name || "",
                    avatar_url: __userInfo.avatar_url || "",
                    email: __userInfo.email || ""
                }}));
            }}

            // 同时提取 token 和 refreshToken，避免几小时后 token 失效
            function tryExtractToken(text) {{
                try {{
                    var data = typeof text === "string" ? JSON.parse(text) : text;
                    if (data && data.Result && data.Result.Token) {{
                        // 关键修复：同时保存 refreshToken，让 TRAE 能在几小时后自动续期
                        if (data.Result.RefreshToken) {{
                            __refreshToken = data.Result.RefreshToken;
                        }}
                        return data.Result.Token;
                    }}
                }} catch(e) {{}}
                return null;
            }}

            // Hook fetch
            var origFetch = window.fetch;
            window.fetch = function() {{
                var url = arguments[0];
                if (typeof url === "object" && url.url) url = url.url;
                var p = origFetch.apply(this, arguments);
                if (typeof url === "string") {{
                    if (url.indexOf("GetUserToken") !== -1) {{
                        p.then(function(resp) {{
                            return resp.clone().text();
                        }}).then(function(text) {{
                            var token = tryExtractToken(text);
                            if (token) sendToBackend(token);
                        }}).catch(function() {{}});
                    }}
                    if (url.indexOf("GetUserInfo") !== -1) {{
                        p.then(function(resp) {{
                            return resp.clone().text();
                        }}).then(function(text) {{
                            tryExtractUserInfo(text);
                        }}).catch(function() {{}});
                    }}
                }}
                return p;
            }};

            // Hook XMLHttpRequest
            var origOpen = XMLHttpRequest.prototype.open;
            var origSend = XMLHttpRequest.prototype.send;
            XMLHttpRequest.prototype.open = function(method, url) {{
                this.__url = url;
                return origOpen.apply(this, arguments);
            }};
            XMLHttpRequest.prototype.send = function() {{
                var self = this;
                if (self.__url) {{
                    if (self.__url.indexOf("GetUserToken") !== -1) {{
                        self.addEventListener("load", function() {{
                            var token = tryExtractToken(self.responseText);
                            if (token) sendToBackend(token);
                        }});
                    }}
                    if (self.__url.indexOf("GetUserInfo") !== -1) {{
                        self.addEventListener("load", function() {{
                            tryExtractUserInfo(self.responseText);
                        }});
                    }}
                }}
                return origSend.apply(this, arguments);
            }};
        }})();
    "#,
        port = port
    );

    // 使用 incognito 模式，每次打开都是全新的会话
    // 这样第二次添加账号时不会使用上次的登录状态
    let window = WebviewWindowBuilder::new(
        &app,
        "trae-login",
        WebviewUrl::External("https://work.trae.cn/".parse().unwrap()),
    )
    .title("登录 Trae 账号")
    .inner_size(500.0, 700.0)
    .center()
    .incognito(true)
    .initialization_script(&init_script)
    .build()
    .map_err(|e| e.to_string())?;

    // 监听窗口关闭，停止 warp 服务并通知前端
    let shutdown_on_close = shutdown_tx.clone();
    let app_for_close = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            let shutdown = shutdown_on_close.clone();
            let app = app_for_close.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(tx) = shutdown.lock().await.take() {
                    // shutdown 还在说明不是登录成功后关的窗口，是用户手动关的
                    let _ = app.emit("login-cancelled", ());
                    let _ = tx.send(());
                }
            });
        }
    });

    Ok(())
}
