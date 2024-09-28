const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

/**
 * @typedef Configuration
 * @property {ServerConfiguration} server
 */

/**
 * @typedef ServerConfiguration
 * @property {string} path
 * @property {{ enabled: boolean, version: string }} update
 */

exports.Mf2Extension = class Mf2Extension {
  /** @type {vscode.ExtensionContext} */
  #context;

  /** @type {Configuration} */
  #configuration;

  /** @type {LanguageClient | null} */
  #ls = null;
  /** @type {vscode.LogOutputChannel} */
  #outputChannel;

  /** @param {vscode.ExtensionContext} context */
  async activate(context) {
    this.#context = context;
    this.#configuration = /** @type {any} */ (
      vscode.workspace.getConfiguration("mf2")
    );

    vscode.workspace.onDidChangeConfiguration(
      async (e) => {
        if (e.affectsConfiguration("mf2")) {
          this.#configuration = /** @type {any} */ (
            vscode.workspace.getConfiguration("mf2")
          );
          await this.#stopLanguageServer();
          await this.#startLanguageServer();
        }
      },
      this,
      context.subscriptions,
    );

    context.subscriptions.push(
      vscode.commands.registerCommand("mf2.restart", async () => {
        await this.#stopLanguageServer();
        await this.#startLanguageServer();
      }),
    );

    this.#outputChannel = vscode.window.createOutputChannel(
      "MessageFormat 2",
      { log: true },
    );
    context.subscriptions.push(this.#outputChannel);

    await this.#startLanguageServer();
  }

  async deactivate() {
    await this.#stopLanguageServer();
  }

  async #startLanguageServer() {
    if (this.#ls) return;

    if (!this.#configuration.server.path) {
      vscode.window.showErrorMessage(
        "MessageFormat 2 Language Server could not start up because no path to the language server binary was set, and automatic downloading is disabled.",
      );
      return;
    }

    /** @type {import("vscode-languageclient/node").ServerOptions} */
    const serverOptions = {
      run: {
        command: this.#configuration.server.path,
        args: [],
      },
      debug: {
        command: this.#configuration.server.path,
        args: [],
      },
    };

    /** @type {import("vscode-languageclient").LanguageClientOptions} */
    const clientOptions = {
      documentSelector: [{ scheme: "file", language: "mf2" }],
      outputChannel: this.#outputChannel,
    };

    this.#ls = new LanguageClient(
      "mf2lsp",
      "MessageFormat 2 Language Server",
      serverOptions,
      clientOptions,
    );

    await this.#ls.start();
  }

  async #stopLanguageServer() {
    await this.#ls?.stop(2000);
    this.#ls = null;
  }
};
