use lsp_server::Connection;
use lsp_types::notification::PublishDiagnostics;

pub struct LanguageClient<'a> {
  connection: &'a Connection,
}

impl<'a> LanguageClient<'a> {
  pub fn new(connection: &'a Connection) -> Self {
    Self { connection }
  }
}

macro_rules! language_client {
  (
      notifications: {
        $($name:ident: $typ:path),* $(,)?
      }
    ) => {
    impl LanguageClient<'_> {
    $(
      pub fn $name(
        &self,
        params: <$typ as lsp_types::notification::Notification>::Params,
      ) {
        if let Err(err) =
          self
            .connection
            .sender
            .send(lsp_server::Message::Notification(
              lsp_server::Notification {
                method: <$typ as lsp_types::notification::Notification>::METHOD
                  .to_string(),
                params: serde_json::to_value(params).unwrap(),
              },
            ))
        {
          eprintln!(
            "Error sending {} notification: {:?}",
            <$typ as lsp_types::notification::Notification>::METHOD,
            err
          );
        }
      }
    )*
    }
  };
}

language_client! {
  notifications: {
    publish_diagnostics: PublishDiagnostics,
  }
}
