pub fn add_example() -> &'static str {
  r#"
  theorem add_proof
  (a: Private Nat)
  (b: Private Nat)
  (c: Public Nat):
  c = a + b
  "#
} 