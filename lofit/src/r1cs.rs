use num_bigint::BigInt;
use std::io::{self, Read};

#[derive(Debug, Clone)]
pub struct ConstraintSystem {
    pub public_inputs: Vec<String>,
    pub witnesses: Vec<String>,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub a: LinearCombination,
    pub b: LinearCombination,
    pub c: LinearCombination,
}

#[derive(Debug, Clone)]
pub struct LinearCombination {
    pub terms: Vec<(u32, BigInt)>, // (variable_index, coefficient)
}

impl ConstraintSystem {
    pub fn from_file(mut reader: impl Read) -> io::Result<Self> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != b"lof-r1cs" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid magic bytes - not a lof-r1cs file",
            ));
        }

        let mut version = [0u8; 4];
        reader.read_exact(&mut version)?;
        if u32::from_le_bytes(version) != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported r1cs version",
            ));
        }

        let mut buf = [0u8; 4];

        reader.read_exact(&mut buf)?;
        let pub_inputs_count = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let witnesses_count = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let constraints_count = u32::from_le_bytes(buf);

        let mut public_inputs = Vec::new();
        for _ in 0..pub_inputs_count {
            reader.read_exact(&mut buf)?;
            let len = u32::from_le_bytes(buf) as usize;
            let mut name = vec![0u8; len];
            reader.read_exact(&mut name)?;
            public_inputs.push(
                String::from_utf8(name)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            );
        }

        let mut witnesses = Vec::new();
        for _ in 0..witnesses_count {
            reader.read_exact(&mut buf)?;
            let len = u32::from_le_bytes(buf) as usize;
            let mut name = vec![0u8; len];
            reader.read_exact(&mut name)?;
            witnesses.push(
                String::from_utf8(name)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            );
        }

        let mut constraints = Vec::new();
        for _ in 0..constraints_count {
            let a = read_linear_combination(&mut reader)?;
            let b = read_linear_combination(&mut reader)?;
            let c = read_linear_combination(&mut reader)?;
            constraints.push(Constraint { a, b, c });
        }

        Ok(Self {
            public_inputs,
            witnesses,
            constraints,
        })
    }
}

fn read_linear_combination(reader: &mut impl Read) -> io::Result<LinearCombination> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    let terms_count = u32::from_le_bytes(buf);

    let mut terms = Vec::new();
    for _ in 0..terms_count {
        reader.read_exact(&mut buf)?;
        let var_idx = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let bytes_len = u32::from_le_bytes(buf) as usize;
        let mut bytes = vec![0u8; bytes_len];
        reader.read_exact(&mut bytes)?;

        let coeff = BigInt::from_signed_bytes_le(&bytes);

        terms.push((var_idx, coeff));
    }

    Ok(LinearCombination { terms })
}
