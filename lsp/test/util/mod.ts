import { assert, assertEquals } from "@std/assert";
import { concat, indexOfNeedle } from "@std/bytes";
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

const BINARY_URL = new URL(
  `../../../target/${Deno.env.get("PROFILE") ?? "debug"}/mf2lsp`,
  import.meta.url,
);

export class LSPTest implements AsyncDisposable {
  #child: Deno.ChildProcess;
  #outPipe: Promise<void>;
  #inPromise: Promise<void>;
  #out: WritableStream<Message>;
  #nextId = 1;

  #responsePromises = new Map<number, (msg: ResponseMessage) => void>();
  #notificationListeners = new Map<
    string,
    (msg: NotificationMessage) => void
  >();

  constructor() {
    this.#child = new Deno.Command(BINARY_URL, {
      stdin: "piped",
      stdout: "piped",
    }).spawn();
    const encoder = new LSPEncoderStream();
    this.#outPipe = encoder.readable.pipeTo(this.#child.stdin);
    this.#out = encoder.writable;
    this.#inPromise = (async () => {
      const in_ = this!.#child.stdout.pipeThrough(new LSPDecoderStream());
      for await (const message of in_) {
        if (Message.isResponse(message)) {
          this.#responsePromises.get(message.id as number)!(message);
          this.#responsePromises.delete(message.id as number);
        } else if (Message.isNotification(message)) {
          const listener = this.#notificationListeners.get(message.method);
          listener?.(message);
          this.#notificationListeners.delete(message.method);
        }
      }
    })();
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
    const writer = this.#out.getWriter();
    try {
      await writer.write(
        { jsonrpc: "2.0", method, params } as NotificationMessage,
      );
    } finally {
      writer.releaseLock();
    }
  }

  async request<const Method extends RequestMethods>(
    method: Method,
    params: RequestParams<Method>,
  ): Promise<RequestResponse<Method>> {
    const writer = this.#out.getWriter();
    const id = this.#nextId++;
    const pwr = Promise.withResolvers<ResponseMessage>();
    this.#responsePromises.set(id, pwr.resolve);
    try {
      await writer.write(
        { jsonrpc: "2.0", id, method, params } as RequestMessage,
      );
    } finally {
      writer.releaseLock();
    }
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
    const status = await this.#child.status;
    assertEquals(status.code, 0);
    await this.#out.close();
    await this.#outPipe;
    await this.#inPromise;
  }
}

class LSPDecoderStream extends TransformStream<
  Uint8Array,
  RequestMessage | ResponseMessage | NotificationMessage
> {
  #message: Uint8Array | null = null;
  constructor() {
    super({
      transform: (chunk, controller) => {
        while (true) {
          if (this.#message === null) {
            if (chunk.length === 0) return;
            this.#message = chunk;
          } else if (chunk.length > 0) {
            this.#message = concat([this.#message, chunk]);
          }
          const endOfHeaders = indexOfNeedle(
            this.#message,
            new Uint8Array([0x0d, 0x0a, 0x0d, 0x0a]),
          );
          if (endOfHeaders === -1) return;
          const headers = new TextDecoder().decode(
            this.#message.subarray(0, endOfHeaders),
          );
          const lines = headers.split("\r\n");
          let index: number = 0;
          let contentLength: number | undefined;
          for (const line of lines) {
            const [key, value] = line.split(": ");
            if (key.toLowerCase() === "content-length") {
              contentLength = parseInt(value, 10);
            }
            index += line.length + 2;
          }
          assert(contentLength !== undefined);
          const bodyBytes = this.#message.subarray(
            endOfHeaders + 4,
            endOfHeaders + 4 + contentLength,
          );
          if (bodyBytes.length < contentLength) return;
          const body = new TextDecoder().decode(bodyBytes);
          try {
            const message = JSON.parse(body);
            controller.enqueue(message);
          } catch (e) {
            (e as Error).message += " while parsing " + JSON.stringify(body);
            throw e;
          }
          const length = endOfHeaders + 4 + contentLength;
          if (this.#message.length === length) {
            this.#message = null;
          } else {
            this.#message = this.#message.subarray(length);
          }
          chunk = new Uint8Array();
        }
      },
      flush: () => {
        if (this.#message !== null) {
          throw new TypeError(
            "Incomplete message to decode.\n" +
              new TextDecoder().decode(this.#message),
          );
        }
      },
    });
  }
}

class LSPEncoderStream extends TransformStream<
  RequestMessage | ResponseMessage | NotificationMessage,
  Uint8Array
> {
  constructor() {
    super({
      transform: (chunk, controller) => {
        const json = JSON.stringify(chunk);
        const encodedJson = new TextEncoder().encode(json);
        const headers =
          `content-length: ${encodedJson.length}\r\ncontent-type: application/json; charset=utf-8\r\n\r\n`;
        const encodedHeaders = new TextEncoder().encode(headers);
        controller.enqueue(encodedHeaders);
        controller.enqueue(encodedJson);
      },
    });
  }
}
