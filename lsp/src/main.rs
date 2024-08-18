mod protocol;

use lsp_server::Connection;
use lsp_server::Message;
use lsp_types::Diagnostic;
use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidCloseTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::InitializeParams;
use lsp_types::Position;
use lsp_types::PublishDiagnosticsParams;
use lsp_types::Range;
use lsp_types::ServerCapabilities;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use lsp_types::Uri;
use mf2_parser::parse;
use mf2_parser::Location;
use mf2_parser::SourceTextInfo;
use protocol::LanguageServer;

use crate::protocol::LanguageClient;

fn main() -> Result<(), anyhow::Error> {
  eprintln!(
    "Starting server... mflsp {}{}",
    env!("CARGO_PKG_VERSION"),
    if option_env!("MF2LSP_OFFICIAL_BUILD") == Some("true") {
      " (official)"
    } else {
      ""
    }
  );

  let (connection, _threads) = Connection::stdio();

  let capabilities = ServerCapabilities {
    text_document_sync: Some(TextDocumentSyncCapability::Kind(
      TextDocumentSyncKind::FULL,
    )),
    hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
    ..ServerCapabilities::default()
  };

  let server_capabilities = serde_json::to_value(capabilities).unwrap();
  let (initialize_id, initialize_params) = connection.initialize_start()?;

  let initialize_result = serde_json::json!({
    "capabilities": server_capabilities,
    "serverInfo": {
      "name": "mf2lsp",
      "version": env!("CARGO_PKG_VERSION"),
    },
  });
  connection.initialize_finish(initialize_id, initialize_result)?;

  let initialize_params =
    serde_json::from_value::<InitializeParams>(initialize_params)?;

  eprint!("Server initialized.");
  if let Some(client_info) = initialize_params.client_info {
    eprint!(" Connected to: {}", client_info.name);
    if let Some(version) = client_info.version {
      eprint!(" {}", version);
    }
  }
  eprintln!();

  let client = LanguageClient::new(&connection);
  let mut server = Server { client };

  loop {
    match connection.receiver.recv()? {
      Message::Request(request) => {
        if connection.handle_shutdown(&request).unwrap_or(true) {
          break;
        }

        let response = server.handle_request(request);
        connection.sender.send(Message::Response(response))?;
      }
      Message::Response(_) => todo!(),
      Message::Notification(notification) => {
        server.handle_notification(notification);
      }
    }
  }

  eprintln!("Shutting down.");
  Ok(())
}

fn validate_message(
  text: &str,
  uri: Uri,
  version: i32,
  client: &LanguageClient<'_>,
) -> Result<(), anyhow::Error> {
  let (_ast, diagnostics, text_info) = parse(text);

  let diagnostics = diagnostics
    .into_iter()
    .map(|diag| {
      let span = diag.span();

      fn loc_to_pos(info: &SourceTextInfo, loc: Location) -> Position {
        let (line, character) = info.utf16_line_col(loc);
        Position { line, character }
      }

      Diagnostic {
        range: Range {
          start: loc_to_pos(&text_info, span.start),
          end: loc_to_pos(&text_info, span.end),
        },
        severity: Some(lsp_types::DiagnosticSeverity::ERROR),
        message: diag.to_string(),
        source: Some("mf2".to_string()),
        ..Diagnostic::default()
      }
    })
    .collect();

  let params = PublishDiagnosticsParams {
    uri,
    version: Some(version),
    diagnostics,
  };

  client.publish_diagnostics(params);

  Ok(())
}

struct Server<'a> {
  client: LanguageClient<'a>,
}

impl LanguageServer for Server<'_> {
  fn on_open_text_document(&mut self, params: DidOpenTextDocumentParams) {
    if let Err(err) = validate_message(
      &params.text_document.text,
      params.text_document.uri,
      params.text_document.version,
      &self.client,
    ) {
      eprintln!("Error validating message: {:#?}", err);
    }
  }

  fn on_change_text_document(&mut self, params: DidChangeTextDocumentParams) {
    if let Err(err) = validate_message(
      &params.content_changes[0].text,
      params.text_document.uri,
      params.text_document.version,
      &self.client,
    ) {
      eprintln!("Error validating message: {:#?}", err);
    }
  }

  fn on_close_text_document(&mut self, _params: DidCloseTextDocumentParams) {}

  fn hover(
    &mut self,
    params: lsp_types::HoverParams,
  ) -> Result<Option<lsp_types::Hover>, anyhow::Error> {
    eprintln!("Hover request: {:#?}", params);
    Ok(None)
  }
}
