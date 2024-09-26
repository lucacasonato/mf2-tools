use lsp_server::Connection;
use lsp_types::notification::DidChangeTextDocument;
use lsp_types::notification::DidCloseTextDocument;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::notification::PublishDiagnostics;
use lsp_types::request::CodeActionRequest;
use lsp_types::request::HoverRequest;
use lsp_types::request::Rename as RenameRequest;

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
                    let message = format!("Error handling request {}: {:?}", <$request_typ as lsp_types::request::Request>::METHOD, err);
                    const REQUEST_FAILED_ERROR_CODE: i32 = -32803;
                    lsp_server::Response::new_err(request.id, REQUEST_FAILED_ERROR_CODE, message)
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

language_server! {
  notifications: {
    on_open_text_document: DidOpenTextDocument,
    on_change_text_document: DidChangeTextDocument,
    on_close_text_document: DidCloseTextDocument,
  },
  requests: {
    hover: HoverRequest,
    code_action: CodeActionRequest,
    rename: RenameRequest,
  }
}

language_client! {
  notifications: {
    publish_diagnostics: PublishDiagnostics,
  }
}
