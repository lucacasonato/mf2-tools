mod ast_utils;
mod completions;
mod document;
mod protocol;
mod semantic_tokens;
mod server;

pub use crate::protocol::ConnectionManager;
pub use crate::server::Server;

#[cfg(target_arch = "wasm32")]
pub use wasm::WasmServer;

#[cfg(target_arch = "wasm32")]
mod wasm {
  use std::cell::RefCell;
  use std::rc::Rc;

  use lsp_server::Connection;
  use serde::Serialize as _;
  use wasm_bindgen::prelude::*;
  use yoke::Yoke;
  use yoke::Yokeable;

  use crate::protocol::ConnectionManager;
  use crate::server::Server;

  #[wasm_bindgen]
  pub struct WasmServer {
    connection: Connection,
    connection_manager:
      Yoke<WasmServerConnectionManager<'static>, Box<Connection>>,
  }

  #[derive(Yokeable)]
  struct WasmServerConnectionManager<'a>(ConnectionManager<'a, Server<'a>>);

  #[wasm_bindgen]
  impl WasmServer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmServer, JsError> {
      let (connection, connection2) = Connection::memory();

      Ok(WasmServer {
        connection: connection2,
        connection_manager: Yoke::attach_to_cart(
          Box::new(connection),
          |connection| {
            let server = Server::start(connection);
            WasmServerConnectionManager(ConnectionManager::new(
              connection, server,
            ))
          },
        ),
      })
    }

    #[wasm_bindgen]
    pub fn write(&mut self, value: JsValue) -> Result<bool, JsError> {
      let message = serde_wasm_bindgen::from_value(value).map_err(|err| {
        JsError::new(&format!("Error deserializing message: {:?}", err))
      })?;
      let rv = Rc::new(RefCell::new(None));
      let rv_ = rv.clone();
      self.connection_manager.with_mut(move |c| {
        rv_
          .borrow_mut()
          .replace(c.0.handle_message(message).map_err(|err| {
            JsError::new(&format!("Error handling message: {:?}", err))
          }));
      });
      let cf = rv.borrow_mut().take().unwrap()?;
      match cf {
        std::ops::ControlFlow::Break(_) => Ok(true),
        std::ops::ControlFlow::Continue(_) => Ok(false),
      }
    }

    #[wasm_bindgen]
    pub fn read(&mut self) -> JsValue {
      match self.connection.receiver.try_recv().ok() {
        Some(message) => message
          .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
          .unwrap(),
        None => JsValue::NULL,
      }
    }
  }
}
