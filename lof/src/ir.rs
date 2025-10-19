/// Intermediate Representation (IR) for executable circuit logic
///
/// This IR represents the computational flow of a circuit, allowing
/// witness generation by executing the circuit with concrete inputs.
///
/// Unlike R1CS which is declarative constraints, IR is imperative instructions.

use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};

/// An executable instruction in the circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRInstruction {
    /// Assign a computed value to a variable
    /// Example: age = current_year - birth_year
    Assign { target: String, expr: IRExpr },

    /// Assert a boolean condition (for debugging/validation)
    /// Example: assert birth_year >= 1900
    Assert { condition: IRExpr },

    /// Create a constraint equality (marks where R1CS constraints exist)
    /// Example: is_adult === 1
    Constrain { left: IRExpr, right: IRExpr },
}

/// Executable expression that can be evaluated to a field element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRExpr {
    /// Constant value
    Constant(String), // BigInt as string for JSON compatibility

    /// Variable reference
    Variable(String),

    /// Binary operations
    Add(Box<IRExpr>, Box<IRExpr>),
    Sub(Box<IRExpr>, Box<IRExpr>),
    Mul(Box<IRExpr>, Box<IRExpr>),
    Div(Box<IRExpr>, Box<IRExpr>),

    /// Comparison operations (return 0 or 1)
    Lt(Box<IRExpr>, Box<IRExpr>),
    Gt(Box<IRExpr>, Box<IRExpr>),
    Le(Box<IRExpr>, Box<IRExpr>),
    Ge(Box<IRExpr>, Box<IRExpr>),
    Equal(Box<IRExpr>, Box<IRExpr>),
    NotEqual(Box<IRExpr>, Box<IRExpr>),

    /// Logical operations (on 0/1 values)
    And(Box<IRExpr>, Box<IRExpr>),
    Or(Box<IRExpr>, Box<IRExpr>),
    Not(Box<IRExpr>),

    /// Array indexing (constant indices only)
    ArrayIndex { array: String, index: usize },

    /// Tuple field access
    TupleField { tuple: String, index: usize },
}

/// Type information for IR variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRType {
    Field,
    Bool,
    Array { element_type: Box<IRType>, size: usize },
    Tuple(Vec<IRType>),
}

/// Complete circuit in IR form
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRCircuit {
    /// Circuit name
    pub name: String,

    /// Public input signals
    pub pub_inputs: Vec<(String, IRType)>,

    /// Private witness signals
    pub witnesses: Vec<(String, IRType)>,

    /// Public output signals
    pub outputs: Vec<(String, IRType)>,

    /// Execution instructions (in order)
    pub instructions: Vec<IRInstruction>,

    /// Function definitions (name -> (params, instructions))
    pub functions: HashMap<String, (Vec<String>, Vec<IRInstruction>)>,
}

impl IRCircuit {
    /// Serialize IR to binary format
    pub fn write_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        // Write magic header
        writer.write_all(b"lof-ir\x00\x00")?;

        // Write version
        writer.write_all(&1u32.to_le_bytes())?;

        // Serialize to JSON for readability (can optimize to binary later)
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writer.write_all(json.as_bytes())?;

        Ok(())
    }

    /// Deserialize IR from binary format
    pub fn read_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);

        // Read and validate magic header
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != b"lof-ir\x00\x00" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid magic bytes - not a lof-ir file",
            ));
        }

        // Read and validate version
        let mut version = [0u8; 4];
        reader.read_exact(&mut version)?;
        if u32::from_le_bytes(version) != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported IR version",
            ));
        }

        // Read JSON content
        let mut json = String::new();
        reader.read_to_string(&mut json)?;

        // Deserialize
        let circuit: IRCircuit = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(circuit)
    }
}

/// Helper to convert BigInt to IR constant
pub fn bigint_to_ir_constant(value: &BigInt) -> String {
    value.to_string()
}

/// Helper to convert IR constant back to BigInt
pub fn ir_constant_to_bigint(s: &str) -> Result<BigInt, String> {
    s.parse::<BigInt>()
        .map_err(|e| format!("Invalid constant: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_serialization() {
        let circuit = IRCircuit {
            name: "test".to_string(),
            pub_inputs: vec![("x".to_string(), IRType::Field)],
            witnesses: vec![("y".to_string(), IRType::Field)],
            outputs: vec![("z".to_string(), IRType::Field)],
            instructions: vec![
                IRInstruction::Assign {
                    target: "z".to_string(),
                    expr: IRExpr::Add(
                        Box::new(IRExpr::Variable("x".to_string())),
                        Box::new(IRExpr::Variable("y".to_string())),
                    ),
                },
                IRInstruction::Constrain {
                    left: IRExpr::Variable("z".to_string()),
                    right: IRExpr::Constant("42".to_string()),
                },
            ],
            functions: HashMap::new(),
        };

        // Test serialization/deserialization
        let json = serde_json::to_string_pretty(&circuit).unwrap();
        let deserialized: IRCircuit = serde_json::from_str(&json).unwrap();

        assert_eq!(circuit.name, deserialized.name);
        assert_eq!(circuit.pub_inputs.len(), deserialized.pub_inputs.len());
        assert_eq!(circuit.instructions.len(), deserialized.instructions.len());
    }
}
