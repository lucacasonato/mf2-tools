import {
  InitializeParams,
  Message,
  NotificationMessage,
  RequestMessage,
  ResponseMessage,
} from "vscode-languageserver-protocol";

import type {
  NotificationMethods,
  NotificationParams,
  RequestMethods,
  RequestParams,
  RequestResponse,
} from "./types.ts";

export class LSPTest implements AsyncDisposable {
  #nextId = 1;
  #responsePromises = new Map<
    number,
    { cb: (msg: ResponseMessage) => void; isShutdown: boolean }
  >();
  #notificationListeners = new Map<
    string,
    (msg: NotificationMessage) => void
  >();

  constructor() {
  }

  protected send(_msg: Message): Promise<void> {
    throw new Error("LSPTest.send not implemented.");
  }

  protected recv(msg: Message): boolean {
    if (Message.isResponse(msg)) {
      const { cb, isShutdown } = this.#responsePromises.get(msg.id as number)!;
      cb(msg);
      this.#responsePromises.delete(msg.id as number);
      return isShutdown;
    } else if (Message.isNotification(msg)) {
      const listener = this.#notificationListeners.get(msg.method);
      listener?.(msg);
      this.#notificationListeners.delete(msg.method);
    }
    return false;
  }

  async initialize() {
    await this.request(
      "initialize",
      {
        processId: Deno.pid,
        rootUri: "file:///src/",
        capabilities: {},
        clientInfo: {
          name: "lsp-test",
          version: "0.0.0",
        },
      } satisfies InitializeParams,
    );
    await this.notify("initialized", {});
  }

  async notify<const Method extends NotificationMethods>(
    method: Method,
    params: NotificationParams<Method>,
  ) {
    await this.send({ jsonrpc: "2.0", method, params } as NotificationMessage);
  }

  async request<const Method extends RequestMethods>(
    method: Method,
    params: RequestParams<Method>,
  ): Promise<RequestResponse<Method>> {
    const id = this.#nextId++;
    const pwr = Promise.withResolvers<ResponseMessage>();
    this.#responsePromises.set(id, {
      cb: pwr.resolve,
      isShutdown: method === "shutdown",
    });
    await this.send({ jsonrpc: "2.0", id, method, params } as RequestMessage);
    const result = await pwr.promise;
    if ("result" in result) {
      return result.result;
    } else {
      throw result.error;
    }
  }

  async waitNotify<const Method extends NotificationMethods>(
    method: Method,
  ): Promise<NotificationParams<Method>> {
    const pwr = Promise.withResolvers<NotificationMessage>();
    this.#notificationListeners.set(method, pwr.resolve);
    const notification = await pwr.promise;
    return notification.params as NotificationParams<Method>;
  }

  async [Symbol.asyncDispose]() {
    await this.request("shutdown", undefined);
    await this.notify("exit", undefined);
  }
}
