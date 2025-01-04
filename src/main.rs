pub mod frontend;
pub mod examples;
pub mod constraint_gen;

use examples::{multiply_example, add_example, compare_example};
use frontend::parser::Parser;
use constraint_gen::r1cs::R1CS;

fn parse_and_print(name: &str, input: &str) {
  println!("\nParsing {}:", name);
  println!("Input:\n{}", input);
  
  let mut parser = Parser::new(input);
  match parser.parse_theorem() {
    Ok(ast) => {
      println!("AST:");
      println!("{}", ast.print_tree());
      
      println!("\nGenerating R1CS:");
      match R1CS::from_ast(&ast) {
        Ok(r1cs) => {
          println!("Variables: {}", r1cs.num_vars);
          println!("Inputs: {}", r1cs.num_inputs);
          println!("Auxiliary: {}", r1cs.num_aux);
          println!("\nConstraints:");
          for (i, constraint) in r1cs.constraints.iter().enumerate() {
            println!("Constraint {}:", i);
            println!("  A: {:?}", constraint.a);
            println!("  B: {:?}", constraint.b);
            println!("  C: {:?}", constraint.c);
          }
        },
        Err(e) => println!("R1CS generation error: {}", e),
      }
    },
    Err(e) => println!("Error: {}", e),
  }
}

fn main() {
  // parse_and_print("Multiplication Example", multiply_example());
  parse_and_print("Addition Example", add_example());
  //parse_and_print("Comparison Example", compare_example());
}
