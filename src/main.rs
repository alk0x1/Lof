pub mod frontend;
use frontend::lexer::Lexer;
use frontend::parser::Parser;

pub fn visualize_ast(input: &str) {
  println!("Input Program:");
  println!("{}\n", input);

  println!("Tokens:");
  let lexer = Lexer::new(input);
  let tokens: Vec<_> = lexer.collect();
  for token in &tokens {
    println!("{:?}", token);
  }
  println!();

  println!("AST:");
  let mut parser = Parser::new(tokens.into_iter());
  match parser.parse_program() {
    Ok(ast) => {
      for node in ast {
        println!("{}", node.print_tree_helper("", ""));
      }
    }
    Err(e) => println!("Parse error: {:?}", e),
  }
}
fn main() {
  // Example 1: Simple Proof
  let input1 = r#"
    proof RangeCheck {
      input value: Field<0..255>;
      witness bits: Bits<8>;
      assert value === decompose(bits);
    }
  "#;
  
  println!("=== Example 1: Simple Proof ===");
  visualize_ast(input1);

  // Example 2: Pattern Matching
  let input2 = r#"
    proof TreeSum {
      input tree: Tree;
      output sum: Field;

      match tree {
        Leaf(v) => {
          sum === v
        },
        Node(v, left, right) => {
          let left_sum = TreeSum(left);
          let right_sum = TreeSum(right);
          sum === v + left_sum + right_sum
        }
      }
    }
  "#;
  
  println!("\n=== Example 2: Pattern Matching ===");
  visualize_ast(input2);

  // Example 3: Component
  let input3 = r#"
    component HashFunction {
      input preimage: Field;
      output hash: Field;

      let t1 = preimage * preimage * preimage;
      let t2 = t1 + 7;
      hash === t2 * t2;
    }
  "#;
  
  println!("\n=== Example 3: Component ===");
  visualize_ast(input3);

  // Example 5: Recursive Proof
  let input5 = r#"
    proof ListSum {
      input list: List;
      output sum: Field;

      match list {
        Nil => {
          sum === 0
        },
        Cons(head, tail) => {
          let tail_sum = ListSum(tail);
          sum === head + tail_sum
        }
      }
    }
  "#;

  println!("\n=== Example 5: Recursive Proof ===");
  visualize_ast(input5);
}