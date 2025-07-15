use crate::lexer::{Token, Keyword, Symbol};
use crate::ast::{Expression, Type, Signal, Visibility, Pattern, Operator, GenericParam, MatchPattern, Parameter};
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

  /// Grammar: `Program ::= TopLevelDeclaration*`
  pub fn parse_program(&mut self) -> ParseResult<Vec<Expression>> {
    let mut declarations = Vec::new();
    while self.peek().is_some() && self.peek() != Some(&Token::EOF) {
        declarations.push(self.parse_toplevel_declaration()?);
    }
    Ok(declarations)
  }

  /// Grammar: `TopLevelDeclaration ::= Proof | FunctionDefinition`
  fn parse_toplevel_declaration(&mut self) -> ParseResult<Expression> {
      match self.peek() {
          Some(Token::Keyword(Keyword::Proof)) => self.parse_proof(),
          Some(Token::Keyword(Keyword::Let)) => self.parse_function_definition(),
          Some(other) => Err(ParseError::UnexpectedToken(other.clone())),
          None => Err(ParseError::UnexpectedEOF),
      }
  }

  /// Grammar: `FunctionDefinition ::= "let" Identifier Parameter* ":" Type "=" Expression`
  /// where `Parameter ::= "(" Identifier ":" Type ")"`
  fn parse_function_definition(&mut self) -> ParseResult<Expression> {
      self.expect(Token::Keyword(Keyword::Let))?;

      let name = match self.tokens.next() {
          Some(Token::Identifier(name)) => name,
          Some(token) => return Err(ParseError::UnexpectedToken(token)),
          None => return Err(ParseError::UnexpectedEOF),
      };

      let mut params = Vec::new();
      while let Some(&Token::Symbol(Symbol::LParen)) = self.peek() {
          self.tokens.next();

          let param_name = match self.tokens.next() {
              Some(Token::Identifier(name)) => name,
              Some(token) => return Err(ParseError::UnexpectedToken(token)),
              None => return Err(ParseError::UnexpectedEOF),
          };

          self.expect(Token::Symbol(Symbol::Colon))?;
          let param_type = self.parse_type()?;
          self.expect(Token::Symbol(Symbol::RParen))?;

          params.push(Parameter { name: param_name, typ: param_type });
      }

      self.expect(Token::Symbol(Symbol::Colon))?;
      let return_type = self.parse_type()?;

      self.expect(Token::Symbol(Symbol::Equals))?;

      let body = self.parse_expression()?;

      Ok(Expression::FunctionDef {
          name,
          params,
          return_type,
          body: Box::new(body),
      })
  }

  /// Grammar: `Proof ::= "proof" Identifier GenericParams? "{" Signal* Expression "}"`
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

    let mut signals = Vec::new();
    while let Some(Token::Keyword(kw)) = self.peek() {
        match kw {
            Keyword::Input | Keyword::Witness | Keyword::Output => {
                signals.push(self.parse_signal()?);
            }
            _ => break,
        }
    }

    let body = self.parse_expression()?;

    self.expect(Token::Symbol(Symbol::RBrace))?;

    Ok(Expression::Proof {
      name,
      generics,
      signals,
      body: Box::new(body),
    })
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
          let let_expr = self.parse_let_binding()?;
          statements.push(let_expr);
        },
        _ => {
          let expr = self.parse_expression()?;
          if let Some(Token::Symbol(Symbol::Semi)) = self.peek() {
            self.tokens.next();
          }
          statements.push(expr);
        }
      };
    }

    if statements.is_empty() {
      Ok(Expression::Block {
        statements: vec![],
        final_expr: None,
    })
    } else if statements.len() == 1 {
      Ok(statements.remove(0))
    } else {
      let mut stmts = statements;
      let final_expr = stmts.pop().map(Box::new);
      Ok(Expression::Block {
          statements: stmts,
          final_expr,
      })
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
    let next_token = match self.peek() {
        Some(token) => token.clone(),
        None => return Err(ParseError::UnexpectedEOF),
    };

    match next_token {
        Token::Identifier(name) => {
            self.tokens.next();
            if name == "array" {
                self.expect(Token::Symbol(Symbol::LAngle))?;
                let element_type = Box::new(self.parse_type()?);
                self.expect(Token::Symbol(Symbol::Comma))?;
                let size = match self.tokens.next() {
                    Some(Token::Number(n)) => n as usize,
                    Some(other) => return Err(ParseError::UnexpectedToken(other)),
                    None => return Err(ParseError::UnexpectedEOF),
                };
                self.expect(Token::Symbol(Symbol::RAngle))?;
                Ok(Type::Array { element_type, size })
            } else {
                Ok(Type::Identifier(name))
            }
        }
        Token::Symbol(Symbol::LParen) => {
            self.tokens.next();
            let mut types = Vec::new();

            if self.peek() == Some(&Token::Symbol(Symbol::RParen)) {
                self.tokens.next();
                return Ok(Type::Tuple(types));
            }

            loop {
                types.push(self.parse_type()?);
                if self.peek() == Some(&Token::Symbol(Symbol::Comma)) {
                    self.tokens.next();
                } else {
                    break;
                }
            }

            self.expect(Token::Symbol(Symbol::RParen))?;
            Ok(Type::Tuple(types))
        }
        other => Err(ParseError::UnexpectedToken(other)),
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
  /// with operator precedence: Assert(1) < Equal(2) < Add,Sub(3) < Mul(4)
  fn parse_binary_expression(&mut self) -> ParseResult<Expression> {
    let mut expr_stack = vec![self.parse_primary_expression()?];
    let mut op_stack = Vec::new();

    while let Some(token) = self.peek() {
      let (op, precedence) = match token {
        Token::Symbol(Symbol::TripleEqual) => (Operator::Assert, 1),
        Token::Symbol(Symbol::Equal) => (Operator::Equal, 2),
        Token::Symbol(Symbol::Plus) => (Operator::Add, 3),
        Token::Symbol(Symbol::Star) => (Operator::Mul, 4),
        Token::Symbol(Symbol::Minus) => (Operator::Sub, 3),
        _ => break,
      };

      self.tokens.next();
      
      let right = self.parse_primary_expression()?;

      while let Some(top_op) = op_stack.last() {
        let top_precedence = match top_op {
          Operator::Assert => 1,
          Operator::Equal => 2,
          Operator::Add | Operator::Sub => 3,
          Operator::Mul => 4,
          _ => 0,
        };

        if top_precedence < precedence {
          break;
        }

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

  /// Grammar: `PrimaryExpression ::= Number | FunctionCall | Variable | Tuple | GroupedExpr | Assert | Match`
  fn parse_primary_expression(&mut self) -> ParseResult<Expression> {
    let next_token = match self.peek() {
        Some(token) => token.clone(),
        None => return Err(ParseError::UnexpectedEOF),
    };

    match next_token {
      Token::Number(_) | Token::Identifier(_) | Token::Symbol(Symbol::LParen) => {
        self.parse_simple_primary()
      },
      Token::Keyword(Keyword::Assert) => {
        self.tokens.next();
        let condition = self.parse_expression()?;
        Ok(Expression::Assert(Box::new(condition)))
      },
      Token::Keyword(Keyword::Match) => self.parse_match_expression(),
      _ => Err(ParseError::UnexpectedToken(next_token)),
    }
  }

  fn parse_simple_primary(&mut self) -> ParseResult<Expression> {
    match self.tokens.next() {
      Some(Token::Number(n)) => Ok(Expression::Number(n)),
      Some(Token::Identifier(name)) => {
        if self.peek() == Some(&Token::Symbol(Symbol::LParen)) {
            self.parse_function_call(name)
        } else {
            Ok(Expression::Variable(name))
        }
      },
      Some(Token::Symbol(Symbol::LParen)) => {
        self.parse_tuple_or_grouped_expr()
      },
      Some(token) => Err(ParseError::UnexpectedToken(token)),
      None => Err(ParseError::UnexpectedEOF),
    }
  }

  fn parse_function_call(&mut self, name: String) -> ParseResult<Expression> {
    self.expect(Token::Symbol(Symbol::LParen))?;
    let mut args = Vec::new();
    
    if self.peek() == Some(&Token::Symbol(Symbol::RParen)) {
      self.tokens.next();
      return Ok(Expression::FunctionCall { function: name, arguments: args });
    }
    
    loop {
      args.push(self.parse_expression()?);
      if self.peek() == Some(&Token::Symbol(Symbol::Comma)) {
        self.tokens.next();
      } else {
        break;
      }
    }
    
    self.expect(Token::Symbol(Symbol::RParen))?;
    
    Ok(Expression::FunctionCall {
      function: name,
      arguments: args,
    })
  }

  fn parse_tuple_or_grouped_expr(&mut self) -> ParseResult<Expression> {
    if self.peek() == Some(&Token::Symbol(Symbol::RParen)) {
        self.tokens.next();
        return Ok(Expression::Tuple(vec![]));
    }

    let first_expr = self.parse_expression()?;

    if self.peek() == Some(&Token::Symbol(Symbol::Comma)) {
        self.tokens.next();
        let mut elements = vec![first_expr];
        
        while self.peek() != Some(&Token::Symbol(Symbol::RParen)) {
            elements.push(self.parse_expression()?);
            if self.peek() == Some(&Token::Symbol(Symbol::Comma)) {
                self.tokens.next();
            } else {
                break;
            }
        }
        self.expect(Token::Symbol(Symbol::RParen))?;
        Ok(Expression::Tuple(elements))
    } else {
        self.expect(Token::Symbol(Symbol::RParen))?;
        Ok(first_expr)
    }
  }

  /// Grammar: `Match ::= "match" Expression "{" MatchArm* "}"`
  fn parse_match_expression(&mut self) -> ParseResult<Expression> {
    self.expect(Token::Keyword(Keyword::Match))?;
    let value = Box::new(self.parse_expression()?);
    
    self.expect(Token::Symbol(Symbol::LBrace))?;
    
    let mut patterns = Vec::new();
    while let Some(token) = self.peek() {
      
      if token == &Token::Symbol(Symbol::RBrace) {
        self.tokens.next();
        break;
      }
      
      let pattern = self.parse_pattern()?;
      self.expect(Token::Symbol(Symbol::FatArrow))?;
      
      let body = Box::new(self.parse_block()?);
      
      patterns.push(MatchPattern { pattern, body });
      
      if let Some(Token::Symbol(Symbol::Comma)) = self.peek() {
        self.tokens.next();
      }
    }
    
    Ok(Expression::Match { value, patterns })
  }

  /// Grammar: `Pattern ::= Constructor | Variable | Wildcard | Tuple`
  /// where Constructor ::= Identifier "(" (Pattern ("," Pattern)*)? ")"
  /// and Variable ::= Identifier
  /// and Wildcard ::= "_"
  /// and Tuple ::= "(" (Pattern ("," Pattern)*)? ")"`
  fn parse_pattern(&mut self) -> ParseResult<Pattern> {
    match self.peek().cloned() {
      Some(Token::Identifier(name)) => {
        self.tokens.next();
        if let Some(Token::Symbol(Symbol::LParen)) = self.peek() {
          self.tokens.next();
          let subpatterns = Vec::new();
          Ok(Pattern::Constructor(name, subpatterns))
        } else {
          Ok(Pattern::Variable(name))
        }
      }
      Some(Token::Symbol(Symbol::Underscore)) => {
        self.tokens.next();
        Ok(Pattern::Wildcard)
      },
      Some(Token::Symbol(Symbol::LParen)) => {
        self.tokens.next();
        let mut patterns = Vec::new();
        if self.peek() == Some(&Token::Symbol(Symbol::RParen)) {
            self.tokens.next();
            return Ok(Pattern::Tuple(patterns));
        }
        loop {
            patterns.push(self.parse_pattern()?);
            if self.peek() == Some(&Token::Symbol(Symbol::Comma)) {
                self.tokens.next();
            } else {
                break;
            }
        }
        self.expect(Token::Symbol(Symbol::RParen))?;
        Ok(Pattern::Tuple(patterns))
      }
      Some(token) => Err(ParseError::UnexpectedToken(token)),
      None => Err(ParseError::UnexpectedEOF),
    }
  }

  /// Grammar: `Component ::= "component" Identifier GenericParams? "{" Signal* Expression "}"`
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

    let mut signals = Vec::new();
    while let Some(Token::Keyword(kw)) = self.peek() {
        match kw {
            Keyword::Input | Keyword::Witness | Keyword::Output => {
                signals.push(self.parse_signal()?);
            }
            _ => break,
        }
    }

    let body = self.parse_expression()?;

    self.expect(Token::Symbol(Symbol::RBrace))?;

    Ok(Expression::Component {
      name,
      generics,
      signals,
      body: Box::new(body),
    })
  }

  /// Grammar: `Let ::= "let" Pattern "=" Expression "in" Expression`
  fn parse_let_binding(&mut self) -> ParseResult<Expression> {
    self.expect(Token::Keyword(Keyword::Let))?;
    
    let pattern = self.parse_pattern()?;

    self.expect(Token::Symbol(Symbol::Equals))?;
    
    let value = Box::new(self.parse_expression()?);
    
    self.expect(Token::Keyword(Keyword::In))?;
    
    let body = Box::new(self.parse_expression()?);

    Ok(Expression::Let {
        pattern,
        value,
        body,
    })
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
