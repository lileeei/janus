use std::collections::HashMap;
use actix::{Actor, Context, Handler, Supervised, SystemService};
use log::{error, info, warn};

use crate::error::CoreError;
use super::{
    ActorConfig, ActorError, ActorMetrics, ActorState,
    messages::{LifecycleMessage, SupervisionMessage},
};

#[derive(Debug)]
pub enum SupervisorState {
    Starting,
    Running,
    Stopping,
    Failed(CoreError),
}

impl ActorState for SupervisorState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "Starting",
            Self::Running => "Running",
            Self::Stopping => "Stopping",
            Self::Failed(_) => "Failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    pub max_restarts: u32,
    pub restart_window: std::time::Duration,
}

impl ActorConfig for SupervisorConfig {
    fn validate(&self) -> Result<(), ActorError> {
        if self.max_restarts == 0 {
            return Err(ActorError::InitializationError(
                "max_restarts must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

pub struct SupervisorActor {
    config: SupervisorConfig,
    state: SupervisorState,
    children: HashMap<String, ChildActorInfo>,
    metrics: SupervisorMetrics,
}

#[derive(Debug)]
struct ChildActorInfo {
    actor_type: &'static str,
    restarts: Vec<std::time::SystemTime>,
    last_error: Option<CoreError>,
}

#[derive(Debug, Default)]
struct SupervisorMetrics {
    total_restarts: u64,
    total_failures: u64,
    last_restart: Option<std::time::SystemTime>,
}

impl Actor for SupervisorActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("SupervisorActor started");
        self.state = SupervisorState::Running;
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("SupervisorActor stopped");
    }
}

impl Supervised for SupervisorActor {}
impl SystemService for SupervisorActor {}

impl SupervisorActor {
    pub fn new(config: SupervisorConfig) -> Result<Self, ActorError> {
        config.validate()?;
        Ok(Self {
            config,
            state: SupervisorState::Starting,
            children: HashMap::new(),
            metrics: SupervisorMetrics::default(),
        })
    }

    fn handle_child_failure(&mut self, actor_type: &'static str, id: String, error: CoreError) {
        let now = std::time::SystemTime::now();
        let child = self.children.entry(id.clone()).or_insert_with(|| ChildActorInfo {
            actor_type,
            restarts: Vec::new(),
            last_error: None,
        });

        child.last_error = Some(error.clone());
        child.restarts.push(now);

        // Clean up old restart records outside the window
        let window_start = now - self.config.restart_window;
        child.restarts.retain(|&time| time >= window_start);

        if child.restarts.len() as u32 > self.config.max_restarts {
            error!(
                "Actor {}/{} exceeded maximum restarts ({}) within window",
                actor_type, id, self.config.max_restarts
            );
            // Could escalate to system supervisor or implement custom recovery
        } else {
            warn!(
                "Actor {}/{} failed, attempting restart. Error: {}",
                actor_type, id, error
            );
            // Implement restart logic here
            self.metrics.total_restarts += 1;
            self.metrics.last_restart = Some(now);
        }
    }
}

impl Handler<SupervisionMessage> for SupervisorActor {
    type Result = ();

    fn handle(&mut self, msg: SupervisionMessage, _ctx: &mut Context<Self>) {
        match msg {
            SupervisionMessage::RegisterChild { actor_type, id } => {
                info!("Registering child actor: {}/{}", actor_type, id);
                self.children.insert(
                    id,
                    ChildActorInfo {
                        actor_type,
                        restarts: Vec::new(),
                        last_error: None,
                    },
                );
            }
            SupervisionMessage::ChildFailed {
                actor_type,
                id,
                error,
            } => {
                self.metrics.total_failures += 1;
                self.handle_child_failure(actor_type, id, error);
            }
            SupervisionMessage::ChildStopped { actor_type, id } => {
                info!("Child actor stopped: {}/{}", actor_type, id);
                self.children.remove(&id);
            }
        }
    }
}

impl Handler<LifecycleMessage> for SupervisorActor {
    type Result = ();

    fn handle(&mut self, msg: LifecycleMessage, ctx: &mut Context<Self>) {
        match msg {
            LifecycleMessage::Initialize => {
                info!("Initializing supervisor");
                self.state = SupervisorState::Starting;
            }
            LifecycleMessage::Start => {
                info!("Starting supervisor");
                self.state = SupervisorState::Running;
            }
            LifecycleMessage::Stop => {
                info!("Stopping supervisor");
                self.state = SupervisorState::Stopping;
                // Implement graceful shutdown of children
                ctx.stop();
            }
            LifecycleMessage::Restart => {
                warn!("Restarting supervisor");
                self.state = SupervisorState::Starting;
                // Implement restart logic
            }
        }
    }
}

impl ActorMetrics for SupervisorActor {
    fn get_metrics(&self) -> super::ActorMetricsData {
        super::ActorMetricsData {
            message_count: self.metrics.total_restarts + self.metrics.total_failures,
            error_count: self.metrics.total_failures,
            last_activity: self.metrics.last_restart.unwrap_or_else(|| std::time::SystemTime::now()),
        }
    }
} 