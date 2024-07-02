const { LanguageClient } = require("vscode-languageclient/node");

/** @type {LanguageClient} */
let client;

/**
 *
 * @param {import("vscode").ExtensionContext} context
 */
exports.activate = function activate(context) {
  /** @type {import("vscode-languageclient/node").ServerOptions} */
  const serverOptions = {
    run: {
      command:
        "<path>",
      args: [],
    },
    debug: {
      command:
        "<path>",
      args: [],
    },
  };

  /** @type {import("vscode-languageclient/node").LanguageClientOptions} */
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "mf2" }],
  };

  // Create the language client and start the client.
  client = new LanguageClient(
    "mf2lsp",
    "MessageFormat 2.0 Language Server",
    serverOptions,
    clientOptions
  );

  // Start the client. This will also launch the server
  client.start();
};

exports.deactivate = function deactivate() {
  return client?.stop();
};
