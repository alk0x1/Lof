use lof::lexer::{Keyword, Lexer, Symbol, Token};

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

#[test]
fn test_iterator_interface() {
    let mut lexer = Lexer::new("let x = 1;");
    assert_eq!(lexer.next(), Some(Token::Keyword(Keyword::Let)));
    assert_eq!(lexer.next(), Some(Token::Identifier("x".to_string())));
    assert_eq!(lexer.next(), Some(Token::Symbol(Symbol::Equals)));
    assert_eq!(lexer.next(), Some(Token::Number(1)));
    assert_eq!(lexer.next(), Some(Token::Symbol(Symbol::Semi)));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_operators() {
    let mut lexer = Lexer::new("+ - * / == != === <= >= < > && ||");
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Plus));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Minus));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Star));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Slash));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Equal));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::NotEqual));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::TripleEqual));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::LessEq));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::GreaterEq));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Less));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Greater));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::And));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::Or));
}

#[test]
fn test_numbers() {
    let mut lexer = Lexer::new("42 0 999");
    assert_eq!(lexer.next_token(), Token::Number(42));
    assert_eq!(lexer.next_token(), Token::Number(0));
    assert_eq!(lexer.next_token(), Token::Number(999));
}

#[test]
fn test_comments() {
    let mut lexer = Lexer::new("x // this is a comment\ny");
    assert_eq!(lexer.next_token(), Token::Identifier("x".to_string()));
    assert_eq!(
        lexer.next_token(),
        Token::Comment(" this is a comment".to_string())
    );
    assert_eq!(lexer.next_token(), Token::Identifier("y".to_string()));
}

#[test]
fn test_whitespace_handling() {
    let mut lexer = Lexer::new("   x    y\n\tz   ");
    assert_eq!(lexer.next_token(), Token::Identifier("x".to_string()));
    assert_eq!(lexer.next_token(), Token::Identifier("y".to_string()));
    assert_eq!(lexer.next_token(), Token::Identifier("z".to_string()));
    assert_eq!(lexer.next_token(), Token::EOF);
}

#[test]
fn test_identifiers_and_underscores() {
    let mut lexer = Lexer::new("my_var CamelCase x1 var_123");
    assert_eq!(lexer.next_token(), Token::Identifier("my_var".to_string()));
    assert_eq!(
        lexer.next_token(),
        Token::Identifier("CamelCase".to_string())
    );
    assert_eq!(lexer.next_token(), Token::Identifier("x1".to_string()));
    assert_eq!(lexer.next_token(), Token::Identifier("var_123".to_string()));
}

#[test]
fn test_brackets_and_braces() {
    let mut lexer = Lexer::new("{ } ( ) [ ]");
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::LBrace));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::RBrace));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::LParen));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::RParen));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::LBracket));
    assert_eq!(lexer.next_token(), Token::Symbol(Symbol::RBracket));
}

#[test]
fn test_complete_proof_structure() {
    let input = r#"
    proof MyProof {
      input x: Field;
      witness w: Field;
      witness y: Field;

      let temp = x * w in
      assert y === temp;
    }"#;

    let tokens: Vec<Token> = Lexer::new(input).collect();

    let expected_tokens = vec![
        Token::Keyword(Keyword::Proof),
        Token::Identifier("MyProof".to_string()),
        Token::Symbol(Symbol::LBrace),
        Token::Keyword(Keyword::Input),
        Token::Identifier("x".to_string()),
        Token::Symbol(Symbol::Colon),
        Token::Keyword(Keyword::Field),
        Token::Symbol(Symbol::Semi),
        Token::Keyword(Keyword::Witness),
        Token::Identifier("w".to_string()),
        Token::Symbol(Symbol::Colon),
        Token::Keyword(Keyword::Field),
        Token::Symbol(Symbol::Semi),
        Token::Keyword(Keyword::Witness),
        Token::Identifier("y".to_string()),
        Token::Symbol(Symbol::Colon),
        Token::Keyword(Keyword::Field),
        Token::Symbol(Symbol::Semi),
        Token::Keyword(Keyword::Let),
        Token::Identifier("temp".to_string()),
        Token::Symbol(Symbol::Equals),
        Token::Identifier("x".to_string()),
        Token::Symbol(Symbol::Star),
        Token::Identifier("w".to_string()),
        Token::Keyword(Keyword::In),
        Token::Keyword(Keyword::Assert),
        Token::Identifier("y".to_string()),
        Token::Symbol(Symbol::TripleEqual),
        Token::Identifier("temp".to_string()),
        Token::Symbol(Symbol::Semi),
        Token::Symbol(Symbol::RBrace),
    ];

    assert_eq!(
        tokens.len(),
        expected_tokens.len(),
        "Expected {} tokens, but got {}",
        expected_tokens.len(),
        tokens.len()
    );

    for (i, (actual, expected)) in tokens.iter().zip(expected_tokens.iter()).enumerate() {
        assert_eq!(
            actual, expected,
            "Token mismatch at position {}: expected {:?}, got {:?}",
            i, expected, actual
        );
    }
}
