mod documents;
mod protocol;

use lsp_server::Connection;
use lsp_server::Message;
use lsp_server::Response;
use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidCloseTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::Hover;
use lsp_types::HoverParams;
use lsp_types::InitializeParams;
use lsp_types::PublishDiagnosticsParams;
use lsp_types::ServerCapabilities;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use protocol::LanguageClient;
use protocol::LanguageServer;

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

  let mut server = Server {
    client: LanguageClient::new(connection),
    documents: Documents::default(),
  };

  loop {
    match server.client.raw_connection().receiver.recv()? {
      Message::Request(req) => {
        if server
          .client
          .raw_connection()
          .handle_shutdown(&req)
          .unwrap_or(true)
        {
          break;
        }

        let response = server.handle_request(&req.method, req.params);
        let response = match response {
          Ok(response) => lsp_server::Response::new_ok(req.id, response),
          Err(err) => Response {
            id: req.id,
            result: None,
            error: Some(err),
          },
        };
        server
          .client
          .raw_connection()
          .sender
          .send(Message::Response(response))?;
      }

      Message::Response(resp) => {
        if let Some(cb) = server.client.handle_response(resp) {
          cb(&mut server);
        }
      }

      Message::Notification(notification) => {
        server.handle_notification(&notification.method, notification.params);
      }
    }
  }

  eprintln!("Shutting down.");
  Ok(())
}

pub struct Server {
  client: LanguageClient<Server>,
  documents: Documents,
}

impl LanguageServer for Server {
  fn on_open_text_document(&mut self, params: DidOpenTextDocumentParams) {
    let document = self.documents.on_change(
      params.text_document.uri.clone(),
      params.text_document.version,
      params.text_document.text,
    );
    let params = PublishDiagnosticsParams {
      uri: params.text_document.uri.clone(),
      version: Some(document.version),
      diagnostics: document.diagnostics(),
    };
    self.client.publish_diagnostics(params);
  }

  fn on_change_text_document(&mut self, params: DidChangeTextDocumentParams) {
    let text = params.content_changes.into_iter().next().unwrap().text;
    let document = self.documents.on_change(
      params.text_document.uri.clone(),
      params.text_document.version,
      text,
    );
    let params = PublishDiagnosticsParams {
      uri: params.text_document.uri.clone(),
      version: Some(document.version),
      diagnostics: document.diagnostics(),
    };
    self.client.publish_diagnostics(params);
  }

  fn on_close_text_document(&mut self, params: DidCloseTextDocumentParams) {
    self.documents.on_close(params.text_document.uri.clone());
  }

  fn hover(&mut self, _params: HoverParams) -> Option<Hover> {
    None
  }
}
