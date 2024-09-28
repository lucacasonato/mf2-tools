use std::ops::ControlFlow;

use lsp_server::Connection;
use lsp_server::ErrorCode;
use lsp_server::Message;
use lsp_server::Response;
use lsp_types::notification::DidChangeTextDocument;
use lsp_types::notification::DidCloseTextDocument;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::notification::Initialized;
use lsp_types::notification::PublishDiagnostics;
use lsp_types::request::CodeActionRequest;
use lsp_types::request::Completion as CompletionRequest;
use lsp_types::request::GotoDeclaration;
use lsp_types::request::GotoDefinition;
use lsp_types::request::HoverRequest;
use lsp_types::request::Initialize;
use lsp_types::request::PrepareRenameRequest;
use lsp_types::request::Rename as RenameRequest;
use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::request::SemanticTokensRangeRequest;
use yoke::Yokeable;

pub struct LanguageClient<'a> {
  connection: &'a Connection,
}

impl<'a> LanguageClient<'a> {
  pub fn new(connection: &'a Connection) -> Self {
    Self { connection }
  }
}

macro_rules! language_client {
  (
    notifications: {
      $($name:ident: $typ:path),* $(,)?
    }
  ) => {
    impl LanguageClient<'_> {
    $(
      pub fn $name(
        &self,
        params: <$typ as lsp_types::notification::Notification>::Params,
      ) {
        if let Err(err) =
          self
            .connection
            .sender
            .send(lsp_server::Message::Notification(
              lsp_server::Notification {
                method: <$typ as lsp_types::notification::Notification>::METHOD
                  .to_string(),
                params: serde_json::to_value(params).unwrap(),
              },
            ))
        {
          eprintln!(
            "Error sending {} notification: {:?}",
            <$typ as lsp_types::notification::Notification>::METHOD,
            err
          );
        }
      }
    )*
    }
  };
}

macro_rules! language_server {
  (
    notifications: {
      $($notification_name:ident: $notification_typ:path),* $(,)?
    } $(,)?
    requests: {
      $($request_name:ident: $request_typ:path),* $(,)?
    }
  ) => {
    pub trait LanguageServer {
      fn handle_notification(&mut self, notification: lsp_server::Notification) {
        match notification.method.as_str() {
        $(
          <$notification_typ as lsp_types::notification::Notification>::METHOD => {
            match serde_json::from_value(notification.params) {
              Ok(params) => {
                self.$notification_name(params);
              }
              Err(err) => {
                eprintln!("Error deserializing params in {}: {:?}", <$notification_typ as lsp_types::notification::Notification>::METHOD, err);
              }
            }
          }
        )*
          _ => {
            eprintln!("Unrecognized notification: {}", notification.method);
          }
        }
      }

      fn handle_request(&mut self, request: lsp_server::Request) -> lsp_server::Response {
        match request.method.as_str() {
        $(
          <$request_typ as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value(request.params) {
              Ok(params) => {
                let result = self.$request_name(params);
                match result {
                  Ok(result) => match serde_json::to_value(result) {
                    Ok(result) => lsp_server::Response::new_ok(request.id, result),
                    Err(err) => {
                      let message = format!("Failed to serialize response to {}: {:?}", <$request_typ as lsp_types::request::Request>::METHOD, err);
                      const INTERNAL_ERROR_CODE: i32 = -32603;
                      lsp_server::Response::new_err(request.id, INTERNAL_ERROR_CODE, message)
                    }
                  },
                  Err(err) => {
                    const REQUEST_FAILED_ERROR_CODE: i32 = -32803;
                    lsp_server::Response::new_err(request.id, REQUEST_FAILED_ERROR_CODE, err.to_string())
                  }
                }
              }
              Err(err) => {
                let message = format!("Error deserializing params in {}: {:?}", <$request_typ as lsp_types::request::Request>::METHOD, err);
                const INVALID_PARAMS_ERROR_CODE: i32 = -32602;
                lsp_server::Response::new_err(request.id, INVALID_PARAMS_ERROR_CODE, message)
              }
            }
          }
        )*
          _ => {
            let message = format!("Unrecognized request: {}", request.method);
            const METHOD_NOT_FOUND_ERROR_CODE: i32 = -32601;
            lsp_server::Response::new_err(request.id, METHOD_NOT_FOUND_ERROR_CODE, message)
          }
        }
      }

    $(
      fn $notification_name(
        &mut self,
        params: <$notification_typ as lsp_types::notification::Notification>::Params,
      );
    )*

    $(
      fn $request_name(
        &mut self,
        params: <$request_typ as lsp_types::request::Request>::Params,
      ) -> Result<<$request_typ as lsp_types::request::Request>::Result, anyhow::Error>;
    )*
    }
  };
}

#[derive(Debug, Clone, Copy)]
enum LanguageServerState {
  Uninitialized,
  Initializing,
  Initialized,
  ShuttingDown,
}

#[derive(Yokeable)]
pub struct ConnectionManager<'a, LS: LanguageServer + 'a> {
  connection: &'a Connection,
  state: LanguageServerState,
  server: LS,
}

impl<'a, LS: LanguageServer> ConnectionManager<'a, LS> {
  pub fn new(connection: &'a Connection, server: LS) -> Self {
    Self {
      connection,
      state: LanguageServerState::Uninitialized,
      server,
    }
  }

  pub fn handle_message(
    &mut self,
    message: lsp_server::Message,
  ) -> Result<ControlFlow<(), ()>, anyhow::Error> {
    match self.state {
      LanguageServerState::Uninitialized => {
        self.handle_message_uninitialized(message)
      }
      LanguageServerState::Initializing => {
        self.handle_message_initializing(message)
      }
      LanguageServerState::Initialized => {
        self.handle_message_initialized(message)
      }
      LanguageServerState::ShuttingDown => {
        self.handle_message_shutting_down(message)
      }
    }
  }

  fn handle_message_uninitialized(
    &mut self,
    message: Message,
  ) -> Result<ControlFlow<()>, anyhow::Error> {
    match message {
      Message::Request(req) if req.method == "initialize" => {
        let resp = self.server.handle_request(req);
        self.connection.sender.send(resp.into())?;
        self.state = LanguageServerState::Initializing;
      }
      Message::Request(req) => {
        let resp = Response::new_err(
          req.id.clone(),
          ErrorCode::ServerNotInitialized as i32,
          format!("expected initialize request, got {req:?}"),
        );
        self.connection.sender.send(resp.into()).unwrap();
      }
      Message::Notification(n) if n.method == "exit" => {}
      msg => {
        return Err(anyhow::anyhow!(
          "expected initialize request, got {msg:?}"
        ));
      }
    }
    Ok(ControlFlow::Continue(()))
  }

  fn handle_message_initializing(
    &mut self,
    message: Message,
  ) -> Result<ControlFlow<()>, anyhow::Error> {
    match message {
      Message::Notification(n) if n.method == "initialized" => {
        self.server.handle_notification(n);
        self.state = LanguageServerState::Initialized;
      }
      msg => {
        return Err(anyhow::anyhow!(
          "expected initialized notification, got {msg:?}"
        ));
      }
    }
    Ok(ControlFlow::Continue(()))
  }

  fn handle_message_initialized(
    &mut self,
    message: Message,
  ) -> Result<ControlFlow<()>, anyhow::Error> {
    match message {
      lsp_server::Message::Notification(notification) => {
        self.server.handle_notification(notification)
      }
      lsp_server::Message::Request(req) if req.method == "shutdown" => {
        let resp = Response::new_ok(req.id.clone(), ());
        self.connection.sender.send(resp.into())?;
        self.state = LanguageServerState::ShuttingDown;
      }
      lsp_server::Message::Request(req) => {
        let resp = self.server.handle_request(req);
        self.connection.sender.send(resp.into())?;
      }
      lsp_server::Message::Response(_) => todo!(),
    }
    Ok(ControlFlow::Continue(()))
  }

  fn handle_message_shutting_down(
    &self,
    message: Message,
  ) -> Result<ControlFlow<()>, anyhow::Error> {
    match message {
      Message::Notification(n) if n.method == "exit" => {
        Ok(ControlFlow::Break(()))
      }
      msg => Err(anyhow::anyhow!("expected exit notification, got {msg:?}")),
    }
  }
}

language_server! {
  notifications: {
    initialized: Initialized,
    on_open_text_document: DidOpenTextDocument,
    on_change_text_document: DidChangeTextDocument,
    on_close_text_document: DidCloseTextDocument,
  },
  requests: {
    initialize: Initialize,
    hover: HoverRequest,
    go_to_declaration: GotoDeclaration,
    go_to_definition: GotoDefinition,
    code_action: CodeActionRequest,
    rename: RenameRequest,
    prepare_rename: PrepareRenameRequest,
    completion: CompletionRequest,
    semantic_tokens_full: SemanticTokensFullRequest,
    semantic_tokens_range: SemanticTokensRangeRequest,
  }
}

language_client! {
  notifications: {
    publish_diagnostics: PublishDiagnostics,
  }
}
