use lsp_server::Connection;

macro_rules! language_server {
  (
    notifications: {
      $($notif_fn:ident: $notif_type:ident),*$(,)?
    },
    requests: {
      $($req_fn:ident: $req_type:ident),*$(,)?
    },
  ) => {

    pub trait LanguageServer {
      fn handle_notification(&mut self, method: &str, params: serde_json::Value) {
        match method {
          $(
            <lsp_types::notification::$notif_type as lsp_types::notification::Notification>::METHOD => {
                let params_res = serde_json::from_value::<
                  <lsp_types::notification::$notif_type as lsp_types::notification::Notification>::Params,
                >(params);
              let params = match params_res {
                Ok(params) => params,
                Err(err) => {
                  eprintln!(
                    "Failed to parse notification params ({}): {}",
                    method,
                    err
                  );
                  return;
                }
              };
              self.$notif_fn(params);
            }
          )*
          _ => {
            eprintln!("Unrecognized notification: {}", method);
          }
        }
      }

      fn handle_request(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, lsp_server::ResponseError> {
        match method {
          $(
            <lsp_types::request::$req_type as lsp_types::request::Request>::METHOD => {
              let params_res = serde_json::from_value::<
                <lsp_types::request::$req_type as lsp_types::request::Request>::Params,
              >(params);
              let params = match params_res {
                Ok(params) => params,
                Err(err) => {
                  eprintln!(
                    "Failed to parse request params ({}): {}",
                    method,
                    err,
                  );
                  return Err(lsp_server::ResponseError {
                    code: -32700,
                    message: err.to_string(),
                    data: None,
                  })
                }
              };
              let result = self.$req_fn(params);
              let result = match serde_json::to_value(result) {
                Ok(result) => result,
                Err(err) => {
                  eprintln!(
                    "Failed to serialize response ({}): {}",
                    method,
                    err,
                  );
                  return Err(lsp_server::ResponseError {
                    code: -32603,
                    message: err.to_string(),
                    data: None,
                  })
                }
              };
              Ok(result)
            }
          )*
          _ => {
            eprintln!("Unrecognized request: {}", method);
            Err(lsp_server::ResponseError {
              code: -32600,
              message: "Request method not supported".to_string(),
              data: None,
            })
          }
        }

      }

      $(
        fn $notif_fn(&mut self, params: <lsp_types::notification::$notif_type as lsp_types::notification::Notification>::Params);
      )*

      $(
        fn $req_fn(&mut self, params: <lsp_types::request::$req_type as lsp_types::request::Request>::Params) -> <lsp_types::request::$req_type as lsp_types::request::Request>::Result;
      )*
    }
  };
}

pub struct LanguageClient<T> {
  connection: Connection,
  next_request_id: i32,
  response_handlers: std::collections::HashMap<
    lsp_server::RequestId,
    Box<
      dyn FnOnce(&mut T, Result<serde_json::Value, lsp_server::ResponseError>),
    >,
  >,
  _phantom_state: std::marker::PhantomData<T>,
}

impl<State> LanguageClient<State> {
  pub fn new(connection: Connection) -> Self {
    Self {
      connection,
      next_request_id: 1,
      response_handlers: std::collections::HashMap::new(),
      _phantom_state: std::marker::PhantomData,
    }
  }

  pub fn raw_connection(&self) -> &Connection {
    &self.connection
  }

  pub fn send_notification(&self, notification: lsp_server::Notification) {
    let res = self
      .connection
      .sender
      .send(lsp_server::Message::Notification(notification));
    if let Err(err) = res {
      eprintln!("Failed to send notification: {}", err);
    }
  }

  pub fn send_request(
    &mut self,
    method: String,
    params: serde_json::Value,
    cb: impl FnOnce(&mut State, Result<serde_json::Value, lsp_server::ResponseError>)
      + 'static,
  ) {
    let request_id = self.next_request_id;
    self.next_request_id += 1;

    let request = lsp_server::Request {
      id: lsp_server::RequestId::from(request_id),
      method,
      params,
    };

    let id = request.id.clone();

    self
      .response_handlers
      .insert(request.id.clone(), Box::new(cb));

    let res = self
      .connection
      .sender
      .send(lsp_server::Message::Request(request));
    if let Err(err) = res {
      self.response_handlers.remove(&id);
      eprintln!("Failed to send request: {}", err);
    }
  }

  pub fn handle_response<'a>(
    &mut self,
    response: lsp_server::Response,
  ) -> Option<impl FnOnce(&'a mut State)> {
    let id = response.id.clone();
    let handler = self.response_handlers.remove(&id);
    if let Some(handler) = handler {
      let result = match (response.result, response.error) {
        (Some(result), None) => Ok(result),
        (None, Some(error)) => Err(error),
        (result, error) => {
          eprintln!(
            "Response with result and error, or neither result or error: {:?} {:?}",
            result, error
          );
          return None;
        }
      };
      Some(|state| handler(state, result))
    } else {
      eprintln!("Response for ID that was not requested: {:?}", response);
      None
    }
  }
}

macro_rules! language_client {
  (
    notifications: {
      $($notif_fn:ident: $notif_type:ident),*$(,)?
    },
    requests: {
      $($req_fn:ident: $req_type:ident),*$(,)?
    },
  ) => {
    impl<T> LanguageClient<T> {
      $(
        pub fn $notif_fn(&self, params: <lsp_types::notification::$notif_type as lsp_types::notification::Notification>::Params) {
          let notification = lsp_server::Notification {
            method: <lsp_types::notification::$notif_type as lsp_types::notification::Notification>::METHOD.to_owned(),
            params: serde_json::to_value(params).unwrap(),
          };
          self.send_notification(notification);
        }
      )*

      $(
        pub fn $req_fn(
          &mut self,
          params: <lsp_types::request::$req_type as lsp_types::request::Request>::Params,
          cb: impl FnOnce(&mut T, Result<<lsp_types::request::$req_type as lsp_types::request::Request>::Result, lsp_server::ResponseError>) + 'static,
        ) {
          let method = <lsp_types::request::$req_type as lsp_types::request::Request>::METHOD.to_owned();
          let params = serde_json::to_value(params).unwrap();
          self.send_request(method, params, |state, result| {
            let result = match result {
              Ok(result) => serde_json::from_value::<
                <lsp_types::request::$req_type as lsp_types::request::Request>::Result,
              >(result),
              Err(err) => {
                cb(state, Err(err));
                return;
              }
            };
            match result {
              Ok(result) => cb(state, Ok(result)),
              Err(err) => {
                eprintln!("Failed to parse response: {}", err);
              },
            }
          });
        }
      )*
    }
  };
}

language_server!(
  notifications: {
    on_open_text_document: DidOpenTextDocument,
    on_change_text_document: DidChangeTextDocument,
    on_close_text_document: DidCloseTextDocument,
  },
  requests: {
    hover: HoverRequest,
  },
);

language_client!(
  notifications: {
    publish_diagnostics: PublishDiagnostics,
  },
  requests: {
    show_notification: ShowMessageRequest,
  },
);
