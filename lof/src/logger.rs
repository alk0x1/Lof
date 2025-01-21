use std::path::PathBuf;
use colored::*;

pub struct CompilerLogger {
  verbose: bool,
}

impl CompilerLogger {
  pub fn new(verbose: bool) -> Self {
    Self { verbose }
  }

  pub fn start_compilation(&self) {
    println!("{}", "🔄 Starting compilation process...".bold());
  }

  pub fn start_parsing(&self) {
    println!("\n{}", "📝 Parsing source code...".blue().bold());
  }

  pub fn parsing_failed(&self, error: &str) {
    println!("{} {}", "❌ Parsing failed:".red().bold(), error);
  }

  pub fn no_proofs_found(&self) {
    println!("{}", "❌ No proofs found in source code".red().bold());
  }

  pub fn parsing_completed(&self) {
    println!("{}", "✅ Parsing completed successfully".green());
  }

  pub fn start_type_checking(&self) {
    println!("\n{}", "🔍 Performing type checking...".blue().bold());
  }

  pub fn checking_proof(&self, name: &str) {
    if self.verbose {
      println!("  Checking proof '{}'...", name);
    }
  }

  pub fn type_check_failed(&self, name: &str, error: &str) {
      println!("{} {} {}", "❌ Type checking failed for proof".red().bold(), name.bold(), error);
  }

  pub fn proof_type_checked(&self, name: &str) {
    if self.verbose {
      println!("  {} Proof '{}' type-checked successfully", "✅".green(), name);
    }
  }

  pub fn type_checking_completed(&self) {
      println!("{}", "✅ Type checking completed successfully".green());
  }

  pub fn start_r1cs_generation(&self) {
    println!("\n{}", "🔧 Generating R1CS constraints...".blue().bold());
  }

  pub fn converting_proof_to_r1cs(&self, name: &str) {
    if self.verbose {
      println!("  Converting proof '{}' to R1CS...", name);
    }
  }

  pub fn r1cs_generation_failed(&self, name: &str, error: &str) {
      println!("{} {} {}", "❌ R1CS generation failed for proof".red().bold(), name.bold(), error);
  }

  pub fn no_constraints_warning(&self, name: &str) {
    println!("{} Proof '{}' generated no constraints", "⚠️ Warning:".yellow().bold(), name);
  }

  pub fn proof_statistics(&self, name: &str, pub_inputs: &[String], witnesses: &[String], constraints: usize) {
    if self.verbose {
      println!("  📊 Statistics for proof '{}':", name);
      println!("    - Public inputs: {:?}", pub_inputs);
      println!("    - Witnesses: {:?}", witnesses);
      println!("    - Constraints: {}", constraints);
    }
  }

  pub fn writing_r1cs(&self, path: &PathBuf) {
    println!("📂 Writing R1CS file to: {}", path.display());
  }

  pub fn r1cs_write_failed(&self, error: &std::io::Error) {
    println!("{} {}", "❌ Failed to create file:".red().bold(), error);
  }

  pub fn r1cs_write_success(&self, path: &PathBuf, size: u64, constraints: usize) {
    println!("{} {} bytes to {}", "✅ Successfully wrote".green(), size, path.display());
    println!("   - Wrote {} constraints", constraints);
  }

  pub fn r1cs_metadata(&self, pub_inputs: usize, witnesses: usize, constraints: usize) {
    println!("📊 Writing R1CS metadata:");
    println!("   - Public inputs: {}", pub_inputs);
    println!("   - Witnesses: {}", witnesses);
    println!("   - Constraints: {}", constraints);
  }

  pub fn compilation_summary(&self, proof_count: usize, constraint_count: usize) {
    println!("\n{}", "✨ Compilation successful!".green().bold());
    println!("{}", "📋 Summary:".bold());
    println!("   ✓ {} proof(s) parsed", proof_count);
    println!("   ✓ All proofs type checked");
    println!("   ✓ {} R1CS constraint(s) generated", constraint_count);
    println!("   ✓ R1CS files written successfully");
  }
}