use crate::linter::diagnostic::Diagnostic;
use crate::linter::Linter;
use crate::lsp::convert::{to_lsp_diagnostic, to_workspace_edit};
use lsp_types::{
    CodeActionOrCommand, CodeActionParams, CodeActionResponse, DiagnosticOptions,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    Hover, HoverContents, HoverParams, InitializeParams, InitializeResult, MessageType,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer};

pub struct IonLspServer {
    client: Client,
    linter: Arc<Linter>,
    last: Arc<Mutex<HashMap<Url, Vec<Diagnostic>>>>,
}

impl IonLspServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            linter: Arc::new(Linter::new()),
            last: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn analyze_uri(&self, uri: &Url) -> Vec<Diagnostic> {
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        self.linter.run_on_files(&[path], None).unwrap_or_default()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for IonLspServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                diagnostic_provider: Some(
                    lsp_types::DiagnosticServerCapabilities::Options(DiagnosticOptions {
                        identifier: Some("ion".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: Default::default(),
                    }),
                ),
                code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
                hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: lsp_types::InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Ion LSP initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
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
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        if let Ok(map) = self.last.lock() {
            if let Some(diags) = map.get(&uri) {
                for d in diags {
                    if let Some(fix) = &d.fix {
                        let edit = to_workspace_edit(fix, &uri);
                        let action = lsp_types::CodeAction {
                            title: format!("Apply ion fix: {}", d.rule),
                            kind: Some(lsp_types::CodeActionKind::QUICKFIX),
                            diagnostics: None,
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
                            contents: HoverContents::Scalar(lsp_types::MarkedString::String(
                                format!(
                                    "{}\n{}\nhttps://github.com/cybergenii/ion",
                                    d.rule, d.message
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
