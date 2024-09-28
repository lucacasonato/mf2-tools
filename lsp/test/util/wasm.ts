import { Message } from "vscode-languageserver-protocol";

import { LSPTest } from "./mod.ts";

declare class IWasmServer {
  free(): void;
  constructor();
  // deno-lint-ignore no-explicit-any
  write(value: any): boolean;
  // deno-lint-ignore no-explicit-any
  read(): any;
}

const { instantiate } = await import(
  `../../../target/wasm/${
    Deno.env.get("PROFILE") ?? "debug"
  }/mf2lsp.generated.js`
);

const { WasmServer }: { WasmServer: typeof IWasmServer } = await instantiate();

export class WasmLSPTest extends LSPTest {
  #wasmServer: IWasmServer;

  constructor() {
    super();
    this.#wasmServer = new WasmServer();
  }

  // deno-lint-ignore require-await
  override async send(msg: Message): Promise<void> {
    const stop = this.#wasmServer.write(msg);
    if (stop) return;
    let message: Message;
    while ((message = this.#wasmServer.read()) !== null) {
      this.recv(message);
    }
  }

  override async [Symbol.asyncDispose]() {
    await super[Symbol.asyncDispose]();
    this.#wasmServer.free();
  }
}
