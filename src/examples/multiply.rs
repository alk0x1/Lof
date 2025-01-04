pub fn multiply_example() -> &'static str {
  r#"
  theorem multiply_proof 
  (x: Private Nat) 
  (y: Private Nat)
  (z: Public Nat): 
  z = x * y
  "#
} 