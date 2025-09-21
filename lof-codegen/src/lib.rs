use std::path::PathBuf;

pub mod wasm;

#[derive(Debug)]
pub enum CodegenTarget {
    Wasm,
}

#[derive(Debug)]
pub struct CodegenConfig {
    pub r1cs_path: PathBuf,
    pub output_dir: PathBuf,
    pub circuit_name: String,
    pub target: CodegenTarget,
}

#[derive(Debug)]
pub enum CodegenError {
    Wasm(wasm::WasmError),
    InvalidConfig(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::Wasm(e) => write!(f, "WASM generation error: {}", e),
            CodegenError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

impl From<wasm::WasmError> for CodegenError {
    fn from(error: wasm::WasmError) -> Self {
        CodegenError::Wasm(error)
    }
}

pub fn generate_code(config: CodegenConfig) -> Result<(), CodegenError> {
    // Validate config
    if !config.r1cs_path.exists() {
        return Err(CodegenError::InvalidConfig(
            format!("R1CS file not found: {}", config.r1cs_path.display())
        ));
    }
    
    if config.circuit_name.is_empty() {
        return Err(CodegenError::InvalidConfig(
            "Circuit name cannot be empty".to_string()
        ));
    }

    match config.target {
        CodegenTarget::Wasm => {
            let wasm_config = wasm::WasmConfig {
                r1cs_path: config.r1cs_path,
                output_dir: config.output_dir,
                circuit_name: config.circuit_name,
            };
            wasm::generate_witness_calculator(wasm_config)?;
            Ok(())
        }
    }
}

// Convenience functions for each target
pub fn generate_wasm_witness_calculator(
    r1cs_path: PathBuf,
    output_dir: PathBuf,
    circuit_name: String,
) -> Result<(), CodegenError> {
    let config = CodegenConfig {
        r1cs_path,
        output_dir,
        circuit_name,
        target: CodegenTarget::Wasm,
    };
    generate_code(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_config_validation() {
        let config = CodegenConfig {
            r1cs_path: PathBuf::from("nonexistent.r1cs"),
            output_dir: PathBuf::from("/tmp"),
            circuit_name: "".to_string(),
            target: CodegenTarget::Wasm,
        };
        
        let result = generate_code(config);
        assert!(result.is_err());
    }
}