use std::collections::hash_map::Entry;
use std::collections::HashMap;

use lsp_server::Connection;
use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidCloseTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::InitializeParams;
use lsp_types::InitializeResult;
use lsp_types::InitializedParams;
use lsp_types::PublishDiagnosticsParams;
use lsp_types::ServerCapabilities;
use lsp_types::ServerInfo;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use lsp_types::Uri;

use crate::document::Document;
use crate::protocol::LanguageClient;
use crate::protocol::LanguageServer;

pub struct Server<'a> {
  client: LanguageClient<'a>,
  initialize_params: Option<InitializeParams>,
  documents: HashMap<Uri, Document>,
}

impl Server<'_> {
  pub fn start(connection: &Connection) -> Server {
    Server {
      client: LanguageClient::new(connection),
      initialize_params: None,
      documents: HashMap::new(),
    }
  }

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

    self.client.publish_diagnostics(PublishDiagnosticsParams {
      uri,
      version: Some(document.version),
      diagnostics: document.lsp_diagnostics().clone(),
    });
  }
}

impl LanguageServer for Server<'_> {
  fn initialize(
    &mut self,
    params: InitializeParams,
  ) -> Result<InitializeResult, anyhow::Error> {
    self.initialize_params = Some(params);

    Ok(InitializeResult {
      capabilities: ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
          TextDocumentSyncKind::FULL,
        )),
        ..ServerCapabilities::default()
      },
      server_info: Some(ServerInfo {
        name: "mfrlsp".to_string(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
      }),
    })
  }

  fn initialized(&mut self, _params: InitializedParams) {
    let _ = self
      .initialize_params
      .as_ref()
      .expect("Initialized before initialize");
  }

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
}
