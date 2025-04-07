use std::collections::HashMap;
use std::time::{Duration, Instant};
use actix::{Actor, Addr, Context, Handler, Supervised};
use log::{error, info, warn};
use tokio::process::Child;

use janus_core::actor::{
    ActorMetrics, ActorState, messages::{ExecuteCommand, LifecycleMessage, SupervisionMessage},
    supervisor::SupervisorActor,
    command::CommandActor,
    event::EventActor,
    connection::ConnectionActor,
};
use janus_core::error::CoreError;

use crate::config::ChromeBrowserConfig;
use crate::error::ChromeError;
use crate::launcher::ChromeLauncher;
use crate::protocol::{self, Command, Response};

#[derive(Debug)]
pub enum ChromeBrowserState {
    Starting,
    Ready,
    Degraded,
    Failed(ChromeError),
    Closed,
}

impl ActorState for ChromeBrowserState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "Starting",
            Self::Ready => "Ready",
            Self::Degraded => "Degraded",
            Self::Failed(_) => "Failed",
            Self::Closed => "Closed",
        }
    }
}

#[derive(Debug)]
struct ChromePageInfo {
    target_id: String,
    session_id: Option<String>,
    url: String,
    title: String,
}

#[derive(Debug, Default)]
struct ChromeBrowserMetrics {
    pages_created: u64,
    pages_closed: u64,
    crashes: u64,
    last_page_at: Option<std::time::SystemTime>,
    last_error_at: Option<std::time::SystemTime>,
}

pub struct ChromeBrowserActor {
    config: ChromeBrowserConfig,
    state: ChromeBrowserState,
    supervisor: Addr<SupervisorActor>,
    command: Addr<CommandActor>,
    event: Addr<EventActor>,
    connection: Addr<ConnectionActor>,
    process: Option<Child>,
    ws_url: Option<String>,
    pages: HashMap<String, ChromePageInfo>,
    metrics: ChromeBrowserMetrics,
}

impl Actor for ChromeBrowserActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("ChromeBrowserActor started");
        
        // Register with supervisor
        self.supervisor.do_send(SupervisionMessage::RegisterChild {
            actor_type: "chrome_browser",
            id: "main".to_string(),
        });

        // Start Chrome process and establish connection
        self.launch_browser(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("ChromeBrowserActor stopped");
        self.cleanup();
    }
}

impl Supervised for ChromeBrowserActor {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        warn!("ChromeBrowserActor is being restarted");
        self.cleanup();
    }
}

impl ChromeBrowserActor {
    pub fn new(
        config: ChromeBrowserConfig,
        supervisor: Addr<SupervisorActor>,
        command: Addr<CommandActor>,
        event: Addr<EventActor>,
        connection: Addr<ConnectionActor>,
    ) -> Result<Self, ChromeError> {
        config.validate().map_err(|e| ChromeError::CoreError(e.into()))?;
        
        Ok(Self {
            config,
            state: ChromeBrowserState::Starting,
            supervisor,
            command,
            event,
            connection,
            process: None,
            ws_url: None,
            pages: HashMap::new(),
            metrics: ChromeBrowserMetrics::default(),
        })
    }

    fn launch_browser(&mut self, ctx: &mut Context<Self>) {
        self.state = ChromeBrowserState::Starting;

        // 创建 launcher
        let launcher = match ChromeLauncher::new() {
            Ok(launcher) => launcher,
            Err(e) => {
                error!("Failed to create Chrome launcher: {}", e);
                self.handle_browser_error(e);
                return;
            }
        };

        // 准备 Chrome 启动参数
        let mut args = vec![
            format!("--remote-debugging-port={}", launcher.get_port()),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
        ];

        if self.config.headless {
            args.push("--headless=new".to_string());
        }

        if self.config.ignore_https_errors {
            args.push("--ignore-certificate-errors".to_string());
        }

        args.extend(self.config.args.clone());

        // 添加用户数据目录（如果指定）
        if let Some(ref dir) = self.config.user_data_dir {
            args.push("--user-data-dir".to_string());
            args.push(dir.to_str().unwrap().to_string());
        }

        // 查找 Chrome 可执行文件
        let chrome_path = self.config.executable_path.clone()
            .unwrap_or_else(|| {
                #[cfg(target_os = "macos")]
                return "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".to_string();
                #[cfg(target_os = "windows")]
                return r"C:\Program Files\Google\Chrome\Application\chrome.exe".to_string();
                #[cfg(target_os = "linux")]
                return "google-chrome".to_string();
            });

        // 启动 Chrome 进程
        let process = tokio::process::Command::new(&chrome_path)
            .args(&args)
            .envs(&self.config.env)
            .kill_on_drop(true)
            .spawn();

        match process {
            Ok(child) => {
                let mut launcher = launcher;
                launcher.set_process(child);
                self.process = launcher.take_process();

                // 查找 WebSocket URL
                let launcher_clone = launcher;
                let addr = ctx.address();
                
                tokio::spawn(async move {
                    match launcher_clone.find_ws_url().await {
                        Ok(ws_url) => {
                            addr.do_send(BrowserMessage::WebSocketUrlFound(ws_url));
                        }
                        Err(e) => {
                            addr.do_send(BrowserMessage::WebSocketUrlError(e));
                        }
                    }
                });
            }
            Err(e) => {
                error!("Failed to launch Chrome: {}", e);
                self.handle_browser_error(ChromeError::LaunchError(e.to_string()));
            }
        }
    }

    fn handle_ws_url_found(&mut self, ws_url: String, ctx: &mut Context<Self>) {
        info!("Chrome WebSocket URL found: {}", ws_url);
        self.ws_url = Some(ws_url);
        self.connect_to_browser(ctx);
    }

    fn connect_to_browser(&mut self, ctx: &mut Context<Self>) {
        if let Some(ref ws_url) = self.ws_url {
            // 通过 ConnectionActor 建立连接
            let connect_msg = janus_core::actor::messages::Connect {
                url: ws_url.clone(),
            };

            let connection = self.connection.clone();
            let event = self.event.clone();
            let command = self.command.clone();
            let addr = ctx.address();

            tokio::spawn(async move {
                match connection.send(connect_msg).await {
                    Ok(Ok(_)) => {
                        // 设置事件订阅
                        // TODO: 实现事件订阅逻辑

                        // 初始化浏览器会话
                        let version_cmd = protocol::browser::GetVersion;
                        if let Ok(Ok(version)) = command.send(version_cmd.into()).await {
                            addr.do_send(BrowserMessage::BrowserReady);
                        }
                    }
                    Ok(Err(e)) => {
                        addr.do_send(BrowserMessage::ConnectionError(e.into()));
                    }
                    Err(e) => {
                        addr.do_send(BrowserMessage::ConnectionError(ChromeError::CoreError(e.into())));
                    }
                }
            });
        } else {
            self.handle_browser_error(ChromeError::LaunchError(
                "No WebSocket URL available".to_string()
            ));
        }
    }

    fn cleanup(&mut self) {
        // Close all pages
        self.pages.clear();

        // Kill Chrome process
        if let Some(mut process) = self.process.take() {
            let _ = process.start_kill();
        }

        self.ws_url = None;
        self.state = ChromeBrowserState::Closed;
    }

    fn handle_browser_error(&mut self, error: ChromeError) {
        error!("Browser error: {}", error);
        self.state = ChromeBrowserState::Failed(error.clone());
        self.metrics.crashes += 1;
        self.metrics.last_error_at = Some(std::time::SystemTime::now());
        
        self.supervisor.do_send(SupervisionMessage::ChildFailed {
            actor_type: "chrome_browser",
            id: "main".to_string(),
            error: error.into(),
        });
    }

    async fn create_page(&mut self) -> Result<String, ChromeError> {
        if self.pages.len() >= self.config.max_concurrent_pages {
            return Err(ChromeError::PageError(
                "Maximum number of concurrent pages reached".to_string()
            ));
        }

        // Create new target
        let create_target = ExecuteCommand {
            method: "Target.createTarget".to_string(),
            params: Some(serde_json::json!({
                "url": "about:blank",
                "width": self.config.default_viewport.as_ref().map(|v| v.width).unwrap_or(1280),
                "height": self.config.default_viewport.as_ref().map(|v| v.height).unwrap_or(720),
            })),
            timeout: Some(self.config.default_timeout),
        };

        match self.command.send(create_target).await {
            Ok(Ok(response)) => {
                let target_id = response.get("targetId")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ChromeError::ProtocolError(
                        "Missing targetId in response".to_string()
                    ))?
                    .to_string();

                // Create page info
                let page_info = ChromePageInfo {
                    target_id: target_id.clone(),
                    session_id: None,
                    url: "about:blank".to_string(),
                    title: String::new(),
                };

                self.pages.insert(target_id.clone(), page_info);
                self.metrics.pages_created += 1;
                self.metrics.last_page_at = Some(std::time::SystemTime::now());

                Ok(target_id)
            }
            Ok(Err(e)) => Err(ChromeError::ProtocolError(e.to_string())),
            Err(e) => Err(ChromeError::CoreError(e.into())),
        }
    }

    async fn close_page(&mut self, target_id: &str) -> Result<(), ChromeError> {
        if let Some(_page) = self.pages.remove(target_id) {
            let close_target = ExecuteCommand {
                method: "Target.closeTarget".to_string(),
                params: Some(serde_json::json!({
                    "targetId": target_id,
                })),
                timeout: Some(self.config.default_timeout),
            };

            match self.command.send(close_target).await {
                Ok(Ok(_)) => {
                    self.metrics.pages_closed += 1;
                    Ok(())
                }
                Ok(Err(e)) => Err(ChromeError::ProtocolError(e.to_string())),
                Err(e) => Err(ChromeError::CoreError(e.into())),
            }
        } else {
            Err(ChromeError::TargetNotFound(target_id.to_string()))
        }
    }
}

impl Handler<LifecycleMessage> for ChromeBrowserActor {
    type Result = ();

    fn handle(&mut self, msg: LifecycleMessage, ctx: &mut Context<Self>) {
        match msg {
            LifecycleMessage::Initialize => {
                self.state = ChromeBrowserState::Starting;
            }
            LifecycleMessage::Start => {
                if matches!(self.state, ChromeBrowserState::Starting) {
                    self.launch_browser(ctx);
                }
            }
            LifecycleMessage::Stop => {
                self.cleanup();
                ctx.stop();
            }
            LifecycleMessage::Restart => {
                self.cleanup();
                self.launch_browser(ctx);
            }
        }
    }
}

impl ActorMetrics for ChromeBrowserActor {
    fn get_metrics(&self) -> janus_core::actor::ActorMetricsData {
        janus_core::actor::ActorMetricsData {
            message_count: self.metrics.pages_created + self.metrics.pages_closed,
            error_count: self.metrics.crashes,
            last_activity: self.metrics.last_page_at
                .or(self.metrics.last_error_at)
                .unwrap_or_else(|| std::time::SystemTime::now()),
        }
    }
}

// 内部消息类型
#[derive(Debug)]
enum BrowserMessage {
    WebSocketUrlFound(String),
    WebSocketUrlError(ChromeError),
    ConnectionError(ChromeError),
    BrowserReady,
}

impl Handler<BrowserMessage> for ChromeBrowserActor {
    type Result = ();

    fn handle(&mut self, msg: BrowserMessage, ctx: &mut Context<Self>) {
        match msg {
            BrowserMessage::WebSocketUrlFound(ws_url) => {
                self.handle_ws_url_found(ws_url, ctx);
            }
            BrowserMessage::WebSocketUrlError(error) => {
                self.handle_browser_error(error);
            }
            BrowserMessage::ConnectionError(error) => {
                self.handle_browser_error(error);
            }
            BrowserMessage::BrowserReady => {
                info!("Chrome browser is ready");
                self.state = ChromeBrowserState::Ready;
            }
        }
    }
} 