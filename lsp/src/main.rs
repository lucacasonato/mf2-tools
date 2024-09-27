mod ast_utils;
mod diagnostics;
mod document;
mod protocol;
mod scope;
mod semantic_tokens;
mod server;

use crate::server::Server;
use lsp_server::Connection;

fn main() -> Result<(), anyhow::Error> {
  let (connection, threads) = Connection::stdio();

  Server::run(connection)?;
  threads.join()?;

  Ok(())
}
