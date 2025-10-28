use ark_bn254::Fr;
use ark_ff::{BigInt as ArkBigInt, PrimeField};
use num_bigint::{BigInt, ParseBigIntError, Sign};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FieldElementParseError {
    #[error("failed to parse integer: {0}")]
    InvalidInteger(#[from] ParseBigIntError),
    #[error("value exceeds BN254 field capacity")]
    OutOfRange,
    #[error("failed to convert integer into field element")]
    NotInField,
}

pub fn fr_from_str(input: &str) -> Result<Fr, FieldElementParseError> {
    let bigint = input.parse::<BigInt>()?;
    let (sign, magnitude) = bigint.to_bytes_le();

    let mut limbs = [0u64; 4];
    for (i, chunk) in magnitude.chunks(8).enumerate() {
        if i >= limbs.len() {
            if chunk.iter().any(|&byte| byte != 0) {
                return Err(FieldElementParseError::OutOfRange);
            }
            continue;
        }

        let mut limb_bytes = [0u8; 8];
        limb_bytes[..chunk.len()].copy_from_slice(chunk);
        limbs[i] = u64::from_le_bytes(limb_bytes);
    }

    let base = ArkBigInt::new(limbs);
    let mut fr = Fr::from_bigint(base).ok_or(FieldElementParseError::NotInField)?;

    if sign == Sign::Minus {
        fr = -fr;
    }

    Ok(fr)
}
