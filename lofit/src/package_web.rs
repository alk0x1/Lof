use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{ConstraintSystem, LofCircuit, ProverKey};
use ark_bn254::Fr;
use tracing::{error, info, warn};

pub fn package_for_web(
    r1cs_path: &Path,
    output_dir: Option<&Path>,
    skip_wasm: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    info!("Packaging circuit for web deployment...\n");

    let circuit_name = resolve_circuit_name(r1cs_path)?;
    let package_dir = prepare_package_dir(&circuit_name, output_dir);

    info!("Circuit: {}", circuit_name);
    info!("Package directory: {}\n", package_dir.display());

    create_directory_structure(&package_dir)?;

    info!("Step 1/5: Generating proving and verification keys...");
    generate_keys(r1cs_path, &package_dir, &circuit_name)?;
    info!("✅ Keys generated\n");

    info!("Step 2/5: Copying R1CS file...");
    copy_r1cs_to_build(r1cs_path, &package_dir, &circuit_name)?;
    info!("✅ R1CS copied\n");

    handle_wasm_steps(r1cs_path, &package_dir, &circuit_name, skip_wasm)?;

    info!("Step 5/5: Generating integration examples...");
    generate_integration_code(&package_dir, &circuit_name)?;
    info!("✅ Integration examples generated\n");

    if skip_wasm {
        info!("Web package ready! (WASM build deferred)");
    } else {
        info!("Web package ready!");
    }
    info!("\nPackage location: {}", package_dir.display());
    info!(
        "\nNext steps:\n  1. Copy the package to your web project\n  2. Serve the files with a web server\n  3. See integration.js for usage examples"
    );
    info!(
        "\nQuick check:\n  Review integration.js for wiring details\n  Serve the package directory with your preferred dev server"
    );

    Ok(package_dir)
}

fn create_directory_structure(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(base)?;
    fs::create_dir_all(base.join("build"))?;
    fs::create_dir_all(base.join("keys"))?;
    fs::create_dir_all(base.join("witness"))?;
    fs::create_dir_all(base.join("prover"))?;
    fs::create_dir_all(base.join("inputs"))?;
    fs::create_dir_all(base.join("proofs"))?;
    Ok(())
}

fn generate_keys(
    r1cs_path: &Path,
    package_dir: &Path,
    circuit_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let r1cs_file = File::open(r1cs_path)?;
    let r1cs = ConstraintSystem::from_file(r1cs_file)?;

    let circuit = LofCircuit {
        public_inputs: vec![Fr::from(0u64); r1cs.public_inputs.len()],
        witness: vec![Fr::from(0u64); 1],
        constraints: r1cs.constraints,
    };

    let (pk, vk) = ProverKey::setup(circuit)?;

    let keys_dir = package_dir.join("keys");
    let pk_path = keys_dir.join(format!("{}_pk.bin", circuit_name));
    let vk_path = keys_dir.join(format!("{}_vk.bin", circuit_name));

    let pk_writer = BufWriter::new(File::create(&pk_path)?);
    pk.write(pk_writer)?;

    let vk_writer = BufWriter::new(File::create(&vk_path)?);
    vk.write(vk_writer)?;

    info!("  Proving key: {}", pk_path.display());
    info!("  Verification key: {}", vk_path.display());

    Ok(())
}

fn generate_witness_wasm(
    r1cs_path: &Path,
    package_dir: &Path,
    circuit_name: &str,
    skip_wasm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ir_path = ensure_ir_exists(r1cs_path)?;

    let witness_output_dir = package_dir.join("witness_temp");
    fs::create_dir_all(&witness_output_dir)?;

    let lof_witness_gen = locate_lof_witness_gen()?;
    info!("  Using lof-witness-gen: {}", lof_witness_gen.display());

    run_witness_generator(&lof_witness_gen, &ir_path, &witness_output_dir)?;
    info!("  Generated witness calculator source");

    let wasm_project_dir = witness_output_dir.join(format!("{}_witness_wasm", circuit_name));

    if skip_wasm {
        finalize_skip_wasm_sources(
            package_dir,
            circuit_name,
            &wasm_project_dir,
            &witness_output_dir,
        )?;
        return Ok(());
    }

    ensure_wasm_project_exists(&wasm_project_dir)?;
    build_witness_wasm_project(&wasm_project_dir)?;
    copy_witness_artifacts(&wasm_project_dir, package_dir)?;
    fs::remove_dir_all(&witness_output_dir)?;

    info!("  Witness WASM: {}", package_dir.join("witness").display());

    Ok(())
}

fn build_lofit_wasm(package_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    match try_build_lofit_wasm(package_dir) {
        Ok(_) => Ok(()),
        Err(build_err) => {
            warn!("  wasm-pack build for lofit failed: {}", build_err);
            warn!("  Falling back to prebuilt prover bundle shipped with lofit");
            match copy_prebuilt_lofit_wasm(package_dir) {
                Ok(_) => {
                    info!("  Using prebuilt prover bundle from lofit/pkg");
                    Ok(())
                }
                Err(fallback_err) => {
                    error!("  Failed to copy prebuilt prover bundle: {}", fallback_err);
                    Err(build_err)
                }
            }
        }
    }
}

fn try_build_lofit_wasm(package_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(path) = std::env::var("LOFIT_SOURCE_DIR") {
        candidates.push(PathBuf::from(path));
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(root) = current_exe
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists())
        {
            candidates.push(root.join("lofit"));
        }
    }

    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let maybe_source = candidates
        .iter()
        .find(|dir| dir.join("Cargo.toml").exists())
        .cloned();

    let lofit_dir = if let Some(dir) = maybe_source {
        dir
    } else {
        info!("  Unable to locate lofit source automatically, attempting current directory...");
        return build_lofit_wasm_from_cwd(package_dir);
    };

    info!("  Building lofit WASM from: {}", lofit_dir.display());

    let prover_out_dir = package_dir.join("prover");
    fs::create_dir_all(&prover_out_dir)?;
    let prover_out_abs = prover_out_dir
        .canonicalize()
        .unwrap_or_else(|_| prover_out_dir.clone());

    let output = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(&prover_out_abs)
        .current_dir(&lofit_dir)
        .output()?;

    if !output.status.success() {
        error!("Failed to build lofit WASM:");
        error!("{}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to build lofit WASM".into());
    }

    info!("  Lofit WASM: {}/prover", package_dir.display());

    Ok(())
}

fn write_prover_skip_instructions(package_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let prover_dir = package_dir.join("prover");
    fs::create_dir_all(&prover_dir)?;

    let instructions = format!(
        "WASM build skipped.\n\nTo build the prover module manually:\n  1. Ensure the lofit source is available (e.g. export LOFIT_SOURCE_DIR=/path/to/lofit).\n  2. From the lofit crate directory, run:\n       wasm-pack build --target web --out-dir {}\n",
        prover_dir.display()
    );
    fs::write(prover_dir.join("README.txt"), instructions)?;
    Ok(())
}

fn read_pkg_asset(name: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let asset_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("pkg")
        .join(name);

    fs::read(&asset_path).map_err(|err| {
        let context = format!(
            "Failed to read fallback prover asset '{}': {}. \
Run `wasm-pack build --target web --out-dir pkg` inside the lofit crate to generate it.",
            asset_path.display(),
            err
        );
        std::io::Error::new(err.kind(), context).into()
    })
}

fn copy_prebuilt_lofit_wasm(package_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dest_dir = package_dir.join("prover");
    fs::create_dir_all(&dest_dir)?;

    fs::write(dest_dir.join("lofit.js"), read_pkg_asset("lofit.js")?)?;
    fs::write(dest_dir.join("lofit.d.ts"), read_pkg_asset("lofit.d.ts")?)?;
    fs::write(
        dest_dir.join("lofit_bg.wasm.d.ts"),
        read_pkg_asset("lofit_bg.wasm.d.ts")?,
    )?;
    fs::write(
        dest_dir.join("lofit_bg.wasm"),
        read_pkg_asset("lofit_bg.wasm")?,
    )?;

    Ok(())
}

fn build_lofit_wasm_from_cwd(package_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new("Cargo.toml").exists() {
        error!("Cannot find lofit to build WASM");
        error!("Please run this command from the Lof project root");
        return Err("Lofit not found".into());
    }

    let output = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(package_dir.join("prover"))
        .output()?;

    if !output.status.success() {
        error!("Failed to build lofit WASM:");
        error!("{}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to build lofit WASM".into());
    }

    Ok(())
}

fn generate_integration_code(
    package_dir: &Path,
    circuit_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let r1cs_path = package_dir
        .join("build")
        .join(format!("{}.r1cs", circuit_name));
    let r1cs_file = File::open(&r1cs_path)?;
    let r1cs = ConstraintSystem::from_file(r1cs_file)?;

    let public_inputs_json = serde_json::to_string(&r1cs.public_inputs)?;
    let witness_inputs_json = serde_json::to_string(&r1cs.witnesses)?;

    let integration_template = r#"// Integration Example for __CIRCUIT_NAME__
// Auto-generated helper that wires the witness calculator and prover WASM modules together.

import initWitness, { compute_witness } from './witness/__CIRCUIT_NAME___witness_wasm.js';
import initLofit, { WasmProver, init_panic_hook } from './prover/lofit.js';

// Global state
let witnessReady = false;
let proverReady = false;
let wasmProver = null;

export const PUBLIC_INPUT_SIGNALS = __PUBLIC_INPUTS__;
export const WITNESS_SIGNALS = __WITNESS_INPUTS__;

// Initialize all WASM modules
async function initializeWasm() {
    console.log('Loading WASM modules...');

    try {
        // Initialize witness calculator
        await initWitness();
        witnessReady = true;
        console.log('✅ Witness calculator ready');

        // Initialize lofit prover
        await initLofit();
        init_panic_hook();
        console.log('✅ Lofit prover ready');

        // Load R1CS and proving key
        const r1csResp = await fetch('./build/__CIRCUIT_NAME__.r1cs');
        const r1csBytes = new Uint8Array(await r1csResp.arrayBuffer());

        const pkResp = await fetch('./keys/__CIRCUIT_NAME___pk.bin');
        const pkBytes = new Uint8Array(await pkResp.arrayBuffer());

        // Create prover instance
        wasmProver = new WasmProver(r1csBytes, pkBytes);
        proverReady = true;
        console.log('✅ Prover initialized');

        return true;
    } catch (error) {
        console.error('Failed to initialize WASM:', error);
        return false;
    }
}

// Generate a zero-knowledge proof
async function generateProof(inputs) {
    if (!witnessReady || !proverReady) {
        throw new Error('WASM not initialized. Call initializeWasm() first.');
    }

    console.log('Computing witness...');
    const witness = compute_witness(inputs);
    console.log('✅ Witness computed:', witness);

    const publicInputs = buildPublicInputs(witness);
    const witnessArray = buildWitnessArray(witness);

    console.log('Generating proof...');
    const proofBytes = await wasmProver.prove(witnessArray);
    console.log('✅ Proof generated:', proofBytes.length, 'bytes');

    return {
        proofBytes,
        witness,
        witnessArray,
        publicInputs,
    };
}

// Send proof to server for verification
async function verifyProof(proofBytes, publicInputs) {
    const response = await fetch('/api/verify', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            proof_bytes: Array.from(proofBytes),
            public_inputs: publicInputs,
        }),
    });

    const result = await response.json();
    return result.verified;
}

function buildWitnessArray(witnessOutput) {
    const orderedValues = [];

    for (const name of PUBLIC_INPUT_SIGNALS) {
        if (!(name in witnessOutput)) {
            throw new Error(`Missing public input '${name}' in witness output`);
        }
        orderedValues.push(witnessOutput[name].toString());
    }

    for (const name of WITNESS_SIGNALS) {
        if (!(name in witnessOutput)) {
            throw new Error(`Missing witness signal '${name}' in witness output`);
        }
        orderedValues.push(witnessOutput[name].toString());
    }

    return orderedValues;
}

function buildPublicInputs(witnessOutput) {
    const pubInputs = {};
    for (const name of PUBLIC_INPUT_SIGNALS) {
        pubInputs[name] = witnessOutput[name].toString();
    }
    return pubInputs;
}

// Export for use in other modules
export { initializeWasm, generateProof, verifyProof, buildWitnessArray, buildPublicInputs };
"#;

    let integration_code = integration_template
        .replace("__CIRCUIT_NAME__", circuit_name)
        .replace("__PUBLIC_INPUTS__", &public_inputs_json)
        .replace("__WITNESS_INPUTS__", &witness_inputs_json);

    fs::write(package_dir.join("integration.js"), integration_code)?;

    let readme_template = r#"# __CIRCUIT_NAME__ Web Package

This package contains everything needed to generate and verify zero-knowledge proofs for the `__CIRCUIT_NAME__` circuit in a web browser.

## Package Contents

```
__CIRCUIT_NAME___web/
├── build/
│   └── __CIRCUIT_NAME__.r1cs              # Compiled circuit constraints
├── keys/
│   ├── __CIRCUIT_NAME___pk.bin            # Proving key
│   └── __CIRCUIT_NAME___vk.bin            # Verification key
├── witness/
│   ├── __CIRCUIT_NAME___witness_wasm.js   # Witness calculator (JS bindings)
│   └── __CIRCUIT_NAME___witness_wasm_bg.wasm  # Witness calculator (WASM binary)
├── prover/
│   ├── lofit.js             # zkSNARK prover (JS bindings)
│   └── lofit_bg.wasm        # zkSNARK prover (WASM binary)
├── integration.js           # Example integration code (ES module)
└── README.md                # This file
```

## Quick Start

Serve this directory with your preferred dev server and import the generated helpers:

```js
import { initializeWasm, generateProof } from './integration.js';

await initializeWasm({
  r1csUrl: './build/__CIRCUIT_NAME__.r1cs',
  provingKeyUrl: './keys/__CIRCUIT_NAME___pk.bin',
  witnessModuleUrl: './witness/__CIRCUIT_NAME___witness_wasm.js',
});

const { proofBytes } = await generateProof({ /* your inputs */ });
```

## API Reference

### initializeWasm()

Initializes the witness calculator and prover WASM modules.

```js
await initializeWasm();
```

### generateProof(inputs)

Computes the witness, packs it into the expected order, and returns proof bytes plus helper metadata.

```js
const {
  proofBytes,
  witness,
  witnessArray,
  publicInputs,
} = await generateProof({ field_a: '1', private_secret: '42' });
```

### verifyProof(proofBytes, publicInputs)

POST helper for sending proofs to a server endpoint.

```js
await verifyProof(proofBytes, publicInputs);
```

## Server-Side Verification

Use the `lofit verify` command to verify proofs on the server:

```bash
cargo run --bin lofit -- verify \
    --input build/__CIRCUIT_NAME__.r1cs \
    --verification-key keys/__CIRCUIT_NAME___vk.bin \
    --proof proofs/__CIRCUIT_NAME___proof.bin \
    --public-inputs inputs/__CIRCUIT_NAME___public.json
```

## Privacy Notes

- **Witness values** (private inputs) are computed in the browser
- **Only the proof** is sent to the server for verification
"#;

    let readme = readme_template.replace("__CIRCUIT_NAME__", circuit_name);
    fs::write(package_dir.join("README.md"), readme)?;

    info!("  integration.js: Example integration code");
    info!("  README.md: Usage documentation");

    Ok(())
}
fn resolve_circuit_name(r1cs_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    r1cs_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid R1CS filename".into())
}

fn prepare_package_dir(circuit_name: &str, output_dir: Option<&Path>) -> PathBuf {
    output_dir
        .map(|dir| dir.to_path_buf())
        .unwrap_or_else(|| Path::new(".").join(format!("{}_web", circuit_name)))
}

fn copy_r1cs_to_build(
    r1cs_path: &Path,
    package_dir: &Path,
    circuit_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let build_dir = package_dir.join("build");
    fs::copy(r1cs_path, build_dir.join(format!("{}.r1cs", circuit_name)))?;
    Ok(())
}

fn handle_wasm_steps(
    r1cs_path: &Path,
    package_dir: &Path,
    circuit_name: &str,
    skip_wasm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if skip_wasm {
        info!("Step 3/5: Skipping witness calculator WASM build (sources will be generated)");
        generate_witness_wasm(r1cs_path, package_dir, circuit_name, true)?;
        info!("Witness calculator sources generated (build later with wasm-pack)\n");

        info!("Step 4/5: Skipping lofit WASM prover build");
        write_prover_skip_instructions(package_dir)?;
        info!("Added instructions for building prover WASM later\n");
    } else {
        info!("Step 3/5: Generating witness calculator WASM...");
        generate_witness_wasm(r1cs_path, package_dir, circuit_name, false)?;
        info!("Witness calculator WASM generated\n");

        info!("Step 4/5: Building lofit WASM prover...");
        build_lofit_wasm(package_dir)?;
        info!("Lofit WASM prover ready\n");
    }

    Ok(())
}
fn ensure_ir_exists(r1cs_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let ir_path = r1cs_path.with_extension("ir");
    if !ir_path.exists() {
        error!("IR file not found: {}", ir_path.display());
        error!("Make sure you run 'lof compile' which generates both .r1cs and .ir files");
        Err("IR file not found".into())
    } else {
        Ok(ir_path)
    }
}

fn locate_lof_witness_gen() -> Result<PathBuf, Box<dyn std::error::Error>> {
    which::which("lof-witness-gen")
        .or_else(|_| -> Result<PathBuf, which::Error> {
            let current_exe =
                std::env::current_exe().map_err(|_| which::Error::CannotFindBinaryPath)?;
            let target_dir = current_exe
                .ancestors()
                .find(|p| p.ends_with("target"))
                .ok_or(which::Error::CannotFindBinaryPath)?;

            let debug_path = target_dir.join("debug").join("lof-witness-gen");
            let release_path = target_dir.join("release").join("lof-witness-gen");

            if release_path.exists() {
                Ok(release_path)
            } else if debug_path.exists() {
                Ok(debug_path)
            } else {
                Err(which::Error::CannotFindBinaryPath)
            }
        })
        .map_err(|_| {
            error!("Failed to find lof-witness-gen");
            error!("Make sure lof-witness-gen is installed or built");
            error!("Run: cargo build --bin lof-witness-gen --release");
            "lof-witness-gen not found".into()
        })
}

fn run_witness_generator(
    binary: &Path,
    ir_path: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(binary).arg(ir_path).arg(output_dir).output()?;

    if output.status.success() {
        Ok(())
    } else {
        error!("lof-witness-gen failed:");
        error!("{}", String::from_utf8_lossy(&output.stderr));
        Err("Failed to generate witness calculator".into())
    }
}

fn finalize_skip_wasm_sources(
    package_dir: &Path,
    circuit_name: &str,
    wasm_project_dir: &Path,
    witness_output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let sources_dir = package_dir.join("witness_sources");
    if sources_dir.exists() {
        fs::remove_dir_all(&sources_dir)?;
    }
    fs::create_dir_all(&sources_dir)?;
    let target_dir = sources_dir.join(format!("{}_witness_wasm", circuit_name));
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::rename(wasm_project_dir, &target_dir)?;
    fs::remove_dir_all(witness_output_dir)?;

    let notes_path = package_dir.join("witness").join("README.txt");
    if let Some(parent) = notes_path.parent() {
        fs::create_dir_all(parent)?;
    } else {
        return Err("Unable to determine destination for witness README".into());
    }
    let notes = format!(
        "WASM build skipped.\n\nTo build the witness calculator manually:\n  cd ../witness_sources/{}_witness_wasm\n  wasm-pack build --target web\n  cp pkg/* ../witness/\n",
        circuit_name
    );
    fs::write(&notes_path, notes)?;
    Ok(())
}

fn ensure_wasm_project_exists(wasm_project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if wasm_project_dir.exists() {
        Ok(())
    } else {
        error!(
            "Witness WASM project not found at: {}",
            wasm_project_dir.display()
        );
        Err("Witness WASM project not created".into())
    }
}

fn build_witness_wasm_project(wasm_project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    info!("  Building WASM with wasm-pack...");
    let output = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .current_dir(wasm_project_dir)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            error!("Failed to run wasm-pack: {}", e);
            error!("Install wasm-pack: https://rustwasm.github.io/wasm-pack/installer/");
            return Err(e.into());
        }
    };

    if output.status.success() {
        Ok(())
    } else {
        error!("wasm-pack build failed:");
        error!("{}", String::from_utf8_lossy(&output.stderr));
        Err("Failed to build witness WASM".into())
    }
}

fn copy_witness_artifacts(
    wasm_project_dir: &Path,
    package_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let pkg_dir = wasm_project_dir.join("pkg");
    let target_dir = package_dir.join("witness");

    for entry in fs::read_dir(&pkg_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "js" || ext == "wasm" || ext == "d.ts" {
                let file_name = path.file_name().ok_or("Invalid filename")?;
                fs::copy(&path, target_dir.join(file_name))?;
            }
        }
    }

    Ok(())
}
