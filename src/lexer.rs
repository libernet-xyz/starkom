use anyhow::{Result, anyhow};
use regex::{Captures, Regex};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    VersionNumber(usize, usize, usize, usize),

    SingleLineComment(usize, String),
    MultiLineComment(usize, String),

    Identifier(usize, String),

    KeywordAssert(usize),
    KeywordBus(usize),
    KeywordComponent(usize),
    KeywordDo(usize),
    KeywordElse(usize),
    KeywordFor(usize),
    KeywordFunction(usize),
    KeywordIf(usize),
    KeywordInclude(usize),
    KeywordInput(usize),
    KeywordLog(usize),
    KeywordOutput(usize),
    KeywordParallel(usize),
    KeywordPragma(usize),
    KeywordPublic(usize),
    KeywordReturn(usize),
    KeywordSignal(usize),
    KeywordStarkom(usize),
    KeywordTemplate(usize),
    KeywordVar(usize),
    KeywordWhile(usize),

    Number8(usize, String),
    Number10(usize, String),
    Number16(usize, String),

    LeftParenthesis(usize),
    RightParenthesis(usize),
    LeftSquareBracket(usize),
    RightSquareBracket(usize),
    LeftCurlyBracket(usize),
    RightCurlyBracket(usize),

    OperatorAssignLeft(usize),
    OperatorAssignRight(usize),
    OperatorConstrainedAssignLeft(usize),
    OperatorConstrainedAssignRight(usize),
    OperatorConstrainedEquality(usize),

    OperatorAssign(usize),
    OperatorAdd(usize),
    OperatorSubtract(usize),
    OperatorMultiply(usize),
    OperatorPower(usize),
    OperatorDivide(usize),
    OperatorDivideInteger(usize),
    OperatorModulus(usize),
    OperatorIncrement(usize),
    OperatorDecrement(usize),

    OperatorBooleanAnd(usize),
    OperatorBooleanOr(usize),
    OperatorBooleanNot(usize),

    OperatorBitwiseAnd(usize),
    OperatorBitwiseOr(usize),
    OperatorBitwiseXor(usize),
    OperatorBitwiseNot(usize),
    OperatorShiftLeft(usize),
    OperatorShiftRight(usize),

    OperatorCompoundAdd(usize),
    OperatorCompoundSubtract(usize),
    OperatorCompoundMultiply(usize),
    OperatorCompoundPower(usize),
    OperatorCompoundDivide(usize),
    OperatorCompoundDivideInteger(usize),
    OperatorCompoundModulus(usize),

    OperatorCompoundBooleanAnd(usize),
    OperatorCompoundBooleanOr(usize),

    OperatorCompoundBitwiseAnd(usize),
    OperatorCompoundBitwiseOr(usize),
    OperatorCompoundBitwiseXor(usize),
    OperatorCompoundShiftLeft(usize),
    OperatorCompoundShiftRight(usize),

    OperatorCompareEqual(usize),
    OperatorCompareNotEqual(usize),
    OperatorLessThan(usize),
    OperatorLessThanOrEqualTo(usize),
    OperatorGreaterThan(usize),
    OperatorGreaterThanOrEqualTo(usize),

    Dot(usize),
    Comma(usize),
    Colon(usize),
    Semicolon(usize),

    EndOfFile(usize),
}

static KEYWORD_TOKENS: LazyLock<BTreeMap<&str, fn(usize) -> Token>> = LazyLock::new(|| {
    BTreeMap::from([
        ("assert", Token::KeywordAssert as fn(usize) -> Token),
        ("bus", Token::KeywordBus as fn(usize) -> Token),
        ("component", Token::KeywordComponent as fn(usize) -> Token),
        ("do", Token::KeywordDo as fn(usize) -> Token),
        ("else", Token::KeywordElse as fn(usize) -> Token),
        ("for", Token::KeywordFor as fn(usize) -> Token),
        ("function", Token::KeywordFunction as fn(usize) -> Token),
        ("if", Token::KeywordIf as fn(usize) -> Token),
        ("include", Token::KeywordInclude as fn(usize) -> Token),
        ("input", Token::KeywordInput as fn(usize) -> Token),
        ("log", Token::KeywordLog as fn(usize) -> Token),
        ("output", Token::KeywordOutput as fn(usize) -> Token),
        ("parallel", Token::KeywordParallel as fn(usize) -> Token),
        ("pragma", Token::KeywordPragma as fn(usize) -> Token),
        ("public", Token::KeywordPublic as fn(usize) -> Token),
        ("return", Token::KeywordReturn as fn(usize) -> Token),
        ("signal", Token::KeywordSignal as fn(usize) -> Token),
        ("starkom", Token::KeywordStarkom as fn(usize) -> Token),
        ("template", Token::KeywordTemplate as fn(usize) -> Token),
        ("var", Token::KeywordVar as fn(usize) -> Token),
        ("while", Token::KeywordWhile as fn(usize) -> Token),
    ])
});

static SYMBOL_TOKENS: LazyLock<BTreeMap<&'static str, fn(usize) -> Token>> = LazyLock::new(|| {
    BTreeMap::from([
        ("<--", Token::OperatorAssignLeft as fn(usize) -> Token),
        ("-->", Token::OperatorAssignRight),
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
        ("&&", Token::OperatorBooleanAnd),
        ("||", Token::OperatorBooleanOr),
        ("!", Token::OperatorBooleanNot),
        ("+", Token::OperatorAdd),
        ("-", Token::OperatorSubtract),
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
        ("&&=", Token::OperatorCompoundBooleanAnd),
        ("||=", Token::OperatorCompoundBooleanOr),
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
    LazyLock::new(|| Regex::new(r"^(\d+)\.(\d+)\.(\d+)").unwrap());

static REGEX_SINGLE_LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^//([^\n]*)(?:\n|$)").unwrap());

static REGEX_MULTI_LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/\*((?:[^*]|\*[^/])*)\*/").unwrap());

static REGEX_IDENTIFIER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9$_]*").unwrap());

static REGEX_NUMBER_8: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0[0-7]+").unwrap());
static REGEX_NUMBER_10: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:0|[1-9]\d*)").unwrap());
static REGEX_NUMBER_16: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0x[0-9a-fA-F]+").unwrap());

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
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn consume_prefix(&mut self, pattern: &Regex) -> Option<(usize, Captures<'a>)> {
        let start_pos = self.pos;
        match pattern.captures(self.input) {
            Some(captures) => {
                let n = captures[0].len();
                self.input = &self.input[n..];
                self.pos += n;
                Some((start_pos, captures))
            }
            None => None,
        }
    }

    fn consume_any_prefix(
        &mut self,
        prefixes: &BTreeSet<&'static str>,
    ) -> Option<(usize, &'a str)> {
        for &prefix in prefixes {
            if self.input.starts_with(prefix) {
                let result = (self.pos, &self.input[0..prefix.len()]);
                self.input = &self.input[prefix.len()..];
                self.pos += prefix.len();
                return Some(result);
            }
        }
        None
    }

    fn consume_symbol(&mut self) -> Option<(usize, &'a str)> {
        if let Some((pos, prefix)) = self.consume_any_prefix(&SYMBOLS_LEN3) {
            return Some((pos, prefix));
        }
        if let Some((pos, prefix)) = self.consume_any_prefix(&SYMBOLS_LEN2) {
            return Some((pos, prefix));
        }
        if let Some((pos, prefix)) = self.consume_any_prefix(&SYMBOLS_LEN1) {
            return Some((pos, prefix));
        }
        None
    }

    fn parse_usize(s: &str) -> usize {
        usize::from_str_radix(s, 10).unwrap()
    }

    fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        self.consume_prefix(&REGEX_WHITESPACE);
        while !self.input.is_empty() {
            if let Some((pos, captures)) = self.consume_prefix(&REGEX_VERSION_NUMBER) {
                tokens.push(Token::VersionNumber(
                    pos,
                    Self::parse_usize(&captures[1]),
                    Self::parse_usize(&captures[2]),
                    Self::parse_usize(&captures[3]),
                ))
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_SINGLE_LINE_COMMENT) {
                tokens.push(Token::SingleLineComment(pos, captures[1].to_string()));
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_MULTI_LINE_COMMENT) {
                tokens.push(Token::MultiLineComment(pos, captures[1].to_string()));
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_IDENTIFIER) {
                let capture = &captures[0];
                match KEYWORD_TOKENS.get(capture) {
                    Some(token) => tokens.push(token(pos)),
                    None => tokens.push(Token::Identifier(pos, capture.to_string())),
                };
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_NUMBER_8) {
                tokens.push(Token::Number8(pos, captures[0].to_string()));
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_NUMBER_16) {
                tokens.push(Token::Number16(pos, captures[0].to_string()));
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_NUMBER_10) {
                tokens.push(Token::Number10(pos, captures[0].to_string()));
            } else if let Some((pos, symbol)) = self.consume_symbol() {
                let token = SYMBOL_TOKENS.get(symbol).unwrap();
                tokens.push(token(pos));
            } else {
                return Err(anyhow!("syntax error at offset {}", self.pos));
            }
            self.consume_prefix(&REGEX_WHITESPACE);
        }
        tokens.push(Token::EndOfFile(self.pos));
        Ok(tokens)
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    Lexer::new(input).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO: &'static str = include_str!("../test/hello.starkom");

    #[test]
    fn test_hello() {
        assert_eq!(
            tokenize(HELLO).unwrap(),
            vec![
                Token::KeywordPragma(0),
                Token::KeywordStarkom(7),
                Token::VersionNumber(15, 2, 0, 0),
                Token::Semicolon(20),
                Token::KeywordTemplate(23),
                Token::Identifier(32, "Hello".to_string()),
                Token::LeftParenthesis(37),
                Token::RightParenthesis(38),
                Token::LeftCurlyBracket(40),
                Token::KeywordSignal(44),
                Token::KeywordInput(51),
                Token::Identifier(57, "x".to_string()),
                Token::Semicolon(58),
                Token::KeywordSignal(63),
                Token::Identifier(70, "square".to_string()),
                Token::Semicolon(76),
                Token::KeywordSignal(80),
                Token::Identifier(87, "cube".to_string()),
                Token::Semicolon(91),
                Token::Identifier(96, "square".to_string()),
                Token::OperatorConstrainedAssignLeft(103),
                Token::Identifier(107, "x".to_string()),
                Token::OperatorMultiply(109),
                Token::Identifier(111, "x".to_string()),
                Token::Semicolon(112),
                Token::Identifier(116, "cube".to_string()),
                Token::OperatorConstrainedAssignLeft(121),
                Token::Identifier(125, "square".to_string()),
                Token::OperatorMultiply(132),
                Token::Identifier(134, "x".to_string()),
                Token::Semicolon(135),
                Token::Identifier(140, "cube".to_string()),
                Token::OperatorAdd(145),
                Token::Identifier(147, "x".to_string()),
                Token::OperatorAdd(149),
                Token::Number10(151, "5".to_string()),
                Token::OperatorConstrainedEquality(153),
                Token::Number10(157, "35".to_string()),
                Token::Semicolon(159),
                Token::RightCurlyBracket(161),
                Token::KeywordComponent(164),
                Token::Identifier(174, "main".to_string()),
                Token::OperatorAssign(179),
                Token::Identifier(181, "Hello".to_string()),
                Token::LeftParenthesis(186),
                Token::RightParenthesis(187),
                Token::Semicolon(188),
                Token::EndOfFile(190),
            ]
        );
    }

    // TODO
}
