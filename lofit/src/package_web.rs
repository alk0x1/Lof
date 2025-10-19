/// Package Web - Automates packaging of Lof circuits for web deployment
///
/// This module handles the entire workflow of preparing a circuit for browser-based proving:
/// 1. Generate proving/verification keys
/// 2. Generate witness calculator WASM
/// 3. Build lofit WASM for proving
/// 4. Create directory structure
/// 5. Generate integration example code

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use std::process::Command;

use crate::{ConstraintSystem, LofCircuit, ProverKey};
use ark_bn254::Fr;
use tracing::{error, info, warn};

pub fn package_for_web(
    r1cs_path: &Path,
    output_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🚀 Packaging circuit for web deployment...");
    info!("");

    // Extract circuit name
    let circuit_name = r1cs_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid R1CS filename")?;

    // Determine output directory
    let output_base = output_dir.unwrap_or_else(|| Path::new("."));
    let package_dir = output_base.join(format!("{}_web", circuit_name));

    info!("Circuit: {}", circuit_name);
    info!("Package directory: {}", package_dir.display());
    info!("");

    // Create directory structure
    create_directory_structure(&package_dir)?;

    // Step 1: Generate keys
    info!("Step 1/5: Generating proving and verification keys...");
    generate_keys(r1cs_path, &package_dir, circuit_name)?;
    info!("✅ Keys generated");
    info!("");

    // Step 2: Copy R1CS to build directory
    info!("Step 2/5: Copying R1CS file...");
    let build_dir = package_dir.join("build");
    fs::copy(r1cs_path, build_dir.join(format!("{}.r1cs", circuit_name)))?;
    info!("✅ R1CS copied");
    info!("");

    // Step 3: Generate witness calculator WASM
    info!("Step 3/5: Generating witness calculator WASM...");
    generate_witness_wasm(r1cs_path, &package_dir, circuit_name)?;
    info!("✅ Witness calculator WASM generated");
    info!("");

    // Step 4: Build/copy lofit WASM
    info!("Step 4/5: Building lofit WASM prover...");
    build_lofit_wasm(&package_dir)?;
    info!("✅ Lofit WASM prover ready");
    info!("");

    // Step 5: Generate integration examples
    info!("Step 5/5: Generating integration examples...");
    generate_integration_code(&package_dir, circuit_name)?;
    info!("✅ Integration examples generated");
    info!("");

    // Success!
    info!("🎉 Web package ready!");
    info!("");
    info!("Package location: {}", package_dir.display());
    info!("");
    info!("Next steps:");
    info!("  1. Copy the package to your web project");
    info!("  2. Serve the files with a web server");
    info!("  3. See integration.js for usage examples");
    info!("");
    info!("Quick test:");
    info!("  cd {}", package_dir.display());
    info!("  python3 -m http.server 8000");
    info!("  # Open http://localhost:8000 in your browser");

    Ok(())
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
    // Read R1CS
    let r1cs_file = File::open(r1cs_path)?;
    let r1cs = ConstraintSystem::from_file(r1cs_file)?;

    // Create dummy circuit for setup
    let circuit = LofCircuit {
        public_inputs: vec![Fr::from(0u64); r1cs.public_inputs.len()],
        witness: vec![Fr::from(0u64); 1],
        constraints: r1cs.constraints,
    };

    // Generate keys
    let (pk, vk) = ProverKey::setup(circuit)?;

    // Write keys
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
) -> Result<(), Box<dyn std::error::Error>> {
    // Find the .ir file (should be next to .r1cs)
    let ir_path = r1cs_path.with_extension("ir");

    if !ir_path.exists() {
        error!("IR file not found: {}", ir_path.display());
        error!("Make sure you run 'lof compile' which generates both .r1cs and .ir files");
        return Err("IR file not found".into());
    }

    // Run lof-witness-gen
    let witness_output_dir = package_dir.join("witness_temp");
    fs::create_dir_all(&witness_output_dir)?;

    // Try to find lof-witness-gen (PATH or cargo target)
    let lof_witness_gen = which::which("lof-witness-gen")
        .or_else(|_| -> Result<std::path::PathBuf, which::Error> {
            // Try to find in cargo target directory
            let current_exe = std::env::current_exe()
                .map_err(|_| which::Error::CannotFindBinaryPath)?;
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
        .map_err(|_| -> Box<dyn std::error::Error> {
            error!("Failed to find lof-witness-gen");
            error!("Make sure lof-witness-gen is installed or built");
            error!("Run: cargo build --bin lof-witness-gen --release");
            "lof-witness-gen not found".into()
        })?;

    info!("  Using lof-witness-gen: {}", lof_witness_gen.display());

    let output = Command::new(&lof_witness_gen)
        .arg(&ir_path)
        .arg(&witness_output_dir)
        .output()?;

    if !output.status.success() {
        error!("lof-witness-gen failed:");
        error!("{}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to generate witness calculator".into());
    }

    info!("  Generated witness calculator source");

    // Build WASM using wasm-pack
    let wasm_project_dir = witness_output_dir.join(format!("{}_witness_wasm", circuit_name));

    if !wasm_project_dir.exists() {
        error!("Witness WASM project not found at: {}", wasm_project_dir.display());
        return Err("Witness WASM project not created".into());
    }

    info!("  Building WASM with wasm-pack...");
    let wasm_output = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .current_dir(&wasm_project_dir)
        .output();

    let wasm_output = match wasm_output {
        Ok(o) => o,
        Err(e) => {
            error!("Failed to run wasm-pack: {}", e);
            error!("Install wasm-pack: https://rustwasm.github.io/wasm-pack/installer/");
            return Err(e.into());
        }
    };

    if !wasm_output.status.success() {
        error!("wasm-pack build failed:");
        error!("{}", String::from_utf8_lossy(&wasm_output.stderr));
        return Err("Failed to build witness WASM".into());
    }

    // Copy WASM artifacts to package witness/ directory
    let pkg_dir = wasm_project_dir.join("pkg");
    let target_dir = package_dir.join("witness");

    // Copy the .js and .wasm files
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

    // Clean up temp directory
    fs::remove_dir_all(&witness_output_dir)?;

    info!("  Witness WASM: {}", target_dir.display());

    Ok(())
}

fn build_lofit_wasm(package_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Find lofit source directory (assuming we're running from project)
    let current_exe = std::env::current_exe()?;
    let project_root = current_exe
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists())
        .ok_or("Could not find project root")?;

    let lofit_dir = project_root.join("lofit");

    if !lofit_dir.exists() {
        warn!("Lofit directory not found at: {}", lofit_dir.display());
        warn!("Attempting to build from current directory...");

        // Try building from current directory
        return build_lofit_wasm_from_cwd(package_dir);
    }

    info!("  Building lofit WASM from: {}", lofit_dir.display());

    // Build lofit WASM
    let output = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(package_dir.join("prover"))
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

fn build_lofit_wasm_from_cwd(package_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we're in the lofit directory
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
    // Generate integration.js
    let integration_code = format!(r#"// Integration Example for {}
// This file shows how to use the generated WASM modules

import initWitness, {{ compute_witness }} from './witness/{}_witness_wasm.js';
import initLofit, {{ WasmProver, init_panic_hook }} from './prover/lofit.js';

// Global state
let witnessReady = false;
let proverReady = false;
let wasmProver = null;

// Initialize all WASM modules
async function initializeWasm() {{
    console.log('Loading WASM modules...');

    try {{
        // Initialize witness calculator
        await initWitness();
        witnessReady = true;
        console.log('✅ Witness calculator ready');

        // Initialize lofit prover
        await initLofit();
        init_panic_hook();
        console.log('✅ Lofit prover ready');

        // Load R1CS and proving key
        const r1csResp = await fetch('./build/{}.r1cs');
        const r1csBytes = new Uint8Array(await r1csResp.arrayBuffer());

        const pkResp = await fetch('./keys/{}_pk.bin');
        const pkBytes = new Uint8Array(await pkResp.arrayBuffer());

        // Create prover instance
        wasmProver = new WasmProver(r1csBytes, pkBytes);
        proverReady = true;
        console.log('✅ Prover initialized');

        return true;
    }} catch (error) {{
        console.error('Failed to initialize WASM:', error);
        return false;
    }}
}}

// Generate a zero-knowledge proof
async function generateProof(inputs) {{
    if (!witnessReady || !proverReady) {{
        throw new Error('WASM not initialized. Call initializeWasm() first.');
    }}

    console.log('Computing witness...');
    const witness = compute_witness(inputs);
    console.log('✅ Witness computed:', witness);

    // Convert witness to array format
    // NOTE: Order must match your circuit's signal order!
    // Adjust this based on your circuit structure
    const witnessArray = Object.values(witness).map(v => v.toString());

    console.log('Generating proof...');
    const proofBytes = await wasmProver.prove(witnessArray);
    console.log('✅ Proof generated:', proofBytes.length, 'bytes');

    return proofBytes;
}}

// Send proof to server for verification
async function verifyProof(proofBytes, publicInputs) {{
    const response = await fetch('/api/verify', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{
            proof_bytes: Array.from(proofBytes),
            public_inputs: publicInputs
        }})
    }});

    const result = await response.json();
    return result.verified;
}}

// Example usage
async function example() {{
    // Initialize
    await initializeWasm();

    // Your circuit inputs (adjust based on your circuit)
    const inputs = {{
        // Add your input fields here
        // Example: input_value: "42"
    }};

    // Generate proof
    const proof = await generateProof(inputs);

    // Verify proof (requires server endpoint)
    const publicInputs = {{
        // Add your public inputs here
    }};

    const isValid = await verifyProof(proof, publicInputs);
    console.log('Proof verified:', isValid);
}}

// Export for use in other modules
export {{ initializeWasm, generateProof, verifyProof }};
"#, circuit_name, circuit_name, circuit_name, circuit_name);

    fs::write(package_dir.join("integration.js"), integration_code)?;

    // Generate README
    let readme = format!(r#"# {} Web Package

This package contains everything needed to generate and verify zero-knowledge proofs for the `{}` circuit in a web browser.

## Package Contents

```
{}_web/
├── build/
│   └── {}.r1cs              # Compiled circuit constraints
├── keys/
│   ├── {}_pk.bin            # Proving key
│   └── {}_vk.bin            # Verification key
├── witness/
│   ├── {}_witness_wasm.js   # Witness calculator (JS bindings)
│   └── {}_witness_wasm_bg.wasm  # Witness calculator (WASM binary)
├── prover/
│   ├── lofit.js             # zkSNARK prover (JS bindings)
│   └── lofit_bg.wasm        # zkSNARK prover (WASM binary)
├── integration.js           # Example integration code
└── README.md                # This file
```

## Quick Start

### 1. Serve the files

```bash
# Python 3
python3 -m http.server 8000

# Node.js (with http-server)
npx http-server -p 8000

# Or use any web server
```

### 2. Use in your web application

```html
<!DOCTYPE html>
<html>
<head>
    <title>{} Proof</title>
</head>
<body>
    <h1>Zero-Knowledge Proof Demo</h1>
    <script type="module">
        import {{ initializeWasm, generateProof }} from './integration.js';

        async function main() {{
            // Initialize WASM modules
            await initializeWasm();

            // Your circuit inputs
            const inputs = {{
                // TODO: Add your input fields
            }};

            // Generate proof
            const proof = await generateProof(inputs);
            console.log('Proof generated!', proof);
        }}

        main();
    </script>
</body>
</html>
```

### 3. Customize integration.js

The `integration.js` file contains example code. You'll need to:

1. **Update witness array construction** - Match your circuit's signal order
2. **Add your input fields** - Based on your circuit's public inputs and witnesses
3. **Configure public inputs** - For proof verification

## API Reference

### initializeWasm()

Loads all WASM modules and initializes the prover.

```javascript
await initializeWasm();
```

### generateProof(inputs)

Generates a zero-knowledge proof from inputs.

```javascript
const inputs = {{
    input_field: "value",
    witness_field: "secret_value"
}};

const proofBytes = await generateProof(inputs);
```

### verifyProof(proofBytes, publicInputs)

Verifies a proof (requires server endpoint).

```javascript
const publicInputs = {{
    input_field: "value"
}};

const isValid = await verifyProof(proofBytes, publicInputs);
```

## Server-Side Verification

Use the `lofit verify` command to verify proofs on the server:

```bash
cargo run --bin lofit -- verify \
    -i {}.r1cs \
    -v keys/{}_vk.bin \
    -p proofs/proof.bin \
    -u inputs/public.json
```

## Privacy Notes

- **Witness values** (private inputs) are computed in the browser
- **Only the proof** is sent to the server for verification
- The server **never sees** your private witness values
- This achieves true zero-knowledge privacy

## Troubleshooting

### WASM modules not loading

- Ensure you're serving files over HTTP/HTTPS (not `file://`)
- Check browser console for CORS errors
- Verify all files are in the correct directories

### Proof generation fails

- Check that witness values are in the correct order
- Verify all required inputs are provided
- Enable verbose logging: check browser console

### Verification fails

- Ensure public inputs match between prover and verifier
- Check that you're using the correct verification key
- Verify the proof wasn't corrupted during transmission

## Next Steps

- Read the [Lof documentation](../../docs/) for circuit syntax
- See example circuits in `../../features/`
- Check the verification tests in `../../verification/`

## License

Same as the Lof project.
"#, circuit_name, circuit_name, circuit_name, circuit_name, circuit_name, circuit_name, circuit_name, circuit_name, circuit_name, circuit_name, circuit_name);

    fs::write(package_dir.join("README.md"), readme)?;

    info!("  integration.js: Example integration code");
    info!("  README.md: Usage documentation");

    Ok(())
}
