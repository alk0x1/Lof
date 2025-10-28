mod wasm_gen;

use lof::IRCircuit;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: lof-witness-gen <circuit.ir> [output_dir]");
        eprintln!("\nGenerates a Rust witness calculator from IR");
        std::process::exit(1);
    }

    let ir_path = PathBuf::from(&args[1]);
    let output_dir = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        ir_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf()
    };

    println!("Reading IR from: {}", ir_path.display());
    let circuit = IRCircuit::read_from_file(&ir_path)?;

    println!("Circuit: {}", circuit.name);
    println!("  Public inputs: {}", circuit.pub_inputs.len());
    println!("  Witnesses: {}", circuit.witnesses.len());
    println!("  Outputs: {}", circuit.outputs.len());
    println!("  Instructions: {}", circuit.instructions.len());

    let calculator_code = generate_witness_calculator(&circuit)?;

    let wasm_code = wasm_gen::generate_wasm_witness_calculator(&circuit)?;

    let circuit_name = ir_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file name")?;

    let output_path = output_dir.join(format!("{}_witness.rs", circuit_name));
    std::fs::write(&output_path, calculator_code)?;

    let wasm_output_path = output_dir.join(format!("{}_witness_wasm", circuit_name));
    std::fs::create_dir_all(&wasm_output_path)?;

    let lib_path = wasm_output_path.join("src").join("lib.rs");
    std::fs::create_dir_all(wasm_output_path.join("src"))?;
    std::fs::write(&lib_path, wasm_code)?;

    let cargo_toml = wasm_gen::generate_wasm_cargo_toml(circuit_name)?;
    std::fs::write(wasm_output_path.join("Cargo.toml"), cargo_toml)?;

    println!("\nGenerated witness calculator: {}", output_path.display());
    println!("Generated WASM project: {}", wasm_output_path.display());
    println!("\nNext steps:");
    println!("  Regular Rust:");
    println!("    1. Include {}_witness.rs in your project", circuit_name);
    println!("    2. Call compute_witness(inputs)");
    println!("\n  WASM (for browser):");
    println!("    1. cd {}", wasm_output_path.display());
    println!("    2. wasm-pack build --target web");
    println!(
        "    3. Use in browser with import {{ compute_witness }} from './pkg/{}_witness_wasm.js'",
        circuit_name
    );

    Ok(())
}

fn generate_witness_calculator(circuit: &IRCircuit) -> Result<String, Box<dyn std::error::Error>> {
    use std::fmt::Write;

    let mut code = String::new();

    writeln!(
        &mut code,
        "/// Generated witness calculator for circuit: {}",
        circuit.name
    )?;
    writeln!(
        &mut code,
        "/// This code executes the circuit logic to compute all witness values."
    )?;
    writeln!(&mut code)?;
    writeln!(&mut code, "use num_bigint::BigInt;")?;
    writeln!(&mut code, "use std::collections::HashMap;")?;
    writeln!(&mut code)?;

    writeln!(&mut code, "/// Compute the full witness for this circuit")?;
    writeln!(&mut code, "///")?;
    writeln!(&mut code, "/// # Arguments")?;
    writeln!(
        &mut code,
        "/// * `pub_inputs` - Map of public input names to field values"
    )?;
    writeln!(&mut code, "///")?;
    writeln!(&mut code, "/// # Returns")?;
    writeln!(
        &mut code,
        "/// A map containing all signal values (inputs, witnesses, outputs)"
    )?;
    writeln!(&mut code, "pub fn compute_witness(pub_inputs: HashMap<String, BigInt>) -> Result<HashMap<String, BigInt>, String> {{")?;
    writeln!(&mut code, "    let mut witness = HashMap::new();")?;
    writeln!(&mut code)?;

    writeln!(&mut code, "    // Public inputs")?;
    for (name, _typ) in &circuit.pub_inputs {
        writeln!(&mut code, "    witness.insert(\"{}\".to_string(), pub_inputs.get(\"{}\").ok_or(\"Missing input: {}\")?. clone());", name, name, name)?;
    }
    writeln!(&mut code)?;

    writeln!(&mut code, "    // Execute circuit logic")?;
    for (i, instruction) in circuit.instructions.iter().enumerate() {
        write_instruction(&mut code, instruction, i)?;
    }

    writeln!(&mut code)?;
    writeln!(&mut code, "    Ok(witness)")?;
    writeln!(&mut code, "}}")?;

    writeln!(&mut code)?;
    writeln!(&mut code, "// Helper: Evaluate an expression")?;
    writeln!(&mut code, "fn eval_expr(expr: &IRExpr, witness: &HashMap<String, BigInt>) -> Result<BigInt, String> {{")?;
    writeln!(&mut code, "    match expr {{")?;
    writeln!(
        &mut code,
        "        IRExpr::Constant(s) => s.parse().map_err(|e| format(\"Parse error: {{}}\", e)),"
    )?;
    writeln!(&mut code, "        IRExpr::Variable(name) => witness.get(name).ok_or(format!(\"Unknown variable: {{}}\", name)).cloned(),")?;
    writeln!(
        &mut code,
        "        IRExpr::Add(l, r) => Ok(eval_expr(l, witness)? + eval_expr(r, witness)?),"
    )?;
    writeln!(
        &mut code,
        "        IRExpr::Sub(l, r) => Ok(eval_expr(l, witness)? - eval_expr(r, witness)?),"
    )?;
    writeln!(
        &mut code,
        "        IRExpr::Mul(l, r) => Ok(eval_expr(l, witness)? * eval_expr(r, witness)?),"
    )?;
    writeln!(&mut code, "        IRExpr::Div(l, r) => {{")?;
    writeln!(
        &mut code,
        "            let divisor = eval_expr(r, witness)?;"
    )?;
    writeln!(&mut code, "            if divisor == BigInt::from(0) {{")?;
    writeln!(
        &mut code,
        "                return Err(\"Division by zero\".to_string());"
    )?;
    writeln!(&mut code, "            }}")?;
    writeln!(
        &mut code,
        "            Ok(eval_expr(l, witness)? / divisor)"
    )?;
    writeln!(&mut code, "        }},")?;
    writeln!(&mut code, "        // Comparisons return 0 or 1")?;
    writeln!(&mut code, "        IRExpr::Lt(l, r) => Ok(if eval_expr(l, witness)? < eval_expr(r, witness)? {{ BigInt::from(1) }} else {{ BigInt::from(0) }}),")?;
    writeln!(&mut code, "        IRExpr::Gt(l, r) => Ok(if eval_expr(l, witness)? > eval_expr(r, witness)? {{ BigInt::from(1) }} else {{ BigInt::from(0) }}),")?;
    writeln!(&mut code, "        IRExpr::Le(l, r) => Ok(if eval_expr(l, witness)? <= eval_expr(r, witness)? {{ BigInt::from(1) }} else {{ BigInt::from(0) }}),")?;
    writeln!(&mut code, "        IRExpr::Ge(l, r) => Ok(if eval_expr(l, witness)? >= eval_expr(r, witness)? {{ BigInt::from(1) }} else {{ BigInt::from(0) }}),")?;
    writeln!(&mut code, "        IRExpr::Equal(l, r) => Ok(if eval_expr(l, witness)? == eval_expr(r, witness)? {{ BigInt::from(1) }} else {{ BigInt::from(0) }}),")?;
    writeln!(&mut code, "        IRExpr::NotEqual(l, r) => Ok(if eval_expr(l, witness)? != eval_expr(r, witness)? {{ BigInt::from(1) }} else {{ BigInt::from(0) }}),")?;
    writeln!(&mut code, "        // Logical ops")?;
    writeln!(&mut code, "        IRExpr::And(l, r) => {{")?;
    writeln!(&mut code, "            let lv = eval_expr(l, witness)?;")?;
    writeln!(&mut code, "            let rv = eval_expr(r, witness)?;")?;
    writeln!(&mut code, "            Ok(if lv != BigInt::from(0) && rv != BigInt::from(0) {{ BigInt::from(1) }} else {{ BigInt::from(0) }})")?;
    writeln!(&mut code, "        }},")?;
    writeln!(&mut code, "        IRExpr::Or(l, r) => {{")?;
    writeln!(&mut code, "            let lv = eval_expr(l, witness)?;")?;
    writeln!(&mut code, "            let rv = eval_expr(r, witness)?;")?;
    writeln!(&mut code, "            Ok(if lv != BigInt::from(0) || rv != BigInt::from(0) {{ BigInt::from(1) }} else {{ BigInt::from(0) }})")?;
    writeln!(&mut code, "        }},")?;
    writeln!(&mut code, "        IRExpr::Not(e) => {{")?;
    writeln!(&mut code, "            let v = eval_expr(e, witness)?;")?;
    writeln!(
        &mut code,
        "            Ok(if v == BigInt::from(0) {{ BigInt::from(1) }} else {{ BigInt::from(0) }})"
    )?;
    writeln!(&mut code, "        }},")?;
    writeln!(
        &mut code,
        "        IRExpr::ArrayIndex {{ array, index }} => {{"
    )?;
    writeln!(
        &mut code,
        "            let arr_elem_name = format!(\"{{}}[{{}}]\", array, index);"
    )?;
    writeln!(&mut code, "            witness.get(&arr_elem_name).ok_or(format!(\"Unknown array element: {{}}\", arr_elem_name)).cloned()")?;
    writeln!(&mut code, "        }},")?;
    writeln!(
        &mut code,
        "        IRExpr::TupleField {{ tuple, index }} => {{"
    )?;
    writeln!(
        &mut code,
        "            let tuple_field_name = format!(\"{{}}_{{}}\", tuple, index);"
    )?;
    writeln!(&mut code, "            witness.get(&tuple_field_name).ok_or(format!(\"Unknown tuple field: {{}}\", tuple_field_name)).cloned()")?;
    writeln!(&mut code, "        }},")?;
    writeln!(&mut code, "    }}")?;
    writeln!(&mut code, "}}")?;

    writeln!(&mut code)?;
    writeln!(
        &mut code,
        "// IR expression types (embedded for standalone use)"
    )?;
    writeln!(&mut code, "#[allow(dead_code)]")?;
    writeln!(&mut code, "#[derive(Debug, Clone)]")?;
    writeln!(&mut code, "enum IRExpr {{")?;
    writeln!(&mut code, "    Constant(String),")?;
    writeln!(&mut code, "    Variable(String),")?;
    writeln!(&mut code, "    Add(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Sub(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Mul(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Div(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Lt(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Gt(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Le(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Ge(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Equal(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    NotEqual(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    And(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Or(Box<IRExpr>, Box<IRExpr>),")?;
    writeln!(&mut code, "    Not(Box<IRExpr>),")?;
    writeln!(
        &mut code,
        "    ArrayIndex {{ array: String, index: usize }},"
    )?;
    writeln!(
        &mut code,
        "    TupleField {{ tuple: String, index: usize }},"
    )?;
    writeln!(&mut code, "}}")?;

    Ok(code)
}

fn write_instruction(
    code: &mut String,
    instruction: &lof::IRInstruction,
    index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write;

    match instruction {
        lof::IRInstruction::Assign { target, expr } => {
            writeln!(code, "    // Instruction {}: {} = ...", index, target)?;
            let expr_code = expr_to_code(expr)?;
            writeln!(
                code,
                "    witness.insert(\"{}\".to_string(), {});",
                target, expr_code
            )?;
        }
        lof::IRInstruction::Assert { condition } => {
            writeln!(code, "    // Instruction {}: assert ...", index)?;
            let expr_code = expr_to_code(condition)?;
            writeln!(code, "    let cond_value = {};", expr_code)?;
            writeln!(code, "    if cond_value == BigInt::from(0) {{")?;
            writeln!(
                code,
                "        return Err(\"Assertion failed at instruction {}\".to_string());",
                index
            )?;
            writeln!(code, "    }}")?;
        }
        lof::IRInstruction::Constrain { left, right } => {
            writeln!(
                code,
                "    // Instruction {}: constrain (left === right)",
                index
            )?;
            let left_code = expr_to_code(left)?;
            let right_code = expr_to_code(right)?;
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

fn expr_to_code(expr: &lof::IRExpr) -> Result<String, Box<dyn std::error::Error>> {
    match expr {
        lof::IRExpr::Constant(s) => Ok(format!("BigInt::from_str(\"{}\")?", s)),
        lof::IRExpr::Variable(name) => Ok(format!(
            "witness.get(\"{}\").ok_or(\"Missing: {}\")?.clone()",
            name, name
        )),
        lof::IRExpr::Add(l, r) => Ok(format!("({} + {})", expr_to_code(l)?, expr_to_code(r)?)),
        lof::IRExpr::Sub(l, r) => Ok(format!("({} - {})", expr_to_code(l)?, expr_to_code(r)?)),
        lof::IRExpr::Mul(l, r) => Ok(format!("({} * {})", expr_to_code(l)?, expr_to_code(r)?)),
        lof::IRExpr::Div(l, r) => Ok(format!("({} / {})", expr_to_code(l)?, expr_to_code(r)?)),
        lof::IRExpr::Lt(l, r) => Ok(format!(
            "if {} < {} {{ BigInt::from(1) }} else {{ BigInt::from(0) }}",
            expr_to_code(l)?,
            expr_to_code(r)?
        )),
        lof::IRExpr::Gt(l, r) => Ok(format!(
            "if {} > {} {{ BigInt::from(1) }} else {{ BigInt::from(0) }}",
            expr_to_code(l)?,
            expr_to_code(r)?
        )),
        lof::IRExpr::Le(l, r) => Ok(format!(
            "if {} <= {} {{ BigInt::from(1) }} else {{ BigInt::from(0) }}",
            expr_to_code(l)?,
            expr_to_code(r)?
        )),
        lof::IRExpr::Ge(l, r) => Ok(format!(
            "if {} >= {} {{ BigInt::from(1) }} else {{ BigInt::from(0) }}",
            expr_to_code(l)?,
            expr_to_code(r)?
        )),
        lof::IRExpr::Equal(l, r) => Ok(format!(
            "if {} == {} {{ BigInt::from(1) }} else {{ BigInt::from(0) }}",
            expr_to_code(l)?,
            expr_to_code(r)?
        )),
        lof::IRExpr::NotEqual(l, r) => Ok(format!(
            "if {} != {} {{ BigInt::from(1) }} else {{ BigInt::from(0) }}",
            expr_to_code(l)?,
            expr_to_code(r)?
        )),
        lof::IRExpr::And(l, r) => {
            let left = expr_to_code(l)?;
            let right = expr_to_code(r)?;
            Ok(format!("if ({}) != BigInt::from(0) && ({}) != BigInt::from(0) {{ BigInt::from(1) }} else {{ BigInt::from(0) }}", left, right))
        }
        lof::IRExpr::Or(l, r) => {
            let left = expr_to_code(l)?;
            let right = expr_to_code(r)?;
            Ok(format!("if ({}) != BigInt::from(0) || ({}) != BigInt::from(0) {{ BigInt::from(1) }} else {{ BigInt::from(0) }}", left, right))
        }
        lof::IRExpr::Not(e) => {
            let inner = expr_to_code(e)?;
            Ok(format!(
                "if ({}) == BigInt::from(0) {{ BigInt::from(1) }} else {{ BigInt::from(0) }}",
                inner
            ))
        }
        lof::IRExpr::ArrayIndex { array, index } => Ok(format!(
            "witness.get(&format!(\"{}[{}]\")).ok_or(\"Missing array element\")?.clone()",
            array, index
        )),
        lof::IRExpr::TupleField { tuple, index } => Ok(format!(
            "witness.get(&format!(\"{}_{}\" )).ok_or(\"Missing tuple field\")?.clone()",
            tuple, index
        )),
    }
}
