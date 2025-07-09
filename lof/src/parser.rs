use crate::lexer::{Token, Keyword, Symbol};
use crate::ast::{Expression, Type, Signal, Visibility, Pattern, Constraint, Operator, GenericParam, MatchPattern};
use std::iter::Peekable;
use std::fmt;
use tracing::debug;

pub struct Parser<T: Iterator<Item = Token>> {
  tokens: Peekable<T>,
}

#[derive(Debug)]
pub enum ParseError {
  UnexpectedToken(Token),
  UnexpectedEOF,
  InvalidType,
  InvalidExpression,
}

type ParseResult<T> = Result<T, ParseError>;

impl<T: Iterator<Item = Token>> Parser<T> {
  pub fn new(tokens: T) -> Self {
    Parser {
      tokens: tokens.peekable(),
    }
  }

  fn peek(&mut self) -> Option<&Token> {
    self.tokens.peek()
  }

  fn expect(&mut self, expected: Token) -> ParseResult<()> {
    match self.tokens.next() {
      Some(token) if token == expected => Ok(()),
      Some(token) => Err(ParseError::UnexpectedToken(token)),
      None => Err(ParseError::UnexpectedEOF),
    }
  }

/// Grammar: `Program ::= (Proof | Component)*`
pub fn parse_program(&mut self) -> ParseResult<Vec<Expression>> {
    let mut declarations = Vec::new();
    while let Some(token) = self.peek() {
      match token {
        Token::Keyword(Keyword::Proof) => {
          declarations.push(self.parse_proof()?);
        }
        Token::Keyword(Keyword::Component) => {
          declarations.push(self.parse_component()?);
        }
        Token::EOF => break,
        _ => return Err(ParseError::UnexpectedToken(token.clone())),
      }
    }
    Ok(declarations)
  }
  
  /// Grammar: `Proof ::= "proof" Identifier GenericParams? "{" ProofBody "}"`
  fn parse_proof(&mut self) -> ParseResult<Expression> {
    self.expect(Token::Keyword(Keyword::Proof))?;
    
    let name = match self.tokens.next() {
      Some(Token::Identifier(name)) => name,
      Some(token) => return Err(ParseError::UnexpectedToken(token)),
      None => return Err(ParseError::UnexpectedEOF),
    };

    let generics = if let Some(Token::Symbol(Symbol::LAngle)) = self.peek() {
      self.parse_generic_params()?
    } else {
      vec![]
    };

    self.expect(Token::Symbol(Symbol::LBrace))?;

    let (signals, constraints) = self.parse_proof_body()?;

    self.expect(Token::Symbol(Symbol::RBrace))?;

    Ok(Expression::Proof {
      name,
      generics,
      signals,
      constraints,
    })
  }

  /// Grammar: `ProofBody ::= (Signal | Constraint)*`
  /// where Signal ::= ("input" | "witness" | "output") Identifier ":" Type ";"
  /// and Constraint ::= Assert | Verify | Let | Match
  fn parse_proof_body(&mut self) -> ParseResult<(Vec<Signal>, Vec<Constraint>)> {
    let mut signals = Vec::new();
    let mut constraints = Vec::new();

    while let Some(token) = self.peek() {
      match token {
        Token::Keyword(Keyword::Input) |
        Token::Keyword(Keyword::Witness) |
        Token::Keyword(Keyword::Output) => {
          signals.push(self.parse_signal()?);
        }
        Token::Keyword(Keyword::Assert) |
        Token::Keyword(Keyword::Verify) => {
          constraints.push(self.parse_constraint()?);
        }
        Token::Keyword(Keyword::Let) => {
          // Handle let bindings
          let binding = self.parse_let_binding()?;
          constraints.push(Constraint::Let(Box::new(binding)));
        }
        Token::Keyword(Keyword::Match) => {
          let match_expr = self.parse_match_expression()?;
          constraints.push(Constraint::Match(Box::new(match_expr)));
        }
        Token::Symbol(Symbol::RBrace) => break,
        _ => return Err(ParseError::UnexpectedToken(token.clone())),
      }
    }

    Ok((signals, constraints))
  }

  /// Grammar: `Block ::= "{" Statement* "}"`
  /// where Statement ::= Let | Expression ";"?
  fn parse_block(&mut self) -> ParseResult<Expression> {
    debug!("Entering parse_block");
    self.expect(Token::Symbol(Symbol::LBrace))?;
    
    let mut statements = Vec::new();
    
    while let Some(token) = self.peek() {
      debug!("Block parsing token: {:?}", token);
      
      if token == &Token::Symbol(Symbol::RBrace) {
        self.tokens.next();
        debug!("Exiting block on RBrace");
        break;
      }

      match token {
        Token::Keyword(Keyword::Let) => {
          // Handle let binding
          let let_expr = self.parse_let_binding()?;
          statements.push(let_expr);
        },
        _ => {
          // Handle regular expression
          let expr = self.parse_expression()?;
          if let Some(Token::Symbol(Symbol::Semi)) = self.peek() {
            self.tokens.next();
          }
          statements.push(expr);
        }
      };
    }

    if statements.is_empty() {
      Ok(Expression::Block(vec![]))
    } else if statements.len() == 1 {
      Ok(statements.remove(0))
    } else {
      Ok(Expression::Block(statements))
    }
  }

  /// Grammar: `GenericParams ::= "<" (GenericParam ("," GenericParam)*)? ">"`
  fn parse_generic_params(&mut self) -> ParseResult<Vec<GenericParam>> {
    self.expect(Token::Symbol(Symbol::LAngle))?;
    
    let mut params = Vec::new();
    while let Some(token) = self.peek() {
      match token {
        Token::Symbol(Symbol::RAngle) => {
          self.tokens.next();
          break;
        }
        Token::Identifier(_) => {
          params.push(self.parse_generic_param()?);
          match self.peek() {
            Some(Token::Symbol(Symbol::Comma)) => {
              self.tokens.next();
            }
            Some(Token::Symbol(Symbol::RAngle)) => continue,
            _ => return Err(ParseError::InvalidType),
          }
        }
        _ => return Err(ParseError::UnexpectedToken(token.clone())),
      }
    }
    
    Ok(params)
  }

  /// Grammar: `GenericParam ::= Identifier (":" Type)?`
  fn parse_generic_param(&mut self) -> ParseResult<GenericParam> {
    let name = match self.tokens.next() {
      Some(Token::Identifier(name)) => name,
      Some(token) => return Err(ParseError::UnexpectedToken(token)),
      None => return Err(ParseError::UnexpectedEOF),
    };

    let bound = if let Some(Token::Symbol(Symbol::Colon)) = self.peek() {
      self.tokens.next();
      Some(self.parse_type()?)
    } else {
      None
    };

    Ok(GenericParam { name, bound })
  }

  /// Grammar: `Signal ::= ("input" | "witness" | "output") Identifier ":" Type ";"`
  fn parse_signal(&mut self) -> ParseResult<Signal> {
    let visibility = match self.tokens.next() {
      Some(Token::Keyword(Keyword::Input)) => Visibility::Input,
      Some(Token::Keyword(Keyword::Witness)) => Visibility::Witness,
      Some(Token::Keyword(Keyword::Output)) => Visibility::Output,
      Some(token) => return Err(ParseError::UnexpectedToken(token)),
      None => return Err(ParseError::UnexpectedEOF),
    };

    let name = match self.tokens.next() {
      Some(Token::Identifier(name)) => name,
      Some(token) => return Err(ParseError::UnexpectedToken(token)),
      None => return Err(ParseError::UnexpectedEOF),
    };

    self.expect(Token::Symbol(Symbol::Colon))?;
    let typ = self.parse_type()?;
    self.expect(Token::Symbol(Symbol::Semi))?;

    Ok(Signal { name, visibility, typ })
  }

  /// Grammar: `Type ::= "field" ("<" Expression ".." Expression ">")?
  ///                  | "bits" "<" Expression ">"
  ///                  | "array" "<" Type "," Expression ">"
  ///                  | "nat" | "bool"
  ///                  | "refined" "{" Type "," Expression "}"
  ///                  | Identifier ("<" ...)?`
  fn parse_type(&mut self) -> ParseResult<Type> {
    match self.tokens.next() {
      Some(Token::Keyword(Keyword::Field)) => {
        if let Some(Token::Symbol(Symbol::LAngle)) = self.peek() {
          self.tokens.next();
          let min = self.parse_expression()?;
          self.expect(Token::Symbol(Symbol::Range))?;
          let max = self.parse_expression()?;
          self.expect(Token::Symbol(Symbol::RAngle))?;
          
          let refinement = Expression::BinaryOp {
            left: Box::new(Expression::BinaryOp {
              left: Box::new(Expression::Variable("self".to_string())),
              op: Operator::Ge,
              right: Box::new(min.clone())
            }),
            op: Operator::And,
            right: Box::new(Expression::BinaryOp {
              left: Box::new(Expression::Variable("self".to_string())),
              op: Operator::Lt,
              right: Box::new(max.clone())
            })
          };
          
          Ok(Type::Refined(
            Box::new(Type::Field),
            Box::new(refinement)
          ))
        } else {
          Ok(Type::Field)
        }
      }
      Some(Token::Keyword(Keyword::Bits)) => {
        self.expect(Token::Symbol(Symbol::LAngle))?;
        let size = self.parse_expression()?;
        self.expect(Token::Symbol(Symbol::RAngle))?;
        Ok(Type::Bits(Box::new(size)))
      }
      Some(Token::Keyword(Keyword::Array)) => {
        self.expect(Token::Symbol(Symbol::LAngle))?;
        let element_type = Box::new(self.parse_type()?);
        self.expect(Token::Symbol(Symbol::Comma))?;
        let size = Box::new(self.parse_expression()?);
        self.expect(Token::Symbol(Symbol::RAngle))?;
        Ok(Type::Array(element_type, size))
      }

      Some(Token::Keyword(Keyword::Nat)) => Ok(Type::Nat),
      Some(Token::Keyword(Keyword::Bool)) => Ok(Type::Bool),

      Some(Token::Keyword(Keyword::Refined)) => {
        self.expect(Token::Symbol(Symbol::LBrace))?;
        let base_type = Box::new(self.parse_type()?);
        self.expect(Token::Symbol(Symbol::Comma))?;
        let refinement = Box::new(self.parse_expression()?);
        self.expect(Token::Symbol(Symbol::RBrace))?;
        Ok(Type::Refined(base_type, refinement))
      }

      Some(Token::Identifier(name)) => {
        if self.peek() == Some(&Token::Symbol(Symbol::LAngle)) {
          Ok(Type::GenericType(name))
        } else {
          Ok(Type::Custom(name))
        }
      }
      Some(token) => Err(ParseError::UnexpectedToken(token)),
      None => Err(ParseError::UnexpectedEOF),
    }
  }

  /// Grammar: `Expression ::= Match | Let | BinaryExpression`
  fn parse_expression(&mut self) -> ParseResult<Expression> {
    match self.peek() {
      Some(Token::Keyword(Keyword::Match)) => self.parse_match_expression(),
      Some(Token::Keyword(Keyword::Let)) => self.parse_let_binding(),
      _ => self.parse_binary_expression(),
    }
  }

  /// Grammar: `BinaryExpression ::= PrimaryExpression (Operator PrimaryExpression)*`
  /// with operator precedence: Assert(1) < Add,Sub(2) < Mul(3)
  fn parse_binary_expression(&mut self) -> ParseResult<Expression> {
    let mut expr_stack = vec![self.parse_primary_expression()?];
    let mut op_stack = Vec::new();

    while let Some(token) = self.peek() {
      let (op, precedence) = match token {
        Token::Symbol(Symbol::TripleEqual) => (Operator::Assert, 1),
        Token::Symbol(Symbol::Plus) => (Operator::Add, 2),
        Token::Symbol(Symbol::Star) => (Operator::Mul, 3),
        Token::Symbol(Symbol::Minus) => (Operator::Sub, 2),
        _ => break,
      };

      self.tokens.next(); // Consume the operator
      let right = self.parse_primary_expression()?;

      // Process operators with higher or equal precedence
      while let Some(top_op) = op_stack.last() {
        let top_precedence = match top_op {
          Operator::Assert => 1,
          Operator::Add => 2,
          Operator::Mul => 3,
          _ => 0,
        };

        if top_precedence <= precedence {
          break;
        }

        // Pop and combine
        let right_expr = expr_stack.pop().unwrap();
        let left_expr = expr_stack.pop().unwrap();
        expr_stack.push(Expression::BinaryOp {
          left: Box::new(left_expr),
          op: op_stack.pop().unwrap(),
          right: Box::new(right_expr),
        });
      }

      op_stack.push(op);
      expr_stack.push(right);
    }

    // Process remaining operators
    while let Some(op) = op_stack.pop() {
      let right_expr = expr_stack.pop().unwrap();
      let left_expr = expr_stack.pop().unwrap();
      expr_stack.push(Expression::BinaryOp {
        left: Box::new(left_expr),
        op,
        right: Box::new(right_expr),
      });
    }

    Ok(expr_stack.pop().unwrap())
  }

  /// Grammar: `PrimaryExpression ::= Number | FunctionCall | Variable | Match`
  /// where FunctionCall ::= Identifier "(" (Expression ("," Expression)*)? ")"
  fn parse_primary_expression(&mut self) -> ParseResult<Expression> {
    match self.tokens.next() {
      Some(Token::Number(n)) => Ok(Expression::Number(n)),
      Some(Token::Identifier(name)) => {
        if let Some(Token::Symbol(Symbol::LParen)) = self.peek() {
          self.tokens.next();
          let mut args = Vec::new();
          
          while let Some(token) = self.peek() {
            if token == &Token::Symbol(Symbol::RParen) {
              self.tokens.next();
              break;
            }
            
            args.push(self.parse_expression()?);
            
            match self.peek() {
              Some(Token::Symbol(Symbol::Comma)) => {
                self.tokens.next();
              }
              Some(Token::Symbol(Symbol::RParen)) => continue,
              _ => return Err(ParseError::UnexpectedToken(self.tokens.next().unwrap())),
            }
          }
          
          Ok(Expression::FunctionCall {
            function: name,
            arguments: args,
          })
        } else {
          Ok(Expression::Variable(name))
        }
      },
      Some(Token::Keyword(Keyword::Match)) => self.parse_match_expression(),
      Some(token) => Err(ParseError::UnexpectedToken(token)),
      None => Err(ParseError::UnexpectedEOF),
    }
  }

  /// Grammar: `Match ::= "match" Expression "{" MatchArm* "}"`
  /// where MatchArm ::= Pattern "=>" Block ","?
  fn parse_match_expression(&mut self) -> ParseResult<Expression> {
    self.expect(Token::Keyword(Keyword::Match))?;
    let value = Box::new(self.parse_expression()?);
    
    self.expect(Token::Symbol(Symbol::LBrace))?;
    
    let mut patterns = Vec::new();
    while let Some(token) = self.peek() {
      
      if token == &Token::Symbol(Symbol::RBrace) {
        self.tokens.next();  // consume closing brace
        break;
      }
      
      let pattern = self.parse_pattern()?;
      self.expect(Token::Symbol(Symbol::FatArrow))?;
      
      let body = Box::new(self.parse_block()?);
      
      patterns.push(MatchPattern { pattern, body });
      
      // Handle optional comma after match arm
      if let Some(Token::Symbol(Symbol::Comma)) = self.peek() {
        self.tokens.next();
      }
    }
    
    Ok(Expression::Match { value, patterns })
  }

  /// Grammar: `Pattern ::= Constructor | Variable | Wildcard`
  /// where Constructor ::= Identifier "(" (Pattern ("," Pattern)*)? ")"
  /// and Variable ::= Identifier
  /// and Wildcard ::= "_"
  fn parse_pattern(&mut self) -> ParseResult<Pattern> {
    match self.tokens.next() {
      Some(Token::Identifier(name)) => {
        if let Some(Token::Symbol(Symbol::LParen)) = self.peek() {
          self.tokens.next();
          let mut subpatterns = Vec::new();
          
          while let Some(token) = self.peek() {
            if token == &Token::Symbol(Symbol::RParen) {
              self.tokens.next();
              break;
            }
            
            subpatterns.push(self.parse_pattern()?);
            
            match self.peek() {
              Some(Token::Symbol(Symbol::Comma)) => {
                self.tokens.next();
              }
              Some(Token::Symbol(Symbol::RParen)) => continue,
              _ => return Err(ParseError::UnexpectedToken(self.tokens.next().unwrap())),
            }
          }
          
          Ok(Pattern::Constructor(name, subpatterns))
        } else {
          Ok(Pattern::Variable(name))
        }
      }
      Some(Token::Symbol(Symbol::Underscore)) => Ok(Pattern::Wildcard),
      Some(token) => Err(ParseError::UnexpectedToken(token)),
      None => Err(ParseError::UnexpectedEOF),
    }
  }

  /// Grammar: `Component ::= "component" Identifier GenericParams? "{" ProofBody "}"`
  fn parse_component(&mut self) -> ParseResult<Expression> {
    self.expect(Token::Keyword(Keyword::Component))?;
    
    let name = match self.tokens.next() {
      Some(Token::Identifier(name)) => name,
      Some(token) => return Err(ParseError::UnexpectedToken(token)),
      None => return Err(ParseError::UnexpectedEOF),
    };

    let generics = if let Some(Token::Symbol(Symbol::LAngle)) = self.peek() {
      self.parse_generic_params()?
    } else {
      vec![]
    };

    self.expect(Token::Symbol(Symbol::LBrace))?;

    let (signals, constraints) = self.parse_proof_body()?;

    self.expect(Token::Symbol(Symbol::RBrace))?;

    Ok(Expression::Component {
      name,
      generics,
      signals,
      constraints,
    })
  }

  /// Grammar: `Let ::= "let" Identifier "=" Expression ";" Expression`
  fn parse_let_binding(&mut self) -> ParseResult<Expression> {
    self.expect(Token::Keyword(Keyword::Let))?;
    
    let name = match self.tokens.next() {
      Some(Token::Identifier(name)) => name,
      Some(token) => return Err(ParseError::UnexpectedToken(token)),
      None => return Err(ParseError::UnexpectedEOF),
    };

    self.expect(Token::Symbol(Symbol::Equals))?;
    
    let value = Box::new(self.parse_expression()?);
    
    // Consume semicolon
    self.expect(Token::Symbol(Symbol::Semi))?;
    
    // Parse the body (which could be another let or the final expression)
    let body = Box::new(self.parse_expression()?);

    Ok(Expression::Let {
      name,
      value,
      body,
    })
  }

  /// Grammar: `Constraint ::= Assert | Verify | Match | Let`
  /// where Assert ::= "assert" Expression ";"
  /// and Verify ::= "verify" Expression ";"
  pub fn parse_constraint(&mut self) -> ParseResult<Constraint> {
    match self.tokens.next() {
      Some(Token::Keyword(Keyword::Assert)) => {
        let expr = Box::new(self.parse_expression()?);
        self.expect(Token::Symbol(Symbol::Semi))?;
        Ok(Constraint::Assert(expr))
      }
      Some(Token::Keyword(Keyword::Verify)) => {
        let expr = Box::new(self.parse_expression()?);
        self.expect(Token::Symbol(Symbol::Semi))?;
        Ok(Constraint::Verify(expr))
      }
      Some(Token::Keyword(Keyword::Match)) => {
        let expr = self.parse_match_expression()?;
        Ok(Constraint::Match(Box::new(expr)))
      }
      Some(Token::Keyword(Keyword::Let)) => {
        // Add support for let bindings in constraints
        let expr = self.parse_let_binding()?;
        Ok(Constraint::Let(Box::new(expr)))
      }
      Some(token) => Err(ParseError::UnexpectedToken(token)),
      None => Err(ParseError::UnexpectedEOF),
    }
  }
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      ParseError::UnexpectedToken(token) => write!(f, "Unexpected token: {:?}", token),
      ParseError::UnexpectedEOF => write!(f, "Unexpected end of file"),
      ParseError::InvalidType => write!(f, "Invalid type"),
      ParseError::InvalidExpression => write!(f, "Invalid expression"),
    }
  }
}
