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
  // Circuit definitions
  Proof,          // proof declaration
  Component,      // reusable component
  
  // Type definitions
  Enum,           // enum type definition
  Type,           // type alias
  
  // Visibility modifiers
  Input,          // public input
  Witness,        // private witness
  Output,         // public output
  
  // Types
  Field,          // field element
  Bits,          // bit array
  Array,          // array type
  Nat,           // natural number
  Bool,          // boolean
  
  // Pattern matching
  Match,          // match keyword
  
  // Circuit constraints
  Assert,         // assert constraint
  Verify,         // verify constraint
  Where,          // type constraints
  Let,            // let binding
  
  // Refined type
  Refined         // No comma after the last item
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Symbol {
  // Braces and brackets
  LBrace,         // {
  RBrace,         // }
  LParen,         // (
  RParen,         // )
  LBracket,       // [
  RBracket,       // ]
  LAngle,         // <
  RAngle,         // >
  
  // Pattern matching
  FatArrow,       // =>
  Pipe,           // |
  
  // Punctuation
  Colon,          // :
  Semi,           // ;
  Comma,          // ,
  Dot,            // .
  
  // Operators
  Equals,         // =
  TripleEqual,    // === (constraint equality)
  Plus,           // +
  Minus,          // -
  Star,           // *
  Slash,          // /

  // Comparison operators
  Lt,             // <
  Gt,             // >
  Le,             // <=
  Ge,             // >=
  Ne,             // !=
  Not,            // !
  
  // Type operators
  Range,          // ..
  Underscore     // _
}

pub struct Lexer {
  input: Vec<char>,
  position: usize,
  line: usize,
  column: usize,
}

impl Lexer {
  pub fn new(input: &str) -> Self {
    Lexer {
      input: input.chars().collect(),
      position: 0,
      line: 1,
      column: 1,
    }
  }

    pub fn next_token(&mut self) -> Token {
    self.skip_whitespace();
    
    if self.position >= self.input.len() {
      return Token::EOF;
    }

    match self.current_char() {
      '{' => self.advance_with(Token::Symbol(Symbol::LBrace)),
      '}' => self.advance_with(Token::Symbol(Symbol::RBrace)),
      '(' => self.advance_with(Token::Symbol(Symbol::LParen)),
      ')' => self.advance_with(Token::Symbol(Symbol::RParen)),
      '[' => self.advance_with(Token::Symbol(Symbol::LBracket)),
      ']' => self.advance_with(Token::Symbol(Symbol::RBracket)),
      '<' => {
        if self.peek() == Some('=') {
          self.position += 2;
          self.column += 2;
          Token::Symbol(Symbol::Le)
        } else {
          self.advance_with(Token::Symbol(Symbol::LAngle))
        }
      },
      '>' => {
        if self.peek() == Some('=') {
          self.position += 2;
          self.column += 2;
          Token::Symbol(Symbol::Ge)
        } else {
          self.advance_with(Token::Symbol(Symbol::RAngle))
        }
      },
      ':' => self.advance_with(Token::Symbol(Symbol::Colon)),
      ';' => self.advance_with(Token::Symbol(Symbol::Semi)),
      ',' => self.advance_with(Token::Symbol(Symbol::Comma)),
      '|' => self.advance_with(Token::Symbol(Symbol::Pipe)),
      '=' => {
        if self.peek() == Some('=') && self.peek_ahead(2) == Some('=') {
          self.position += 3;
          self.column += 3;
          Token::Symbol(Symbol::TripleEqual)
        } else if self.peek() == Some('>') {
          self.position += 2;
          self.column += 2;
          Token::Symbol(Symbol::FatArrow)
        } else {
          self.advance_with(Token::Symbol(Symbol::Equals))
        }
      },
      '!' => {
        if self.peek() == Some('=') {
          self.position += 2;
          self.column += 2;
          Token::Symbol(Symbol::Ne)
        } else {
          self.advance_with(Token::Symbol(Symbol::Not))
        }
      },
      '+' => self.advance_with(Token::Symbol(Symbol::Plus)),
      '-' => self.advance_with(Token::Symbol(Symbol::Minus)),
      '*' => self.advance_with(Token::Symbol(Symbol::Star)),
      '/' => self.advance_with(Token::Symbol(Symbol::Slash)),
      '.' => {
        if self.peek() == Some('.') {
          self.position += 2;
          self.column += 2;
          Token::Symbol(Symbol::Range)
        } else {
          self.advance_with(Token::Symbol(Symbol::Dot))
        }
      },
      c if c.is_alphabetic() => self.read_identifier(),
      c if c.is_numeric() => self.read_number(),
      c => {
        let error_pos = (self.line, self.column);
        panic!("Unexpected character '{}' at {:?}", c, error_pos);
      }
    }
  }

    fn read_identifier(&mut self) -> Token {
    let start = self.position;
    while self.position < self.input.len() && 
          (self.input[self.position].is_alphanumeric() || self.input[self.position] == '_') {
      self.position += 1;
      self.column += 1;
    }
    
    let identifier: String = self.input[start..self.position].iter().collect();
    match identifier.as_str() {
      "proof" => Token::Keyword(Keyword::Proof),
      "component" => Token::Keyword(Keyword::Component),
      "enum" => Token::Keyword(Keyword::Enum),
      "type" => Token::Keyword(Keyword::Type),
      "input" => Token::Keyword(Keyword::Input),
      "witness" => Token::Keyword(Keyword::Witness),
      "output" => Token::Keyword(Keyword::Output),
      "Field" => Token::Keyword(Keyword::Field),
      "Bits" => Token::Keyword(Keyword::Bits),
      "Array" => Token::Keyword(Keyword::Array),
      "Nat" => Token::Keyword(Keyword::Nat),
      "Bool" => Token::Keyword(Keyword::Bool),
      "match" => Token::Keyword(Keyword::Match),
      "assert" => Token::Keyword(Keyword::Assert),
      "verify" => Token::Keyword(Keyword::Verify),
      "where" => Token::Keyword(Keyword::Where),
      "let" => Token::Keyword(Keyword::Let),
      
      "refined" => Token::Keyword(Keyword::Refined),
      "Refined" => Token::Keyword(Keyword::Refined),
      
      _ => Token::Identifier(identifier),
    }
  }

  fn read_number(&mut self) -> Token {
    let start = self.position;
    while self.position < self.input.len() && self.input[self.position].is_numeric() {
      self.position += 1;
      self.column += 1;
    }
    
    let number: String = self.input[start..self.position].iter().collect();
    Token::Number(number.parse().unwrap())
  }

  fn current_char(&self) -> char {
    self.input[self.position]
  }

  fn peek(&self) -> Option<char> {
    if self.position + 1 < self.input.len() {
      Some(self.input[self.position + 1])
    } else {
      None
    }
  }

  fn peek_ahead(&self, n: usize) -> Option<char> {
    if self.position + n < self.input.len() {
      Some(self.input[self.position + n])
    } else {
      None
    }
  }

  fn advance_with(&mut self, token: Token) -> Token {
    self.position += 1;
    self.column += 1;
    token
  }

  fn skip_whitespace(&mut self) {
    while self.position < self.input.len() {
      match self.input[self.position] {
        ' ' | '\t' => {
          self.position += 1;
          self.column += 1;
        }
        '\n' => {
          self.position += 1;
          self.line += 1;
          self.column = 1;
        }
        '\r' => {
          self.position += 1;
        }
        _ => break,
      }
    }
  }
}

impl Iterator for Lexer {
  type Item = Token;
  
  fn next(&mut self) -> Option<Self::Item> {
    let token = self.next_token();
    if token == Token::EOF {
      None
    } else {
      Some(token)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic_tokens() {
    let mut lexer = Lexer::new("proof Test { input x: Field }");
    assert_eq!(lexer.next_token(), Token::Keyword(Keyword::Proof));
    assert_eq!(lexer.next_token(), Token::Identifier("Test".to_string()));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::LBrace));
    assert_eq!(lexer.next_token(), Token::Keyword(Keyword::Input));
    assert_eq!(lexer.next_token(), Token::Identifier("x".to_string()));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Colon));
    assert_eq!(lexer.next_token(), Token::Keyword(Keyword::Field));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::RBrace));
  }

  #[test]
  fn test_pattern_matching() {
    let mut lexer = Lexer::new("match x { Case => value }");
    assert_eq!(lexer.next_token(), Token::Keyword(Keyword::Match));
    assert_eq!(lexer.next_token(), Token::Identifier("x".to_string()));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::LBrace));
    assert_eq!(lexer.next_token(), Token::Identifier("Case".to_string()));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::FatArrow));
    assert_eq!(lexer.next_token(), Token::Identifier("value".to_string()));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::RBrace));
  }
}