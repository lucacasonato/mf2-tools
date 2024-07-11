mod documents;

use lsp_server::Connection;
use lsp_server::Message;
use lsp_server::Notification;
use lsp_server::Response;
use lsp_types::notification::DidChangeTextDocument;
use lsp_types::notification::DidCloseTextDocument;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::request::HoverRequest;
use lsp_types::InitializeParams;
use lsp_types::PublishDiagnosticsParams;
use lsp_types::ServerCapabilities;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use lsp_types::Uri;

use crate::documents::Documents;

fn main() -> Result<(), anyhow::Error> {
  let (connection, _threads) = Connection::stdio();

  let capabilities = ServerCapabilities {
    text_document_sync: Some(TextDocumentSyncCapability::Options(
      lsp_types::TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(TextDocumentSyncKind::FULL),
        ..lsp_types::TextDocumentSyncOptions::default()
      },
    )),
    ..ServerCapabilities::default()
  };

  let server_capabilities = serde_json::to_value(capabilities).unwrap();
  let initialization_params = connection.initialize(server_capabilities)?;

  let _initialization_params =
    serde_json::from_value::<InitializeParams>(initialization_params)?;

  eprintln!("Server initialized.");

  let mut server = LanguageServer {
    connection,
    documents: Documents::default(),
  };

  loop {
    match server.connection.receiver.recv()? {
      Message::Request(req) => {
        if server.connection.handle_shutdown(&req).unwrap_or(true) {
          break;
        }

        macro_rules! match_request {
          (
            $($name:ident$( ($params:ident) )? => $body:tt)*
          ) => {
            match req.method.as_str() {
              $(
                <$name as lsp_types::request::Request>::METHOD => {
                  $(
                    let $params = serde_json::from_value::<
                      <$name as lsp_types::request::Request>::Params,
                    >(req.params)?;
                  )?
                  let result: <$name as lsp_types::request::Request>::Result = $body;
                  server.connection.sender.send(Message::Response(Response::new_ok(req.id, result)))?;
                }
              )*
              _ => {
                eprintln!("Unrecognized request: {}", req.method);
              }
            }
          };
        }

        match_request! {
          HoverRequest(params) => {
            eprintln!("Hover request: {:#?}", params);
            None
          }
        }
      }
      Message::Response(_) => todo!(),

      Message::Notification(notification) => {
        macro_rules! match_notification {
          (
            $($name:ident($params:ident) => $body:tt)*
          ) => {
            match notification.method.as_str() {
              $(
                <$name as lsp_types::notification::Notification>::METHOD => {
                  let $params = serde_json::from_value::<
                    <$name as lsp_types::notification::Notification>::Params,
                  >(notification.params)?;
                  $body
                }
              )*
              _ => {
                eprintln!("Unrecognized notification: {}", notification.method);
              }
            }
          };
        }

        match_notification! {
          DidOpenTextDocument(params) => {
            eprintln!("Opened document: {:#?}", params);

            server.on_document_changed(
              params.text_document.text,
              params.text_document.uri,
              params.text_document.version,
            );
          }
          DidChangeTextDocument(params) => {
            eprintln!("Changed document: {:#?}", params);

            let text = params.content_changes.into_iter().next().unwrap().text;
            server.on_document_changed(
              text,
              params.text_document.uri,
              params.text_document.version,
            );
          }
          DidCloseTextDocument(params) => {
            eprintln!("Closed document: {:#?}", params);

            server.on_document_closed(params.text_document.uri);
          }
        }
      }
    }
  }

  eprintln!("Shutting down.");
  Ok(())
}

struct LanguageServer {
  connection: Connection,
  documents: Documents,
}

impl LanguageServer {
  fn send_notification(&self, notification: Notification) {
    if let Err(err) = self
      .connection
      .sender
      .send(Message::Notification(notification))
    {
      eprintln!("Error sending notification: {:#?}", err);
    }
  }

  fn on_document_changed(&mut self, text: String, uri: Uri, version: i32) {
    let document = self.documents.on_change(uri.clone(), version, text);

    let params = PublishDiagnosticsParams {
      uri,
      version: Some(document.version),
      diagnostics: document.diagnostics(),
    };

    self.send_notification(Notification {
      method: "textDocument/publishDiagnostics".to_string(),
      params: serde_json::to_value(params).unwrap(),
    });
  }

  fn on_document_closed(&mut self, uri: Uri) {
    self.documents.on_close(uri);
  }
}
