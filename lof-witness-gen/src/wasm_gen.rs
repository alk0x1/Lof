use lof::IRCircuit;
use std::fmt::Write;

pub fn generate_wasm_witness_calculator(
    circuit: &IRCircuit,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut code = String::new();

    writeln!(
        &mut code,
        "//! Generated WASM witness calculator for circuit: {}",
        circuit.name
    )?;
    writeln!(
        &mut code,
        "//! This code executes the circuit logic in the browser to compute witness values."
    )?;
    writeln!(&mut code)?;
    writeln!(&mut code, "use wasm_bindgen::prelude::*;")?;
    writeln!(&mut code, "use serde::{{Serialize, Deserialize}};")?;
    writeln!(&mut code, "use std::collections::HashMap;")?;
    writeln!(&mut code)?;

    writeln!(&mut code, "#[wasm_bindgen]")?;
    writeln!(&mut code, "extern \"C\" {{")?;
    writeln!(&mut code, "    #[wasm_bindgen(js_namespace = console)]")?;
    writeln!(&mut code, "    fn log(s: &str);")?;
    writeln!(&mut code, "}}")?;
    writeln!(&mut code)?;

    writeln!(&mut code, "#[derive(Serialize, Deserialize)]")?;
    writeln!(&mut code, "pub struct WitnessInputs {{")?;
    for (name, _) in &circuit.pub_inputs {
        writeln!(&mut code, "    pub {}: String,", name)?;
    }
    for (name, _) in &circuit.witnesses {
        writeln!(
            &mut code,
            "    #[serde(skip_serializing_if = \"Option::is_none\")]"
        )?;
        writeln!(&mut code, "    pub {}: Option<String>,", name)?;
    }
    writeln!(&mut code, "}}")?;
    writeln!(&mut code)?;

    writeln!(&mut code, "#[derive(Serialize, Deserialize)]")?;
    writeln!(&mut code, "pub struct WitnessOutput {{")?;
    for (name, _) in &circuit.pub_inputs {
        writeln!(&mut code, "    pub {}: String,", name)?;
    }
    for (name, _) in &circuit.witnesses {
        writeln!(&mut code, "    pub {}: String,", name)?;
    }
    writeln!(&mut code, "}}")?;
    writeln!(&mut code)?;

    writeln!(
        &mut code,
        "/// Compute witness from inputs (WASM entry point)"
    )?;
    writeln!(&mut code, "///")?;
    writeln!(
        &mut code,
        "/// This function is called from JavaScript with input values."
    )?;
    writeln!(
        &mut code,
        "/// It returns a complete witness including all intermediate values."
    )?;
    writeln!(&mut code, "#[wasm_bindgen]")?;
    writeln!(
        &mut code,
        "pub fn compute_witness(inputs_js: JsValue) -> Result<JsValue, JsValue> {{"
    )?;
    writeln!(&mut code, "    // Parse inputs from JavaScript")?;
    writeln!(
        &mut code,
        "    let inputs: WitnessInputs = serde_wasm_bindgen::from_value(inputs_js)"
    )?;
    writeln!(
        &mut code,
        "        .map_err(|e| JsValue::from_str(&format!(\"Failed to parse inputs: {{}}\", e)))?;"
    )?;
    writeln!(&mut code)?;
    writeln!(&mut code, "    log(\"Computing witness in WASM...\");")?;
    writeln!(&mut code)?;

    writeln!(&mut code, "    // Create witness map")?;
    writeln!(&mut code, "    let mut witness = HashMap::new();")?;
    writeln!(&mut code)?;

    writeln!(&mut code, "    // Public inputs")?;
    for (name, _) in &circuit.pub_inputs {
        writeln!(
            &mut code,
            "    witness.insert(\"{}\".to_string(), inputs.{}.clone());",
            name, name
        )?;
    }
    writeln!(&mut code)?;

    writeln!(&mut code, "    // Provided witness values (if any)")?;
    for (name, _) in &circuit.witnesses {
        writeln!(&mut code, "    if let Some(ref val) = inputs.{} {{", name)?;
        writeln!(
            &mut code,
            "        witness.insert(\"{}\".to_string(), val.clone());",
            name
        )?;
        writeln!(&mut code, "    }}")?;
    }
    writeln!(&mut code)?;

    writeln!(&mut code, "    // Execute circuit logic")?;
    for (i, instruction) in circuit.instructions.iter().enumerate() {
        write_wasm_instruction(&mut code, instruction, i)?;
    }
    writeln!(&mut code)?;

    writeln!(&mut code, "    // Build output structure")?;
    writeln!(&mut code, "    let output = WitnessOutput {{")?;
    for (name, _) in &circuit.pub_inputs {
        writeln!(
            &mut code,
            "        {}: witness.get(\"{}\").cloned().unwrap_or_else(|| \"0\".to_string()),",
            name, name
        )?;
    }
    for (name, _) in &circuit.witnesses {
        writeln!(
            &mut code,
            "        {}: witness.get(\"{}\").cloned().unwrap_or_else(|| \"0\".to_string()),",
            name, name
        )?;
    }
    writeln!(&mut code, "    }};")?;
    writeln!(&mut code)?;

    writeln!(&mut code, "    log(\"Witness computed successfully\");")?;
    writeln!(&mut code, "    serde_wasm_bindgen::to_value(&output).map_err(|e| JsValue::from_str(&format!(\"Serialization error: {{}}\", e)))")?;
    writeln!(&mut code, "}}")?;

    Ok(code)
}

fn write_wasm_instruction(
    code: &mut String,
    instruction: &lof::IRInstruction,
    index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    match instruction {
        lof::IRInstruction::Assign { target, expr } => {
            writeln!(code, "    // Instruction {}: {} = ...", index, target)?;
            let expr_code = expr_to_js_code(expr)?;
            writeln!(
                code,
                "    witness.insert(\"{}\".to_string(), {});",
                target, expr_code
            )?;
        }
        lof::IRInstruction::Assert { condition } => {
            writeln!(code, "    // Instruction {}: assert", index)?;
            let expr_code = expr_to_js_code(condition)?;
            writeln!(code, "    let cond_val: i64 = {}.parse().map_err(|_| JsValue::from_str(\"Parse error\"))?;", expr_code)?;
            writeln!(code, "    if cond_val == 0 {{")?;
            writeln!(
                code,
                "        return Err(JsValue::from_str(\"Assertion failed at instruction {}\"));",
                index
            )?;
            writeln!(code, "    }}")?;
        }
        lof::IRInstruction::Constrain { left, right } => {
            writeln!(code, "    // Instruction {}: constrain", index)?;
            let left_code = expr_to_js_code(left)?;
            let right_code = expr_to_js_code(right)?;
            writeln!(
                code,
                "    witness.insert(\"__left_{}\".to_string(), {});",
                index, left_code
            )?;
            writeln!(
                code,
                "    witness.insert(\"__right_{}\".to_string(), {});",
                index, right_code
            )?;
        }
    }
    Ok(())
}

fn expr_to_js_code(expr: &lof::IRExpr) -> Result<String, Box<dyn std::error::Error>> {
    Ok(match expr {
        lof::IRExpr::Constant(s) => format!("String::from(\"{}\")", s),
        lof::IRExpr::Variable(name) => format!(
            "witness.get(\"{}\").cloned().unwrap_or_else(|| \"0\".to_string())",
            name
        ),
        lof::IRExpr::Add(l, r) => {
            let left = expr_to_js_code(l)?;
            let right = expr_to_js_code(r)?;
            format!(
                "(({}).parse::<i64>().unwrap() + ({}).parse::<i64>().unwrap()).to_string()",
                left, right
            )
        }
        lof::IRExpr::Sub(l, r) => {
            let left = expr_to_js_code(l)?;
            let right = expr_to_js_code(r)?;
            format!(
                "(({}).parse::<i64>().unwrap() - ({}).parse::<i64>().unwrap()).to_string()",
                left, right
            )
        }
        lof::IRExpr::Mul(l, r) => {
            let left = expr_to_js_code(l)?;
            let right = expr_to_js_code(r)?;
            format!(
                "(({}).parse::<i64>().unwrap() * ({}).parse::<i64>().unwrap()).to_string()",
                left, right
            )
        }
        lof::IRExpr::Div(l, r) => {
            let left = expr_to_js_code(l)?;
            let right = expr_to_js_code(r)?;
            format!(
                "(({}).parse::<i64>().unwrap() / ({}).parse::<i64>().unwrap()).to_string()",
                left, right
            )
        }
        lof::IRExpr::Ge(l, r) => {
            let left = expr_to_js_code(l)?;
            let right = expr_to_js_code(r)?;
            format!("if ({}).parse::<i64>().unwrap() >= ({}).parse::<i64>().unwrap() {{ String::from(\"1\") }} else {{ String::from(\"0\") }}", left, right)
        }
        lof::IRExpr::Le(l, r) => {
            let left = expr_to_js_code(l)?;
            let right = expr_to_js_code(r)?;
            format!("if ({}).parse::<i64>().unwrap() <= ({}).parse::<i64>().unwrap() {{ String::from(\"1\") }} else {{ String::from(\"0\") }}", left, right)
        }
        _ => "String::from(\"0\")".to_string(),
    })
}

pub fn generate_wasm_cargo_toml(circuit_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        r#"[package]
name = "{}_witness_wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[workspace]
# Empty workspace to prevent being part of parent workspace

[dependencies]
wasm-bindgen = "0.2"
serde = {{ version = "1.0", features = ["derive"] }}
serde-wasm-bindgen = "0.6"

[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Enable link-time optimization
codegen-units = 1   # Reduce number of codegen units to increase optimizations
"#,
        circuit_name
    ))
}
