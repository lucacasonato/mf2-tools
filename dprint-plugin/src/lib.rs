use anyhow::anyhow;
use serde::Serialize;

use anyhow::Result;
use dprint_core::configuration::get_unknown_property_diagnostics;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::plugins::FileMatchingInfo;
use dprint_core::plugins::PluginInfo;
use dprint_core::plugins::PluginResolveConfigurationResult;
use dprint_core::plugins::SyncPluginHandler;

#[cfg(target_arch = "wasm32")]
use dprint_core::generate_plugin_code;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {}

pub struct Mf2PluginHandler;

impl SyncPluginHandler<Configuration> for Mf2PluginHandler {
  fn plugin_info(&mut self) -> PluginInfo {
    PluginInfo {
      name: env!("CARGO_PKG_NAME").to_string(),
      version: env!("CARGO_PKG_VERSION").to_string(),
      config_key: "mf2".to_string(),
      help_url: format!("https://github.com/lucacasonato/mf2-tools/blob/{}/dprint-plugin/README.md", env!("CARGO_PKG_VERSION")),
      config_schema_url: format!("https://plugins.dprint.dev/lucacasonato/mf2-tools/{}/schema.json", env!("CARGO_PKG_VERSION")),
      update_url: Some("https://plugins.dprint.dev/lucacasonato/mf2-tools/latest.json".to_string()),
    }
  }

  fn license_text(&mut self) -> String {
    include_str!("../LICENSE").to_owned()
  }

  fn resolve_config(
    &mut self,
    config: ConfigKeyMap,
    _global_config: &GlobalConfiguration,
  ) -> PluginResolveConfigurationResult<Configuration> {
    let diagnostics = get_unknown_property_diagnostics(config);

    PluginResolveConfigurationResult {
      config: Configuration {},
      diagnostics,
      file_matching: FileMatchingInfo {
        file_extensions: vec!["mf2".to_string()],
        file_names: vec![],
      },
    }
  }

  fn check_config_updates(
    &self,
    _message: dprint_core::plugins::CheckConfigUpdatesMessage,
  ) -> Result<Vec<dprint_core::plugins::ConfigChange>> {
    Ok(vec![])
  }

  fn format(
    &mut self,
    request: dprint_core::plugins::SyncFormatRequest<Configuration>,
    _format_with_host: impl FnMut(
      dprint_core::plugins::SyncHostFormatRequest,
    ) -> dprint_core::plugins::FormatResult,
  ) -> dprint_core::plugins::FormatResult {
    let message = std::str::from_utf8(&request.file_bytes)?;
    let (ast, diagnostics, info) = mf2_parser::parse(message);
    for diagnostic in diagnostics {
      if diagnostic.fatal() {
        return Err(anyhow!("failed to format: {:?}", diagnostic));
      }
    }

    let printed = mf2_printer::print(&ast, Some(&info));
    if printed.as_bytes() != request.file_bytes {
      Ok(Some(printed.into_bytes()))
    } else {
      Ok(None)
    }
  }
}

#[cfg(target_arch = "wasm32")]
generate_plugin_code!(Mf2PluginHandler, Mf2PluginHandler);
