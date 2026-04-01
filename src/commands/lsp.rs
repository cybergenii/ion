use crate::lsp::server::IonLspServer;
use anyhow::Result;
use tower_lsp::{LspService, Server};

pub async fn run() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(IonLspServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
