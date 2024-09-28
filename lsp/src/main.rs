use lsp_server::Connection;
use mf2lsp::ConnectionManager;
use mf2lsp::Server;

fn main() -> Result<(), anyhow::Error> {
  let (connection, _threads) = Connection::stdio();

  let server = Server::start(&connection);
  let mut connection_manager = ConnectionManager::new(&connection, server);

  loop {
    let message = connection.receiver.recv()?;
    match connection_manager.handle_message(message)? {
      std::ops::ControlFlow::Break(_) => break,
      std::ops::ControlFlow::Continue(_) => {}
    }
  }

  eprintln!("Shutting down.");

  Ok(())
}
