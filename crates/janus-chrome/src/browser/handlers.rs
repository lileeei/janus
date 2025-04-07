use actix::{Handler, ResponseFuture};
use log::debug;

use crate::actor::ChromeBrowserActor;
use crate::error::ChromeError;
use crate::protocol::browser;
use crate::browser::messages::*;

impl Handler<Close> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<(), ChromeError>>;

    fn handle(&mut self, _msg: Close, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.close command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::Close;
            let result = command.send(cmd.into()).await??;
            Ok(())
        })
    }
}

impl Handler<GetVersion> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<browser::Version, ChromeError>>;

    fn handle(&mut self, _msg: GetVersion, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.getVersion command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::GetVersion;
            let result = command.send(cmd.into()).await??;
            serde_json::from_value(result).map_err(|e| {
                ChromeError::ProtocolError(format!("Failed to parse version response: {}", e))
            })
        })
    }
}

impl Handler<GetWindowBounds> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<browser::Bounds, ChromeError>>;

    fn handle(&mut self, msg: GetWindowBounds, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.getWindowBounds command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::GetWindowBounds {
                window_id: msg.window_id,
            };
            let result = command.send(cmd.into()).await??;
            let response: browser::GetWindowBoundsResponse = serde_json::from_value(result)
                .map_err(|e| ChromeError::ProtocolError(format!("Failed to parse bounds response: {}", e)))?;
            Ok(response.bounds)
        })
    }
}

impl Handler<GetWindowForTarget> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<(i32, browser::Bounds), ChromeError>>;

    fn handle(&mut self, msg: GetWindowForTarget, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.getWindowForTarget command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::GetWindowForTarget {
                target_id: msg.target_id,
            };
            let result = command.send(cmd.into()).await??;
            let response: browser::GetWindowForTargetResponse = serde_json::from_value(result)
                .map_err(|e| ChromeError::ProtocolError(format!("Failed to parse window target response: {}", e)))?;
            Ok((response.window_id, response.bounds))
        })
    }
}

impl Handler<SetWindowBounds> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<(), ChromeError>>;

    fn handle(&mut self, msg: SetWindowBounds, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.setWindowBounds command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::SetWindowBounds {
                window_id: msg.window_id,
                bounds: msg.bounds,
            };
            let result = command.send(cmd.into()).await??;
            Ok(())
        })
    }
}

impl Handler<GetHistogram> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<browser::Histogram, ChromeError>>;

    fn handle(&mut self, msg: GetHistogram, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.getHistogram command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::GetHistogram {
                name: msg.name,
                delta: msg.delta,
            };
            let result = command.send(cmd.into()).await??;
            let response: browser::GetHistogramResponse = serde_json::from_value(result)
                .map_err(|e| ChromeError::ProtocolError(format!("Failed to parse histogram response: {}", e)))?;
            Ok(response.histogram)
        })
    }
}

impl Handler<GetHistograms> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<Vec<browser::Histogram>, ChromeError>>;

    fn handle(&mut self, msg: GetHistograms, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.getHistograms command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::GetHistograms {
                query: msg.query,
                delta: msg.delta,
            };
            let result = command.send(cmd.into()).await??;
            let response: browser::GetHistogramsResponse = serde_json::from_value(result)
                .map_err(|e| ChromeError::ProtocolError(format!("Failed to parse histograms response: {}", e)))?;
            Ok(response.histograms)
        })
    }
}

impl Handler<SetPermission> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<(), ChromeError>>;

    fn handle(&mut self, msg: SetPermission, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.setPermission command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::SetPermission {
                permission: msg.permission,
                setting: msg.setting,
                origin: msg.origin,
                browser_context_id: msg.browser_context_id,
            };
            let result = command.send(cmd.into()).await??;
            Ok(())
        })
    }
}

impl Handler<GrantPermissions> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<(), ChromeError>>;

    fn handle(&mut self, msg: GrantPermissions, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.grantPermissions command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::GrantPermissions {
                permissions: msg.permissions,
                origin: msg.origin,
                browser_context_id: msg.browser_context_id,
            };
            let result = command.send(cmd.into()).await??;
            Ok(())
        })
    }
}

impl Handler<ResetPermissions> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<(), ChromeError>>;

    fn handle(&mut self, msg: ResetPermissions, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.resetPermissions command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::ResetPermissions {
                browser_context_id: msg.browser_context_id,
            };
            let result = command.send(cmd.into()).await??;
            Ok(())
        })
    }
}

impl Handler<ExecuteBrowserCommand> for ChromeBrowserActor {
    type Result = ResponseFuture<Result<(), ChromeError>>;

    fn handle(&mut self, msg: ExecuteBrowserCommand, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Handling Browser.executeBrowserCommand command");
        let command = self.command.clone();
        
        Box::pin(async move {
            let cmd = browser::ExecuteBrowserCommand {
                command_id: msg.command_id,
            };
            let result = command.send(cmd.into()).await??;
            Ok(())
        })
    }
} 