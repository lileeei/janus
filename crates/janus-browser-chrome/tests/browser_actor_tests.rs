// crates/janus-browser-chrome/tests/browser_actor_tests.rs
#[cfg(test)]
mod tests {
    use actix::prelude::*;
    use futures_channel::oneshot;
    use janus_browser_chrome::actors::{ChromeBrowserActor, ResetPermissions, ShutdownBrowser};
    use janus_browser_chrome::protocol::{ResetPermissionsParams}; // Assuming this is used by actors
    use janus_protocol_handler::{SendCommand, ProtocolEvent};
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // --- MockCommandActor Definition ---
    #[derive(Debug, Clone)]
    struct LastCommand {
        method: String,
        params: serde_json::Value,
    }

    #[derive(Default, Debug, Clone)]
    struct MockCommandActorState {
        last_command: Option<LastCommand>,
    }

    struct MockCommandActor {
        state: Arc<Mutex<MockCommandActorState>>,
    }

    impl MockCommandActor {
        fn new(state: Arc<Mutex<MockCommandActorState>>) -> Self {
            Self { state }
        }
    }

    impl Actor for MockCommandActor {
        type Context = Context<Self>;
    }

    // Message to get the last command from MockCommandActor
    #[derive(Message)]
    #[rtype(result = "Option<LastCommand>")]
    struct GetLastCommand;

    impl Handler<GetLastCommand> for MockCommandActor {
        type Result = Option<LastCommand>;

        fn handle(&mut self, _msg: GetLastCommand, _ctx: &mut Context<Self>) -> Self::Result {
            self.state.lock().unwrap().last_command.clone()
        }
    }

    impl Handler<SendCommand> for MockCommandActor {
        type Result = Result<Result<serde_json::Value, janus_core::error::InternalError>, janus_core::error::InternalError>; // Match CommandActor's SendCommand Result

        fn handle(&mut self, msg: SendCommand, _ctx: &mut Context<Self>) -> Self::Result {
            println!("MockCommandActor received command: {} with params: {}", msg.method, msg.params.to_string());
            let mut state = self.state.lock().unwrap();
            state.last_command = Some(LastCommand {
                method: msg.method.clone(),
                params: msg.params.clone(),
            });

            // Send a generic success response back
            if let Err(e) = msg.result_tx.send(Ok(json!(null))) {
                 eprintln!("MockCommandActor: Failed to send mock result: {:?}", e);
            }
            Ok(Ok(json!(null))) // This outer Ok is for the actor framework, inner Ok for the SendCommand success
        }
    }

    // Helper to create a dummy EventActor recipient
    fn create_dummy_event_actor_recipient() -> Recipient<ProtocolEvent> {
        // Create a simple actor that does nothing with ProtocolEvent
        struct DummyEventActor;
        impl Actor for DummyEventActor { type Context = Context<Self>; }
        impl Handler<ProtocolEvent> for DummyEventActor {
            type Result = ();
            fn handle(&mut self, _msg: ProtocolEvent, _ctx: &mut Context<Self>) -> Self::Result { () }
        }
        DummyEventActor.start().recipient()
    }


    // --- Tests ---
    #[actix_rt::test]
    async fn test_reset_permissions_sends_correct_cdp_command() {
        let sys = System::new();
        let state = Arc::new(Mutex::new(MockCommandActorState::default()));
        let mock_cmd_actor_addr = MockCommandActor::new(state.clone()).start();
        let dummy_event_recipient = create_dummy_event_actor_recipient();

        let browser_actor_addr = ChromeBrowserActor::new(
            mock_cmd_actor_addr.clone().recipient(), // Pass Recipient<SendCommand>
            dummy_event_recipient,
        )
        .start();
        
        // Allow actors to start and initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test case 1: With browser_context_id
        let context_id = "test_context_123".to_string();
        let res = browser_actor_addr
            .send(ResetPermissions {
                browser_context_id: Some(context_id.clone()),
            })
            .await;

        assert!(res.is_ok(), "Sending ResetPermissions message failed");
        assert!(res.unwrap().is_ok(), "ResetPermissions handler returned an error");
        
        tokio::time::sleep(Duration::from_millis(50)).await; // Ensure command is processed

        let last_cmd = mock_cmd_actor_addr.send(GetLastCommand).await.unwrap();
        assert!(last_cmd.is_some(), "No command was captured by MockCommandActor");

        let captured_cmd = last_cmd.unwrap();
        assert_eq!(captured_cmd.method, "Browser.resetPermissions");
        assert_eq!(
            captured_cmd.params,
            json!({ "browserContextId": context_id })
        );

        // Test case 2: Without browser_context_id (None)
        let res_none = browser_actor_addr
            .send(ResetPermissions {
                browser_context_id: None,
            })
            .await;

        assert!(res_none.is_ok(), "Sending ResetPermissions (None) message failed");
        assert!(res_none.unwrap().is_ok(), "ResetPermissions (None) handler returned an error");

        tokio::time::sleep(Duration::from_millis(50)).await; // Ensure command is processed

        let last_cmd_none = mock_cmd_actor_addr.send(GetLastCommand).await.unwrap();
        assert!(last_cmd_none.is_some(), "No command was captured for ResetPermissions (None)");
        
        let captured_cmd_none = last_cmd_none.unwrap();
        assert_eq!(captured_cmd_none.method, "Browser.resetPermissions");
        // When browser_context_id is None, it should be omitted due to skip_serializing_if = "Option::is_none"
        // or be present as json!(null) if not skipped. The ResetPermissionsParams struct uses skip_serializing_if.
        // So it should be an empty object, or an object where the key is not present.
        // serde_json::to_value(ResetPermissionsParams { browser_context_id: None }) would produce {}
        assert_eq!(captured_cmd_none.params, json!({}));


        System::current().stop(); // Stop the system for cleanup
    }

    #[actix_rt::test]
    async fn test_shutdown_browser_sends_browser_close_cdp_command() {
        let sys = System::new();
        let state = Arc::new(Mutex::new(MockCommandActorState::default()));
        let mock_cmd_actor_addr = MockCommandActor::new(state.clone()).start();
        let dummy_event_recipient = create_dummy_event_actor_recipient();

        let browser_actor_addr = ChromeBrowserActor::new(
            mock_cmd_actor_addr.clone().recipient(),
            dummy_event_recipient,
        )
        .start();

        // Allow actors to start and initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Action: Send ShutdownBrowser message
        // The handler for ShutdownBrowser is a ResponseFuture<()>, not Result<(), _>
        // so we don't check the inner result like for ResetPermissions
        let res = browser_actor_addr.send(ShutdownBrowser).await;
        assert!(res.is_ok(), "Sending ShutdownBrowser message failed");
        
        tokio::time::sleep(Duration::from_millis(50)).await; // Ensure command is processed by mock

        let last_cmd = mock_cmd_actor_addr.send(GetLastCommand).await.unwrap();
        assert!(last_cmd.is_some(), "No command was captured by MockCommandActor for ShutdownBrowser");

        let captured_cmd = last_cmd.unwrap();
        assert_eq!(captured_cmd.method, "Browser.close");
        assert_eq!(captured_cmd.params, json!(null), "Params for Browser.close should be null or empty object");
        // The actual implementation sends `serde_json::Value::Null` which becomes `json!(null)`.

        System::current().stop(); // Stop the system for cleanup
    }
}
