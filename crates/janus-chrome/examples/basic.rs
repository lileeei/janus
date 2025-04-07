use std::time::Duration;
use actix::System;
use log::{error, info, LevelFilter};
use janus_core::actor::{
    supervisor::SupervisorActor,
    command::CommandActor,
    event::EventActor,
    connection::ConnectionActor,
    messages::LifecycleMessage,
};
use janus_chrome::{ChromeBrowserActor, ChromeBrowserConfig};

#[actix::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::Builder::new()
        .filter_level(LevelFilter::Debug)
        .init();

    info!("Starting Janus Chrome example");

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
        headless: false, // 设置为 false 以便看到浏览器窗口
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