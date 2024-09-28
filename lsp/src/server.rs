use lsp_server::Connection;
use lsp_types::CodeAction;
use lsp_types::Diagnostic as LspDiagnostic;
use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidCloseTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::InitializeParams;
use lsp_types::InitializeResult;
use lsp_types::InitializedParams;
use lsp_types::PublishDiagnosticsParams;
use lsp_types::SemanticTokens;
use lsp_types::SemanticTokensOptions;
use lsp_types::SemanticTokensParams;
use lsp_types::SemanticTokensRangeParams;
use lsp_types::SemanticTokensRangeResult;
use lsp_types::SemanticTokensResult;
use lsp_types::ServerCapabilities;
use lsp_types::ServerInfo;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use lsp_types::Uri;
use mf2_parser::ast::AnyNode;
use mf2_parser::is_valid_name;
use mf2_parser::Spanned as _;
use mf2_parser::Visitable;

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::ast_utils::find_node;
use crate::completions::CompletionAction;
use crate::completions::CompletionsProvider;
use crate::diagnostics::Diagnostic;
use crate::document::Document;
use crate::protocol::LanguageClient;
use crate::protocol::LanguageServer;
use crate::semantic_tokens;
use crate::semantic_tokens::SemanticTokenVisitor;

pub struct Server<'a> {
  client: LanguageClient<'a>,
  initialize_params: Option<InitializeParams>,
  documents: HashMap<Uri, Document>,
}

impl Server<'_> {
  pub fn start(connection: &Connection) -> Server {
    eprintln!(
      "Starting server... mflsp {}{}",
      env!("CARGO_PKG_VERSION"),
      if option_env!("MF2LSP_OFFICIAL_BUILD") == Some("true") {
        " (official)"
      } else {
        ""
      }
    );

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
  fn initialize(
    &mut self,
    params: InitializeParams,
  ) -> Result<InitializeResult, anyhow::Error> {
    self.initialize_params = Some(params);

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
      rename_provider: Some(lsp_types::OneOf::Right(
        lsp_types::RenameOptions {
          prepare_provider: Some(true),
          work_done_progress_options:
            lsp_types::WorkDoneProgressOptions::default(),
        },
      )),
      declaration_provider: Some(lsp_types::DeclarationCapability::Simple(
        true,
      )),
      definition_provider: Some(lsp_types::OneOf::Right(
        lsp_types::DefinitionOptions {
          work_done_progress_options:
            lsp_types::WorkDoneProgressOptions::default(),
        },
      )),
      completion_provider: Some(lsp_types::CompletionOptions {
        all_commit_characters: None,
        completion_item: None,
        resolve_provider: Some(false),
        trigger_characters: Some(vec!["$".to_string()]),
        work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(
        ),
      }),
      semantic_tokens_provider: Some(
        lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
          SemanticTokensOptions {
            legend: semantic_tokens::legend(),
            full: Some(lsp_types::SemanticTokensFullOptions::Bool(true)),
            range: Some(true),
            ..Default::default()
          },
        ),
      ),
      ..ServerCapabilities::default()
    };

    Ok(InitializeResult {
      capabilities,
      server_info: Some(ServerInfo {
        name: "mf2lsp".to_string(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
      }),
    })
  }

  fn initialized(&mut self, _params: InitializedParams) {
    let initialize_params = self
      .initialize_params
      .as_ref()
      .expect("Initialized before initialize");
    eprint!("Server initialized.");
    if let Some(client_info) = &initialize_params.client_info {
      eprint!(" Connected to: {}", client_info.name);
      if let Some(version) = &client_info.version {
        eprint!(" {}", version);
      }
    }
    eprintln!();
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

    let Some(node) = find_node(
      document.ast(),
      document.pos_to_loc(params.text_document_position_params.position),
    ) else {
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

  fn go_to_declaration(
    &mut self,
    params: lsp_types::GotoDefinitionParams,
  ) -> Result<Option<lsp_types::GotoDefinitionResponse>, anyhow::Error> {
    let lsp_types::TextDocumentPositionParams {
      text_document,
      position,
    } = params.text_document_position_params;

    let document = self
      .documents
      .get(&text_document.uri)
      .ok_or(anyhow::anyhow!("Document not found."))?;

    Ok(
      document
        .find_variable_at(document.pos_to_loc(position))
        .and_then(|name| document.scope().get_declaration_span(name))
        .map(|span| {
          lsp_types::GotoDefinitionResponse::Scalar(lsp_types::Location {
            uri: text_document.uri,
            range: document.span_to_range(span),
          })
        }),
    )
  }

  fn go_to_definition(
    &mut self,
    params: lsp_types::GotoDefinitionParams,
  ) -> Result<Option<lsp_types::GotoDefinitionResponse>, anyhow::Error> {
    self.go_to_declaration(params)
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
    if !is_valid_name(&params.new_name) {
      return Err(anyhow::anyhow!("Invalid variable name."));
    }

    let lsp_types::TextDocumentPositionParams {
      text_document,
      position,
    } = params.text_document_position;

    let document = self
      .documents
      .get(&text_document.uri)
      .ok_or(anyhow::anyhow!("Document not found."))?;

    let old_name = document
      .find_variable_at(document.pos_to_loc(position))
      .ok_or(anyhow::anyhow!(
        "No variable to rename at the given position."
      ))?;

    if old_name == params.new_name {
      return Ok(None);
    }

    let changes = document
      .scope()
      .get_spans(old_name)
      .expect("Variable is in scope")
      .iter()
      .map(|span| lsp_types::TextEdit {
        range: document.span_to_range(*span),
        new_text: format!("${}", params.new_name),
      })
      .collect();

    Ok(Some(lsp_types::WorkspaceEdit {
      changes: Some([(text_document.uri, changes)].into()),
      document_changes: None,
      change_annotations: None,
    }))
  }

  fn prepare_rename(
    &mut self,
    params: lsp_types::TextDocumentPositionParams,
  ) -> Result<Option<lsp_types::PrepareRenameResponse>, anyhow::Error> {
    let maybe_document = self.documents.get(&params.text_document.uri);
    let Some(document) = maybe_document else {
      return Ok(None);
    };

    let Some(AnyNode::Variable(node)) =
      find_node(document.ast(), document.pos_to_loc(params.position))
    else {
      return Ok(None);
    };

    Ok(Some(lsp_types::PrepareRenameResponse::Range(
      document.span_to_range(node.name_span()),
    )))
  }

  fn completion(
    &mut self,
    params: lsp_types::CompletionParams,
  ) -> Result<Option<lsp_types::CompletionResponse>, anyhow::Error> {
    let lsp_types::TextDocumentPositionParams {
      text_document,
      position,
    } = params.text_document_position;

    let document = self
      .documents
      .get(&text_document.uri)
      .ok_or(anyhow::anyhow!("Document not found."))?;

    let provider = CompletionsProvider::new(
      document.ast(),
      document.pos_to_loc(position),
      document.scope(),
    );

    if !provider.has_completions() {
      return Ok(None);
    }

    Ok(Some(lsp_types::CompletionResponse::Array(
      provider
        .get_completions()
        .into_iter()
        .map(|completion| match completion.action {
          CompletionAction::Insert => lsp_types::CompletionItem {
            label: completion.text,
            kind: Some(lsp_types::CompletionItemKind::VARIABLE),
            ..lsp_types::CompletionItem::default()
          },
          CompletionAction::Replace(span) => lsp_types::CompletionItem {
            label: completion.text.clone(),
            kind: Some(lsp_types::CompletionItemKind::VARIABLE),
            text_edit: Some(lsp_types::CompletionTextEdit::Edit(
              lsp_types::TextEdit {
                range: document.span_to_range(span),
                new_text: completion.text,
              },
            )),
            ..lsp_types::CompletionItem::default()
          },
        })
        .collect(),
    )))
  }

  fn semantic_tokens_full(
    &mut self,
    params: SemanticTokensParams,
  ) -> Result<Option<SemanticTokensResult>, anyhow::Error> {
    let maybe_document = self.documents.get(&params.text_document.uri);
    let Some(document) = maybe_document else {
      return Ok(None);
    };

    let mut visitor = SemanticTokenVisitor {
      document,
      tokens: Vec::new(),
      last_start: lsp_types::Position {
        line: 0,
        character: 0,
      },
    };
    document.parsed.get().ast.apply_visitor(&mut visitor);

    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
      result_id: None,
      data: visitor.tokens,
    })))
  }

  fn semantic_tokens_range(
    &mut self,
    params: SemanticTokensRangeParams,
  ) -> Result<Option<SemanticTokensRangeResult>, anyhow::Error> {
    let maybe_document = self.documents.get(&params.text_document.uri);
    let Some(document) = maybe_document else {
      return Ok(None);
    };

    // TODO: only compute tokens for the range

    let mut visitor = SemanticTokenVisitor {
      document,
      tokens: Vec::new(),
      last_start: lsp_types::Position {
        line: 0,
        character: 0,
      },
    };
    document.parsed.get().ast.apply_visitor(&mut visitor);

    Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
      result_id: None,
      data: visitor.tokens,
    })))
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
