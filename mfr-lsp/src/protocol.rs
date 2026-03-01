use std::ops::ControlFlow;

use lsp_server::Connection;
use lsp_server::ErrorCode;
use lsp_server::Message;
use lsp_server::Response;
use lsp_types::notification::DidChangeTextDocument;
use lsp_types::notification::DidCloseTextDocument;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::notification::Initialized;
use lsp_types::notification::Notification;
use lsp_types::notification::PublishDiagnostics;
use lsp_types::request::Initialize;
use lsp_types::request::Request;
use yoke::Yokeable;

pub struct LanguageClient<'a> {
  connection: &'a Connection,
}

impl<'a> LanguageClient<'a> {
  pub fn new(connection: &'a Connection) -> Self {
    Self { connection }
  }

  pub fn publish_diagnostics(
    &self,
    params: <PublishDiagnostics as lsp_types::notification::Notification>::Params,
  ) {
    if let Err(err) =
      self
        .connection
        .sender
        .send(lsp_server::Message::Notification(
          lsp_server::Notification {
            method: PublishDiagnostics::METHOD.to_string(),
            params: serde_json::to_value(params).unwrap(),
          },
        ))
    {
      eprintln!("Error sending diagnostics notification: {:?}", err);
    }
  }
}

pub trait LanguageServer {
  fn initialize(
    &mut self,
    params: <Initialize as lsp_types::request::Request>::Params,
  ) -> Result<<Initialize as lsp_types::request::Request>::Result, anyhow::Error>;

  fn initialized(
    &mut self,
    params: <Initialized as lsp_types::notification::Notification>::Params,
  );

  fn on_open_text_document(
    &mut self,
    params: <DidOpenTextDocument as lsp_types::notification::Notification>::Params,
  );

  fn on_change_text_document(
    &mut self,
    params: <DidChangeTextDocument as lsp_types::notification::Notification>::Params,
  );

  fn on_close_text_document(
    &mut self,
    params: <DidCloseTextDocument as lsp_types::notification::Notification>::Params,
  );

  fn handle_notification(&mut self, notification: lsp_server::Notification) {
    match notification.method.as_str() {
      Initialized::METHOD => {
        if let Ok(params) = serde_json::from_value(notification.params) {
          self.initialized(params);
        }
      }
      DidOpenTextDocument::METHOD => {
        if let Ok(params) = serde_json::from_value(notification.params) {
          self.on_open_text_document(params);
        }
      }
      DidChangeTextDocument::METHOD => {
        if let Ok(params) = serde_json::from_value(notification.params) {
          self.on_change_text_document(params);
        }
      }
      DidCloseTextDocument::METHOD => {
        if let Ok(params) = serde_json::from_value(notification.params) {
          self.on_close_text_document(params);
        }
      }
      _ => {}
    }
  }

  fn handle_request(
    &mut self,
    request: lsp_server::Request,
  ) -> lsp_server::Response {
    match request.method.as_str() {
      Initialize::METHOD => match serde_json::from_value(request.params) {
        Ok(params) => match self.initialize(params) {
          Ok(result) => lsp_server::Response::new_ok(
            request.id,
            serde_json::to_value(result).unwrap(),
          ),
          Err(err) => {
            lsp_server::Response::new_err(request.id, -32803, err.to_string())
          }
        },
        Err(err) => lsp_server::Response::new_err(
          request.id,
          -32602,
          format!("Error deserializing initialize params: {:?}", err),
        ),
      },
      _ => lsp_server::Response::new_err(
        request.id,
        -32601,
        format!("Unrecognized request: {}", request.method),
      ),
    }
  }
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
      Message::Request(req) if req.method == Initialize::METHOD => {
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
        self.connection.sender.send(resp.into())?;
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
      Message::Notification(n) if n.method == Initialized::METHOD => {
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
      Message::Notification(notification) => {
        self.server.handle_notification(notification)
      }
      Message::Request(req) if req.method == "shutdown" => {
        let resp = Response::new_ok(req.id.clone(), ());
        self.connection.sender.send(resp.into())?;
        self.state = LanguageServerState::ShuttingDown;
      }
      Message::Request(req) => {
        let resp = self.server.handle_request(req);
        self.connection.sender.send(resp.into())?;
      }
      Message::Response(_) => {
        return Err(anyhow::anyhow!(
          "unexpected response from client while initialized"
        ));
      }
    }
    Ok(ControlFlow::Continue(()))
  }

  fn handle_message_shutting_down(
    &mut self,
    message: Message,
  ) -> Result<ControlFlow<()>, anyhow::Error> {
    match message {
      Message::Notification(n) if n.method == "exit" => {
        Ok(ControlFlow::Break(()))
      }
      Message::Request(req) => {
        let resp = Response::new_err(
          req.id,
          ErrorCode::InvalidRequest as i32,
          "server is shutting down".to_string(),
        );
        self.connection.sender.send(resp.into())?;
        Ok(ControlFlow::Continue(()))
      }
      msg => Err(anyhow::anyhow!(
        "unexpected message while shutting down: {msg:?}"
      )),
    }
  }
}
