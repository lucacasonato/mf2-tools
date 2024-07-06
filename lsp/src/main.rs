use lsp_server::Connection;
use lsp_server::Message;
use lsp_server::Notification;
use lsp_types::notification::DidChangeTextDocument;
use lsp_types::notification::DidCloseTextDocument;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::Diagnostic;
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

fn main() -> Result<(), anyhow::Error> {
  let (connection, _threads) = Connection::stdio();

  let capabilities = ServerCapabilities {
    text_document_sync: Some(TextDocumentSyncCapability::Kind(
      TextDocumentSyncKind::FULL,
    )),
    ..ServerCapabilities::default()
  };

  let server_capabilities = serde_json::to_value(capabilities).unwrap();
  let initialization_params = connection.initialize(server_capabilities)?;

  let _initialization_params =
    serde_json::from_value::<InitializeParams>(initialization_params)?;

  eprintln!("Server initialized.");

  loop {
    match connection.receiver.recv()? {
      Message::Request(_) => todo!(),
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

            validate_message(
              &params.text_document.text,
              params.text_document.uri,
              params.text_document.version,
              &connection
            )?;
          }
          DidChangeTextDocument(params) => {
            eprintln!("Changed document: {:#?}", params);

            validate_message(
              &params.content_changes[0].text,
              params.text_document.uri,
              params.text_document.version,
              &connection
            )?;
          }
          DidCloseTextDocument(params) => {
            eprintln!("Closed document: {:#?}", params);
          }
        }
      }
    }
  }
}

fn validate_message(
  text: &str,
  uri: Uri,
  version: i32,
  connection: &Connection,
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

  connection.sender.send(Message::Notification(Notification {
    method: "textDocument/publishDiagnostics".to_string(),
    params: serde_json::to_value(params).unwrap(),
  }))?;

  Ok(())
}
