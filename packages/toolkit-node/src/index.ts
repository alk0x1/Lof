import { promises as defaultFs } from 'fs';
import path from 'node:path';

type InputValue = string | number | bigint;

export interface VerifierModuleExports {
  default?: (input?: unknown) => Promise<unknown> | unknown;
  init_panic_hook?: () => void;
  WasmVerifier: new (verificationKeyBytes: Uint8Array) => WasmVerifierLike;
}

export interface WasmVerifierLike {
  verify(
    proofBytes: Uint8Array,
    publicInputs: string[]
  ): boolean | Promise<boolean>;
}

export interface LoadVerifierOptions {
  /**
   * Directory containing the packaged circuit bundle (keys/, prover-node/, etc.).
   */
  bundlePath?: string;
  /**
   * Base name of the circuit artifacts (defaults to the final segment of `bundlePath`).
   */
  artifactName?: string;
  /**
   * Pre-loaded verification key bytes. If omitted, `verificationKeyPath` must be provided.
   */
  verificationKey?: Uint8Array;
  /**
   * Path to the verification key produced by `lof compile --target wasm`.
   */
  verificationKeyPath?: string;
  /**
   * Custom file reading implementation (defaults to `fs.promises.readFile`).
   */
  fs?: Pick<typeof defaultFs, 'readFile'>;

  /**
   * Provide an already-imported verifier module (from wasm-pack).
   */
  verifierModule?: VerifierModuleExports;
  /**
   * Lazy loader for the verifier module.
   */
  loadVerifierModule?: () => Promise<VerifierModuleExports>;
  /**
   * URL used by the default dynamic import helper when no loader/module is supplied.
   */
  verifierModuleUrl?: string;
  /**
   * Optional argument forwarded to the module's default initializer (useful when the wasm pack build expects a path).
   */
  verifierModuleInitArg?: unknown;
}

export interface VerifierHandle {
  module: VerifierModuleExports;
  instance: WasmVerifierLike;
}

export async function loadVerifier(
  options: LoadVerifierOptions
): Promise<VerifierHandle> {
  const normalizedBundle = normalizeBundlePath(options.bundlePath);
  const artifactName = resolveArtifactName(options, normalizedBundle);

  const verificationKeyPath = options.verificationKeyPath ?? deriveVerificationKeyPath(normalizedBundle, artifactName);
  const verifierModuleUrl = options.verifierModuleUrl ?? deriveVerifierModuleUrl(normalizedBundle);

  const resolvedOptions: LoadVerifierOptions = {
    ...options,
    verificationKeyPath,
    verifierModuleUrl,
  };

  const module = await resolveVerifierModule(resolvedOptions);

  // Load WASM file manually for Node.js (fetch doesn't work with file:// URLs)
  const wasmPath = deriveWasmPath(verifierModuleUrl);
  if (wasmPath && typeof module.default === 'function') {
    const reader = resolvedOptions.fs?.readFile ?? defaultFs.readFile;
    const wasmBytes = await reader(wasmPath);
    await module.default(wasmBytes);
  } else if (typeof module.default === 'function') {
    await module.default(resolvedOptions.verifierModuleInitArg);
  }

  if (typeof module.init_panic_hook === 'function') {
    module.init_panic_hook();
  }

  const verificationKey = await resolveVerificationKey(resolvedOptions);
  const instance = new module.WasmVerifier(verificationKey);

  return { module, instance };
}

export async function verifyProof(
  verifier: VerifierHandle | WasmVerifierLike,
  proofBytes: Uint8Array | ArrayBuffer | ArrayBufferView | Iterable<number>,
  publicInputs: Iterable<InputValue> | Record<string, InputValue>
): Promise<boolean> {
  const instance = isVerifierHandle(verifier) ? verifier.instance : verifier;
  const proof = coerceUint8Array(proofBytes);
  const inputs = toStringArray(publicInputs);
  return Promise.resolve(instance.verify(proof, inputs));
}

function isVerifierHandle(value: unknown): value is VerifierHandle {
  return (
    typeof value === 'object' &&
    value !== null &&
    'instance' in value &&
    typeof (value as VerifierHandle).instance.verify === 'function'
  );
}

async function resolveVerificationKey(
  options: LoadVerifierOptions
): Promise<Uint8Array> {
  if (options.verificationKey && options.verificationKey.length > 0) {
    return options.verificationKey;
  }

  if (!options.verificationKeyPath) {
    throw new Error(
      'loadVerifier: verification key missing – provide either `verificationKey` or `verificationKeyPath`'
    );
  }

  const reader = options.fs?.readFile ?? defaultFs.readFile;
  const data = await reader(options.verificationKeyPath);
  if (typeof data === 'string') {
    return Uint8Array.from(Buffer.from(data, 'utf8'));
  }
  if (Buffer.isBuffer(data)) {
    return new Uint8Array(data);
  }
  if (ArrayBuffer.isView(data)) {
    const view = data as ArrayBufferView;
    return new Uint8Array(view.buffer.slice(view.byteOffset, view.byteOffset + view.byteLength));
  }

  throw new Error(
    'loadVerifier: unexpected verification key type returned from readFile()'
  );
}

async function resolveVerifierModule(
  options: LoadVerifierOptions
): Promise<VerifierModuleExports> {
  if (options.verifierModule) {
    return options.verifierModule;
  }
  if (options.loadVerifierModule) {
    return options.loadVerifierModule();
  }
  if (options.verifierModuleUrl) {
    return dynamicImport<VerifierModuleExports>(options.verifierModuleUrl);
  }

  throw new Error(
    'loadVerifier: verifier module not provided – supply `verifierModule`, `loadVerifierModule`, or `verifierModuleUrl`'
  );
}

async function dynamicImport<T>(url: string): Promise<T> {
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  return import(/* webpackIgnore: true */ /* @vite-ignore */ url) as Promise<T>;
}

function coerceUint8Array(
  value:
    | Uint8Array
    | ArrayBuffer
    | ArrayBufferView
    | Iterable<number>
): Uint8Array {
  if (value instanceof Uint8Array) {
    return value;
  }
  if (value instanceof ArrayBuffer) {
    return new Uint8Array(value);
  }
  if (ArrayBuffer.isView(value)) {
    const view = value as ArrayBufferView;
    return new Uint8Array(view.buffer.slice(view.byteOffset, view.byteOffset + view.byteLength));
  }
  if (isIterable(value)) {
    return Uint8Array.from(value);
  }
  throw new Error('verifyProof: proof bytes must be Uint8Array-compatible');
}

function isIterable<T>(value: unknown): value is Iterable<T> {
  return typeof value === 'object' && value !== null && Symbol.iterator in value;
}

function toStringArray(
  input: Iterable<InputValue> | Record<string, InputValue>
): string[] {
  if (isIterable<InputValue>(input)) {
    const result: string[] = [];
    for (const item of input) {
      result.push(stringifyValue(item));
    }
    return result;
  }

  if (typeof input === 'object' && input !== null) {
    return Object.values(input).map(stringifyValue);
  }

  throw new Error(
    'verifyProof: expected public inputs as an iterable or object mapping signals to values'
  );
}

function stringifyValue(value: InputValue): string {
  if (typeof value === 'string') {
    return value;
  }
  if (typeof value === 'number' || typeof value === 'bigint') {
    return value.toString();
  }
  return String(value);
}

export type VerifierOptions = LoadVerifierOptions;

function normalizeBundlePath(bundlePath?: string): string | undefined {
  if (!bundlePath) {
    return undefined;
  }
  return path.resolve(bundlePath);
}

function resolveArtifactName(
  options: LoadVerifierOptions,
  bundlePath?: string
): string | undefined {
  if (options.artifactName && options.artifactName.length > 0) {
    return options.artifactName;
  }

  const fromKey = extractArtifactNameFromKeyPath(options.verificationKeyPath);
  if (fromKey) {
    return fromKey;
  }

  if (bundlePath) {
    const base = path.basename(bundlePath);
    if (base) {
      return base;
    }
  }

  return undefined;
}

function extractArtifactNameFromKeyPath(keyPath?: string): string | undefined {
  if (!keyPath) {
    return undefined;
  }
  const filename = path.basename(keyPath);
  const match = filename.match(/^(.*)_vk\.bin$/);
  return match?.[1];
}

function deriveVerificationKeyPath(
  bundlePath?: string,
  artifactName?: string
): string | undefined {
  if (!bundlePath || !artifactName) {
    return undefined;
  }
  return path.join(bundlePath, 'keys', `${artifactName}_vk.bin`);
}

function deriveVerifierModuleUrl(bundlePath?: string): string | undefined {
  if (!bundlePath) {
    return undefined;
  }
  return `file://${path.join(bundlePath, 'prover', 'lofit.js')}`;
}

function deriveWasmPath(moduleUrl?: string): string | undefined {
  if (!moduleUrl || !moduleUrl.startsWith('file://')) {
    return undefined;
  }
  // Convert file:///path/to/lofit.js -> /path/to/lofit_bg.wasm
  const jsPath = moduleUrl.replace('file://', '');
  return jsPath.replace(/\.js$/, '_bg.wasm');
}
