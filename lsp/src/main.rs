mod diagnostics;
mod document;
mod protocol;
mod scope;

use diagnostics::Diagnostic;
use lsp_server::Connection;
use lsp_server::Message;
use lsp_types::CodeAction;
use lsp_types::Diagnostic as LspDiagnostic;
use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidCloseTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::InitializeParams;
use lsp_types::PublishDiagnosticsParams;
use lsp_types::ServerCapabilities;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use lsp_types::Uri;
use mf2_parser::ast::AnyNode;
use mf2_parser::Spanned;

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::document::Document;
use crate::protocol::LanguageClient;
use crate::protocol::LanguageServer;

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
    code_action_provider: Some(
      lsp_types::CodeActionProviderCapability::Options(
        lsp_types::CodeActionOptions {
          code_action_kinds: Some(vec![lsp_types::CodeActionKind::QUICKFIX]),
          ..lsp_types::CodeActionOptions::default()
        },
      ),
    ),
    rename_provider: Some(lsp_types::OneOf::Left(true)),
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
  let mut server = Server {
    client,
    documents: HashMap::new(),
  };

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

struct Server<'a> {
  client: LanguageClient<'a>,
  documents: HashMap<Uri, Document>,
}

impl Server<'_> {
  fn on_document_change(&mut self, uri: Uri, version: i32, text: String) {
    let document = Document::new(uri.clone(), version, text.into_boxed_str());
    let entry = self.documents.entry(uri.clone());
    let document = match entry {
      Entry::Occupied(mut entry) => {
        assert!(entry.get().version < document.version);
        entry.insert(document);
        entry.into_mut()
      }
      Entry::Vacant(entry) => entry.insert(document),
    };

    let parsed = document.parsed.get();

    let diagnostics = &parsed.diagnostics;

    self.client.publish_diagnostics(PublishDiagnosticsParams {
      uri,
      version: Some(document.version),
      diagnostics: diagnostics
        .iter()
        .map(|diag| diag.to_lsp(document))
        .collect(),
    });
  }
}

impl LanguageServer for Server<'_> {
  fn on_open_text_document(&mut self, params: DidOpenTextDocumentParams) {
    self.on_document_change(
      params.text_document.uri.clone(),
      params.text_document.version,
      params.text_document.text,
    );
  }

  fn on_change_text_document(
    &mut self,
    mut params: DidChangeTextDocumentParams,
  ) {
    assert_eq!(params.content_changes.len(), 1);
    self.on_document_change(
      params.text_document.uri.clone(),
      params.text_document.version,
      params.content_changes.remove(0).text,
    );
  }

  fn on_close_text_document(&mut self, params: DidCloseTextDocumentParams) {
    self.documents.remove(&params.text_document.uri);
  }

  fn hover(
    &mut self,
    params: lsp_types::HoverParams,
  ) -> Result<Option<lsp_types::Hover>, anyhow::Error> {
    let maybe_document = self
      .documents
      .get(&params.text_document_position_params.text_document.uri);
    let Some(document) = maybe_document else {
      return Ok(None);
    };

    let maybe_node =
      document.find_node(params.text_document_position_params.position);
    let Some(node) = maybe_node else {
      return Ok(None);
    };

    Ok(Some(lsp_types::Hover {
      contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
        kind: lsp_types::MarkupKind::PlainText,
        value: format!("{:?}", node),
      }),
      range: Some(document.span_to_range(node.span())),
    }))
  }

  fn code_action(
    &mut self,
    params: lsp_types::CodeActionParams,
  ) -> Result<Option<lsp_types::CodeActionResponse>, anyhow::Error> {
    let maybe_document = self.documents.get(&params.text_document.uri);
    let Some(document) = maybe_document else {
      return Ok(None);
    };

    let span = document.range_to_span(params.range);

    let diagnostics = document
      .parsed
      .get()
      .diagnostics
      .iter()
      .filter(|diag| diag.span().contains(dbg!(&span)))
      .filter_map(|d| fix_for_diagnostic(document, d).map(Into::into))
      .collect::<Vec<_>>();

    Ok(Some(diagnostics))
  }

  fn rename(
    &mut self,
    params: lsp_types::RenameParams,
  ) -> Result<Option<lsp_types::WorkspaceEdit>, anyhow::Error> {
    let lsp_types::TextDocumentPositionParams {
      text_document,
      position,
    } = params.text_document_position;

    let maybe_document = self.documents.get(&text_document.uri);
    let Some(document) = maybe_document else {
      return Ok(None);
    };

    let Some(AnyNode::Variable(node)) = document.find_node(position) else {
      return Ok(None);
    };

    let Some(usage) = document.parsed.get().scope.get(node.name) else {
      return Ok(None);
    };

    let mut changes = Vec::new();

    if let Some(declaration_span) = usage.declaration {
      changes.push(lsp_types::TextEdit {
        range: document.span_to_range(declaration_span),
        new_text: format!("${}", params.new_name),
      });
    }
    for reference_span in &usage.references {
      changes.push(lsp_types::TextEdit {
        range: document.span_to_range(*reference_span),
        new_text: format!("${}", params.new_name),
      });
    }

    Ok(Some(lsp_types::WorkspaceEdit {
      changes: Some([(text_document.uri, changes)].into()),
      document_changes: None,
      change_annotations: None,
    }))
  }
}

fn fix_for_diagnostic(
  document: &Document,
  diag: &Diagnostic,
) -> Option<lsp_types::CodeAction> {
  use mf2_parser::Diagnostic::*;

  match diag {
    Diagnostic::Parser(MarkupInvalidSpaceBeforeIdentifier { .. }) => {
      Some(CodeAction {
        title: "Remove space before identifier".to_string(),
        kind: Some(lsp_types::CodeActionKind::QUICKFIX),
        edit: Some(lsp_types::WorkspaceEdit {
          changes: Some(
            [(
              document.uri.clone(),
              vec![lsp_types::TextEdit {
                range: document.span_to_range(diag.span()),
                new_text: "".to_string(),
              }],
            )]
            .into_iter()
            .collect(),
          ),
          change_annotations: None,
          document_changes: None,
        }),
        command: None,
        diagnostics: Some(vec![LspDiagnostic {
          range: document.span_to_range(diag.span()),
          severity: Some(lsp_types::DiagnosticSeverity::ERROR),
          message: diag.to_string(),
          source: Some("mf2".to_string()),
          ..LspDiagnostic::default()
        }]),
        is_preferred: Some(true),
        disabled: None,
        data: None,
      })
    }
    _ => None,
  }
}
