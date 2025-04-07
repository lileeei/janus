use std::time::Duration;
use actix::{Actor, Addr, Context, Handler, Supervised};
use log::{error, info, warn};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::error::{CoreError, TransportError};
use super::{
    ActorConfig, ActorError, ActorMetrics, ActorState,
    messages::{IncomingRawMessage, LifecycleMessage, SendRawMessage, SupervisionMessage},
    supervisor::SupervisorActor,
};

#[derive(Debug)]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected(Option<TransportError>),
}

impl ActorState for ConnectionState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Connecting => "Connecting",
            Self::Connected => "Connected",
            Self::Disconnecting => "Disconnecting",
            Self::Disconnected(_) => "Disconnected",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub url: String,
    pub connect_timeout: Duration,
    pub heartbeat_interval: Option<Duration>,
    pub max_message_size: Option<usize>,
}

impl ActorConfig for ConnectionConfig {
    fn validate(&self) -> Result<(), ActorError> {
        if self.url.is_empty() {
            return Err(ActorError::InitializationError("URL cannot be empty".to_string()));
        }
        if self.connect_timeout.is_zero() {
            return Err(ActorError::InitializationError("Connect timeout cannot be zero".to_string()));
        }
        Ok(())
    }
}

pub struct ConnectionActor {
    config: ConnectionConfig,
    state: ConnectionState,
    supervisor: Addr<SupervisorActor>,
    tx: Option<mpsc::Sender<WsMessage>>,
    metrics: ConnectionMetrics,
}

#[derive(Debug, Default)]
struct ConnectionMetrics {
    messages_sent: u64,
    messages_received: u64,
    errors: u64,
    last_message_at: Option<std::time::SystemTime>,
    last_error_at: Option<std::time::SystemTime>,
}

impl Actor for ConnectionActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("ConnectionActor started");
        
        // Register with supervisor
        self.supervisor.do_send(SupervisionMessage::RegisterChild {
            actor_type: "connection",
            id: self.config.url.clone(),
        });

        // Start connection process
        self.connect(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("ConnectionActor stopped");
        self.supervisor.do_send(SupervisionMessage::ChildStopped {
            actor_type: "connection",
            id: self.config.url.clone(),
        });
    }
}

impl Supervised for ConnectionActor {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        warn!("ConnectionActor is being restarted");
        self.state = ConnectionState::Idle;
        self.tx = None;
    }
}

impl ConnectionActor {
    pub fn new(config: ConnectionConfig, supervisor: Addr<SupervisorActor>) -> Result<Self, ActorError> {
        config.validate()?;
        Ok(Self {
            config,
            state: ConnectionState::Idle,
            supervisor,
            tx: None,
            metrics: ConnectionMetrics::default(),
        })
    }

    fn connect(&mut self, ctx: &mut Context<Self>) {
        self.state = ConnectionState::Connecting;
        
        let url = self.config.url.clone();
        let timeout = self.config.connect_timeout;
        let addr = ctx.address();

        // Create channel for WebSocket messages
        let (tx, mut rx) = mpsc::channel(32);
        self.tx = Some(tx);

        // Spawn connection task
        let fut = async move {
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();

                    // Handle incoming messages
                    tokio::spawn(async move {
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(WsMessage::Text(text)) => {
                                    addr.do_send(IncomingRawMessage { payload: text });
                                }
                                Ok(WsMessage::Close(_)) => {
                                    break;
                                }
                                Err(e) => {
                                    error!("WebSocket read error: {}", e);
                                    break;
                                }
                                _ => {} // Ignore other message types
                            }
                        }
                    });

                    // Handle outgoing messages
                    tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            if let Err(e) = write.send(msg).await {
                                error!("WebSocket write error: {}", e);
                                break;
                            }
                        }
                    });

                    Ok(())
                }
                Err(e) => Err(TransportError::ConnectionFailed(e.to_string())),
            }
        };

        // Spawn timeout future
        ctx.spawn(
            tokio::time::timeout(timeout, fut)
                .map(|result| match result {
                    Ok(Ok(_)) => {
                        self.state = ConnectionState::Connected;
                        info!("WebSocket connected to {}", url);
                    }
                    Ok(Err(e)) => {
                        self.handle_connection_error(e);
                    }
                    Err(_) => {
                        self.handle_connection_error(TransportError::Timeout(
                            "Connection timed out".to_string(),
                        ));
                    }
                })
                .into_actor(self),
        );
    }

    fn handle_connection_error(&mut self, error: TransportError) {
        error!("Connection error: {}", error);
        self.state = ConnectionState::Disconnected(Some(error.clone()));
        self.metrics.errors += 1;
        self.metrics.last_error_at = Some(std::time::SystemTime::now());
        
        self.supervisor.do_send(SupervisionMessage::ChildFailed {
            actor_type: "connection",
            id: self.config.url.clone(),
            error: CoreError::Transport(error),
        });
    }
}

impl Handler<SendRawMessage> for ConnectionActor {
    type Result = Result<(), CoreError>;

    fn handle(&mut self, msg: SendRawMessage, ctx: &mut Context<Self>) -> Self::Result {
        match &self.state {
            ConnectionState::Connected => {
                if let Some(tx) = &self.tx {
                    let tx = tx.clone();
                    let payload = msg.payload;
                    let timeout = msg.timeout.unwrap_or(self.config.connect_timeout);

                    ctx.spawn(
                        async move {
                            match tokio::time::timeout(
                                timeout,
                                tx.send(WsMessage::Text(payload)),
                            )
                            .await
                            {
                                Ok(Ok(_)) => Ok(()),
                                Ok(Err(e)) => Err(CoreError::Transport(
                                    TransportError::SendFailed(e.to_string()),
                                )),
                                Err(_) => Err(CoreError::Transport(TransportError::Timeout(
                                    "Send timed out".to_string(),
                                ))),
                            }
                        }
                        .into_actor(self)
                        .map(|result, actor, _ctx| {
                            if result.is_ok() {
                                actor.metrics.messages_sent += 1;
                                actor.metrics.last_message_at = Some(std::time::SystemTime::now());
                            } else {
                                actor.metrics.errors += 1;
                                actor.metrics.last_error_at = Some(std::time::SystemTime::now());
                            }
                            result
                        }),
                    );
                    Ok(())
                } else {
                    Err(CoreError::Transport(TransportError::NotConnected))
                }
            }
            _ => Err(CoreError::Transport(TransportError::NotConnected)),
        }
    }
}

impl Handler<IncomingRawMessage> for ConnectionActor {
    type Result = ();

    fn handle(&mut self, _msg: IncomingRawMessage, _ctx: &mut Context<Self>) {
        self.metrics.messages_received += 1;
        self.metrics.last_message_at = Some(std::time::SystemTime::now());
        // Forward to appropriate handler (e.g., CommandActor or EventActor)
    }
}

impl Handler<LifecycleMessage> for ConnectionActor {
    type Result = ();

    fn handle(&mut self, msg: LifecycleMessage, ctx: &mut Context<Self>) {
        match msg {
            LifecycleMessage::Initialize => {
                self.state = ConnectionState::Idle;
            }
            LifecycleMessage::Start => {
                if matches!(self.state, ConnectionState::Idle) {
                    self.connect(ctx);
                }
            }
            LifecycleMessage::Stop => {
                self.state = ConnectionState::Disconnecting;
                if let Some(tx) = &self.tx {
                    let _ = tx.try_send(WsMessage::Close(None));
                }
                ctx.stop();
            }
            LifecycleMessage::Restart => {
                self.state = ConnectionState::Idle;
                self.tx = None;
                self.connect(ctx);
            }
        }
    }
}

impl ActorMetrics for ConnectionActor {
    fn get_metrics(&self) -> super::ActorMetricsData {
        super::ActorMetricsData {
            message_count: self.metrics.messages_sent + self.metrics.messages_received,
            error_count: self.metrics.errors,
            last_activity: self.metrics.last_message_at
                .or(self.metrics.last_error_at)
                .unwrap_or_else(|| std::time::SystemTime::now()),
        }
    }
} 