use crate::ast::{Token, token::Type as TokenType};
use crate::error::{Error, Result};
use regex::{Captures, Regex};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

static KEYWORD_TOKENS: LazyLock<BTreeMap<&str, TokenType>> = LazyLock::new(|| {
    BTreeMap::from([
        ("assert", TokenType::TokenTypeKeywordAssert),
        ("break", TokenType::TokenTypeKeywordBreak),
        ("bus", TokenType::TokenTypeKeywordBus),
        ("component", TokenType::TokenTypeKeywordComponent),
        ("const", TokenType::TokenTypeKeywordConst),
        ("continue", TokenType::TokenTypeKeywordContinue),
        ("do", TokenType::TokenTypeKeywordDo),
        ("else", TokenType::TokenTypeKeywordElse),
        ("false", TokenType::TokenTypeKeywordFalse),
        ("for", TokenType::TokenTypeKeywordFor),
        ("function", TokenType::TokenTypeKeywordFunction),
        ("if", TokenType::TokenTypeKeywordIf),
        ("include", TokenType::TokenTypeKeywordInclude),
        ("input", TokenType::TokenTypeKeywordInput),
        ("log", TokenType::TokenTypeKeywordLog),
        ("output", TokenType::TokenTypeKeywordOutput),
        ("parallel", TokenType::TokenTypeKeywordParallel),
        ("pragma", TokenType::TokenTypeKeywordPragma),
        ("public", TokenType::TokenTypeKeywordPublic),
        ("return", TokenType::TokenTypeKeywordReturn),
        ("signal", TokenType::TokenTypeKeywordSignal),
        ("starkom", TokenType::TokenTypeKeywordStarkom),
        ("template", TokenType::TokenTypeKeywordTemplate),
        ("true", TokenType::TokenTypeKeywordTrue),
        ("var", TokenType::TokenTypeKeywordVar),
        ("while", TokenType::TokenTypeKeywordWhile),
    ])
});

static SYMBOL_TOKENS: LazyLock<BTreeMap<&'static str, TokenType>> = LazyLock::new(|| {
    BTreeMap::from([
        ("<--", TokenType::TokenTypeOperatorUnconstrainedAssignLeft),
        ("-->", TokenType::TokenTypeOperatorUnconstrainedAssignRight),
        ("<==", TokenType::TokenTypeOperatorConstrainedAssignLeft),
        ("==>", TokenType::TokenTypeOperatorConstrainedAssignRight),
        ("===", TokenType::TokenTypeOperatorConstrainedEquality),
        ("==", TokenType::TokenTypeOperatorCompareEqual),
        ("!=", TokenType::TokenTypeOperatorCompareNotEqual),
        ("<=", TokenType::TokenTypeOperatorLessThanOrEqualTo),
        ("<", TokenType::TokenTypeOperatorLessThan),
        (">=", TokenType::TokenTypeOperatorGreaterThanOrEqualTo),
        (">", TokenType::TokenTypeOperatorGreaterThan),
        ("++", TokenType::TokenTypeOperatorIncrement),
        ("--", TokenType::TokenTypeOperatorDecrement),
        ("<<", TokenType::TokenTypeOperatorShiftLeft),
        (">>", TokenType::TokenTypeOperatorShiftRight),
        ("&&", TokenType::TokenTypeOperatorLogicalAnd),
        ("||", TokenType::TokenTypeOperatorLogicalOr),
        ("!", TokenType::TokenTypeOperatorLogicalNot),
        ("+", TokenType::TokenTypeOperatorPlus),
        ("-", TokenType::TokenTypeOperatorMinus),
        ("*", TokenType::TokenTypeOperatorMultiply),
        ("**", TokenType::TokenTypeOperatorPower),
        ("/", TokenType::TokenTypeOperatorDivide),
        ("\\", TokenType::TokenTypeOperatorDivideInteger),
        ("%", TokenType::TokenTypeOperatorModulus),
        ("&", TokenType::TokenTypeOperatorBitwiseAnd),
        ("|", TokenType::TokenTypeOperatorBitwiseOr),
        ("^", TokenType::TokenTypeOperatorBitwiseXor),
        ("~", TokenType::TokenTypeOperatorBitwiseNot),
        ("=", TokenType::TokenTypeOperatorAssign),
        ("+=", TokenType::TokenTypeOperatorCompoundAdd),
        ("-=", TokenType::TokenTypeOperatorCompoundSubtract),
        ("*=", TokenType::TokenTypeOperatorCompoundMultiply),
        ("**=", TokenType::TokenTypeOperatorCompoundPower),
        ("/=", TokenType::TokenTypeOperatorCompoundDivide),
        ("\\=", TokenType::TokenTypeOperatorCompoundDivideInteger),
        ("%=", TokenType::TokenTypeOperatorCompoundModulus),
        ("&&=", TokenType::TokenTypeOperatorCompoundLogicalAnd),
        ("||=", TokenType::TokenTypeOperatorCompoundLogicalOr),
        ("&=", TokenType::TokenTypeOperatorCompoundBitwiseAnd),
        ("|=", TokenType::TokenTypeOperatorCompoundBitwiseOr),
        ("^=", TokenType::TokenTypeOperatorCompoundBitwiseXor),
        ("<<=", TokenType::TokenTypeOperatorCompoundShiftLeft),
        (">>=", TokenType::TokenTypeOperatorCompoundShiftRight),
        ("(", TokenType::TokenTypeLeftParenthesis),
        (")", TokenType::TokenTypeRightParenthesis),
        ("[", TokenType::TokenTypeLeftSquareBracket),
        ("]", TokenType::TokenTypeRightSquareBracket),
        ("{", TokenType::TokenTypeLeftCurlyBracket),
        ("}", TokenType::TokenTypeRightCurlyBracket),
        (".", TokenType::TokenTypeDot),
        (",", TokenType::TokenTypeComma),
        (":", TokenType::TokenTypeColon),
        (";", TokenType::TokenTypeSemicolon),
    ])
});

static REGEX_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s+").unwrap());

static REGEX_VERSION_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{1,9}\.\d{1,9}\.\d{1,9})\b").unwrap());

static REGEX_SINGLE_LINE_COMMENT_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^//").unwrap());
static REGEX_SINGLE_LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^//([^\n]*)(?:\n|$)").unwrap());

static REGEX_MULTI_LINE_COMMENT_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/\*").unwrap());
static REGEX_MULTI_LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/\*((?:[^*]|\*+[^/*])*\**)\*/").unwrap());

static REGEX_IDENTIFIER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9$_]*\b").unwrap());

static REGEX_NUMBER_8: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0[0-7]+\b").unwrap());
static REGEX_NUMBER_10: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:0|[1-9]\d*)\b").unwrap());
static REGEX_NUMBER_16: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^0x[0-9a-fA-F]+\b").unwrap());

// TODO: handle escape codes -- the current pattern simply rejects them.
static REGEX_STRING_LITERAL: LazyLock<Regex> = LazyLock::new(|| Regex::new("^\"[^\"]*\"").unwrap());

static SYMBOLS_LEN3: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    BTreeSet::from([
        "<--", "-->", "<==", "==>", "===", "**=", "&&=", "||=", "<<=", ">>=",
    ])
});

static SYMBOLS_LEN2: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    BTreeSet::from([
        "==", "!=", "<=", ">=", "++", "--", "<<", ">>", "&&", "||", "**", "+=", "-=", "*=", "/=",
        "\\=", "%=", "&=", "|=", "^=",
    ])
});

static SYMBOLS_LEN1: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    BTreeSet::from([
        "<", ">", "!", "=", "+", "-", "*", "/", "\\", "%", "&", "|", "^", "~", "(", ")", "[", "]",
        "{", "}", ".", ",", ":", ";",
    ])
});

#[derive(Debug, Clone)]
struct Lexer<'a> {
    path: &'a str,
    input: &'a str,
    current_offset: usize,
    last_offset: usize,
    line_starts: Vec<usize>,
}

impl<'a> Lexer<'a> {
    fn new(path: &'a str, input: &'a str) -> Self {
        Self {
            path,
            input,
            current_offset: 0,
            last_offset: 0,
            line_starts: vec![0],
        }
    }

    fn error<T>(&self, message: &str) -> Result<T> {
        Err(Error::new(
            self.path.to_string(),
            message.to_string(),
            self.last_offset,
            self.line_starts.as_slice(),
        ))
    }

    fn token(&self, token: TokenType) -> Token {
        Token {
            offset: self.last_offset as u32,
            r#type: token.into(),
            label: String::default(),
        }
    }

    fn token_with_label(&self, token: TokenType, label: String) -> Token {
        Token {
            offset: self.last_offset as u32,
            r#type: token.into(),
            label,
        }
    }

    fn add_line_starts(&mut self, span: usize) {
        let input = self.input.as_bytes();
        for i in 0..span {
            if input[i] == 10 {
                self.line_starts.push(self.current_offset + i + 1);
            }
        }
    }

    fn advance(&mut self, count: usize) {
        self.add_line_starts(count);
        self.input = &self.input[count..];
        self.last_offset = self.current_offset;
        self.current_offset += count;
    }

    fn match_prefix(&mut self, pattern: &Regex) -> bool {
        pattern.is_match(self.input)
    }

    fn consume_prefix(&mut self, pattern: &Regex) -> Option<Captures<'a>> {
        match pattern.captures(self.input) {
            Some(captures) => {
                let n = captures[0].len();
                self.advance(n);
                Some(captures)
            }
            None => None,
        }
    }

    fn consume_any_prefix(&mut self, prefixes: &BTreeSet<&'static str>) -> Option<&'a str> {
        for &prefix in prefixes {
            if self.input.starts_with(prefix) {
                let result = &self.input[0..prefix.len()];
                self.advance(prefix.len());
                return Some(result);
            }
        }
        None
    }

    fn consume_symbol(&mut self) -> Option<&'a str> {
        if let Some(prefix) = self.consume_any_prefix(&SYMBOLS_LEN3) {
            return Some(prefix);
        }
        if let Some(prefix) = self.consume_any_prefix(&SYMBOLS_LEN2) {
            return Some(prefix);
        }
        if let Some(prefix) = self.consume_any_prefix(&SYMBOLS_LEN1) {
            return Some(prefix);
        }
        None
    }

    fn tokenize(mut self) -> Result<(Vec<Token>, Vec<usize>)> {
        let mut tokens = Vec::new();
        self.consume_prefix(&REGEX_WHITESPACE);
        while !self.input.is_empty() {
            if let Some(captures) = self.consume_prefix(&REGEX_VERSION_NUMBER) {
                tokens.push(
                    self.token_with_label(
                        TokenType::TokenTypeVersionNumber,
                        captures[1].to_string(),
                    ),
                );
            } else if self.match_prefix(&REGEX_SINGLE_LINE_COMMENT_START) {
                match self.consume_prefix(&REGEX_SINGLE_LINE_COMMENT) {
                    Some(captures) => {
                        tokens.push(self.token_with_label(
                            TokenType::TokenTypeSingleLineComment,
                            captures[1].to_string(),
                        ));
                    }
                    None => {
                        return self.error("syntax error");
                    }
                };
            } else if self.match_prefix(&REGEX_MULTI_LINE_COMMENT_START) {
                match self.consume_prefix(&REGEX_MULTI_LINE_COMMENT) {
                    Some(captures) => {
                        tokens.push(self.token_with_label(
                            TokenType::TokenTypeMultiLineComment,
                            captures[1].to_string(),
                        ));
                    }
                    None => {
                        return self.error("syntax error");
                    }
                };
            } else if let Some(captures) = self.consume_prefix(&REGEX_IDENTIFIER) {
                let capture = &captures[0];
                match KEYWORD_TOKENS.get(capture) {
                    Some(token) => tokens.push(self.token(token.clone())),
                    None => tokens.push(
                        self.token_with_label(TokenType::TokenTypeIdentifier, capture.to_string()),
                    ),
                };
            } else if let Some(captures) = self.consume_prefix(&REGEX_NUMBER_8) {
                tokens.push(
                    self.token_with_label(TokenType::TokenTypeNumber8, captures[0].to_string()),
                );
            } else if let Some(captures) = self.consume_prefix(&REGEX_NUMBER_16) {
                tokens.push(
                    self.token_with_label(TokenType::TokenTypeNumber16, captures[0].to_string()),
                );
            } else if let Some(captures) = self.consume_prefix(&REGEX_NUMBER_10) {
                tokens.push(
                    self.token_with_label(TokenType::TokenTypeNumber10, captures[0].to_string()),
                );
            } else if let Some(captures) = self.consume_prefix(&REGEX_STRING_LITERAL) {
                tokens.push(
                    self.token_with_label(
                        TokenType::TokenTypeStringLiteral,
                        captures[0].to_string(),
                    ),
                );
            } else if let Some(symbol) = self.consume_symbol() {
                let token = SYMBOL_TOKENS.get(symbol).unwrap();
                tokens.push(self.token(token.clone()));
            } else {
                return self.error("syntax error");
            }
            self.consume_prefix(&REGEX_WHITESPACE);
        }
        tokens.push(Token {
            offset: self.current_offset as u32,
            r#type: TokenType::TokenTypeEndOfFile.into(),
            label: String::default(),
        });
        Ok((tokens, self.line_starts))
    }
}

/// Reads an input Starkom source file and produces the corresponding array of lexical tokens.
///
/// The second component of the returned pair is the array of line start offsets (see the
/// `File::line_starts` field in `ast.proto` for details).
pub fn tokenize(path: &str, input: &str) -> Result<(Vec<Token>, Vec<usize>)> {
    Lexer::new(path, input).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(pos: usize, token: TokenType) -> Token {
        Token {
            offset: pos as u32,
            r#type: token.into(),
            label: String::default(),
        }
    }

    fn token_with_label(pos: usize, token: TokenType, label: &str) -> Token {
        Token {
            offset: pos as u32,
            r#type: token.into(),
            label: label.to_string(),
        }
    }

    fn tokenize(input: &str) -> Result<Vec<Token>> {
        let (tokens, _) = super::tokenize("<test>", input)?;
        Ok(tokens)
    }

    #[test]
    fn test_version_number() {
        assert_eq!(
            tokenize("1.2.3").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeVersionNumber, "1.2.3"),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("44.55.66").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeVersionNumber, "44.55.66"),
                token(8, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_single_line_comment_basic() {
        assert_eq!(
            tokenize("// hello").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeSingleLineComment, " hello"),
                token(8, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_single_line_comment_empty() {
        assert_eq!(
            tokenize("//").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeSingleLineComment, ""),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("//\n").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeSingleLineComment, ""),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_single_line_comment_consumes_newline() {
        // Newline is consumed by the comment; subsequent token starts right after.
        assert_eq!(
            tokenize("// comment\nx").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeSingleLineComment, " comment"),
                token_with_label(11, TokenType::TokenTypeIdentifier, "x"),
                token(12, TokenType::TokenTypeEndOfFile),
            ]
        );
    }

    #[test]
    fn test_single_line_comment_non_zero_position() {
        assert_eq!(
            tokenize("x // comment").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeIdentifier, "x"),
                token_with_label(2, TokenType::TokenTypeSingleLineComment, " comment"),
                token(12, TokenType::TokenTypeEndOfFile),
            ]
        );
    }

    #[test]
    fn test_single_line_comment_multiple() {
        assert_eq!(
            tokenize("// first\n// second").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeSingleLineComment, " first"),
                token_with_label(9, TokenType::TokenTypeSingleLineComment, " second"),
                token(18, TokenType::TokenTypeEndOfFile),
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_basic() {
        assert_eq!(
            tokenize("/* hello */").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, " hello "),
                token(11, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_empty() {
        assert_eq!(
            tokenize("/**/").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, ""),
                token(4, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_asterisks() {
        assert_eq!(
            tokenize("/***/").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, "*"),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/****/").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, "**"),
                token(6, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/* ***/").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, " **"),
                token(7, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/*** */").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, "** "),
                token(7, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/* ** */").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, " ** "),
                token(8, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_slash() {
        assert_eq!(
            tokenize("/*/*/").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, "/"),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_spans_lines() {
        assert_eq!(
            tokenize("/* line1\nline2 */").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeMultiLineComment, " line1\nline2 "),
                token(17, TokenType::TokenTypeEndOfFile),
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_non_zero_position() {
        assert_eq!(
            tokenize("x /* note */ y").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeIdentifier, "x"),
                token_with_label(2, TokenType::TokenTypeMultiLineComment, " note "),
                token_with_label(13, TokenType::TokenTypeIdentifier, "y"),
                token(14, TokenType::TokenTypeEndOfFile),
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_unterminated_is_error() {
        assert!(tokenize("/* unterminated").is_err());
    }

    #[test]
    fn test_identifiers() {
        assert_eq!(
            tokenize("foo").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeIdentifier, "foo"),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("bar").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeIdentifier, "bar"),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("foo42").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeIdentifier, "foo42"),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("foo12bar34").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeIdentifier, "foo12bar34"),
                token(10, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            tokenize("assert").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordAssert),
                token(6, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("break").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordBreak),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("bus").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordBus),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("component").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordComponent),
                token(9, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("const").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordConst),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("continue").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordContinue),
                token(8, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("do").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordDo),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("else").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordElse),
                token(4, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("false").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordFalse),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("for").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordFor),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("function").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordFunction),
                token(8, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("if").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordIf),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("include").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordInclude),
                token(7, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("input").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordInput),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("log").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordLog),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("output").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordOutput),
                token(6, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("parallel").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordParallel),
                token(8, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("pragma").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordPragma),
                token(6, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("public").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordPublic),
                token(6, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("return").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordReturn),
                token(6, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("signal").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordSignal),
                token(6, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("starkom").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordStarkom),
                token(7, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("template").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordTemplate),
                token(8, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("true").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordTrue),
                token(4, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("var").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordVar),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("while").unwrap(),
            vec![
                token(0, TokenType::TokenTypeKeywordWhile),
                token(5, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_number_decimal_zero() {
        assert_eq!(
            tokenize("0").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber10, "0"),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_number_decimal_basic() {
        assert_eq!(
            tokenize("42").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber10, "42"),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("123").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber10, "123"),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("1000000").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber10, "1000000"),
                token(7, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_number_octal_basic() {
        assert_eq!(
            tokenize("077").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber8, "077"),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("0755").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber8, "0755"),
                token(4, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_number_hex_basic() {
        assert_eq!(
            tokenize("0xFF").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber16, "0xFF"),
                token(4, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("0x1a").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber16, "0x1a"),
                token(4, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("0x1ee7").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeNumber16, "0x1ee7"),
                token(6, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_string_literal_basic() {
        assert_eq!(
            tokenize("\"hello\"").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeStringLiteral, "\"hello\""),
                token(7, TokenType::TokenTypeEndOfFile),
            ]
        );
    }

    #[test]
    fn test_string_literal_empty() {
        assert_eq!(
            tokenize("\"\"").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeStringLiteral, "\"\""),
                token(2, TokenType::TokenTypeEndOfFile),
            ]
        );
    }

    #[test]
    fn test_string_literal_with_spaces() {
        assert_eq!(
            tokenize("\"hello world\"").unwrap(),
            vec![
                token_with_label(0, TokenType::TokenTypeStringLiteral, "\"hello world\""),
                token(13, TokenType::TokenTypeEndOfFile),
            ]
        );
    }

    #[test]
    fn test_symbols() {
        assert_eq!(
            tokenize("(").unwrap(),
            vec![
                token(0, TokenType::TokenTypeLeftParenthesis),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(")").unwrap(),
            vec![
                token(0, TokenType::TokenTypeRightParenthesis),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("[").unwrap(),
            vec![
                token(0, TokenType::TokenTypeLeftSquareBracket),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("]").unwrap(),
            vec![
                token(0, TokenType::TokenTypeRightSquareBracket),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("{").unwrap(),
            vec![
                token(0, TokenType::TokenTypeLeftCurlyBracket),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("}").unwrap(),
            vec![
                token(0, TokenType::TokenTypeRightCurlyBracket),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(".").unwrap(),
            vec![
                token(0, TokenType::TokenTypeDot),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(",").unwrap(),
            vec![
                token(0, TokenType::TokenTypeComma),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(":").unwrap(),
            vec![
                token(0, TokenType::TokenTypeColon),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(";").unwrap(),
            vec![
                token(0, TokenType::TokenTypeSemicolon),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_operators() {
        assert_eq!(
            tokenize("<--").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorUnconstrainedAssignLeft),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("-->").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorUnconstrainedAssignRight),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<==").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorConstrainedAssignLeft),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("==>").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorConstrainedAssignRight),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("===").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorConstrainedEquality),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("==").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompareEqual),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("!=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompareNotEqual),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorLessThanOrEqualTo),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorLessThan),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(">=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorGreaterThanOrEqualTo),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(">").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorGreaterThan),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("++").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorIncrement),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("--").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorDecrement),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<<").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorShiftLeft),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(">>").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorShiftRight),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("&&").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorLogicalAnd),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("||").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorLogicalOr),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("!").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorLogicalNot),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("+").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorPlus),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("-").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorMinus),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("**").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorPower),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("*").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorMultiply),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorDivide),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("\\").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorDivideInteger),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("%").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorModulus),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("&").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorBitwiseAnd),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("|").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorBitwiseOr),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("^").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorBitwiseXor),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("~").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorBitwiseNot),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorAssign),
                token(1, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("+=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundAdd),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("-=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundSubtract),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("**=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundPower),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("*=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundMultiply),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundDivide),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("\\=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundDivideInteger),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("%=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundModulus),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("&&=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundLogicalAnd),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("||=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundLogicalOr),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("&=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundBitwiseAnd),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("|=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundBitwiseOr),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("^=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundBitwiseXor),
                token(2, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<<=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundShiftLeft),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            tokenize(">>=").unwrap(),
            vec![
                token(0, TokenType::TokenTypeOperatorCompoundShiftRight),
                token(3, TokenType::TokenTypeEndOfFile)
            ]
        );
    }

    #[test]
    fn test_empty_file() {
        assert_eq!(
            tokenize("").unwrap(),
            vec![token(0, TokenType::TokenTypeEndOfFile)]
        );
        assert_eq!(
            tokenize("\n").unwrap(),
            vec![token(1, TokenType::TokenTypeEndOfFile)]
        );
        assert_eq!(
            tokenize(" \n \n ").unwrap(),
            vec![token(5, TokenType::TokenTypeEndOfFile)]
        );
    }

    #[test]
    fn test_vitalik() {
        static VITALIK: &'static str = include_str!("../test/vitalik.starkom");
        let (tokens, line_starts) = super::tokenize("vitalik.starkom", VITALIK).unwrap();
        assert_eq!(
            tokens,
            vec![
                token_with_label(
                    0,
                    TokenType::TokenTypeSingleLineComment,
                    " This is the circuit from Vitalik's PLONK tutorial. See"
                ),
                token_with_label(
                    58,
                    TokenType::TokenTypeSingleLineComment,
                    " https://vitalik.eth.limo/general/2019/09/22/plonk.html#how-plonk-works"
                ),
                token(133, TokenType::TokenTypeKeywordPragma),
                token(140, TokenType::TokenTypeKeywordStarkom),
                token_with_label(148, TokenType::TokenTypeVersionNumber, "1.0.0"),
                token(153, TokenType::TokenTypeSemicolon),
                token(156, TokenType::TokenTypeKeywordTemplate),
                token_with_label(165, TokenType::TokenTypeIdentifier, "Vitalik"),
                token(172, TokenType::TokenTypeLeftParenthesis),
                token(173, TokenType::TokenTypeRightParenthesis),
                token(175, TokenType::TokenTypeLeftCurlyBracket),
                token(179, TokenType::TokenTypeKeywordSignal),
                token(186, TokenType::TokenTypeKeywordInput),
                token_with_label(192, TokenType::TokenTypeIdentifier, "x"),
                token(193, TokenType::TokenTypeSemicolon),
                token(198, TokenType::TokenTypeKeywordSignal),
                token_with_label(205, TokenType::TokenTypeIdentifier, "square"),
                token(211, TokenType::TokenTypeSemicolon),
                token(215, TokenType::TokenTypeKeywordSignal),
                token_with_label(222, TokenType::TokenTypeIdentifier, "cube"),
                token(226, TokenType::TokenTypeSemicolon),
                token_with_label(231, TokenType::TokenTypeIdentifier, "square"),
                token(238, TokenType::TokenTypeOperatorConstrainedAssignLeft),
                token_with_label(242, TokenType::TokenTypeIdentifier, "x"),
                token(244, TokenType::TokenTypeOperatorMultiply),
                token_with_label(246, TokenType::TokenTypeIdentifier, "x"),
                token(247, TokenType::TokenTypeSemicolon),
                token_with_label(251, TokenType::TokenTypeIdentifier, "cube"),
                token(256, TokenType::TokenTypeOperatorConstrainedAssignLeft),
                token_with_label(260, TokenType::TokenTypeIdentifier, "square"),
                token(267, TokenType::TokenTypeOperatorMultiply),
                token_with_label(269, TokenType::TokenTypeIdentifier, "x"),
                token(270, TokenType::TokenTypeSemicolon),
                token_with_label(275, TokenType::TokenTypeIdentifier, "cube"),
                token(280, TokenType::TokenTypeOperatorPlus),
                token_with_label(282, TokenType::TokenTypeIdentifier, "x"),
                token(284, TokenType::TokenTypeOperatorPlus),
                token_with_label(286, TokenType::TokenTypeNumber10, "5"),
                token(288, TokenType::TokenTypeOperatorConstrainedEquality),
                token_with_label(292, TokenType::TokenTypeNumber10, "35"),
                token(294, TokenType::TokenTypeSemicolon),
                token(296, TokenType::TokenTypeRightCurlyBracket),
                token(299, TokenType::TokenTypeKeywordComponent),
                token_with_label(309, TokenType::TokenTypeIdentifier, "main"),
                token(314, TokenType::TokenTypeOperatorAssign),
                token_with_label(316, TokenType::TokenTypeIdentifier, "Vitalik"),
                token(323, TokenType::TokenTypeLeftParenthesis),
                token(324, TokenType::TokenTypeRightParenthesis),
                token(325, TokenType::TokenTypeSemicolon),
                token(327, TokenType::TokenTypeEndOfFile)
            ]
        );
        assert_eq!(
            line_starts,
            vec![
                0, 58, 132, 133, 155, 156, 177, 195, 196, 213, 228, 229, 249, 272, 273, 296, 298,
                299, 327
            ]
        );
    }
}
