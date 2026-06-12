use crate::error::{Error, Result};
use regex::{Captures, Regex};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

/// Starkom language tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    VersionNumber(u32, u32, u32),

    SingleLineComment(String),
    MultiLineComment(String),

    Identifier(String),

    KeywordAssert,
    KeywordBreak,
    KeywordBus,
    KeywordComponent,
    KeywordConst,
    KeywordContinue,
    KeywordDo,
    KeywordElse,
    KeywordFalse,
    KeywordFor,
    KeywordFunction,
    KeywordIf,
    KeywordInclude,
    KeywordInput,
    KeywordLog,
    KeywordOutput,
    KeywordParallel,
    KeywordPragma,
    KeywordPublic,
    KeywordReturn,
    KeywordSignal,
    KeywordStarkom,
    KeywordTemplate,
    KeywordTrue,
    KeywordVar,
    KeywordWhile,

    // Octal numeric literal.
    Number8(String),

    // Decimal numeric literal.
    Number10(String),

    // Hexadecimal numeric literal.
    Number16(String),

    // Note: the `String` field contains the full original literal with quotes and with all escapes
    // unprocessed.
    StringLiteral(String),

    LeftParenthesis,
    RightParenthesis,
    LeftSquareBracket,
    RightSquareBracket,
    LeftCurlyBracket,
    RightCurlyBracket,

    OperatorUnconstrainedAssignLeft,
    OperatorUnconstrainedAssignRight,
    OperatorConstrainedAssignLeft,
    OperatorConstrainedAssignRight,
    OperatorConstrainedEquality,

    OperatorAssign,
    OperatorPlus,
    OperatorMinus,
    OperatorMultiply,
    OperatorPower,
    OperatorDivide,
    OperatorDivideInteger,
    OperatorModulus,
    OperatorIncrement,
    OperatorDecrement,

    OperatorLogicalAnd,
    OperatorLogicalOr,
    OperatorLogicalNot,

    OperatorBitwiseAnd,
    OperatorBitwiseOr,
    OperatorBitwiseXor,
    OperatorBitwiseNot,
    OperatorShiftLeft,
    OperatorShiftRight,

    OperatorCompoundAdd,
    OperatorCompoundSubtract,
    OperatorCompoundMultiply,
    OperatorCompoundPower,
    OperatorCompoundDivide,
    OperatorCompoundDivideInteger,
    OperatorCompoundModulus,

    OperatorCompoundLogicalAnd,
    OperatorCompoundLogicalOr,

    OperatorCompoundBitwiseAnd,
    OperatorCompoundBitwiseOr,
    OperatorCompoundBitwiseXor,
    OperatorCompoundShiftLeft,
    OperatorCompoundShiftRight,

    OperatorCompareEqual,
    OperatorCompareNotEqual,
    OperatorLessThan,
    OperatorLessThanOrEqualTo,
    OperatorGreaterThan,
    OperatorGreaterThanOrEqualTo,

    Dot,
    Comma,
    Colon,
    Semicolon,

    EndOfFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenEntry {
    pub pos: usize,
    pub token: Token,
}

static KEYWORD_TOKENS: LazyLock<BTreeMap<&str, Token>> = LazyLock::new(|| {
    BTreeMap::from([
        ("assert", Token::KeywordAssert),
        ("break", Token::KeywordBreak),
        ("bus", Token::KeywordBus),
        ("component", Token::KeywordComponent),
        ("const", Token::KeywordConst),
        ("continue", Token::KeywordContinue),
        ("do", Token::KeywordDo),
        ("else", Token::KeywordElse),
        ("false", Token::KeywordFalse),
        ("for", Token::KeywordFor),
        ("function", Token::KeywordFunction),
        ("if", Token::KeywordIf),
        ("include", Token::KeywordInclude),
        ("input", Token::KeywordInput),
        ("log", Token::KeywordLog),
        ("output", Token::KeywordOutput),
        ("parallel", Token::KeywordParallel),
        ("pragma", Token::KeywordPragma),
        ("public", Token::KeywordPublic),
        ("return", Token::KeywordReturn),
        ("signal", Token::KeywordSignal),
        ("starkom", Token::KeywordStarkom),
        ("template", Token::KeywordTemplate),
        ("true", Token::KeywordTrue),
        ("var", Token::KeywordVar),
        ("while", Token::KeywordWhile),
    ])
});

static SYMBOL_TOKENS: LazyLock<BTreeMap<&'static str, Token>> = LazyLock::new(|| {
    BTreeMap::from([
        ("<--", Token::OperatorUnconstrainedAssignLeft),
        ("-->", Token::OperatorUnconstrainedAssignRight),
        ("<==", Token::OperatorConstrainedAssignLeft),
        ("==>", Token::OperatorConstrainedAssignRight),
        ("===", Token::OperatorConstrainedEquality),
        ("==", Token::OperatorCompareEqual),
        ("!=", Token::OperatorCompareNotEqual),
        ("<=", Token::OperatorLessThanOrEqualTo),
        ("<", Token::OperatorLessThan),
        (">=", Token::OperatorGreaterThanOrEqualTo),
        (">", Token::OperatorGreaterThan),
        ("++", Token::OperatorIncrement),
        ("--", Token::OperatorDecrement),
        ("<<", Token::OperatorShiftLeft),
        (">>", Token::OperatorShiftRight),
        ("&&", Token::OperatorLogicalAnd),
        ("||", Token::OperatorLogicalOr),
        ("!", Token::OperatorLogicalNot),
        ("+", Token::OperatorPlus),
        ("-", Token::OperatorMinus),
        ("*", Token::OperatorMultiply),
        ("**", Token::OperatorPower),
        ("/", Token::OperatorDivide),
        ("\\", Token::OperatorDivideInteger),
        ("%", Token::OperatorModulus),
        ("&", Token::OperatorBitwiseAnd),
        ("|", Token::OperatorBitwiseOr),
        ("^", Token::OperatorBitwiseXor),
        ("~", Token::OperatorBitwiseNot),
        ("=", Token::OperatorAssign),
        ("+=", Token::OperatorCompoundAdd),
        ("-=", Token::OperatorCompoundSubtract),
        ("*=", Token::OperatorCompoundMultiply),
        ("**=", Token::OperatorCompoundPower),
        ("/=", Token::OperatorCompoundDivide),
        ("\\=", Token::OperatorCompoundDivideInteger),
        ("%=", Token::OperatorCompoundModulus),
        ("&&=", Token::OperatorCompoundLogicalAnd),
        ("||=", Token::OperatorCompoundLogicalOr),
        ("&=", Token::OperatorCompoundBitwiseAnd),
        ("|=", Token::OperatorCompoundBitwiseOr),
        ("^=", Token::OperatorCompoundBitwiseXor),
        ("<<=", Token::OperatorCompoundShiftLeft),
        (">>=", Token::OperatorCompoundShiftRight),
        ("(", Token::LeftParenthesis),
        (")", Token::RightParenthesis),
        ("[", Token::LeftSquareBracket),
        ("]", Token::RightSquareBracket),
        ("{", Token::LeftCurlyBracket),
        ("}", Token::RightCurlyBracket),
        (".", Token::Dot),
        (",", Token::Comma),
        (":", Token::Colon),
        (";", Token::Semicolon),
    ])
});

static REGEX_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s+").unwrap());

static REGEX_VERSION_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{1,9})\.(\d{1,9})\.(\d{1,9})\b").unwrap());

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

    fn token(&self, token: Token) -> TokenEntry {
        TokenEntry {
            pos: self.last_offset,
            token,
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

    fn parse_u32(s: &str) -> u32 {
        u32::from_str_radix(s, 10).unwrap()
    }

    fn tokenize(mut self) -> Result<(Vec<TokenEntry>, Vec<usize>)> {
        let mut tokens = Vec::new();
        self.consume_prefix(&REGEX_WHITESPACE);
        while !self.input.is_empty() {
            if let Some(captures) = self.consume_prefix(&REGEX_VERSION_NUMBER) {
                tokens.push(self.token(Token::VersionNumber(
                    Self::parse_u32(&captures[1]),
                    Self::parse_u32(&captures[2]),
                    Self::parse_u32(&captures[3]),
                )));
            } else if self.match_prefix(&REGEX_SINGLE_LINE_COMMENT_START) {
                match self.consume_prefix(&REGEX_SINGLE_LINE_COMMENT) {
                    Some(captures) => {
                        tokens.push(self.token(Token::SingleLineComment(captures[1].to_string())));
                    }
                    None => {
                        return self.error("syntax error");
                    }
                };
            } else if self.match_prefix(&REGEX_MULTI_LINE_COMMENT_START) {
                match self.consume_prefix(&REGEX_MULTI_LINE_COMMENT) {
                    Some(captures) => {
                        tokens.push(self.token(Token::MultiLineComment(captures[1].to_string())));
                    }
                    None => {
                        return self.error("syntax error");
                    }
                };
            } else if let Some(captures) = self.consume_prefix(&REGEX_IDENTIFIER) {
                let capture = &captures[0];
                match KEYWORD_TOKENS.get(capture) {
                    Some(token) => tokens.push(self.token(token.clone())),
                    None => tokens.push(self.token(Token::Identifier(capture.to_string()))),
                };
            } else if let Some(captures) = self.consume_prefix(&REGEX_NUMBER_8) {
                tokens.push(self.token(Token::Number8(captures[0].to_string())));
            } else if let Some(captures) = self.consume_prefix(&REGEX_NUMBER_16) {
                tokens.push(self.token(Token::Number16(captures[0].to_string())));
            } else if let Some(captures) = self.consume_prefix(&REGEX_NUMBER_10) {
                tokens.push(self.token(Token::Number10(captures[0].to_string())));
            } else if let Some(captures) = self.consume_prefix(&REGEX_STRING_LITERAL) {
                tokens.push(self.token(Token::StringLiteral(captures[0].to_string())));
            } else if let Some(symbol) = self.consume_symbol() {
                let token = SYMBOL_TOKENS.get(symbol).unwrap();
                tokens.push(self.token(token.clone()));
            } else {
                return self.error("syntax error");
            }
            self.consume_prefix(&REGEX_WHITESPACE);
        }
        tokens.push(TokenEntry {
            pos: self.current_offset,
            token: Token::EndOfFile,
        });
        Ok((tokens, self.line_starts))
    }
}

/// Reads an input Starkom source file and produces the corresponding array of lexical tokens.
pub fn tokenize(path: &str, input: &str) -> Result<(Vec<TokenEntry>, Vec<usize>)> {
    Lexer::new(path, input).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(pos: usize, token: Token) -> TokenEntry {
        TokenEntry { pos, token }
    }

    fn tokenize(input: &str) -> Result<Vec<TokenEntry>> {
        let (tokens, _) = super::tokenize("<test>", input)?;
        Ok(tokens)
    }

    #[test]
    fn test_version_number() {
        assert_eq!(
            tokenize("1.2.3").unwrap(),
            vec![
                token(0, Token::VersionNumber(1, 2, 3)),
                token(5, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("44.55.66").unwrap(),
            vec![
                token(0, Token::VersionNumber(44, 55, 66)),
                token(8, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_single_line_comment_basic() {
        assert_eq!(
            tokenize("// hello").unwrap(),
            vec![
                token(0, Token::SingleLineComment(" hello".to_string())),
                token(8, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_single_line_comment_empty() {
        assert_eq!(
            tokenize("//").unwrap(),
            vec![
                token(0, Token::SingleLineComment("".to_string())),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("//\n").unwrap(),
            vec![
                token(0, Token::SingleLineComment("".to_string())),
                token(3, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_single_line_comment_consumes_newline() {
        // Newline is consumed by the comment; subsequent token starts right after.
        assert_eq!(
            tokenize("// comment\nx").unwrap(),
            vec![
                token(0, Token::SingleLineComment(" comment".to_string())),
                token(11, Token::Identifier("x".to_string())),
                token(12, Token::EndOfFile),
            ]
        );
    }

    #[test]
    fn test_single_line_comment_non_zero_position() {
        assert_eq!(
            tokenize("x // comment").unwrap(),
            vec![
                token(0, Token::Identifier("x".to_string())),
                token(2, Token::SingleLineComment(" comment".to_string())),
                token(12, Token::EndOfFile),
            ]
        );
    }

    #[test]
    fn test_single_line_comment_multiple() {
        assert_eq!(
            tokenize("// first\n// second").unwrap(),
            vec![
                token(0, Token::SingleLineComment(" first".to_string())),
                token(9, Token::SingleLineComment(" second".to_string())),
                token(18, Token::EndOfFile),
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_basic() {
        assert_eq!(
            tokenize("/* hello */").unwrap(),
            vec![
                token(0, Token::MultiLineComment(" hello ".to_string())),
                token(11, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_empty() {
        assert_eq!(
            tokenize("/**/").unwrap(),
            vec![
                token(0, Token::MultiLineComment("".to_string())),
                token(4, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_asterisks() {
        assert_eq!(
            tokenize("/***/").unwrap(),
            vec![
                token(0, Token::MultiLineComment("*".to_string())),
                token(5, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/****/").unwrap(),
            vec![
                token(0, Token::MultiLineComment("**".to_string())),
                token(6, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/* ***/").unwrap(),
            vec![
                token(0, Token::MultiLineComment(" **".to_string())),
                token(7, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/*** */").unwrap(),
            vec![
                token(0, Token::MultiLineComment("** ".to_string())),
                token(7, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/* ** */").unwrap(),
            vec![
                token(0, Token::MultiLineComment(" ** ".to_string())),
                token(8, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_slash() {
        assert_eq!(
            tokenize("/*/*/").unwrap(),
            vec![
                token(0, Token::MultiLineComment("/".to_string())),
                token(5, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_spans_lines() {
        assert_eq!(
            tokenize("/* line1\nline2 */").unwrap(),
            vec![
                token(0, Token::MultiLineComment(" line1\nline2 ".to_string())),
                token(17, Token::EndOfFile),
            ]
        );
    }

    #[test]
    fn test_multi_line_comment_non_zero_position() {
        assert_eq!(
            tokenize("x /* note */ y").unwrap(),
            vec![
                token(0, Token::Identifier("x".to_string())),
                token(2, Token::MultiLineComment(" note ".to_string())),
                token(13, Token::Identifier("y".to_string())),
                token(14, Token::EndOfFile),
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
                token(0, Token::Identifier("foo".to_string())),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("bar").unwrap(),
            vec![
                token(0, Token::Identifier("bar".to_string())),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("foo42").unwrap(),
            vec![
                token(0, Token::Identifier("foo42".to_string())),
                token(5, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("foo12bar34").unwrap(),
            vec![
                token(0, Token::Identifier("foo12bar34".to_string())),
                token(10, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            tokenize("assert").unwrap(),
            vec![token(0, Token::KeywordAssert), token(6, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("break").unwrap(),
            vec![token(0, Token::KeywordBreak), token(5, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("bus").unwrap(),
            vec![token(0, Token::KeywordBus), token(3, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("component").unwrap(),
            vec![
                token(0, Token::KeywordComponent),
                token(9, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("const").unwrap(),
            vec![token(0, Token::KeywordConst), token(5, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("continue").unwrap(),
            vec![token(0, Token::KeywordContinue), token(8, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("do").unwrap(),
            vec![token(0, Token::KeywordDo), token(2, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("else").unwrap(),
            vec![token(0, Token::KeywordElse), token(4, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("false").unwrap(),
            vec![token(0, Token::KeywordFalse), token(5, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("for").unwrap(),
            vec![token(0, Token::KeywordFor), token(3, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("function").unwrap(),
            vec![token(0, Token::KeywordFunction), token(8, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("if").unwrap(),
            vec![token(0, Token::KeywordIf), token(2, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("include").unwrap(),
            vec![token(0, Token::KeywordInclude), token(7, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("input").unwrap(),
            vec![token(0, Token::KeywordInput), token(5, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("log").unwrap(),
            vec![token(0, Token::KeywordLog), token(3, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("output").unwrap(),
            vec![token(0, Token::KeywordOutput), token(6, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("parallel").unwrap(),
            vec![token(0, Token::KeywordParallel), token(8, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("pragma").unwrap(),
            vec![token(0, Token::KeywordPragma), token(6, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("public").unwrap(),
            vec![token(0, Token::KeywordPublic), token(6, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("return").unwrap(),
            vec![token(0, Token::KeywordReturn), token(6, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("signal").unwrap(),
            vec![token(0, Token::KeywordSignal), token(6, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("starkom").unwrap(),
            vec![token(0, Token::KeywordStarkom), token(7, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("template").unwrap(),
            vec![token(0, Token::KeywordTemplate), token(8, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("true").unwrap(),
            vec![token(0, Token::KeywordTrue), token(4, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("var").unwrap(),
            vec![token(0, Token::KeywordVar), token(3, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("while").unwrap(),
            vec![token(0, Token::KeywordWhile), token(5, Token::EndOfFile)]
        );
    }

    #[test]
    fn test_number_decimal_zero() {
        assert_eq!(
            tokenize("0").unwrap(),
            vec![
                token(0, Token::Number10("0".to_string())),
                token(1, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_number_decimal_basic() {
        assert_eq!(
            tokenize("42").unwrap(),
            vec![
                token(0, Token::Number10("42".to_string())),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("123").unwrap(),
            vec![
                token(0, Token::Number10("123".to_string())),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("1000000").unwrap(),
            vec![
                token(0, Token::Number10("1000000".to_string())),
                token(7, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_number_octal_basic() {
        assert_eq!(
            tokenize("077").unwrap(),
            vec![
                token(0, Token::Number8("077".to_string())),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("0755").unwrap(),
            vec![
                token(0, Token::Number8("0755".to_string())),
                token(4, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_number_hex_basic() {
        assert_eq!(
            tokenize("0xFF").unwrap(),
            vec![
                token(0, Token::Number16("0xFF".to_string())),
                token(4, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("0x1a").unwrap(),
            vec![
                token(0, Token::Number16("0x1a".to_string())),
                token(4, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("0x1ee7").unwrap(),
            vec![
                token(0, Token::Number16("0x1ee7".to_string())),
                token(6, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_string_literal_basic() {
        assert_eq!(
            tokenize("\"hello\"").unwrap(),
            vec![
                token(0, Token::StringLiteral("\"hello\"".to_string())),
                token(7, Token::EndOfFile),
            ]
        );
    }

    #[test]
    fn test_string_literal_empty() {
        assert_eq!(
            tokenize("\"\"").unwrap(),
            vec![
                token(0, Token::StringLiteral("\"\"".to_string())),
                token(2, Token::EndOfFile),
            ]
        );
    }

    #[test]
    fn test_string_literal_with_spaces() {
        assert_eq!(
            tokenize("\"hello world\"").unwrap(),
            vec![
                token(0, Token::StringLiteral("\"hello world\"".to_string())),
                token(13, Token::EndOfFile),
            ]
        );
    }

    #[test]
    fn test_symbols() {
        assert_eq!(
            tokenize("(").unwrap(),
            vec![token(0, Token::LeftParenthesis), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize(")").unwrap(),
            vec![
                token(0, Token::RightParenthesis),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("[").unwrap(),
            vec![
                token(0, Token::LeftSquareBracket),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("]").unwrap(),
            vec![
                token(0, Token::RightSquareBracket),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("{").unwrap(),
            vec![
                token(0, Token::LeftCurlyBracket),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("}").unwrap(),
            vec![
                token(0, Token::RightCurlyBracket),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize(".").unwrap(),
            vec![token(0, Token::Dot), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize(",").unwrap(),
            vec![token(0, Token::Comma), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize(":").unwrap(),
            vec![token(0, Token::Colon), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize(";").unwrap(),
            vec![token(0, Token::Semicolon), token(1, Token::EndOfFile)]
        );
    }

    #[test]
    fn test_operators() {
        assert_eq!(
            tokenize("<--").unwrap(),
            vec![
                token(0, Token::OperatorUnconstrainedAssignLeft),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("-->").unwrap(),
            vec![
                token(0, Token::OperatorUnconstrainedAssignRight),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<==").unwrap(),
            vec![
                token(0, Token::OperatorConstrainedAssignLeft),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("==>").unwrap(),
            vec![
                token(0, Token::OperatorConstrainedAssignRight),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("===").unwrap(),
            vec![
                token(0, Token::OperatorConstrainedEquality),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("==").unwrap(),
            vec![
                token(0, Token::OperatorCompareEqual),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("!=").unwrap(),
            vec![
                token(0, Token::OperatorCompareNotEqual),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<=").unwrap(),
            vec![
                token(0, Token::OperatorLessThanOrEqualTo),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<").unwrap(),
            vec![
                token(0, Token::OperatorLessThan),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize(">=").unwrap(),
            vec![
                token(0, Token::OperatorGreaterThanOrEqualTo),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize(">").unwrap(),
            vec![
                token(0, Token::OperatorGreaterThan),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("++").unwrap(),
            vec![
                token(0, Token::OperatorIncrement),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("--").unwrap(),
            vec![
                token(0, Token::OperatorDecrement),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<<").unwrap(),
            vec![
                token(0, Token::OperatorShiftLeft),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize(">>").unwrap(),
            vec![
                token(0, Token::OperatorShiftRight),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("&&").unwrap(),
            vec![
                token(0, Token::OperatorLogicalAnd),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("||").unwrap(),
            vec![
                token(0, Token::OperatorLogicalOr),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("!").unwrap(),
            vec![
                token(0, Token::OperatorLogicalNot),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("+").unwrap(),
            vec![token(0, Token::OperatorPlus), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("-").unwrap(),
            vec![token(0, Token::OperatorMinus), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("**").unwrap(),
            vec![token(0, Token::OperatorPower), token(2, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("*").unwrap(),
            vec![
                token(0, Token::OperatorMultiply),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/").unwrap(),
            vec![token(0, Token::OperatorDivide), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("\\").unwrap(),
            vec![
                token(0, Token::OperatorDivideInteger),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("%").unwrap(),
            vec![token(0, Token::OperatorModulus), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("&").unwrap(),
            vec![
                token(0, Token::OperatorBitwiseAnd),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("|").unwrap(),
            vec![
                token(0, Token::OperatorBitwiseOr),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("^").unwrap(),
            vec![
                token(0, Token::OperatorBitwiseXor),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("~").unwrap(),
            vec![
                token(0, Token::OperatorBitwiseNot),
                token(1, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("=").unwrap(),
            vec![token(0, Token::OperatorAssign), token(1, Token::EndOfFile)]
        );
        assert_eq!(
            tokenize("+=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundAdd),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("-=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundSubtract),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("**=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundPower),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("*=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundMultiply),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("/=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundDivide),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("\\=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundDivideInteger),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("%=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundModulus),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("&&=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundLogicalAnd),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("||=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundLogicalOr),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("&=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundBitwiseAnd),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("|=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundBitwiseOr),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("^=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundBitwiseXor),
                token(2, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize("<<=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundShiftLeft),
                token(3, Token::EndOfFile)
            ]
        );
        assert_eq!(
            tokenize(">>=").unwrap(),
            vec![
                token(0, Token::OperatorCompoundShiftRight),
                token(3, Token::EndOfFile)
            ]
        );
    }

    #[test]
    fn test_empty_file() {
        assert_eq!(tokenize("").unwrap(), vec![token(0, Token::EndOfFile)]);
        assert_eq!(tokenize("\n").unwrap(), vec![token(1, Token::EndOfFile)]);
        assert_eq!(
            tokenize(" \n \n ").unwrap(),
            vec![token(5, Token::EndOfFile)]
        );
    }

    #[test]
    fn test_vitalik() {
        static VITALIK: &'static str = include_str!("../test/vitalik.starkom");
        let (tokens, line_starts) = super::tokenize("vitalik.starkom", VITALIK).unwrap();
        assert_eq!(
            tokens,
            vec![
                token(
                    0,
                    Token::SingleLineComment(
                        " This is the circuit from Vitalik's PLONK tutorial. See".to_string()
                    )
                ),
                token(
                    58,
                    Token::SingleLineComment(
                        " https://vitalik.eth.limo/general/2019/09/22/plonk.html#how-plonk-works"
                            .to_string()
                    )
                ),
                token(133, Token::KeywordPragma),
                token(140, Token::KeywordStarkom),
                token(148, Token::VersionNumber(1, 0, 0)),
                token(153, Token::Semicolon),
                token(156, Token::KeywordTemplate),
                token(165, Token::Identifier("Vitalik".to_string())),
                token(172, Token::LeftParenthesis),
                token(173, Token::RightParenthesis),
                token(175, Token::LeftCurlyBracket),
                token(179, Token::KeywordSignal),
                token(186, Token::KeywordInput),
                token(192, Token::Identifier("x".to_string())),
                token(193, Token::Semicolon),
                token(198, Token::KeywordSignal),
                token(205, Token::Identifier("square".to_string())),
                token(211, Token::Semicolon),
                token(215, Token::KeywordSignal),
                token(222, Token::Identifier("cube".to_string())),
                token(226, Token::Semicolon),
                token(231, Token::Identifier("square".to_string())),
                token(238, Token::OperatorConstrainedAssignLeft),
                token(242, Token::Identifier("x".to_string())),
                token(244, Token::OperatorMultiply),
                token(246, Token::Identifier("x".to_string())),
                token(247, Token::Semicolon),
                token(251, Token::Identifier("cube".to_string())),
                token(256, Token::OperatorConstrainedAssignLeft),
                token(260, Token::Identifier("square".to_string())),
                token(267, Token::OperatorMultiply),
                token(269, Token::Identifier("x".to_string())),
                token(270, Token::Semicolon),
                token(275, Token::Identifier("cube".to_string())),
                token(280, Token::OperatorPlus),
                token(282, Token::Identifier("x".to_string())),
                token(284, Token::OperatorPlus),
                token(286, Token::Number10("5".to_string())),
                token(288, Token::OperatorConstrainedEquality),
                token(292, Token::Number10("35".to_string())),
                token(294, Token::Semicolon),
                token(296, Token::RightCurlyBracket),
                token(299, Token::KeywordComponent),
                token(309, Token::Identifier("main".to_string())),
                token(314, Token::OperatorAssign),
                token(316, Token::Identifier("Vitalik".to_string())),
                token(323, Token::LeftParenthesis),
                token(324, Token::RightParenthesis),
                token(325, Token::Semicolon),
                token(327, Token::EndOfFile)
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
