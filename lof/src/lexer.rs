#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    Number(i64),
    Keyword(Keyword),
    Symbol(Symbol),
    Comment(String),
    EOF,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Keyword {
    Proof,
    Component,
    Enum,
    Type,
    Input,
    Witness,
    With,
    Field,
    Bits,
    Array,
    Nat,
    Bool,
    Refined,
    Match,
    Assert,
    Verify,
    Where,
    Let,
    In,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Symbol {
    LBrace,   // {
    RBrace,   // }
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LAngle,   // <
    RAngle,   // >

    FatArrow, // =>
    Pipe,     // |

    Colon, // :
    Semi,  // ;
    Comma, // ,
    Dot,   // .

    Equals,      // =
    TripleEqual, // === (constraint equality)
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /

    Range,      // ..
    Underscore, // _

    Greater,   // >
    Less,      // <
    GreaterEq, // >=
    LessEq,    // <=
    Equal,     // ==
    NotEqual,  // !=
    Not,       // !
    And,       // &&
    Or,        // ||
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

    fn peek_token(&mut self) -> Token {
        let saved_position = self.position;
        let saved_line = self.line;
        let saved_column = self.column;

        let token = self.next_token();

        self.position = saved_position;
        self.line = saved_line;
        self.column = saved_column;

        token
    }

    pub fn skip_comments(&mut self) {
        loop {
            let token = self.peek_token();
            match token {
                Token::Comment(_) => {
                    self.next_token();
                }
                _ => break,
            }
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
                    Token::Symbol(Symbol::LessEq)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Less))
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::GreaterEq)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Greater))
                }
            }
            ':' => self.advance_with(Token::Symbol(Symbol::Colon)),
            ';' => self.advance_with(Token::Symbol(Symbol::Semi)),
            ',' => self.advance_with(Token::Symbol(Symbol::Comma)),
            '|' => {
                if self.peek() == Some('|') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::Or)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Pipe))
                }
            }
            '=' => {
                if self.peek() == Some('=') && self.peek_ahead(2) == Some('=') {
                    self.position += 3;
                    self.column += 3;
                    Token::Symbol(Symbol::TripleEqual)
                } else if self.peek() == Some('=') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::Equal)
                } else if self.peek() == Some('>') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::FatArrow)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Equals))
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::NotEqual)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Not))
                }
            }
            '+' => self.advance_with(Token::Symbol(Symbol::Plus)),
            '-' => self.advance_with(Token::Symbol(Symbol::Minus)),
            '*' => self.advance_with(Token::Symbol(Symbol::Star)),
            '/' => {
                if self.peek() == Some('/') {
                    self.position += 2;
                    self.column += 2;

                    let start = self.position;
                    while self.position < self.input.len() && self.input[self.position] != '\n' {
                        self.position += 1;
                        self.column += 1;
                    }

                    let comment_text: String = self.input[start..self.position].iter().collect();
                    Token::Comment(comment_text)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Slash))
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::Range)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Dot))
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::And)
                } else {
                    let error_pos = (self.line, self.column);
                    panic!("Unexpected character '&' at {:?}", error_pos);
                }
            }
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
        while self.position < self.input.len()
            && (self.input[self.position].is_alphanumeric() || self.input[self.position] == '_')
        {
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
            "field" => Token::Keyword(Keyword::Field),
            "Field" => Token::Keyword(Keyword::Field),
            "Bits" => Token::Keyword(Keyword::Bits),
            "bits" => Token::Keyword(Keyword::Bits),
            "array" => Token::Keyword(Keyword::Array),
            "Array" => Token::Keyword(Keyword::Array),
            "nat" => Token::Keyword(Keyword::Nat),
            "Nat" => Token::Keyword(Keyword::Nat),
            "bool" => Token::Keyword(Keyword::Bool),
            "Bool" => Token::Keyword(Keyword::Bool),
            "match" => Token::Keyword(Keyword::Match),
            "with" => Token::Keyword(Keyword::With),
            "assert" => Token::Keyword(Keyword::Assert),
            "verify" => Token::Keyword(Keyword::Verify),
            "where" => Token::Keyword(Keyword::Where),
            "let" => Token::Keyword(Keyword::Let),
            "in" => Token::Keyword(Keyword::In),
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
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return None;
        }

        let current_char = self.current_char();

        let token = match current_char {
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
                    Token::Symbol(Symbol::LessEq)
                } else {
                    self.advance_with(Token::Symbol(Symbol::LAngle))
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::GreaterEq)
                } else {
                    self.advance_with(Token::Symbol(Symbol::RAngle))
                }
            }
            ':' => self.advance_with(Token::Symbol(Symbol::Colon)),
            ';' => self.advance_with(Token::Symbol(Symbol::Semi)),
            ',' => self.advance_with(Token::Symbol(Symbol::Comma)),
            '|' => {
                if self.peek() == Some('|') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::Or)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Pipe))
                }
            }
            '=' => {
                if self.peek() == Some('=') && self.peek_ahead(2) == Some('=') {
                    self.position += 3;
                    self.column += 3;
                    Token::Symbol(Symbol::TripleEqual)
                } else if self.peek() == Some('=') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::Equal)
                } else if self.peek() == Some('>') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::FatArrow)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Equals))
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::NotEqual)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Not))
                }
            }
            '+' => self.advance_with(Token::Symbol(Symbol::Plus)),
            '-' => self.advance_with(Token::Symbol(Symbol::Minus)),
            '*' => self.advance_with(Token::Symbol(Symbol::Star)),
            '/' => {
                if self.peek() == Some('/') {
                    self.position += 2;
                    self.column += 2;

                    let start = self.position;
                    while self.position < self.input.len() && self.input[self.position] != '\n' {
                        self.position += 1;
                        self.column += 1;
                    }

                    let comment_text: String = self.input[start..self.position].iter().collect();
                    Token::Comment(comment_text)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Slash))
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::Range)
                } else {
                    self.advance_with(Token::Symbol(Symbol::Dot))
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.position += 2;
                    self.column += 2;
                    Token::Symbol(Symbol::And)
                } else {
                    let error_pos = (self.line, self.column);
                    panic!("Unexpected character '&' at {:?}", error_pos);
                }
            }
            '_' => self.advance_with(Token::Symbol(Symbol::Underscore)),
            c if c.is_alphabetic() => self.read_identifier(),
            c if c.is_numeric() => self.read_number(),
            c => {
                let error_pos = (self.line, self.column);
                panic!("Unexpected character '{}' at {:?}", c, error_pos);
            }
        };
        Some(token)
    }
}
