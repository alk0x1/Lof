# Lof

Lof is a domain-specific language and end-to-end toolkit for authoring zero-knowledge circuits and producing Groth16 proofs over BN254. It couples a strong, security-focused type system with a Rust compiler for the `.lof` language, a proving and verification CLI, witness generator utilities, and JavaScript helpers for browser and Node runtimes.

## Features
- Ergonomic `.lof` language with parser, type checker, and IR/R1CS back ends
- `lof` CLI to check, compile, and package circuits (including web-ready bundles) with rich diagnostics
- `lofit` CLI for Groth16 setup/proving/verification plus asset packaging
- Built-in WASM packaging for prover bundles (`lof compile --target wasm`, `lofit package-web`)
- Witness calculator generation via `lof-witness-gen` (Rust and WASM targets)
- Optional TypeScript toolkits for loading circuits in Node (`@lof/toolkit-node`) and browsers (`@lof-lang/toolkit-browser`)

## Workspace Layout
- `lof/` – compiler front-end and CLI for the Lof language
- `lofit/` – proving, verification, and Web/WASM packaging tooling
- `lof-witness-gen/` – IR-driven witness calculator generator
- `packages/` – JavaScript toolkits for Node and browser integrations
- `tests/` – integration fixtures plus shell runners for parser/typechecker/compiler suites

## Getting Started

### Prerequisites
- Rust toolchain (stable, via [`rustup`](https://rustup.rs/))
- Optional: [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) for building web prover bundles
- Optional: Node.js ≥ 18 if you plan to consume the JS toolkits

### Build or Install the CLIs
```bash
cargo build --all
# or install the binaries locally
cargo install --path lof
cargo install --path lofit
cargo install --path lof-witness-gen
# make install (release build + version check) is also available
make install
```

### A Minimal `.lof` Circuit
```lof
proof Multiply {
    input a: field;
    input b: field;
    let c = a * b in
    assert c === a * b;
    c
}
```

## Typical Workflow
1. **Type-check** your source: `lof check path/to/circuit.lof --verbose`
2. **Compile** to R1CS and IR: `lof compile path/to/circuit.lof --generate-templates`
   - Produces `build/`, `inputs/`, `keys/`, and `proofs/` directories alongside your source
   - JSON templates for public inputs and witness assignments land in `inputs/`
3. **Generate keys** (Groth16): `lofit setup --input build/circuit.r1cs`
4. **Create a proof**: `lofit prove --input build/circuit.r1cs --public-inputs inputs/circuit_public.json --witness inputs/circuit_witness.json`
5. **Verify** the proof: `lofit verify --verification-key keys/circuit_vk.bin --proof proofs/circuit_proof.bin --public-inputs inputs/circuit_public.json`

The `lof compile` command requires a `.lof` extension and can be re-run safely; artifacts in the source directory are refreshed each time.

### Web / WASM Packaging
- Quick bundle generation: `lof compile circuit.lof --target wasm --output dist/circuit`
- Standalone packaging via `lofit`: `lofit package-web --input build/circuit.r1cs --output dist/circuit --skip-wasm`

Both paths create a directory containing R1CS, Groth16 keys, witness calculator sources, and the prover WASM bundle. When `wasm-pack` is installed the prover bindings are rebuilt; otherwise the tool falls back to the prebuilt artifacts shipped with `lofit`.

### Node and Browser Toolkits
- `packages/toolkit-node` exposes helpers for loading verification keys and verifying proofs in Node environments
- `packages/toolkit-browser` wraps the web assets emitted by `lof compile --target wasm` with ergonomic TypeScript APIs

Consult each package's README for API details and usage examples.

## Running Tests
- `cargo test --all` runs Rust unit tests across the workspace
- `make test-fast` executes formatting, linting, and unit suites
- `make test-integration` (or the individual scripts under `tests/scripts/`) runs parser, typechecker, and compiler integration checks against the `.lof` fixtures

## Development Tips
- `cargo fmt --all` and `cargo clippy --all-targets --all-features` keep the codebase tidy
- Use `make help` to list the most common development and verification commands
- Verbose compiler output (`--verbose`) is useful when diagnosing parser/typechecker issues

## License

Licensed under either of
- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT License (`LICENSE-MIT`)

at your option.
