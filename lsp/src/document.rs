use lsp_types::Uri;
use mf2_parser::ast::Message;
use mf2_parser::Diagnostic;
use mf2_parser::SourceTextInfo;
use yoke::Yoke;
use yoke::Yokeable;

pub struct Document {
  pub _uri: Uri,
  pub version: i32,
  pub parsed: Yoke<ParsedDocument<'static>, Box<str>>,
}

#[derive(Yokeable)]
pub struct ParsedDocument<'a> {
  pub _ast: Message<'a>,
  pub diagnostics: Vec<Diagnostic<'a>>,
  pub info: SourceTextInfo<'a>,
}

impl Document {
  pub fn new(uri: Uri, version: i32, text: Box<str>) -> Document {
    let parsed = Yoke::attach_to_cart(text, |text| {
      let (ast, diagnostics, info) = mf2_parser::parse(text);
      ParsedDocument {
        _ast: ast,
        diagnostics,
        info,
      }
    });
    Document {
      _uri: uri,
      version,
      parsed,
    }
  }
}
