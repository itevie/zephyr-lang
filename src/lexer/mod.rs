pub mod lexer;
pub mod tokens;

#[cfg(test)]
mod test {
    use crate::lexer::tokens::TokenType;

    use super::lexer::lex;

    #[test]
    fn numbers() {
        let result = lex("2", String::new()).unwrap();

        assert!(
            matches!(result[0].t, TokenType::Number),
            "Expected token 0 to be number"
        );
        assert_eq!(result[0].value, "2", "Expected token 0 value to be \"2\"");
    }

    #[test]
    fn strings() {
        let result = lex(r#""test""#, String::new()).unwrap();

        assert!(
            matches!(result[0].t, TokenType::String),
            "Expected token 0 to be a string"
        );
        assert_eq!(
            result[0].value, "test",
            "Expected token 0 value to be \"test\""
        );
        assert_eq!(
            result[0].location.end, 6,
            "Expected token location end to be 6"
        );
    }

    #[test]
    fn string_escaping() {
        let result = lex(r#""test \\\"""#, String::new()).unwrap();

        assert_eq!(result[0].value, r#"test \""#);
    }

    #[test]
    fn string_newlines() {
        let result = lex("\"hello\n\"", String::new());
        assert!(result.is_err());
    }

    #[test]
    fn identifiers() {
        let result = lex("test", String::new()).unwrap();
        assert_eq!(result[0].value, "test");
    }

    #[test]
    fn identifier_mut() {
        let result = lex("test!", String::new()).unwrap();
        assert_eq!(result[0].value, "test!");
    }
}
