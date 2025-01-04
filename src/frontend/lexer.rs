#[derive(Debug, Clone, PartialEq)]
pub enum Token {
  Identifier(String),
  Number(i64),
  Keyword(Keyword),
  Symbol(Symbol),
  EOF,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Keyword {
  Theorem,
  Private,
  Public,
  Nat,
  Bool,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Symbol {
  Colon,
  Equals,
  LParen,
  RParen,
  Comma,
  Multiply,
  Add,
  LessThan,
}

pub struct Lexer {
  input: Vec<char>,
  position: usize,
}

impl Lexer {
  pub fn new(input: &str) -> Self {
    Lexer {
      input: input.chars().collect(),
      position: 0,
    }
  }

  pub fn next_token(&mut self) -> Token {
    self.skip_whitespace();
    
    if self.position >= self.input.len() {
      return Token::EOF;
    }

    match self.current_char() {
      '(' => self.advance_with(Token::Symbol(Symbol::LParen)),
      ')' => self.advance_with(Token::Symbol(Symbol::RParen)),
      ':' => self.advance_with(Token::Symbol(Symbol::Colon)),
      '=' => self.advance_with(Token::Symbol(Symbol::Equals)),
      ',' => self.advance_with(Token::Symbol(Symbol::Comma)),
      '*' => self.advance_with(Token::Symbol(Symbol::Multiply)),
      '+' => self.advance_with(Token::Symbol(Symbol::Add)),
      '<' => self.advance_with(Token::Symbol(Symbol::LessThan)),
      c if c.is_alphabetic() => self.read_identifier(),
      c if c.is_numeric() => self.read_number(),
      _ => Token::EOF,
    }
  }

  fn current_char(&self) -> char {
    self.input[self.position]
  }

  fn advance_with(&mut self, token: Token) -> Token {
    self.position += 1;
    token
  }

  fn skip_whitespace(&mut self) {
    while self.position < self.input.len() {
      let c = self.input[self.position];
      if c.is_whitespace() || c == '\n' || c == '\r' {
        self.position += 1;
      } else {
        break;
      }
    }
  }

  fn read_identifier(&mut self) -> Token {
    let start = self.position;
    while self.position < self.input.len() && 
        (self.input[self.position].is_alphanumeric() || self.input[self.position] == '_') {
      self.position += 1;
    }
    
    let identifier: String = self.input[start..self.position].iter().collect();
    match identifier.as_str() {
      "theorem" => Token::Keyword(Keyword::Theorem),
      "Private" => Token::Keyword(Keyword::Private),
      "Public" => Token::Keyword(Keyword::Public),
      "Nat" => Token::Keyword(Keyword::Nat),
      "Bool" => Token::Keyword(Keyword::Bool),
      _ => Token::Identifier(identifier),
    }
  }

  fn read_number(&mut self) -> Token {
    let start = self.position;
    while self.position < self.input.len() && self.input[self.position].is_numeric() {
      self.position += 1;
    }
    
    let number: String = self.input[start..self.position].iter().collect();
    Token::Number(number.parse().unwrap())
  }
}
