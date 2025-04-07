use std::collections::HashMap;
use std::time::{Duration, Instant};
use actix::{Actor, Addr, Context, Handler, Supervised};
use log::{error, info, warn};
use serde_json::Value;
use tokio::sync::oneshot;

use crate::error::{CoreError, ProtocolError};
use super::{
    ActorConfig, ActorError, ActorMetrics, ActorState,
    messages::{ExecuteCommand, IncomingRawMessage, LifecycleMessage, SendRawMessage, SupervisionMessage},
    supervisor::SupervisorActor,
    connection::ConnectionActor,
};

#[derive(Debug)]
pub enum CommandState {
    Ready,
    Busy,
    Failed(ProtocolError),
}

impl ActorState for CommandState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Busy => "Busy",
            Self::Failed(_) => "Failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandConfig {
    pub default_timeout: Duration,
    pub max_pending_commands: usize,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            max_pending_commands: 1000,
        }
    }
}

impl ActorConfig for CommandConfig {
    fn validate(&self) -> Result<(), ActorError> {
        if self.default_timeout.is_zero() {
            return Err(ActorError::InitializationError(
                "Default timeout cannot be zero".to_string(),
            ));
        }
        if self.max_pending_commands == 0 {
            return Err(ActorError::InitializationError(
                "Max pending commands must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

struct PendingCommand {
    method: String,
    started_at: Instant,
    timeout: Duration,
    response_tx: oneshot::Sender<Result<Value, ProtocolError>>,
}

pub struct CommandActor {
    config: CommandConfig,
    state: CommandState,
    supervisor: Addr<SupervisorActor>,
    connection: Addr<ConnectionActor>,
    pending_commands: HashMap<u64, PendingCommand>,
    next_id: u64,
    metrics: CommandMetrics,
}

#[derive(Debug, Default)]
struct CommandMetrics {
    commands_sent: u64,
    commands_completed: u64,
    commands_failed: u64,
    timeouts: u64,
    last_command_at: Option<std::time::SystemTime>,
    last_error_at: Option<std::time::SystemTime>,
}

impl Actor for CommandActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("CommandActor started");
        
        // Register with supervisor
        self.supervisor.do_send(SupervisionMessage::RegisterChild {
            actor_type: "command",
            id: "main".to_string(),
        });

        // Start timeout checker
        self.start_timeout_checker(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("CommandActor stopped");
        // Fail all pending commands
        self.fail_all_pending_commands(ProtocolError::Internal(
            "CommandActor stopped".to_string(),
        ));
    }
}

impl Supervised for CommandActor {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        warn!("CommandActor is being restarted");
        self.fail_all_pending_commands(ProtocolError::Internal(
            "CommandActor restarting".to_string(),
        ));
    }
}

impl CommandActor {
    pub fn new(
        config: CommandConfig,
        supervisor: Addr<SupervisorActor>,
        connection: Addr<ConnectionActor>,
    ) -> Result<Self, ActorError> {
        config.validate()?;
        Ok(Self {
            config,
            state: CommandState::Ready,
            supervisor,
            connection,
            pending_commands: HashMap::new(),
            next_id: 1,
            metrics: CommandMetrics::default(),
        })
    }

    fn start_timeout_checker(&self, ctx: &mut Context<Self>) {
        // Check for timed out commands every second
        ctx.run_interval(Duration::from_secs(1), |actor, _ctx| {
            let now = Instant::now();
            let timed_out: Vec<_> = actor
                .pending_commands
                .iter()
                .filter(|(_, cmd)| now.duration_since(cmd.started_at) > cmd.timeout)
                .map(|(id, _)| *id)
                .collect();

            for id in timed_out {
                if let Some(cmd) = actor.pending_commands.remove(&id) {
                    actor.metrics.timeouts += 1;
                    actor.metrics.commands_failed += 1;
                    actor.metrics.last_error_at = Some(std::time::SystemTime::now());
                    
                    let _ = cmd.response_tx.send(Err(ProtocolError::Timeout));
                    
                    error!("Command {} timed out after {:?}", cmd.method, cmd.timeout);
                }
            }
        });
    }

    fn fail_all_pending_commands(&mut self, error: ProtocolError) {
        for (_, cmd) in self.pending_commands.drain() {
            let _ = cmd.response_tx.send(Err(error.clone()));
        }
        self.metrics.commands_failed += self.pending_commands.len() as u64;
        if !self.pending_commands.is_empty() {
            self.metrics.last_error_at = Some(std::time::SystemTime::now());
        }
    }

    fn handle_command_response(&mut self, id: u64, result: Result<Value, ProtocolError>) {
        if let Some(cmd) = self.pending_commands.remove(&id) {
            self.metrics.commands_completed += 1;
            self.metrics.last_command_at = Some(std::time::SystemTime::now());
            
            if result.is_err() {
                self.metrics.commands_failed += 1;
                self.metrics.last_error_at = Some(std::time::SystemTime::now());
            }
            
            let _ = cmd.response_tx.send(result);
        }
    }
}

impl Handler<ExecuteCommand> for CommandActor {
    type Result = Result<Value, ProtocolError>;

    fn handle(&mut self, msg: ExecuteCommand, ctx: &mut Context<Self>) -> Self::Result {
        if self.pending_commands.len() >= self.config.max_pending_commands {
            return Err(ProtocolError::Internal(
                "Too many pending commands".to_string(),
            ));
        }

        let id = self.next_id;
        self.next_id += 1;

        let (response_tx, response_rx) = oneshot::channel();
        
        let command = PendingCommand {
            method: msg.method.clone(),
            started_at: Instant::now(),
            timeout: msg.timeout.unwrap_or(self.config.default_timeout),
            response_tx,
        };

        // Create protocol message
        let protocol_msg = serde_json::json!({
            "id": id,
            "method": msg.method,
            "params": msg.params.unwrap_or(Value::Null),
        });

        self.pending_commands.insert(id, command);
        self.metrics.commands_sent += 1;
        self.metrics.last_command_at = Some(std::time::SystemTime::now());

        // Send via connection actor
        self.connection.do_send(SendRawMessage {
            payload: protocol_msg.to_string(),
            timeout: Some(self.config.default_timeout),
        });

        // Wait for response
        let timeout = msg.timeout.unwrap_or(self.config.default_timeout);
        let actor_addr = ctx.address();
        
        ctx.spawn(
            async move {
                match tokio::time::timeout(timeout, response_rx).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Err(ProtocolError::Internal(
                        "Response channel closed".to_string(),
                    )),
                    Err(_) => Err(ProtocolError::Timeout),
                }
            }
            .into_actor(self),
        );

        // Return pending future
        Ok(Value::Null) // Actual result will come through the spawned future
    }
}

impl Handler<IncomingRawMessage> for CommandActor {
    type Result = ();

    fn handle(&mut self, msg: IncomingRawMessage, _ctx: &mut Context<Self>) {
        match serde_json::from_str::<Value>(&msg.payload) {
            Ok(value) => {
                if let Some(id) = value.get("id").and_then(Value::as_u64) {
                    let result = if let Some(error) = value.get("error") {
                        Err(ProtocolError::BrowserError {
                            code: error.get("code").and_then(Value::as_i64).unwrap_or(-1),
                            message: error.get("message").and_then(Value::as_str).unwrap_or("Unknown error").to_string(),
                        })
                    } else if let Some(result) = value.get("result") {
                        Ok(result.clone())
                    } else {
                        Err(ProtocolError::ResponseParseError {
                            reason: "Missing result and error fields".to_string(),
                            response: value,
                        })
                    };
                    
                    self.handle_command_response(id, result);
                }
            }
            Err(e) => {
                error!("Failed to parse command response: {}", e);
                self.metrics.commands_failed += 1;
                self.metrics.last_error_at = Some(std::time::SystemTime::now());
            }
        }
    }
}

impl Handler<LifecycleMessage> for CommandActor {
    type Result = ();

    fn handle(&mut self, msg: LifecycleMessage, ctx: &mut Context<Self>) {
        match msg {
            LifecycleMessage::Initialize => {
                self.state = CommandState::Ready;
            }
            LifecycleMessage::Start => {
                // Nothing special needed
            }
            LifecycleMessage::Stop => {
                self.state = CommandState::Failed(ProtocolError::Internal(
                    "Actor stopping".to_string(),
                ));
                self.fail_all_pending_commands(ProtocolError::Internal(
                    "Actor stopping".to_string(),
                ));
                ctx.stop();
            }
            LifecycleMessage::Restart => {
                self.state = CommandState::Ready;
                self.fail_all_pending_commands(ProtocolError::Internal(
                    "Actor restarting".to_string(),
                ));
            }
        }
    }
}

impl ActorMetrics for CommandActor {
    fn get_metrics(&self) -> super::ActorMetricsData {
        super::ActorMetricsData {
            message_count: self.metrics.commands_sent,
            error_count: self.metrics.commands_failed,
            last_activity: self.metrics.last_command_at
                .or(self.metrics.last_error_at)
                .unwrap_or_else(|| std::time::SystemTime::now()),
        }
    }
} 