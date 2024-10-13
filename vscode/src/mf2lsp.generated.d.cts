export interface InstantiateResult {
  instance: WebAssembly.Instance;
  exports: {
    WasmServer: typeof WasmServer;
  };
}
/** Gets if the Wasm module has been instantiated. */
export function isInstantiated(): boolean;
/** Options for instantiating a Wasm instance. */
export interface InstantiateOptions {
  /** Optional url to the Wasm file to instantiate. */
  url?: URL;
  /** Callback to decompress the raw Wasm file bytes before instantiating. */
  decompress?: (bytes: Uint8Array) => Uint8Array;
}
/** Instantiates an instance of the Wasm module returning its functions.
 * @remarks It is safe to call this multiple times and once successfully
 * loaded it will always return a reference to the same object. */
export function instantiate(
  opts?: InstantiateOptions,
): Promise<InstantiateResult["exports"]>;
/** Instantiates an instance of the Wasm module along with its exports.
 * @remarks It is safe to call this multiple times and once successfully
 * loaded it will always return a reference to the same object. */
export function instantiateWithInstance(
  opts?: InstantiateOptions,
): Promise<InstantiateResult>;
/** */
export class WasmServer {
  free(): void;
  /** */
  constructor();
  /**
   * @param {any} value
   * @returns {boolean}
   */
  write(value: any): boolean;
  /**
   * @returns {any}
   */
  read(): any;
}
