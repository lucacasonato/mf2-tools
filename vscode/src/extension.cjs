const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

exports.Mf2Extension = class Mf2Extension {
  /** @type {vscode.ExtensionContext} */
  #context;

  /** @type {LanguageClient | null} */
  #ls = null;
  /** @type {vscode.LogOutputChannel} */
  #outputChannel;

  /** @param {vscode.ExtensionContext} context */
  async activate(context) {
    this.#context = context;

    context.subscriptions.push(
      vscode.commands.registerCommand("mf2.restart", async () => {
        await this.#stopLanguageServer();
        await this.#startLanguageServer();
      })
    );

    this.#outputChannel = vscode.window.createOutputChannel(
      "MessageFormat 2.0",
      { log: true }
    );
    context.subscriptions.push(this.#outputChannel);

    await this.#startLanguageServer();
  }

  async deactivate() {
    await this.#stopLanguageServer();
  }

  async #startLanguageServer() {
    if (this.#ls) return;

    /** @type {import("vscode-languageclient/node").ServerOptions} */
    const serverOptions = {
      run: {
        command: "<path>",
        args: [],
      },
      debug: {
        command: "<path>",
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
      "MessageFormat 2.0 Language Server",
      serverOptions,
      clientOptions
    );

    await this.#ls.start();
  }

  async #stopLanguageServer() {
    await this.#ls?.stop(2000);
    this.#ls = null;
  }
};
