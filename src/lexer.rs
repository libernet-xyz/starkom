use anyhow::{Result, anyhow};
use regex::{Captures, Regex};
use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
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

    OperatorConstrainLeft(usize),
    OperatorConstrainRight(usize),
    OperatorAssignLeft(usize),
    OperatorAssignRight(usize),
    OperatorAdd(usize),
    OperatorSubtract(usize),
    OperatorMultiply(usize),

    Symbol(usize, u8),

    EndOfFile(usize),
}

static KEYWORDS: LazyLock<BTreeMap<&str, fn(usize) -> Token>> = LazyLock::new(|| {
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

static REGEX_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s+").unwrap());

static REGEX_SINGLE_LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^//([^\n]*)(?:\n|$)").unwrap());

static REGEX_MULTI_LINE_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/\*((?:[^*]|\*[^/])*)\*/").unwrap());

static REGEX_IDENTIFIER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9$_]*").unwrap());

static REGEX_NUMBER_8: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0[0-7]+").unwrap());
static REGEX_NUMBER_10: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:0|[1-9]\d*)").unwrap());
static REGEX_NUMBER_16: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0x[0-9a-fA-F]+").unwrap());

#[derive(Debug, Clone)]
struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn consume_prefix(&mut self, pattern: &Regex) -> Option<(usize, Captures)> {
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

    fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        while !self.input.is_empty() {
            self.consume_prefix(&REGEX_WHITESPACE);
            if let Some((pos, captures)) = self.consume_prefix(&REGEX_SINGLE_LINE_COMMENT) {
                tokens.push(Token::SingleLineComment(pos, captures[1].to_string()));
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_MULTI_LINE_COMMENT) {
                tokens.push(Token::MultiLineComment(pos, captures[1].to_string()));
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_IDENTIFIER) {
                let capture = &captures[0];
                match KEYWORDS.get(capture) {
                    Some(token) => tokens.push(token(pos)),
                    None => tokens.push(Token::Identifier(pos, capture.to_string())),
                };
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_NUMBER_8) {
                tokens.push(Token::Number8(pos, captures[0].to_string()));
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_NUMBER_16) {
                tokens.push(Token::Number16(pos, captures[0].to_string()));
            } else if let Some((pos, captures)) = self.consume_prefix(&REGEX_NUMBER_10) {
                tokens.push(Token::Number10(pos, captures[0].to_string()));
            } else {
                return Err(anyhow!("syntax error"));
            }
        }
        Ok(tokens)
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    Lexer::new(input).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO
}
