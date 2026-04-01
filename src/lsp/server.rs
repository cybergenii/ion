use crate::linter::diagnostic::Diagnostic;
use crate::linter::rules::describe_rule;
use crate::linter::Linter;
use crate::lsp::convert::{ranges_overlap, to_lsp_diagnostic, to_lsp_range, to_workspace_edit};
use tower_lsp::lsp_types::{
    CodeActionOrCommand, CodeActionParams, CodeActionResponse, DiagnosticOptions,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
    HoverParams, InitializeParams, InitializeResult, MessageType, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer};

pub struct IonLspServer {
    client: Client,
    linter: Arc<Linter>,
    last: Arc<Mutex<HashMap<Url, Vec<Diagnostic>>>>,
    /// Unsaved document text (full sync).
    documents: Arc<Mutex<HashMap<Url, String>>>,
}

impl IonLspServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            linter: Arc::new(Linter::new()),
            last: Arc::new(Mutex::new(HashMap::new())),
            documents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn analyze_uri(&self, uri: &Url) -> Vec<Diagnostic> {
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let text = self
            .documents
            .lock()
            .ok()
            .and_then(|m| m.get(uri).cloned());
        match text {
            Some(t) => self
                .linter
                .analyze_file_with_source(&path, &t, None)
                .unwrap_or_default(),
            None => std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| self.linter.analyze_file_with_source(&path, &s, None).ok())
                .unwrap_or_default(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for IonLspServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                diagnostic_provider: Some(
                    tower_lsp::lsp_types::DiagnosticServerCapabilities::Options(DiagnosticOptions {
                        identifier: Some("ion".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: Default::default(),
                    }),
                ),
                code_action_provider: Some(tower_lsp::lsp_types::CodeActionProviderCapability::Simple(true)),
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: tower_lsp::lsp_types::InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Ion LSP initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Ok(mut map) = self.documents.lock() {
            map.insert(uri.clone(), params.text_document.text);
        }
        let diagnostics = self.analyze_uri(&uri).await;
        let lsp_diags = diagnostics.iter().map(to_lsp_diagnostic).collect();
        self.client.publish_diagnostics(uri.clone(), lsp_diags, None).await;
        if let Ok(mut map) = self.last.lock() {
            map.insert(uri, diagnostics);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        for change in params.content_changes {
            if change.range.is_none() {
                if let Ok(mut map) = self.documents.lock() {
                    map.insert(uri.clone(), change.text);
                }
            }
        }
        let diagnostics = self.analyze_uri(&uri).await;
        let lsp_diags = diagnostics.iter().map(to_lsp_diagnostic).collect();
        self.client.publish_diagnostics(uri.clone(), lsp_diags, None).await;
        if let Ok(mut map) = self.last.lock() {
            map.insert(uri, diagnostics);
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let diagnostics = self.analyze_uri(&uri).await;
        let lsp_diags = diagnostics.iter().map(to_lsp_diagnostic).collect();
        self.client.publish_diagnostics(uri.clone(), lsp_diags, None).await;
        if let Ok(mut map) = self.last.lock() {
            map.insert(uri, diagnostics);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .publish_diagnostics(params.text_document.uri.clone(), Vec::new(), None)
            .await;
        if let Ok(mut map) = self.last.lock() {
            map.remove(&params.text_document.uri);
        }
        if let Ok(mut map) = self.documents.lock() {
            map.remove(&params.text_document.uri);
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        let req_range = params.range;
        if let Ok(map) = self.last.lock() {
            if let Some(diags) = map.get(&uri) {
                for d in diags {
                    if let Some(fix) = &d.fix {
                        let drange = to_lsp_range(
                            d.line,
                            d.span.map(|s| s.0).unwrap_or(d.column),
                            d.span.map(|s| s.1).unwrap_or(d.column + 1),
                        );
                        if !ranges_overlap(&drange, &req_range) {
                            continue;
                        }
                        let edit = to_workspace_edit(fix, &uri);
                        let lsp_d = to_lsp_diagnostic(d);
                        let action = tower_lsp::lsp_types::CodeAction {
                            title: format!("Apply ion fix: {}", d.rule),
                            kind: Some(tower_lsp::lsp_types::CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![lsp_d]),
                            edit: Some(edit),
                            command: None,
                            is_preferred: Some(true),
                            disabled: None,
                            data: None,
                        };
                        actions.push(CodeActionOrCommand::CodeAction(action));
                    }
                }
            }
        }
        Ok(Some(actions))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        if !self.linter.semantic_available() {
            return Ok(None);
        }
        let source = self
            .documents
            .lock()
            .ok()
            .and_then(|m| m.get(&uri).cloned())
            .or_else(|| std::fs::read_to_string(&path).ok());
        let Some(source) = source else {
            return Ok(None);
        };
        let loc = crate::lsp::navigation::goto_definition(&path, &source, pos.line, pos.character);
        Ok(loc.map(GotoDefinitionResponse::Scalar))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        if let Ok(map) = self.last.lock() {
            if let Some(diags) = map.get(&uri) {
                for d in diags {
                    let line = d.line.saturating_sub(1);
                    let (start, end) = d.span.unwrap_or((d.column, d.column + 1));
                    if line == pos.line
                        && pos.character >= start.saturating_sub(1)
                        && pos.character <= end.saturating_sub(1)
                    {
                        return Ok(Some(Hover {
                            contents: HoverContents::Scalar(tower_lsp::lsp_types::MarkedString::String(
                                format!(
                                    "{}\n{}\n{}\nhttps://github.com/cybergenii/ion",
                                    d.rule,
                                    describe_rule(d.rule),
                                    d.message
                                ),
                            )),
                            range: None,
                        }));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
