# @lof/toolkit-node

Node-focused helpers for loading verification keys and checking proofs using the wasm-bindgen bindings generated from the `lofit` crate.

The package wraps the lower-level WASM exports with a familiar, promise-based API. Pair it with the assets emitted by `lof compile … --target wasm` and the Node-targeted `wasm-pack` build of the prover/verifier.

## Installation

```bash
npm install @lof/toolkit-node
```

## Usage

### Simple (auto-discovery)

The easiest way is to specify `bundlePath`, which automatically discovers the verification key and verifier module:

```ts
import { loadVerifier, verifyProof } from '@lof-lang/toolkit-node';
import path from 'path';

const verifier = await loadVerifier({
  bundlePath: path.resolve(__dirname, './dist/multiply/web/multiply')
});

const ok = await verifyProof(verifier, proofBytes, publicInputs);
```

With `bundlePath`, the toolkit automatically discovers:
- `${bundlePath}/keys/${circuitName}_vk.bin`
- `${bundlePath}/prover-node/lofit.js`

The circuit name is inferred from the basename of `bundlePath` (e.g., `dist/multiply/web/multiply` → `multiply`).

### Explicit paths

For full control, you can specify each path individually:

```ts
import { pathToFileURL } from 'url';

const verifier = await loadVerifier({
  verificationKeyPath: 'web/circuit/keys/circuit_vk.bin',
  verifierModuleUrl: pathToFileURL('./prover-node/lofit.js').href
});

const ok = await verifyProof(verifier, proofBytes, publicSignals);
```

**Notes:**
- `proofBytes` can be a `Uint8Array`, Node `Buffer`, or array-like of numbers
- `publicInputs` can be:
  - An **object**: `{ a: '5', b: '7' }` (field values as strings)
  - An **array**: `['5', '7']` (ordered field elements as strings)
  - An **iterable**: Any iterable of field element strings

## API

### `loadVerifier(options)`

Loads the WASM verifier module, reads the verification key, and returns a handle containing the instantiated `WasmVerifier`.

**Options:**
- `bundlePath?: string` – Directory containing the circuit bundle (enables auto-discovery)
- `artifactName?: string` – Circuit name (defaults to basename of `bundlePath`)
- `verificationKeyPath?: string` – Explicit path to verification key file
- `verifierModuleUrl?: string` – Explicit file:// URL to verifier module (use `pathToFileURL`)
- `verificationKey?: Uint8Array` – Pre-loaded verification key bytes
- `verifierModule?: VerifierModule` – Pre-imported verifier module
- `loadVerifierModule?: () => Promise<VerifierModule>` – Custom module loader
- `fs?: { readFile }` – Custom file reader (for testing)

**Returns:**
A `VerifierHandle` object containing the instantiated verifier.

### `verifyProof(verifier, proofBytes, publicInputs)`

Verifies a Groth16 proof against public inputs.

**Parameters:**
- `verifier` – The verifier handle from `loadVerifier()`
- `proofBytes` – Proof as `Uint8Array`, `Buffer`, or array of numbers
- `publicInputs` – Public inputs as:
  - Object: `{ signal1: 'value1', signal2: 'value2' }`
  - Array: `['value1', 'value2']`
  - Iterable: Any iterable of field element strings

**Returns:**
`Promise<boolean>` – `true` if the proof is valid, `false` otherwise.
