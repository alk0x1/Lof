use super::ast::{Expression, Type, Parameter, Operator};
use super::lexer::{Token, Keyword, Symbol, Lexer};

pub struct Parser {
  lexer: Lexer,
  current_token: Token,
}

impl Parser {
  pub fn new(input: &str) -> Self {
    let mut lexer = Lexer::new(input);
    let current_token = lexer.next_token();
    Parser {
      lexer,
      current_token,
    }
  }

  pub fn parse_theorem(&mut self) -> Result<Expression, String> {
    self.expect_keyword(Keyword::Theorem)?;
    
    let name = match &self.current_token {
      Token::Identifier(id) => id.clone(),
      _ => return Err("Expected theorem name".to_string()),
    };
    self.advance();

    let inputs = self.parse_parameters()?;
    
    self.expect_symbol(Symbol::Colon)?;
    let output = Type::Bool;
    let body = self.parse_expression()?;

    Ok(Expression::Theorem {
      name,
      inputs,
      output,
      body: Box::new(body),
    })
  }

  fn parse_type(&mut self) -> Result<Type, String> {
    println!("Parsing type, current token: {:?}", self.current_token);
    match &self.current_token {
      Token::Keyword(Keyword::Private) => {
        self.advance();
        Ok(Type::Private(Box::new(self.parse_type()?)))
      },
      Token::Keyword(Keyword::Public) => {
        self.advance();
        Ok(Type::Public(Box::new(self.parse_type()?)))
      },
      Token::Keyword(Keyword::Nat) => {
        self.advance();
        Ok(Type::Nat)
      },
      Token::Keyword(Keyword::Bool) => {
        self.advance();
        Ok(Type::Bool)
      },
      _ => Err(format!("Expected type, got {:?}", self.current_token)),
    }
  }

  fn parse_parameters(&mut self) -> Result<Vec<Parameter>, String> {
    let mut parameters = Vec::new();
    
    while self.current_token == Token::Symbol(Symbol::LParen) {
      self.advance();
      
      let name = match &self.current_token {
        Token::Identifier(id) => id.clone(),
        _ => return Err("Expected parameter name".to_string()),
      };
      self.advance();
      
      self.expect_symbol(Symbol::Colon)?;
      let typ = self.parse_type()?;
      
      parameters.push(Parameter { name, typ });
      self.expect_symbol(Symbol::RParen)?;
    }
    
    Ok(parameters)
  }

  fn parse_expression(&mut self) -> Result<Expression, String> {
    let left = match &self.current_token {
      Token::Identifier(id) => {
        let name = id.clone();
        self.advance();
        Expression::Variable(name)
      },
      Token::Number(n) => {
        let value = *n;
        self.advance();
        Expression::Number(value)
      },
      Token::Symbol(Symbol::LParen) => {
        self.advance();
        let expr = self.parse_expression()?;
        self.expect_symbol(Symbol::RParen)?;
        expr
      },
      _ => return Err("Invalid expression".to_string()),
    };

    match &self.current_token {
      Token::Symbol(Symbol::Equals) => {
        self.advance();
        let right = self.parse_expression()?;
        Ok(Expression::BinaryOp {
          left: Box::new(left),
          operator: Operator::Equals,
          right: Box::new(right),
        })
      },
      Token::Symbol(Symbol::Multiply) => {
        self.advance();
        let right = self.parse_expression()?;
        Ok(Expression::BinaryOp {
          left: Box::new(left),
          operator: Operator::Multiply,
          right: Box::new(right),
        })
      },
      Token::Symbol(Symbol::Add) => {
        self.advance();
        let right = self.parse_expression()?;
        Ok(Expression::BinaryOp {
          left: Box::new(left),
          operator: Operator::Add,
          right: Box::new(right),
        })
      },
      Token::Symbol(Symbol::LessThan) => {
        self.advance();
        let right = self.parse_expression()?;
        Ok(Expression::BinaryOp {
          left: Box::new(left),
          operator: Operator::LessThan,
          right: Box::new(right),
        })
      },
      _ => Ok(left),
    }
  }

  fn advance(&mut self) {
    self.current_token = self.lexer.next_token();
  }

  fn expect_keyword(&mut self, keyword: Keyword) -> Result<(), String> {
    if self.current_token == Token::Keyword(keyword) {
      self.advance();
      Ok(())
    } else {
      Err(format!("Expected keyword {:?}", keyword))
    }
  }

  fn expect_symbol(&mut self, symbol: Symbol) -> Result<(), String> {
    if self.current_token == Token::Symbol(symbol) {
      self.advance();
      Ok(())
    } else {
      Err(format!("Expected symbol {:?}", symbol))
    }
  }
}
