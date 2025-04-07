use std::time::Duration;
use actix::System;
use futures::future::try_join_all;
use log::{error, info, LevelFilter};
use janus_core::actor::{
    supervisor::SupervisorActor,
    command::CommandActor,
    event::EventActor,
    connection::ConnectionActor,
    messages::LifecycleMessage,
};
use janus_chrome::{
    ChromeBrowserActor, ChromeBrowserConfig,
    browser::{GetVersion, CreatePage, GetWindowBounds},
};

#[actix::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::Builder::new()
        .filter_level(LevelFilter::Debug)
        .init();

    info!("Starting Janus Chrome navigation example");

    // 创建 Actor 系统
    let system = System::new();

    // 创建 Supervisor
    let supervisor = SupervisorActor::new().start();

    // 创建基础 Actors
    let command = CommandActor::new().start();
    let event = EventActor::new().start();
    let connection = ConnectionActor::new().start();

    // 配置 Chrome
    let config = ChromeBrowserConfig {
        headless: false,
        ignore_https_errors: true,
        default_timeout: Duration::from_secs(30),
        ..Default::default()
    };

    // 创建并启动 ChromeBrowserActor
    let browser = match ChromeBrowserActor::new(
        config,
        supervisor.clone(),
        command.clone(),
        event.clone(),
        connection.clone(),
    ) {
        Ok(browser) => browser.start(),
        Err(e) => {
            error!("Failed to create ChromeBrowserActor: {}", e);
            return Ok(());
        }
    };

    // 启动浏览器
    browser.do_send(LifecycleMessage::Start);

    // 等待浏览器启动完成
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 获取浏览器版本信息
    match browser.send(GetVersion).await {
        Ok(Ok(version)) => {
            info!("Chrome version: {:?}", version);
        }
        Err(e) => error!("Failed to get version: {}", e),
        Ok(Err(e)) => error!("Version error: {}", e),
    }

    // 创建多个页面
    let urls = vec![
        "https://www.rust-lang.org",
        "https://github.com",
        "https://www.google.com",
    ];

    let page_futures = urls.iter().map(|url| async {
        match browser.send(CreatePage::new(url)).await {
            Ok(Ok(page_id)) => {
                info!("Created page {} for {}", page_id, url);
                Some(page_id)
            }
            _ => {
                error!("Failed to create page for {}", url);
                None
            }
        }
    });

    // 等待所有页面创建完成
    let page_ids: Vec<_> = try_join_all(page_futures)
        .await
        .into_iter()
        .flatten()
        .collect();

    info!("Created {} pages", page_ids.len());

    // 获取窗口信息
    for page_id in &page_ids {
        match browser.send(GetWindowBounds { window_id: page_id.parse().unwrap() }).await {
            Ok(Ok(bounds)) => {
                info!("Window bounds for {}: {:?}", page_id, bounds);
            }
            _ => error!("Failed to get window bounds for {}", page_id),
        }
    }

    // 等待用户输入以保持程序运行
    println!("Press Enter to exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // 关闭浏览器
    browser.do_send(LifecycleMessage::Stop);

    // 关闭 Actor 系统
    System::current().stop();

    Ok(())
} 