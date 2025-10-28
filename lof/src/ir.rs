use num_bigint::{BigInt, ParseBigIntError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRInstruction {
    Assign { target: String, expr: IRExpr },
    Assert { condition: IRExpr },
    Constrain { left: IRExpr, right: IRExpr },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRExpr {
    Constant(String),

    Variable(String),

    Add(Box<IRExpr>, Box<IRExpr>),
    Sub(Box<IRExpr>, Box<IRExpr>),
    Mul(Box<IRExpr>, Box<IRExpr>),
    Div(Box<IRExpr>, Box<IRExpr>),

    Lt(Box<IRExpr>, Box<IRExpr>),
    Gt(Box<IRExpr>, Box<IRExpr>),
    Le(Box<IRExpr>, Box<IRExpr>),
    Ge(Box<IRExpr>, Box<IRExpr>),
    Equal(Box<IRExpr>, Box<IRExpr>),
    NotEqual(Box<IRExpr>, Box<IRExpr>),

    And(Box<IRExpr>, Box<IRExpr>),
    Or(Box<IRExpr>, Box<IRExpr>),
    Not(Box<IRExpr>),

    ArrayIndex { array: String, index: usize },

    TupleField { tuple: String, index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRType {
    Field,
    Bool,
    Array {
        element_type: Box<IRType>,
        size: usize,
    },
    Tuple(Vec<IRType>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRCircuit {
    pub name: String,
    pub pub_inputs: Vec<(String, IRType)>,
    pub witnesses: Vec<(String, IRType)>,
    pub outputs: Vec<(String, IRType)>,
    pub instructions: Vec<IRInstruction>,
    pub functions: HashMap<String, (Vec<String>, Vec<IRInstruction>)>,
}

impl IRCircuit {
    pub fn write_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        writer.write_all(b"lof-ir\x00\x00")?;

        writer.write_all(&1u32.to_le_bytes())?;

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writer.write_all(json.as_bytes())?;

        Ok(())
    }

    pub fn read_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);

        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != b"lof-ir\x00\x00" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid magic bytes - not a lof-ir file",
            ));
        }

        let mut version = [0u8; 4];
        reader.read_exact(&mut version)?;
        if u32::from_le_bytes(version) != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported IR version",
            ));
        }

        let mut json = String::new();
        reader.read_to_string(&mut json)?;

        let circuit: IRCircuit = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(circuit)
    }
}

pub fn bigint_to_ir_constant(value: &BigInt) -> String {
    value.to_string()
}

pub fn ir_constant_to_bigint(s: &str) -> Result<BigInt, IRConstantError> {
    s.parse::<BigInt>()
        .map_err(|source| IRConstantError::InvalidConstant {
            value: s.to_string(),
            source,
        })
}

#[derive(Debug, Error)]
pub enum IRConstantError {
    #[error("invalid IR constant '{value}': {source}")]
    InvalidConstant {
        value: String,
        #[source]
        source: ParseBigIntError,
    },
}
