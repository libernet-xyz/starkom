use crate::ast::{self, Token, token::Type as TokenType};
use crate::error::{Error, Result};
use crate::lexer::tokenize;
use regex::Regex;
use std::sync::LazyLock;
use wasm_bindgen::prelude::*;

trait VecUsizeToU32 {
    fn to_u32(self) -> Vec<u32>;
}

impl VecUsizeToU32 for Vec<usize> {
    fn to_u32(self) -> Vec<u32> {
        self.into_iter().map(|index| index as u32).collect()
    }
}

/// Loosely represents a stack frame of the recursive descent algorithm.
///
/// The purpose of this struct is to keep track of where an AST node begins so that we can construct
/// its range if the caller has requested parsing with ranges.
#[derive(Debug, Clone)]
struct NodeFrame {
    pos: usize,
}

impl NodeFrame {
    fn maybe_range<'a>(&self, parser: &mut Parser<'a>) -> Option<ast::Range> {
        if !parser.with_ranges {
            return None;
        }
        static TRAILING_PATTERN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?:\s|//[^\n]*|/\*(?:[^*]|\*+[^/*])*\*+/)*$").unwrap());
        let pos = parser.tokens[parser.pos].offset as usize;
        let slice = &parser.input[0..pos];
        let trailing = TRAILING_PATTERN.find(slice).map_or(0, |m| m.len());
        let length = pos - trailing - self.pos;
        Some(ast::Range {
            offset: self.pos as u32,
            length: length as u32,
        })
    }

    fn make_expression<'a>(
        self,
        parser: &mut Parser<'a>,
        expression: ast::expression_node::Expression,
    ) -> usize {
        let range = self.maybe_range(parser);
        let (_, index) = parser.add_expression(expression, range);
        index
    }

    fn make_statement<'a>(
        self,
        parser: &mut Parser<'a>,
        statement: ast::statement_node::Statement,
    ) -> usize {
        let range = self.maybe_range(parser);
        let (_, index) = parser.add_statement(statement, range);
        index
    }
}

macro_rules! assert_token {
    ($parser:expr, $token:ident) => {
        match $parser.next_token()? {
            TokenType::$token => {}
            _ => panic!("internal error"),
        }
    };
}

macro_rules! expect_token {
    ($parser:expr, $token:ident, $expected:literal) => {
        match $parser.next_token()? {
            TokenType::$token => Ok(()),
            _ => $parser.error(format!("expected {}", $expected).as_str()),
        }?
    };
}

macro_rules! parse_token {
    ($parser:expr, $token:ident, $expected:literal) => {
        match $parser.next_token_and_label()? {
            (TokenType::$token, label) => Ok(label),
            _ => $parser.error(format!("expected {}", $expected).as_str()),
        }?
    };
}

#[derive(Debug, Clone)]
struct Parser<'a> {
    /// Path of the source file being parsed.
    path: &'a str,

    /// Indicates whether to include the raw lexical tokens in the parsed AST.
    with_tokens: bool,

    /// Indicates whether to include range information in the parsed AST.
    with_ranges: bool,

    /// The original Starkom source.
    input: &'a str,

    /// Token array output by the lexer.
    tokens: Vec<Token>,

    /// Index of the current token. We move this forward as we consume tokens.
    pos: usize,

    /// Line start offsets, as per the corresponding field in the root AST node. Must be empty if
    /// `with_ranges` is false.
    line_starts: Vec<usize>,

    /// The main component declared in the parsed file. It's initialized to `None` and updated when
    /// the main component is encountered, otherwise it stays `None`.
    main_component: Option<ast::MainComponent>,

    /// Expression pool. See the corresponding `ast::File::expressions` field for details.
    expressions: Vec<ast::ExpressionNode>,

    /// Statement pool. See the corresponding `ast::File::statements` field for details.
    statements: Vec<ast::StatementNode>,
}

impl<'a> Parser<'a> {
    fn new(
        path: &'a str,
        input: &'a str,
        tokens: Vec<Token>,
        line_starts: Vec<usize>,
        with_tokens: bool,
        with_ranges: bool,
    ) -> Self {
        Self {
            path,
            with_tokens,
            with_ranges,
            input,
            tokens,
            pos: 0,
            line_starts,
            main_component: None,

            // The first expression and the first statement are unused because index 0 must be used
            // as a sentinel value for both arrays.
            expressions: vec![ast::ExpressionNode::default()],
            statements: vec![ast::StatementNode::default()],
        }
    }

    fn error<T>(&self, message: &str) -> Result<T> {
        Err(Error::new(
            self.path.to_string(),
            message.to_string(),
            self.tokens[self.pos].offset as usize,
            self.line_starts.as_slice(),
        ))
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_comments(&mut self) {
        while !self.tokens.is_empty() {
            match self.tokens[self.pos].token_type() {
                TokenType::TokenTypeSingleLineComment | TokenType::TokenTypeMultiLineComment => {
                    self.advance()
                }
                _ => return,
            };
        }
    }

    fn peek_token(&mut self) -> Result<TokenType> {
        self.skip_comments();
        if self.tokens.is_empty() {
            return self.error("unexpected end of file");
        }
        Ok(self.tokens[self.pos].token_type())
    }

    fn next_token_and_label(&mut self) -> Result<(TokenType, &str)> {
        self.skip_comments();
        if self.tokens.is_empty() {
            return self.error("unexpected end of file");
        }
        self.advance();
        let token = &self.tokens[self.pos - 1];
        Ok((token.token_type(), token.label.as_str()))
    }

    fn next_token(&mut self) -> Result<TokenType> {
        let (token, _) = self.next_token_and_label()?;
        Ok(token)
    }

    fn add_expression(
        &mut self,
        expression: ast::expression_node::Expression,
        range: Option<ast::Range>,
    ) -> (&ast::ExpressionNode, usize) {
        let index = self.expressions.len();
        self.expressions.push(ast::ExpressionNode {
            range,
            expression: Some(expression),
        });
        (&self.expressions[index], index)
    }

    fn add_statement(
        &mut self,
        statement: ast::statement_node::Statement,
        range: Option<ast::Range>,
    ) -> (&ast::StatementNode, usize) {
        let index = self.statements.len();
        self.statements.push(ast::StatementNode {
            range,
            statement: Some(statement),
        });
        (&self.statements[index], index)
    }

    fn frame(&mut self) -> NodeFrame {
        self.skip_comments();
        NodeFrame {
            pos: self.tokens[self.pos].offset as usize,
        }
    }

    fn parse_version(&mut self) -> Result<ast::Version> {
        let frame = self.frame();
        expect_token!(self, TokenTypeKeywordPragma, "`pragma`");
        expect_token!(self, TokenTypeKeywordStarkom, "`starkom`");
        let (major, minor, patch) = match self.next_token_and_label()? {
            (TokenType::TokenTypeVersionNumber, label) => {
                static REGEX_VERSION_NUMBER: LazyLock<Regex> =
                    LazyLock::new(|| Regex::new(r"^(\d{1,9})\.(\d{1,9})\.(\d{1,9})\b").unwrap());
                let captures = REGEX_VERSION_NUMBER.captures(label).unwrap();
                Ok((
                    captures[1].parse::<u32>().unwrap(),
                    captures[2].parse::<u32>().unwrap(),
                    captures[3].parse::<u32>().unwrap(),
                ))
            }
            _ => self.error("syntax error"),
        }?;
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(ast::Version {
            range: frame.maybe_range(self),
            major,
            minor,
            patch,
        })
    }

    fn parse_include(&mut self) -> Result<String> {
        assert_token!(self, TokenTypeKeywordInclude);
        let path = parse_token!(self, TokenTypeStringLiteral, "file path").to_string();
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(path)
    }

    fn parse_tuple_or_subexpression(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeLeftParenthesis);
        match self.peek_token()? {
            TokenType::TokenTypeRightParenthesis => {
                self.advance();
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::Tuple(ast::TupleExpression {
                        components: vec![],
                    }),
                ))
            }
            _ => {
                let first_index = self.parse_expression()?;
                match self.next_token()? {
                    TokenType::TokenTypeComma => match self.peek_token()? {
                        TokenType::TokenTypeRightParenthesis => {
                            self.advance();
                            Ok(frame.make_expression(
                                self,
                                ast::expression_node::Expression::Tuple(ast::TupleExpression {
                                    components: vec![first_index as u32],
                                }),
                            ))
                        }
                        _ => {
                            let second_index = self.parse_expression()?;
                            let mut components = vec![first_index, second_index];
                            loop {
                                match self.next_token()? {
                                    TokenType::TokenTypeComma => match self.peek_token()? {
                                        TokenType::TokenTypeRightParenthesis => {
                                            self.advance();
                                            return Ok(frame.make_expression(
                                                self,
                                                ast::expression_node::Expression::Tuple(
                                                    ast::TupleExpression {
                                                        components: components.to_u32(),
                                                    },
                                                ),
                                            ));
                                        }
                                        _ => {
                                            components.push(self.parse_expression()?);
                                        }
                                    },
                                    TokenType::TokenTypeRightParenthesis => {
                                        return Ok(frame.make_expression(
                                            self,
                                            ast::expression_node::Expression::Tuple(
                                                ast::TupleExpression {
                                                    components: components.to_u32(),
                                                },
                                            ),
                                        ));
                                    }
                                    _ => {
                                        return self.error("expected `,` or `)`");
                                    }
                                }
                            }
                        }
                    },
                    TokenType::TokenTypeRightParenthesis => Ok(frame.make_expression(
                        self,
                        ast::expression_node::Expression::SubExpression(ast::SubExpression {
                            inner: first_index as u32,
                        }),
                    )),
                    _ => self.error("expected `,` or `)`"),
                }
            }
        }
    }

    fn parse_array_literal(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeLeftSquareBracket);
        match self.peek_token()? {
            TokenType::TokenTypeRightSquareBracket => {
                self.advance();
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::ArrayLiteral(ast::ArrayLiteral {
                        elements: vec![],
                    }),
                ))
            }
            _ => {
                let index = self.parse_expression()?;
                let mut elements = vec![index];
                loop {
                    match self.next_token()? {
                        TokenType::TokenTypeComma => match self.peek_token()? {
                            TokenType::TokenTypeRightSquareBracket => {
                                self.advance();
                                return Ok(frame.make_expression(
                                    self,
                                    ast::expression_node::Expression::ArrayLiteral(
                                        ast::ArrayLiteral {
                                            elements: elements.to_u32(),
                                        },
                                    ),
                                ));
                            }
                            _ => {
                                elements.push(self.parse_expression()?);
                            }
                        },
                        TokenType::TokenTypeRightSquareBracket => {
                            return Ok(frame.make_expression(
                                self,
                                ast::expression_node::Expression::ArrayLiteral(ast::ArrayLiteral {
                                    elements: elements.to_u32(),
                                }),
                            ));
                        }
                        _ => {
                            return self.error("syntax error");
                        }
                    }
                }
            }
        }
    }

    fn parse_expression_leaf(&mut self) -> Result<usize> {
        match self.peek_token()? {
            TokenType::TokenTypeLeftParenthesis => return self.parse_tuple_or_subexpression(),
            TokenType::TokenTypeLeftSquareBracket => return self.parse_array_literal(),
            _ => {}
        };
        let frame = self.frame();
        match self.next_token_and_label()? {
            (TokenType::TokenTypeKeywordTrue, _) => Ok(frame.make_expression(
                self,
                ast::expression_node::Expression::BooleanLiteral(ast::BooleanLiteral {
                    value: true,
                }),
            )),
            (TokenType::TokenTypeKeywordFalse, _) => Ok(frame.make_expression(
                self,
                ast::expression_node::Expression::BooleanLiteral(ast::BooleanLiteral {
                    value: false,
                }),
            )),
            (TokenType::TokenTypeNumber10, value) => {
                let value = value.to_string();
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::NumericLiteral(ast::NumericLiteral {
                        base: 10,
                        value,
                    }),
                ))
            }
            (TokenType::TokenTypeNumber16, value) => {
                let value = value.to_string();
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::NumericLiteral(ast::NumericLiteral {
                        base: 16,
                        value,
                    }),
                ))
            }
            (TokenType::TokenTypeNumber8, value) => {
                let value = value.to_string();
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::NumericLiteral(ast::NumericLiteral {
                        base: 8,
                        value,
                    }),
                ))
            }
            (TokenType::TokenTypeStringLiteral, value) => {
                let value = value.to_string();
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::StringLiteral(ast::StringLiteral { value }),
                ))
            }
            (TokenType::TokenTypeIdentifier, name) => {
                let name = name.to_string();
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::Variable(ast::VariableExpression { name }),
                ))
            }
            _ => self.error("syntax error"),
        }
    }

    fn parse_invocation(&mut self) -> Result<ast::postfix_expression::Invocation> {
        assert_token!(self, TokenTypeLeftParenthesis);
        match self.peek_token()? {
            TokenType::TokenTypeRightParenthesis => {
                self.advance();
                Ok(ast::postfix_expression::Invocation { arguments: vec![] })
            }
            _ => {
                let mut arguments = vec![self.parse_expression()?];
                loop {
                    match self.next_token()? {
                        TokenType::TokenTypeComma => {
                            arguments.push(self.parse_expression()?);
                        }
                        TokenType::TokenTypeRightParenthesis => {
                            return Ok(ast::postfix_expression::Invocation {
                                arguments: arguments.to_u32(),
                            });
                        }
                        _ => {
                            return self.error("syntax error");
                        }
                    }
                }
            }
        }
    }

    fn parse_postfix_chain(&mut self) -> Result<usize> {
        let frame = self.frame();
        let operand_index = self.parse_expression_leaf()?;
        let mut postfix = vec![];
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeDot => {
                    self.advance();
                    let field_name =
                        parse_token!(self, TokenTypeIdentifier, "identifier").to_string();
                    postfix.push(ast::PostfixExpression {
                        postfix: Some(ast::postfix_expression::Postfix::FieldName(field_name)),
                    });
                }
                TokenType::TokenTypeLeftParenthesis => {
                    let invocation = self.parse_invocation()?;
                    postfix.push(ast::PostfixExpression {
                        postfix: Some(ast::postfix_expression::Postfix::Invocation(invocation)),
                    });
                }
                TokenType::TokenTypeLeftSquareBracket => {
                    self.advance();
                    let subscript_index = self.parse_expression()?;
                    expect_token!(self, TokenTypeRightSquareBracket, "`]`");
                    postfix.push(ast::PostfixExpression {
                        postfix: Some(ast::postfix_expression::Postfix::SubscriptExpression(
                            subscript_index as u32,
                        )),
                    });
                }
                _ => {
                    return Ok(if postfix.is_empty() {
                        operand_index
                    } else {
                        frame.make_expression(
                            self,
                            ast::expression_node::Expression::PostfixChain(
                                ast::PostfixChainExpression {
                                    operand: operand_index as u32,
                                    postfix: postfix,
                                },
                            ),
                        )
                    });
                }
            }
        }
    }

    fn parse_prefix_chain(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut types = vec![];
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeOperatorIncrement => {
                    self.advance();
                    types.push(ast::prefix_chain_expression::Type::PrefixExressionIncrement.into());
                }
                TokenType::TokenTypeOperatorDecrement => {
                    self.advance();
                    types.push(ast::prefix_chain_expression::Type::PrefixExressionDecrement.into());
                }
                TokenType::TokenTypeOperatorLogicalNot => {
                    self.advance();
                    types
                        .push(ast::prefix_chain_expression::Type::PrefixExressionLogicalNot.into());
                }
                TokenType::TokenTypeOperatorBitwiseNot => {
                    self.advance();
                    types
                        .push(ast::prefix_chain_expression::Type::PrefixExressionBitwiseNot.into());
                }
                TokenType::TokenTypeOperatorPlus => {
                    self.advance();
                    types.push(ast::prefix_chain_expression::Type::PrefixExressionUnaryPlus.into());
                }
                TokenType::TokenTypeOperatorMinus => {
                    self.advance();
                    types
                        .push(ast::prefix_chain_expression::Type::PrefixExressionUnaryMinus.into());
                }
                _ => {
                    let operand_index = self.parse_postfix_chain()?;
                    return Ok(if types.is_empty() {
                        operand_index
                    } else {
                        frame.make_expression(
                            self,
                            ast::expression_node::Expression::PrefixChain(
                                ast::PrefixChainExpression {
                                    operand: operand_index as u32,
                                    types,
                                },
                            ),
                        )
                    });
                }
            };
        }
    }

    fn parse_exponentiation(&mut self) -> Result<usize> {
        let frame = self.frame();
        let lhs_index = self.parse_prefix_chain()?;
        match self.peek_token()? {
            TokenType::TokenTypeOperatorPower => {
                self.advance();
                let rhs_index = self.parse_exponentiation()?;
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                        r#type: ast::infix_expression::Type::InfixExpressionTypePower.into(),
                        lhs: lhs_index as u32,
                        rhs: rhs_index as u32,
                    }),
                ))
            }
            _ => Ok(lhs_index),
        }
    }

    fn parse_multiplicative_operations(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_exponentiation()?;
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeOperatorMultiply => {
                    self.advance();
                    let rhs_index = self.parse_exponentiation()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeMultiply.into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorDivide => {
                    self.advance();
                    let rhs_index = self.parse_exponentiation()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeDivide.into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorDivideInteger => {
                    self.advance();
                    let rhs_index = self.parse_exponentiation()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeDivideInteger
                                .into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorModulus => {
                    self.advance();
                    let rhs_index = self.parse_exponentiation()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeModulus.into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                _ => {
                    return Ok(index);
                }
            }
        }
    }

    fn parse_additive_operations(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_multiplicative_operations()?;
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeOperatorPlus => {
                    self.advance();
                    let rhs_index = self.parse_multiplicative_operations()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeAdd.into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorMinus => {
                    self.advance();
                    let rhs_index = self.parse_multiplicative_operations()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeSubtract.into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                _ => {
                    return Ok(index);
                }
            }
        }
    }

    fn parse_bitwise_shifts(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_additive_operations()?;
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeOperatorShiftLeft => {
                    self.advance();
                    let rhs_index = self.parse_additive_operations()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeShiftLeft
                                .into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorShiftRight => {
                    self.advance();
                    let rhs_index = self.parse_additive_operations()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeShiftRight
                                .into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                _ => {
                    return Ok(index);
                }
            }
        }
    }

    fn parse_bitwise_and(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_bitwise_shifts()?;
        while let TokenType::TokenTypeOperatorBitwiseAnd = self.peek_token()? {
            self.advance();
            let rhs_index = self.parse_bitwise_shifts()?;
            index = frame.clone().make_expression(
                self,
                ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                    r#type: ast::infix_expression::Type::InfixExpressionTypeBitwiseAnd.into(),
                    lhs: index as u32,
                    rhs: rhs_index as u32,
                }),
            );
        }
        Ok(index)
    }

    fn parse_bitwise_xor(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_bitwise_and()?;
        while let TokenType::TokenTypeOperatorBitwiseXor = self.peek_token()? {
            self.advance();
            let rhs_index = self.parse_bitwise_and()?;
            index = frame.clone().make_expression(
                self,
                ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                    r#type: ast::infix_expression::Type::InfixExpressionTypeBitwiseXor.into(),
                    lhs: index as u32,
                    rhs: rhs_index as u32,
                }),
            );
        }
        Ok(index)
    }

    fn parse_bitwise_or(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_bitwise_xor()?;
        while let TokenType::TokenTypeOperatorBitwiseOr = self.peek_token()? {
            self.advance();
            let rhs_index = self.parse_bitwise_xor()?;
            index = frame.clone().make_expression(
                self,
                ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                    r#type: ast::infix_expression::Type::InfixExpressionTypeBitwiseOr.into(),
                    lhs: index as u32,
                    rhs: rhs_index as u32,
                }),
            );
        }
        Ok(index)
    }

    fn parse_relational_operations(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_bitwise_or()?;
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeOperatorLessThan => {
                    self.advance();
                    let rhs_index = self.parse_bitwise_or()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeLessThan.into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorLessThanOrEqualTo => {
                    self.advance();
                    let rhs_index = self.parse_bitwise_or()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type:
                                ast::infix_expression::Type::InfixExpressionTypeLessThanOrEqualTo
                                    .into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorGreaterThan => {
                    self.advance();
                    let rhs_index = self.parse_bitwise_or()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeGreaterThan
                                .into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorGreaterThanOrEqualTo => {
                    self.advance();
                    let rhs_index = self.parse_bitwise_or()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type:
                                ast::infix_expression::Type::InfixExpressionTypeGreaterThanOrEqualTo
                                    .into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                _ => {
                    return Ok(index);
                }
            }
        }
    }

    fn parse_equality_operations(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_relational_operations()?;
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeOperatorCompareEqual => {
                    self.advance();
                    let rhs_index = self.parse_relational_operations()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeEqualTo.into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                TokenType::TokenTypeOperatorCompareNotEqual => {
                    self.advance();
                    let rhs_index = self.parse_relational_operations()?;
                    index = frame.clone().make_expression(
                        self,
                        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                            r#type: ast::infix_expression::Type::InfixExpressionTypeNotEqualTo
                                .into(),
                            lhs: index as u32,
                            rhs: rhs_index as u32,
                        }),
                    );
                }
                _ => {
                    return Ok(index);
                }
            }
        }
    }

    fn parse_logical_and(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_equality_operations()?;
        while let TokenType::TokenTypeOperatorLogicalAnd = self.peek_token()? {
            self.advance();
            let rhs_index = self.parse_equality_operations()?;
            index = frame.clone().make_expression(
                self,
                ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                    r#type: ast::infix_expression::Type::InfixExpressionTypeLogicalAnd.into(),
                    lhs: index as u32,
                    rhs: rhs_index as u32,
                }),
            );
        }
        Ok(index)
    }

    fn parse_logical_or(&mut self) -> Result<usize> {
        let frame = self.frame();
        let mut index = self.parse_logical_and()?;
        while let TokenType::TokenTypeOperatorLogicalOr = self.peek_token()? {
            self.advance();
            let rhs_index = self.parse_logical_and()?;
            index = frame.clone().make_expression(
                self,
                ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
                    r#type: ast::infix_expression::Type::InfixExpressionTypeLogicalOr.into(),
                    lhs: index as u32,
                    rhs: rhs_index as u32,
                }),
            );
        }
        Ok(index)
    }

    fn add_variable_assignment(
        &mut self,
        frame: NodeFrame,
        lhs_index: usize,
        assignment_type: ast::assign_expression::Type,
    ) -> Result<usize> {
        self.advance();
        let rhs_index = self.parse_variable_assignment_expression()?;
        Ok(frame.make_expression(
            self,
            ast::expression_node::Expression::Assign(ast::AssignExpression {
                r#type: assignment_type.into(),
                lhs: lhs_index as u32,
                rhs: rhs_index as u32,
            }),
        ))
    }

    fn parse_variable_assignment_expression(&mut self) -> Result<usize> {
        let frame = self.frame();
        let lhs_index = self.parse_logical_or()?;
        match self.peek_token()? {
            TokenType::TokenTypeOperatorAssign => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeSimple,
            ),
            TokenType::TokenTypeOperatorCompoundAdd => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundAdd,
            ),
            TokenType::TokenTypeOperatorCompoundSubtract => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundSubtract,
            ),
            TokenType::TokenTypeOperatorCompoundMultiply => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundMultiply,
            ),
            TokenType::TokenTypeOperatorCompoundPower => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundPower,
            ),
            TokenType::TokenTypeOperatorCompoundDivide => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundDivide,
            ),
            TokenType::TokenTypeOperatorCompoundDivideInteger => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundDivideInteger,
            ),
            TokenType::TokenTypeOperatorCompoundModulus => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundModulus,
            ),
            TokenType::TokenTypeOperatorCompoundLogicalAnd => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundLogicalAnd,
            ),
            TokenType::TokenTypeOperatorCompoundLogicalOr => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundLogicalOr,
            ),
            TokenType::TokenTypeOperatorCompoundBitwiseAnd => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundBitwiseAnd,
            ),
            TokenType::TokenTypeOperatorCompoundBitwiseOr => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundBitwiseOr,
            ),
            TokenType::TokenTypeOperatorCompoundBitwiseXor => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundBitwiseXor,
            ),
            TokenType::TokenTypeOperatorCompoundShiftLeft => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundShiftLeft,
            ),
            TokenType::TokenTypeOperatorCompoundShiftRight => self.add_variable_assignment(
                frame.clone(),
                lhs_index,
                ast::assign_expression::Type::AssignmentTypeCompoundShiftRight,
            ),
            _ => Ok(lhs_index),
        }
    }

    fn parse_expression(&mut self) -> Result<usize> {
        self.parse_variable_assignment_expression()
    }

    fn parse_signal_assignment(&mut self) -> Result<usize> {
        let frame = self.frame();
        let lhs_index = self.parse_expression()?;
        match self.peek_token()? {
            TokenType::TokenTypeOperatorUnconstrainedAssignLeft => {
                self.advance();
                let rhs_index = self.parse_expression()?;
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::UnconstrainedAssign(
                        ast::UnconstrainedAssignExpression {
                            direction: ast::AssignmentDirection::RightToLeft.into(),
                            lhs: lhs_index as u32,
                            rhs: rhs_index as u32,
                        },
                    ),
                ))
            }
            TokenType::TokenTypeOperatorUnconstrainedAssignRight => {
                self.advance();
                let rhs_index = self.parse_expression()?;
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::UnconstrainedAssign(
                        ast::UnconstrainedAssignExpression {
                            direction: ast::AssignmentDirection::LeftToRight.into(),
                            lhs: lhs_index as u32,
                            rhs: rhs_index as u32,
                        },
                    ),
                ))
            }
            TokenType::TokenTypeOperatorConstrainedAssignLeft => {
                self.advance();
                let rhs_index = self.parse_expression()?;
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::ConstrainedAssign(
                        ast::ConstrainedAssignExpression {
                            direction: ast::AssignmentDirection::RightToLeft.into(),
                            lhs: lhs_index as u32,
                            rhs: rhs_index as u32,
                        },
                    ),
                ))
            }
            TokenType::TokenTypeOperatorConstrainedAssignRight => {
                self.advance();
                let rhs_index = self.parse_expression()?;
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::ConstrainedAssign(
                        ast::ConstrainedAssignExpression {
                            direction: ast::AssignmentDirection::LeftToRight.into(),
                            lhs: lhs_index as u32,
                            rhs: rhs_index as u32,
                        },
                    ),
                ))
            }
            TokenType::TokenTypeOperatorConstrainedEquality => {
                self.advance();
                let rhs_index = self.parse_expression()?;
                Ok(frame.make_expression(
                    self,
                    ast::expression_node::Expression::ConstrainedEquality(
                        ast::ConstrainedEqualityExpression {
                            lhs: lhs_index as u32,
                            rhs: rhs_index as u32,
                        },
                    ),
                ))
            }
            _ => Ok(lhs_index),
        }
    }

    fn parse_declaration_inner(&mut self) -> Result<ast::declaration_statement::Declaration> {
        let (name, modifier) = match self.next_token_and_label()? {
            (TokenType::TokenTypeIdentifier, name) => {
                (name.to_string(), ast::declaration_statement::Modifier::None)
            }
            (TokenType::TokenTypeKeywordInput, _) => {
                let name = parse_token!(self, TokenTypeIdentifier, "identifier").to_string();
                (name, ast::declaration_statement::Modifier::SignalTypeInput)
            }
            (TokenType::TokenTypeKeywordOutput, _) => {
                let name = parse_token!(self, TokenTypeIdentifier, "identifier").to_string();
                (name, ast::declaration_statement::Modifier::SignalTypeOutput)
            }
            _ => {
                return self.error("syntax error");
            }
        };
        let mut dimensions = vec![];
        while let TokenType::TokenTypeLeftSquareBracket = self.peek_token()? {
            self.advance();
            dimensions.push(self.parse_expression()?);
            expect_token!(self, TokenTypeRightSquareBracket, "`]`");
        }
        let initializer = if let TokenType::TokenTypeOperatorAssign = self.peek_token()? {
            self.advance();
            self.parse_expression()?
        } else {
            0
        };
        Ok(ast::declaration_statement::Declaration {
            modifier: modifier.into(),
            name,
            dimensions: dimensions.to_u32(),
            initializer: initializer as u32,
        })
    }

    fn parse_declaration_statement(
        &mut self,
        expected_type: Option<ast::declaration_statement::Type>,
    ) -> Result<usize> {
        let frame = self.frame();
        let declaration_type = match expected_type {
            Some(expected_type) => {
                match expected_type {
                    ast::declaration_statement::Type::DeclarationTypeVariable => {
                        expect_token!(self, TokenTypeKeywordVar, "`var`")
                    }
                    ast::declaration_statement::Type::DeclarationTypeConstant => {
                        expect_token!(self, TokenTypeKeywordConst, "`const`")
                    }
                    ast::declaration_statement::Type::DeclarationTypeSignal => {
                        expect_token!(self, TokenTypeKeywordSignal, "`signal`")
                    }
                    ast::declaration_statement::Type::DeclarationTypeComponent => {
                        expect_token!(self, TokenTypeKeywordComponent, "`component`")
                    }
                };
                expected_type
            }
            None => match self.next_token()? {
                TokenType::TokenTypeKeywordVar => {
                    ast::declaration_statement::Type::DeclarationTypeVariable
                }
                TokenType::TokenTypeKeywordConst => {
                    ast::declaration_statement::Type::DeclarationTypeConstant
                }
                TokenType::TokenTypeKeywordSignal => {
                    ast::declaration_statement::Type::DeclarationTypeSignal
                }
                TokenType::TokenTypeKeywordComponent => {
                    ast::declaration_statement::Type::DeclarationTypeComponent
                }
                _ => {
                    return self
                        .error("syntax error: expected `var`, `const`, `signal`, or `component`");
                }
            },
        };
        let mut declarations = vec![self.parse_declaration_inner()?];
        loop {
            match self.next_token()? {
                TokenType::TokenTypeComma => {
                    declarations.push(self.parse_declaration_inner()?);
                }
                TokenType::TokenTypeSemicolon => {
                    return Ok(frame.make_statement(
                        self,
                        ast::statement_node::Statement::Declaration(ast::DeclarationStatement {
                            r#type: declaration_type.into(),
                            declarations,
                        }),
                    ));
                }
                _ => {
                    return self.error("syntax error");
                }
            }
        }
    }

    fn parse_if_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordIf);
        expect_token!(self, TokenTypeLeftParenthesis, "`(`");
        let condition_index = self.parse_expression()?;
        expect_token!(self, TokenTypeRightParenthesis, "`)`");
        let then_branch_index = self.parse_statement()?;
        match self.peek_token()? {
            TokenType::TokenTypeKeywordElse => {
                self.advance();
                let else_branch_index = self.parse_statement()?;
                Ok(frame.make_statement(
                    self,
                    ast::statement_node::Statement::IfStatement(ast::IfStatement {
                        condition: condition_index as u32,
                        then_branch: then_branch_index as u32,
                        else_branch: else_branch_index as u32,
                    }),
                ))
            }
            _ => Ok(frame.make_statement(
                self,
                ast::statement_node::Statement::IfStatement(ast::IfStatement {
                    condition: condition_index as u32,
                    then_branch: then_branch_index as u32,
                    else_branch: 0,
                }),
            )),
        }
    }

    fn parse_while_loop_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordWhile);
        expect_token!(self, TokenTypeLeftParenthesis, "`(`");
        let condition_index = self.parse_expression()?;
        expect_token!(self, TokenTypeRightParenthesis, "`)`");
        let loop_body_index = self.parse_statement()?;
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::WhileLoopStatement(ast::WhileLoopStatement {
                condition: condition_index as u32,
                body: loop_body_index as u32,
            }),
        ))
    }

    fn parse_do_while_loop_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordDo);
        let loop_body_index = self.parse_statement()?;
        expect_token!(self, TokenTypeKeywordWhile, "`while`");
        expect_token!(self, TokenTypeLeftParenthesis, "`(`");
        let condition_index = self.parse_expression()?;
        expect_token!(self, TokenTypeRightParenthesis, "`)`");
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::DoWhileLoopStatement(ast::DoWhileLoopStatement {
                body: loop_body_index as u32,
                condition: condition_index as u32,
            }),
        ))
    }

    fn parse_for_loop_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordFor);
        expect_token!(self, TokenTypeLeftParenthesis, "`(`");
        let initializer_index = match self.peek_token()? {
            TokenType::TokenTypeSemicolon => self.parse_empty_statement(),
            TokenType::TokenTypeKeywordVar => self.parse_declaration_statement(Some(
                ast::declaration_statement::Type::DeclarationTypeVariable,
            )),
            TokenType::TokenTypeKeywordConst => self.parse_declaration_statement(Some(
                ast::declaration_statement::Type::DeclarationTypeConstant,
            )),
            _ => self.parse_expression_statement(),
        }?;
        let condition_index = match self.peek_token()? {
            TokenType::TokenTypeSemicolon => {
                self.advance();
                0
            }
            _ => {
                let condition_index = self.parse_expression()?;
                expect_token!(self, TokenTypeSemicolon, "semicolon");
                condition_index
            }
        };
        let step_index = match self.peek_token()? {
            TokenType::TokenTypeRightParenthesis => {
                self.advance();
                0
            }
            _ => {
                let step_index = self.parse_expression()?;
                expect_token!(self, TokenTypeRightParenthesis, "`)`");
                step_index
            }
        };
        let body_index = self.parse_statement()?;
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::ForLoopStatement(ast::ForLoopStatement {
                initializer: initializer_index as u32,
                condition: condition_index as u32,
                step: step_index as u32,
                body: body_index as u32,
            }),
        ))
    }

    fn parse_break_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordBreak);
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::BreakStatement(ast::BreakStatement {}),
        ))
    }

    fn parse_continue_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordContinue);
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::ContinueStatement(ast::ContinueStatement {}),
        ))
    }

    fn parse_return_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordReturn);
        let value_index = self.parse_expression()?;
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::ReturnStatement(ast::ReturnStatement {
                return_value: value_index as u32,
            }),
        ))
    }

    fn parse_log_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordLog);
        let value_index = self.parse_expression()?;
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::LogStatement(ast::LogStatement {
                value: value_index as u32,
            }),
        ))
    }

    fn parse_assert_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordAssert);
        let condition_index = self.parse_expression()?;
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::AssertStatement(ast::AssertStatement {
                condition: condition_index as u32,
            }),
        ))
    }

    fn parse_expression_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        let expression_index = self.parse_signal_assignment()?;
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::Expression(ast::ExpressionStatement {
                expression: expression_index as u32,
            }),
        ))
    }

    fn parse_empty_statement(&mut self) -> Result<usize> {
        let frame = self.frame();
        assert_token!(self, TokenTypeSemicolon);
        Ok(frame.make_statement(
            self,
            ast::statement_node::Statement::Empty(ast::EmptyStatement {}),
        ))
    }

    fn parse_block(&mut self) -> Result<usize> {
        let frame = self.frame();
        expect_token!(self, TokenTypeLeftCurlyBracket, "`{`");
        let mut statements = vec![];
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeRightCurlyBracket => {
                    self.advance();
                    return Ok(frame.make_statement(
                        self,
                        ast::statement_node::Statement::Block(ast::Block {
                            statements: statements.to_u32(),
                        }),
                    ));
                }
                _ => {
                    statements.push(self.parse_statement()?);
                }
            }
        }
    }

    fn parse_statement(&mut self) -> Result<usize> {
        match self.peek_token()? {
            TokenType::TokenTypeSemicolon => self.parse_empty_statement(),
            TokenType::TokenTypeLeftCurlyBracket => self.parse_block(),
            TokenType::TokenTypeKeywordVar => self.parse_declaration_statement(None),
            TokenType::TokenTypeKeywordConst => self.parse_declaration_statement(None),
            TokenType::TokenTypeKeywordSignal => self.parse_declaration_statement(None),
            TokenType::TokenTypeKeywordComponent => self.parse_declaration_statement(None),
            TokenType::TokenTypeKeywordIf => self.parse_if_statement(),
            TokenType::TokenTypeKeywordWhile => self.parse_while_loop_statement(),
            TokenType::TokenTypeKeywordDo => self.parse_do_while_loop_statement(),
            TokenType::TokenTypeKeywordFor => self.parse_for_loop_statement(),
            TokenType::TokenTypeKeywordBreak => self.parse_break_statement(),
            TokenType::TokenTypeKeywordContinue => self.parse_continue_statement(),
            TokenType::TokenTypeKeywordReturn => self.parse_return_statement(),
            TokenType::TokenTypeKeywordLog => self.parse_log_statement(),
            TokenType::TokenTypeKeywordAssert => self.parse_assert_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_template(&mut self) -> Result<ast::TemplateDefinition> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordTemplate);
        let name = parse_token!(self, TokenTypeIdentifier, "identifier").to_string();
        expect_token!(self, TokenTypeLeftParenthesis, "`(`");
        let mut params: Vec<String> = vec![];
        match self.next_token_and_label()? {
            (TokenType::TokenTypeIdentifier, param_name) => {
                params.push(param_name.to_string());
                loop {
                    match self.next_token()? {
                        TokenType::TokenTypeComma => match self.next_token_and_label()? {
                            (TokenType::TokenTypeIdentifier, param_name) => {
                                params.push(param_name.to_string());
                            }
                            _ => {
                                return self.error("syntax error");
                            }
                        },
                        TokenType::TokenTypeRightParenthesis => {
                            break;
                        }
                        _ => {
                            return self.error("syntax error");
                        }
                    }
                }
            }
            (TokenType::TokenTypeRightParenthesis, _) => {}
            _ => {
                return self.error("syntax error");
            }
        }
        let body_index = match self.peek_token()? {
            TokenType::TokenTypeLeftCurlyBracket => self.parse_block(),
            _ => self.error("syntax error"),
        }?;
        Ok(ast::TemplateDefinition {
            range: frame.maybe_range(self),
            name,
            params,
            body_index: body_index as u32,
        })
    }

    fn parse_function(&mut self) -> Result<ast::FunctionDefinition> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordFunction);
        let name = parse_token!(self, TokenTypeIdentifier, "identifier").to_string();
        expect_token!(self, TokenTypeLeftParenthesis, "`(`");
        let mut params: Vec<String> = vec![];
        match self.next_token_and_label()? {
            (TokenType::TokenTypeIdentifier, param_name) => {
                params.push(param_name.to_string());
                loop {
                    match self.next_token()? {
                        TokenType::TokenTypeComma => match self.next_token_and_label()? {
                            (TokenType::TokenTypeIdentifier, param_name) => {
                                params.push(param_name.to_string());
                            }
                            _ => {
                                return self.error("syntax error");
                            }
                        },
                        TokenType::TokenTypeRightParenthesis => {
                            break;
                        }
                        _ => {
                            return self.error("syntax error");
                        }
                    }
                }
            }
            (TokenType::TokenTypeRightParenthesis, _) => {}
            _ => {
                return self.error("syntax error");
            }
        }
        let body_index = match self.peek_token()? {
            TokenType::TokenTypeLeftCurlyBracket => self.parse_block(),
            _ => self.error("syntax error"),
        }?;
        Ok(ast::FunctionDefinition {
            range: frame.maybe_range(self),
            name,
            params,
            body_index: body_index as u32,
        })
    }

    fn parse_main_component(&mut self) -> Result<ast::MainComponent> {
        let frame = self.frame();
        assert_token!(self, TokenTypeKeywordComponent);
        if parse_token!(self, TokenTypeIdentifier, "`main`") != "main" {
            return self.error("the main component must be called `main`");
        }
        let mut public_signals = vec![];
        if let TokenType::TokenTypeLeftCurlyBracket = self.peek_token()? {
            self.advance();
            expect_token!(self, TokenTypeKeywordPublic, "`public`");
            expect_token!(self, TokenTypeLeftSquareBracket, "`[`");
            match self.next_token_and_label()? {
                (TokenType::TokenTypeIdentifier, signal_name) => {
                    public_signals.push(signal_name.to_string());
                    loop {
                        match self.next_token()? {
                            TokenType::TokenTypeComma => match self.next_token_and_label()? {
                                (TokenType::TokenTypeIdentifier, signal_name) => {
                                    public_signals.push(signal_name.to_string());
                                }
                                (TokenType::TokenTypeRightSquareBracket, _) => {
                                    break;
                                }
                                _ => {
                                    return self.error("syntax error");
                                }
                            },
                            TokenType::TokenTypeRightSquareBracket => {
                                break;
                            }
                            _ => {
                                return self.error("syntax error");
                            }
                        }
                    }
                }
                (TokenType::TokenTypeRightSquareBracket, _) => {}
                _ => {
                    return self.error("syntax error");
                }
            }
            expect_token!(self, TokenTypeRightCurlyBracket, "`}`");
        }
        expect_token!(self, TokenTypeOperatorAssign, "`=`");
        let instantiation_index = self.parse_expression()?;
        expect_token!(self, TokenTypeSemicolon, "semicolon");
        Ok(ast::MainComponent {
            range: frame.maybe_range(self),
            public_signals,
            instantiation: instantiation_index as u32,
        })
    }

    fn parse(mut self) -> Result<ast::File> {
        let version = self.parse_version()?;
        self.skip_comments();
        let mut includes: Vec<String> = vec![];
        let mut definitions: Vec<ast::Definition> = vec![];
        loop {
            match self.peek_token()? {
                TokenType::TokenTypeKeywordInclude => {
                    includes.push(self.parse_include()?);
                }
                TokenType::TokenTypeKeywordTemplate => {
                    definitions.push(ast::Definition {
                        definition: Some(ast::definition::Definition::TemplateDefinition(
                            self.parse_template()?,
                        )),
                    });
                }
                TokenType::TokenTypeKeywordFunction => {
                    definitions.push(ast::Definition {
                        definition: Some(ast::definition::Definition::FunctionDefinition(
                            self.parse_function()?,
                        )),
                    });
                }
                TokenType::TokenTypeKeywordComponent => {
                    match self.main_component {
                        Some(_) => return self.error("there can be only one main component"),
                        None => self.main_component = Some(self.parse_main_component()?),
                    };
                }
                TokenType::TokenTypeEndOfFile => {
                    assert_eq!(self.pos, self.tokens.len() - 1);
                    return Ok(ast::File {
                        path: self.path.to_string(),
                        line_starts: if self.with_ranges {
                            self.line_starts
                                .into_iter()
                                .map(|offset| offset as u32)
                                .collect()
                        } else {
                            vec![]
                        },
                        tokens: if self.with_tokens {
                            self.tokens
                        } else {
                            vec![]
                        },
                        version: Some(version),
                        includes,
                        definitions,
                        main_component: self.main_component,
                        expressions: self.expressions,
                        statements: self.statements,
                    });
                }
                _ => {
                    return self.error("syntax error");
                }
            }
        }
    }
}

#[wasm_bindgen]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Settings {
    pub with_tokens: bool,
    pub with_ranges: bool,
}

/// Parses a Starkom source file, returning the parsed AST.
///
/// If `settings.with_tokens` is true the returned `ast::File` proto contains the original lexical
/// tokens in the `tokens` field.
///
/// If `settings.with_ranges` is true the returned AST is decorated with information about where
/// each node is located the in the source.
pub fn parse(path: &str, input: &str, settings: Settings) -> Result<ast::File> {
    let (tokens, line_starts) = tokenize(path, input)?;
    let parser = Parser::new(
        path,
        input,
        tokens,
        line_starts,
        settings.with_tokens,
        settings.with_ranges,
    );
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Pools<'a> {
        lhs_expressions: &'a [ast::ExpressionNode],
        rhs_expressions: &'a [ast::ExpressionNode],
        lhs_statements: &'a [ast::StatementNode],
        rhs_statements: &'a [ast::StatementNode],
    }

    trait CompareNodes {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool;
    }

    impl<T: CompareNodes> CompareNodes for Option<T> {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            match (self, other) {
                (Some(lhs), Some(rhs)) => lhs.compare_nodes(rhs, pools),
                (None, None) => true,
                _ => false,
            }
        }
    }

    impl<T: CompareNodes> CompareNodes for Vec<T> {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.len() == other.len()
                && self
                    .iter()
                    .zip(other.iter())
                    .all(|(lhs, rhs)| lhs.compare_nodes(rhs, pools))
        }
    }

    struct ExpressionIndex(u32);
    struct StatementIndex(u32);

    impl CompareNodes for ExpressionIndex {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            pools.lhs_expressions[self.0 as usize]
                .compare_nodes(&pools.rhs_expressions[other.0 as usize], pools)
        }
    }

    impl CompareNodes for StatementIndex {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            pools.lhs_statements[self.0 as usize]
                .compare_nodes(&pools.rhs_statements[other.0 as usize], pools)
        }
    }

    impl CompareNodes for ast::ExpressionNode {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            if self.range != other.range {
                return false;
            }
            match (&self.expression, &other.expression) {
                (Some(lhs), Some(rhs)) => lhs.compare_nodes(rhs, pools),
                (None, None) => true,
                _ => false,
            }
        }
    }

    impl CompareNodes for ast::expression_node::Expression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            use ast::expression_node::Expression;
            match (self, other) {
                (Expression::BooleanLiteral(lhs), Expression::BooleanLiteral(rhs)) => {
                    lhs.value == rhs.value
                }
                (Expression::NumericLiteral(lhs), Expression::NumericLiteral(rhs)) => {
                    (lhs.base, lhs.value.as_str()) == (rhs.base, rhs.value.as_str())
                }
                (Expression::StringLiteral(lhs), Expression::StringLiteral(rhs)) => {
                    lhs.value == rhs.value
                }
                (Expression::ArrayLiteral(lhs), Expression::ArrayLiteral(rhs)) => lhs
                    .elements
                    .iter()
                    .map(|&i| ExpressionIndex(i))
                    .collect::<Vec<_>>()
                    .compare_nodes(
                        &rhs.elements
                            .iter()
                            .map(|&i| ExpressionIndex(i))
                            .collect::<Vec<_>>(),
                        pools,
                    ),
                (Expression::Variable(lhs), Expression::Variable(rhs)) => lhs.name == rhs.name,
                (Expression::SubExpression(lhs), Expression::SubExpression(rhs)) => {
                    ExpressionIndex(lhs.inner).compare_nodes(&ExpressionIndex(rhs.inner), pools)
                }
                (Expression::Tuple(lhs), Expression::Tuple(rhs)) => lhs
                    .components
                    .iter()
                    .map(|&i| ExpressionIndex(i))
                    .collect::<Vec<_>>()
                    .compare_nodes(
                        &rhs.components
                            .iter()
                            .map(|&i| ExpressionIndex(i))
                            .collect::<Vec<_>>(),
                        pools,
                    ),
                (Expression::PostfixChain(lhs), Expression::PostfixChain(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Expression::PrefixChain(lhs), Expression::PrefixChain(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Expression::InfixExpression(lhs), Expression::InfixExpression(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Expression::Assign(lhs), Expression::Assign(rhs)) => lhs.compare_nodes(rhs, pools),
                (Expression::UnconstrainedAssign(lhs), Expression::UnconstrainedAssign(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Expression::ConstrainedAssign(lhs), Expression::ConstrainedAssign(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Expression::ConstrainedEquality(lhs), Expression::ConstrainedEquality(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                _ => false,
            }
        }
    }

    impl CompareNodes for ast::PostfixChainExpression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            ExpressionIndex(self.operand).compare_nodes(&ExpressionIndex(other.operand), pools)
                && self.postfix.compare_nodes(&other.postfix, pools)
        }
    }

    impl CompareNodes for ast::PostfixExpression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            use ast::postfix_expression::Postfix;
            match (&self.postfix, &other.postfix) {
                (Some(Postfix::FieldName(lhs)), Some(Postfix::FieldName(rhs))) => lhs == rhs,
                (
                    Some(Postfix::SubscriptExpression(lhs)),
                    Some(Postfix::SubscriptExpression(rhs)),
                ) => ExpressionIndex(*lhs).compare_nodes(&ExpressionIndex(*rhs), pools),
                (Some(Postfix::Invocation(lhs)), Some(Postfix::Invocation(rhs))) => lhs
                    .arguments
                    .iter()
                    .map(|&i| ExpressionIndex(i))
                    .collect::<Vec<_>>()
                    .compare_nodes(
                        &rhs.arguments
                            .iter()
                            .map(|&i| ExpressionIndex(i))
                            .collect::<Vec<_>>(),
                        pools,
                    ),
                (Some(Postfix::Increment(_)), Some(Postfix::Increment(_))) => true,
                (Some(Postfix::Decrement(_)), Some(Postfix::Decrement(_))) => true,
                (None, None) => true,
                _ => false,
            }
        }
    }

    impl CompareNodes for ast::PrefixChainExpression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.types == other.types
                && ExpressionIndex(self.operand)
                    .compare_nodes(&ExpressionIndex(other.operand), pools)
        }
    }

    impl CompareNodes for ast::InfixExpression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.r#type == other.r#type
                && ExpressionIndex(self.lhs).compare_nodes(&ExpressionIndex(other.lhs), pools)
                && ExpressionIndex(self.rhs).compare_nodes(&ExpressionIndex(other.rhs), pools)
        }
    }

    impl CompareNodes for ast::AssignExpression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.r#type == other.r#type
                && ExpressionIndex(self.lhs).compare_nodes(&ExpressionIndex(other.lhs), pools)
                && ExpressionIndex(self.rhs).compare_nodes(&ExpressionIndex(other.rhs), pools)
        }
    }

    impl CompareNodes for ast::UnconstrainedAssignExpression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.direction == other.direction
                && ExpressionIndex(self.lhs).compare_nodes(&ExpressionIndex(other.lhs), pools)
                && ExpressionIndex(self.rhs).compare_nodes(&ExpressionIndex(other.rhs), pools)
        }
    }

    impl CompareNodes for ast::ConstrainedAssignExpression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.direction == other.direction
                && ExpressionIndex(self.lhs).compare_nodes(&ExpressionIndex(other.lhs), pools)
                && ExpressionIndex(self.rhs).compare_nodes(&ExpressionIndex(other.rhs), pools)
        }
    }

    impl CompareNodes for ast::ConstrainedEqualityExpression {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            ExpressionIndex(self.lhs).compare_nodes(&ExpressionIndex(other.lhs), pools)
                && ExpressionIndex(self.rhs).compare_nodes(&ExpressionIndex(other.rhs), pools)
        }
    }

    impl CompareNodes for ast::StatementNode {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            if self.range != other.range {
                return false;
            }
            match (&self.statement, &other.statement) {
                (Some(lhs), Some(rhs)) => lhs.compare_nodes(rhs, pools),
                (None, None) => true,
                _ => false,
            }
        }
    }

    impl CompareNodes for ast::statement_node::Statement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            use ast::statement_node::Statement;
            match (self, other) {
                (Statement::Empty(_), Statement::Empty(_)) => true,
                (Statement::Block(lhs), Statement::Block(rhs)) => lhs.compare_nodes(rhs, pools),
                (Statement::Declaration(lhs), Statement::Declaration(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Statement::Expression(lhs), Statement::Expression(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Statement::IfStatement(lhs), Statement::IfStatement(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Statement::WhileLoopStatement(lhs), Statement::WhileLoopStatement(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Statement::DoWhileLoopStatement(lhs), Statement::DoWhileLoopStatement(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Statement::ForLoopStatement(lhs), Statement::ForLoopStatement(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Statement::BreakStatement(_), Statement::BreakStatement(_)) => true,
                (Statement::ContinueStatement(_), Statement::ContinueStatement(_)) => true,
                (Statement::ReturnStatement(lhs), Statement::ReturnStatement(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Statement::LogStatement(lhs), Statement::LogStatement(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                (Statement::AssertStatement(lhs), Statement::AssertStatement(rhs)) => {
                    lhs.compare_nodes(rhs, pools)
                }
                _ => false,
            }
        }
    }

    impl CompareNodes for ast::Block {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.statements.len() == other.statements.len()
                && self
                    .statements
                    .iter()
                    .zip(other.statements.iter())
                    .all(|(&lhs, &rhs)| {
                        StatementIndex(lhs).compare_nodes(&StatementIndex(rhs), pools)
                    })
        }
    }

    impl CompareNodes for ast::DeclarationStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.r#type == other.r#type
                && self.declarations.compare_nodes(&other.declarations, pools)
        }
    }

    impl CompareNodes for ast::declaration_statement::Declaration {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            (self.modifier, self.name.as_str()) == (other.modifier, other.name.as_str())
                && self
                    .dimensions
                    .iter()
                    .map(|&i| ExpressionIndex(i))
                    .collect::<Vec<_>>()
                    .compare_nodes(
                        &other
                            .dimensions
                            .iter()
                            .map(|&i| ExpressionIndex(i))
                            .collect::<Vec<_>>(),
                        pools,
                    )
                && ExpressionIndex(self.initializer)
                    .compare_nodes(&ExpressionIndex(other.initializer), pools)
        }
    }

    impl CompareNodes for ast::ExpressionStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            ExpressionIndex(self.expression)
                .compare_nodes(&ExpressionIndex(other.expression), pools)
        }
    }

    impl CompareNodes for ast::IfStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            ExpressionIndex(self.condition).compare_nodes(&ExpressionIndex(other.condition), pools)
                && StatementIndex(self.then_branch)
                    .compare_nodes(&StatementIndex(other.then_branch), pools)
                && StatementIndex(self.else_branch)
                    .compare_nodes(&StatementIndex(other.else_branch), pools)
        }
    }

    impl CompareNodes for ast::WhileLoopStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            ExpressionIndex(self.condition).compare_nodes(&ExpressionIndex(other.condition), pools)
                && StatementIndex(self.body).compare_nodes(&StatementIndex(other.body), pools)
        }
    }

    impl CompareNodes for ast::DoWhileLoopStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            StatementIndex(self.body).compare_nodes(&StatementIndex(other.body), pools)
                && ExpressionIndex(self.condition)
                    .compare_nodes(&ExpressionIndex(other.condition), pools)
        }
    }

    impl CompareNodes for ast::ForLoopStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            StatementIndex(self.initializer)
                .compare_nodes(&StatementIndex(other.initializer), pools)
                && ExpressionIndex(self.condition)
                    .compare_nodes(&ExpressionIndex(other.condition), pools)
                && ExpressionIndex(self.step).compare_nodes(&ExpressionIndex(other.step), pools)
                && StatementIndex(self.body).compare_nodes(&StatementIndex(other.body), pools)
        }
    }

    impl CompareNodes for ast::ReturnStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            ExpressionIndex(self.return_value)
                .compare_nodes(&ExpressionIndex(other.return_value), pools)
        }
    }

    impl CompareNodes for ast::LogStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            ExpressionIndex(self.value).compare_nodes(&ExpressionIndex(other.value), pools)
        }
    }

    impl CompareNodes for ast::AssertStatement {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            ExpressionIndex(self.condition).compare_nodes(&ExpressionIndex(other.condition), pools)
        }
    }

    impl CompareNodes for ast::Definition {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.definition.compare_nodes(&other.definition, pools)
        }
    }

    impl CompareNodes for ast::definition::Definition {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            use ast::definition::Definition::*;
            match (self, other) {
                (TemplateDefinition(lhs), TemplateDefinition(rhs)) => lhs.compare_nodes(rhs, pools),
                (FunctionDefinition(lhs), FunctionDefinition(rhs)) => lhs.compare_nodes(rhs, pools),
                _ => false,
            }
        }
    }

    impl CompareNodes for ast::TemplateDefinition {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.range == other.range
                && self.name == other.name
                && self.params == other.params
                && StatementIndex(self.body_index)
                    .compare_nodes(&StatementIndex(other.body_index), pools)
        }
    }

    impl CompareNodes for ast::FunctionDefinition {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.range == other.range
                && self.name == other.name
                && self.params == other.params
                && StatementIndex(self.body_index)
                    .compare_nodes(&StatementIndex(other.body_index), pools)
        }
    }

    impl CompareNodes for ast::MainComponent {
        fn compare_nodes(&self, other: &Self, pools: &Pools<'_>) -> bool {
            self.range == other.range
                && self.public_signals == other.public_signals
                && ExpressionIndex(self.instantiation)
                    .compare_nodes(&ExpressionIndex(other.instantiation), pools)
        }
    }

    fn assert_ast_eq(lhs: &ast::File, rhs: &ast::File) {
        assert_eq!(lhs.line_starts, rhs.line_starts);
        assert_eq!(lhs.tokens, rhs.tokens);
        assert_eq!(lhs.version, rhs.version);
        assert_eq!(lhs.includes, rhs.includes);

        let pools = Pools {
            lhs_expressions: &lhs.expressions,
            rhs_expressions: &rhs.expressions,
            lhs_statements: &lhs.statements,
            rhs_statements: &rhs.statements,
        };

        assert!(
            lhs.definitions.compare_nodes(&rhs.definitions, &pools),
            "definitions mismatch:\n  lhs: {:#?}\n  rhs: {:#?}",
            lhs.definitions,
            rhs.definitions
        );

        assert!(
            lhs.main_component
                .compare_nodes(&rhs.main_component, &pools),
            "main_component mismatch:\n  lhs: {:#?}\n  rhs: {:#?}",
            lhs.main_component,
            rhs.main_component,
        );
    }

    fn r(offset: u32, length: u32) -> Option<ast::Range> {
        Some(ast::Range { offset, length })
    }

    fn var(name: &str) -> ast::expression_node::Expression {
        ast::expression_node::Expression::Variable(ast::VariableExpression { name: name.into() })
    }

    fn num(value: &str) -> ast::expression_node::Expression {
        ast::expression_node::Expression::NumericLiteral(ast::NumericLiteral {
            base: 10,
            value: value.into(),
        })
    }

    fn mul(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
            r#type: ast::infix_expression::Type::InfixExpressionTypeMultiply.into(),
            lhs,
            rhs,
        })
    }

    fn add(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
            r#type: ast::infix_expression::Type::InfixExpressionTypeAdd.into(),
            lhs,
            rhs,
        })
    }

    fn con_assign(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::ConstrainedAssign(ast::ConstrainedAssignExpression {
            direction: ast::AssignmentDirection::RightToLeft.into(),
            lhs,
            rhs,
        })
    }

    fn con_eq(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::ConstrainedEquality(ast::ConstrainedEqualityExpression {
            lhs,
            rhs,
        })
    }

    fn sig_decl(
        modifier: ast::declaration_statement::Modifier,
        name: &str,
    ) -> ast::statement_node::Statement {
        ast::statement_node::Statement::Declaration(ast::DeclarationStatement {
            r#type: ast::declaration_statement::Type::DeclarationTypeSignal.into(),
            declarations: vec![ast::declaration_statement::Declaration {
                modifier: modifier.into(),
                name: name.into(),
                dimensions: vec![],
                initializer: 0,
            }],
        })
    }

    fn expr_stmt(expression: u32) -> ast::statement_node::Statement {
        ast::statement_node::Statement::Expression(ast::ExpressionStatement { expression })
    }

    fn ex(
        range: Option<ast::Range>,
        expression: ast::expression_node::Expression,
    ) -> ast::ExpressionNode {
        ast::ExpressionNode {
            range,
            expression: Some(expression),
        }
    }

    fn st(
        range: Option<ast::Range>,
        statement: ast::statement_node::Statement,
    ) -> ast::StatementNode {
        ast::StatementNode {
            range,
            statement: Some(statement),
        }
    }

    // Literal expression helpers

    fn bool_lit(value: bool) -> ast::expression_node::Expression {
        ast::expression_node::Expression::BooleanLiteral(ast::BooleanLiteral { value })
    }

    fn str_lit(value: &str) -> ast::expression_node::Expression {
        ast::expression_node::Expression::StringLiteral(ast::StringLiteral {
            value: value.into(),
        })
    }

    fn hex(value: &str) -> ast::expression_node::Expression {
        ast::expression_node::Expression::NumericLiteral(ast::NumericLiteral {
            base: 16,
            value: value.into(),
        })
    }

    fn oct(value: &str) -> ast::expression_node::Expression {
        ast::expression_node::Expression::NumericLiteral(ast::NumericLiteral {
            base: 8,
            value: value.into(),
        })
    }

    // Compound expression helpers

    fn sub_expr(inner: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::SubExpression(ast::SubExpression { inner })
    }

    fn tuple(components: Vec<u32>) -> ast::expression_node::Expression {
        ast::expression_node::Expression::Tuple(ast::TupleExpression { components })
    }

    fn arr_lit(elements: Vec<u32>) -> ast::expression_node::Expression {
        ast::expression_node::Expression::ArrayLiteral(ast::ArrayLiteral { elements })
    }

    fn field_access(operand: u32, field: &str) -> ast::expression_node::Expression {
        ast::expression_node::Expression::PostfixChain(ast::PostfixChainExpression {
            operand,
            postfix: vec![ast::PostfixExpression {
                postfix: Some(ast::postfix_expression::Postfix::FieldName(field.into())),
            }],
        })
    }

    fn subscript(operand: u32, index: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::PostfixChain(ast::PostfixChainExpression {
            operand,
            postfix: vec![ast::PostfixExpression {
                postfix: Some(ast::postfix_expression::Postfix::SubscriptExpression(index)),
            }],
        })
    }

    fn invocation(operand: u32, arguments: Vec<u32>) -> ast::expression_node::Expression {
        ast::expression_node::Expression::PostfixChain(ast::PostfixChainExpression {
            operand,
            postfix: vec![ast::PostfixExpression {
                postfix: Some(ast::postfix_expression::Postfix::Invocation(
                    ast::postfix_expression::Invocation { arguments },
                )),
            }],
        })
    }

    fn prefix_chain_expr(
        operand: u32,
        types: Vec<ast::prefix_chain_expression::Type>,
    ) -> ast::expression_node::Expression {
        ast::expression_node::Expression::PrefixChain(ast::PrefixChainExpression {
            operand,
            types: types.into_iter().map(|t| t as i32).collect(),
        })
    }

    fn infix(
        ty: ast::infix_expression::Type,
        lhs: u32,
        rhs: u32,
    ) -> ast::expression_node::Expression {
        ast::expression_node::Expression::InfixExpression(ast::InfixExpression {
            r#type: ty as i32,
            lhs,
            rhs,
        })
    }

    fn sub(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeSubtract,
            lhs,
            rhs,
        )
    }

    fn div(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeDivide,
            lhs,
            rhs,
        )
    }

    fn int_div(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeDivideInteger,
            lhs,
            rhs,
        )
    }

    fn modulus(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeModulus,
            lhs,
            rhs,
        )
    }

    fn power(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypePower,
            lhs,
            rhs,
        )
    }

    fn shift_left(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeShiftLeft,
            lhs,
            rhs,
        )
    }

    fn shift_right(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeShiftRight,
            lhs,
            rhs,
        )
    }

    fn lt(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeLessThan,
            lhs,
            rhs,
        )
    }

    fn le(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeLessThanOrEqualTo,
            lhs,
            rhs,
        )
    }

    fn gt(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeGreaterThan,
            lhs,
            rhs,
        )
    }

    fn ge(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeGreaterThanOrEqualTo,
            lhs,
            rhs,
        )
    }

    fn compare_eq(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeEqualTo,
            lhs,
            rhs,
        )
    }

    fn compare_ne(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeNotEqualTo,
            lhs,
            rhs,
        )
    }

    fn logical_and(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeLogicalAnd,
            lhs,
            rhs,
        )
    }

    fn logical_or(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeLogicalOr,
            lhs,
            rhs,
        )
    }

    fn bitwise_and(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeBitwiseAnd,
            lhs,
            rhs,
        )
    }

    fn bitwise_xor(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeBitwiseXor,
            lhs,
            rhs,
        )
    }

    fn bitwise_or(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        infix(
            ast::infix_expression::Type::InfixExpressionTypeBitwiseOr,
            lhs,
            rhs,
        )
    }

    fn assign(
        ty: ast::assign_expression::Type,
        lhs: u32,
        rhs: u32,
    ) -> ast::expression_node::Expression {
        ast::expression_node::Expression::Assign(ast::AssignExpression {
            r#type: ty as i32,
            lhs,
            rhs,
        })
    }

    fn unconstrained_rtl(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::UnconstrainedAssign(ast::UnconstrainedAssignExpression {
            direction: ast::AssignmentDirection::RightToLeft as i32,
            lhs,
            rhs,
        })
    }

    fn unconstrained_ltr(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::UnconstrainedAssign(ast::UnconstrainedAssignExpression {
            direction: ast::AssignmentDirection::LeftToRight as i32,
            lhs,
            rhs,
        })
    }

    fn con_assign_ltr(lhs: u32, rhs: u32) -> ast::expression_node::Expression {
        ast::expression_node::Expression::ConstrainedAssign(ast::ConstrainedAssignExpression {
            direction: ast::AssignmentDirection::LeftToRight as i32,
            lhs,
            rhs,
        })
    }

    // Statement helpers

    fn block_st(statements: Vec<u32>) -> ast::statement_node::Statement {
        ast::statement_node::Statement::Block(ast::Block { statements })
    }

    fn if_st(condition: u32, then_branch: u32, else_branch: u32) -> ast::statement_node::Statement {
        ast::statement_node::Statement::IfStatement(ast::IfStatement {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn while_st(condition: u32, body: u32) -> ast::statement_node::Statement {
        ast::statement_node::Statement::WhileLoopStatement(ast::WhileLoopStatement {
            condition,
            body,
        })
    }

    fn do_while_st(body: u32, condition: u32) -> ast::statement_node::Statement {
        ast::statement_node::Statement::DoWhileLoopStatement(ast::DoWhileLoopStatement {
            body,
            condition,
        })
    }

    fn for_st(
        initializer: u32,
        condition: u32,
        step: u32,
        body: u32,
    ) -> ast::statement_node::Statement {
        ast::statement_node::Statement::ForLoopStatement(ast::ForLoopStatement {
            initializer,
            condition,
            step,
            body,
        })
    }

    fn break_st() -> ast::statement_node::Statement {
        ast::statement_node::Statement::BreakStatement(ast::BreakStatement {})
    }

    fn continue_st() -> ast::statement_node::Statement {
        ast::statement_node::Statement::ContinueStatement(ast::ContinueStatement {})
    }

    fn return_st(return_value: u32) -> ast::statement_node::Statement {
        ast::statement_node::Statement::ReturnStatement(ast::ReturnStatement { return_value })
    }

    fn log_st(value: u32) -> ast::statement_node::Statement {
        ast::statement_node::Statement::LogStatement(ast::LogStatement { value })
    }

    fn assert_st(condition: u32) -> ast::statement_node::Statement {
        ast::statement_node::Statement::AssertStatement(ast::AssertStatement { condition })
    }

    fn empty_st() -> ast::statement_node::Statement {
        ast::statement_node::Statement::Empty(ast::EmptyStatement {})
    }

    fn var_decl_st(name: &str) -> ast::statement_node::Statement {
        ast::statement_node::Statement::Declaration(ast::DeclarationStatement {
            r#type: ast::declaration_statement::Type::DeclarationTypeVariable as i32,
            declarations: vec![ast::declaration_statement::Declaration {
                modifier: ast::declaration_statement::Modifier::None as i32,
                name: name.into(),
                dimensions: vec![],
                initializer: 0,
            }],
        })
    }

    fn comp_decl_st(name: &str) -> ast::statement_node::Statement {
        ast::statement_node::Statement::Declaration(ast::DeclarationStatement {
            r#type: ast::declaration_statement::Type::DeclarationTypeComponent as i32,
            declarations: vec![ast::declaration_statement::Declaration {
                modifier: ast::declaration_statement::Modifier::None as i32,
                name: name.into(),
                dimensions: vec![],
                initializer: 0,
            }],
        })
    }

    fn sig_out_decl(name: &str) -> ast::statement_node::Statement {
        ast::statement_node::Statement::Declaration(ast::DeclarationStatement {
            r#type: ast::declaration_statement::Type::DeclarationTypeSignal as i32,
            declarations: vec![ast::declaration_statement::Declaration {
                modifier: ast::declaration_statement::Modifier::SignalTypeOutput as i32,
                name: name.into(),
                dimensions: vec![],
                initializer: 0,
            }],
        })
    }

    // Parse a source string and panic on error; all test-only parse calls go through here.
    fn p(source: &str) -> ast::File {
        parse("test", source, Settings::default()).unwrap()
    }

    fn p_ranges(source: &str) -> ast::File {
        parse(
            "test",
            source,
            Settings {
                with_tokens: false,
                with_ranges: true,
            },
        )
        .unwrap()
    }

    // Wrap a body string in a minimal template for expression/statement tests.
    fn in_template(body: &str) -> ast::File {
        p(&format!(
            "pragma starkom 0.0.0;\ntemplate T() {{ {} }}",
            body
        ))
    }

    fn in_template_ranges(body: &str) -> ast::File {
        p_ranges(&format!(
            "pragma starkom 0.0.0;\ntemplate T() {{ {} }}",
            body
        ))
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_include() {
        let ast = p("pragma starkom 0.0.0;\ninclude \"foo.sk\";\n");
        assert_eq!(ast.includes, vec!["\"foo.sk\""]);
    }

    #[test]
    fn test_multiple_includes() {
        let ast = p("pragma starkom 0.0.0;\ninclude \"a.sk\";\ninclude \"b.sk\";\n");
        assert_eq!(ast.includes, vec!["\"a.sk\"", "\"b.sk\""]);
    }

    #[test]
    fn test_function_definition() {
        let ast = p("pragma starkom 0.0.0;\nfunction Add(a, b) { return a + b; }");
        assert_eq!(ast.definitions.len(), 1);
        let def = match ast.definitions[0].definition.as_ref().unwrap() {
            ast::definition::Definition::FunctionDefinition(f) => f,
            _ => panic!("expected FunctionDefinition"),
        };
        assert_eq!(def.name, "Add");
        assert_eq!(def.params, vec!["a", "b"]);
        // expressions: [0]=sentinel, [1]=Variable("a"), [2]=Variable("b"), [3]=Add(1,2)
        assert_eq!(ast.expressions[1], ex(None, var("a")));
        assert_eq!(ast.expressions[2], ex(None, var("b")));
        assert_eq!(ast.expressions[3], ex(None, add(1, 2)));
        // statements: [0]=sentinel, [1]=Return{3}, [2]=Block([1])
        assert_eq!(ast.statements[1], st(None, return_st(3)));
        assert_eq!(ast.statements[2], st(None, block_st(vec![1])));
        assert_eq!(def.body_index, 2);
    }

    #[test]
    fn test_function_no_params() {
        let ast = p("pragma starkom 0.0.0;\nfunction F() { return 0; }");
        let def = match ast.definitions[0].definition.as_ref().unwrap() {
            ast::definition::Definition::FunctionDefinition(f) => f,
            _ => panic!("expected FunctionDefinition"),
        };
        assert_eq!(def.params, Vec::<String>::new());
    }

    #[test]
    fn test_template_with_params() {
        let ast = p("pragma starkom 0.0.0;\ntemplate Adder(n, m) {}");
        let def = match ast.definitions[0].definition.as_ref().unwrap() {
            ast::definition::Definition::TemplateDefinition(t) => t,
            _ => panic!("expected TemplateDefinition"),
        };
        assert_eq!(def.name, "Adder");
        assert_eq!(def.params, vec!["n", "m"]);
        // empty body: statements[1] = Block([])
        assert_eq!(ast.statements[1], st(None, block_st(vec![])));
        assert_eq!(def.body_index, 1);
    }

    #[test]
    fn test_main_with_public_signals() {
        let ast = p("pragma starkom 0.0.0;\ncomponent main {public [x, y]} = T();");
        let mc = ast.main_component.as_ref().unwrap();
        assert_eq!(mc.public_signals, vec!["x", "y"]);
        // expressions: [0]=sentinel, [1]=Variable("T"), [2]=PostfixChain(T, [Invocation([])])
        assert_eq!(ast.expressions[2], ex(None, invocation(1, vec![])));
        assert_eq!(mc.instantiation, 2);
    }

    #[test]
    fn test_main_with_empty_public_signals() {
        let ast = p("pragma starkom 0.0.0;\ncomponent main {public []} = T();");
        let mc = ast.main_component.as_ref().unwrap();
        assert_eq!(mc.public_signals, Vec::<String>::new());
    }

    #[test]
    fn test_main_with_one_public_signal() {
        let ast = p("pragma starkom 0.0.0;\ncomponent main {public [x]} = T();");
        let mc = ast.main_component.as_ref().unwrap();
        assert_eq!(mc.public_signals, vec!["x"]);
    }

    #[test]
    fn test_main_with_public_signals_trailing_comma() {
        let ast = p("pragma starkom 0.0.0;\ncomponent main {public [x, y,]} = T();");
        let mc = ast.main_component.as_ref().unwrap();
        assert_eq!(mc.public_signals, vec!["x", "y"]);
    }

    #[test]
    fn test_multiple_definitions() {
        let ast = p(
            "pragma starkom 0.0.0;\ntemplate A() {}\ntemplate B() {}\nfunction F() { return 0; }",
        );
        assert_eq!(ast.definitions.len(), 3);
    }

    #[test]
    fn test_error_duplicate_main_component() {
        assert!(
            parse(
                "test",
                "pragma starkom 0.0.0;\ncomponent main = T();\ncomponent main = T();\n",
                Settings::default(),
            )
            .is_err()
        );
    }

    #[test]
    fn test_comments_skipped() {
        let ast = p("pragma starkom 0.0.0; // line comment\ntemplate T() { /* block */ }");
        assert_eq!(ast.definitions.len(), 1);
    }

    #[test]
    fn test_literal_true() {
        let ast = in_template("true;");
        assert_eq!(ast.expressions[1], ex(None, bool_lit(true)));
    }

    #[test]
    fn test_literal_false() {
        let ast = in_template("false;");
        assert_eq!(ast.expressions[1], ex(None, bool_lit(false)));
    }

    #[test]
    fn test_literal_hex() {
        let ast = in_template("0xFF;");
        assert_eq!(ast.expressions[1], ex(None, hex("0xFF")));
    }

    #[test]
    fn test_literal_octal() {
        let ast = in_template("077;");
        assert_eq!(ast.expressions[1], ex(None, oct("077")));
    }

    #[test]
    fn test_literal_string() {
        // The lexer preserves the surrounding quotes in the token value.
        let ast = in_template("\"hello\";");
        assert_eq!(ast.expressions[1], ex(None, str_lit("\"hello\"")));
    }

    #[test]
    fn test_sub_expression() {
        // (x) → SubExpression { inner: 1 }
        let ast = in_template("(x);");
        assert_eq!(ast.expressions[1], ex(None, var("x")));
        assert_eq!(ast.expressions[2], ex(None, sub_expr(1)));
    }

    #[test]
    fn test_tuple_empty() {
        let ast = in_template("();");
        assert_eq!(ast.expressions[1], ex(None, tuple(vec![])));
    }

    #[test]
    fn test_tuple_one_element() {
        // (x,) → single-element tuple
        let ast = in_template("(x,);");
        assert_eq!(ast.expressions[2], ex(None, tuple(vec![1])));
    }

    #[test]
    fn test_tuple_two_elements() {
        let ast = in_template("(a, b);");
        assert_eq!(ast.expressions[3], ex(None, tuple(vec![1, 2])));
    }

    #[test]
    fn test_tuple_three_elements() {
        let ast = in_template("(a, b, c);");
        assert_eq!(ast.expressions[4], ex(None, tuple(vec![1, 2, 3])));
    }

    #[test]
    fn test_tuple_trailing_comma() {
        // (a, b,) parses the same as (a, b) but via a different code path.
        let ast = in_template("(a, b,);");
        assert_eq!(ast.expressions[3], ex(None, tuple(vec![1, 2])));
    }

    #[test]
    fn test_array_literal_empty() {
        let ast = in_template("[];");
        assert_eq!(ast.expressions[1], ex(None, arr_lit(vec![])));
    }

    #[test]
    fn test_array_literal_one_element() {
        let ast = in_template("[1];");
        assert_eq!(ast.expressions[2], ex(None, arr_lit(vec![1])));
    }

    #[test]
    fn test_array_literal_multiple_elements() {
        let ast = in_template("[1, 2, 3];");
        assert_eq!(ast.expressions[4], ex(None, arr_lit(vec![1, 2, 3])));
    }

    #[test]
    fn test_array_literal_trailing_comma() {
        let ast = in_template("[1, 2,];");
        assert_eq!(ast.expressions[3], ex(None, arr_lit(vec![1, 2])));
    }

    #[test]
    fn test_field_access() {
        let ast = in_template("a.b;");
        assert_eq!(ast.expressions[1], ex(None, var("a")));
        assert_eq!(ast.expressions[2], ex(None, field_access(1, "b")));
    }

    #[test]
    fn test_field_access_chained() {
        // a.b.c: parsed as PostfixChain(a, [FieldName("b"), FieldName("c")])
        let ast = in_template("a.b.c;");
        assert_eq!(
            ast.expressions[2].expression.as_ref().unwrap(),
            &ast::expression_node::Expression::PostfixChain(ast::PostfixChainExpression {
                operand: 1,
                postfix: vec![
                    ast::PostfixExpression {
                        postfix: Some(ast::postfix_expression::Postfix::FieldName("b".into())),
                    },
                    ast::PostfixExpression {
                        postfix: Some(ast::postfix_expression::Postfix::FieldName("c".into())),
                    },
                ],
            })
        );
    }

    #[test]
    fn test_subscript() {
        let ast = in_template("a[i];");
        assert_eq!(ast.expressions[1], ex(None, var("a")));
        assert_eq!(ast.expressions[2], ex(None, var("i")));
        assert_eq!(ast.expressions[3], ex(None, subscript(1, 2)));
    }

    #[test]
    fn test_invocation_no_args() {
        let ast = in_template("f();");
        assert_eq!(ast.expressions[2], ex(None, invocation(1, vec![])));
    }

    #[test]
    fn test_invocation_with_args() {
        let ast = in_template("f(x, y);");
        // [1]=f, [2]=x, [3]=y, [4]=PostfixChain(1, [Invocation([2,3])])
        assert_eq!(ast.expressions[4], ex(None, invocation(1, vec![2, 3])));
    }

    #[test]
    fn test_prefix_increment() {
        let ast = in_template("++x;");
        assert_eq!(
            ast.expressions[2],
            ex(
                None,
                prefix_chain_expr(
                    1,
                    vec![ast::prefix_chain_expression::Type::PrefixExressionIncrement]
                )
            )
        );
    }

    #[test]
    fn test_prefix_decrement() {
        let ast = in_template("--x;");
        assert_eq!(
            ast.expressions[2],
            ex(
                None,
                prefix_chain_expr(
                    1,
                    vec![ast::prefix_chain_expression::Type::PrefixExressionDecrement]
                )
            )
        );
    }

    #[test]
    fn test_prefix_boolean_not() {
        let ast = in_template("!x;");
        assert_eq!(
            ast.expressions[2],
            ex(
                None,
                prefix_chain_expr(
                    1,
                    vec![ast::prefix_chain_expression::Type::PrefixExressionLogicalNot]
                )
            )
        );
    }

    #[test]
    fn test_prefix_bitwise_not() {
        let ast = in_template("~x;");
        assert_eq!(
            ast.expressions[2],
            ex(
                None,
                prefix_chain_expr(
                    1,
                    vec![ast::prefix_chain_expression::Type::PrefixExressionBitwiseNot]
                )
            )
        );
    }

    #[test]
    fn test_prefix_unary_plus() {
        let ast = in_template("+x;");
        assert_eq!(
            ast.expressions[2],
            ex(
                None,
                prefix_chain_expr(
                    1,
                    vec![ast::prefix_chain_expression::Type::PrefixExressionUnaryPlus]
                )
            )
        );
    }

    #[test]
    fn test_prefix_unary_minus() {
        let ast = in_template("-x;");
        assert_eq!(
            ast.expressions[2],
            ex(
                None,
                prefix_chain_expr(
                    1,
                    vec![ast::prefix_chain_expression::Type::PrefixExressionUnaryMinus]
                )
            )
        );
    }

    #[test]
    fn test_exponentiation() {
        let ast = in_template("2 ** 3;");
        assert_eq!(ast.expressions[3], ex(None, power(1, 2)));
    }

    #[test]
    fn test_exponentiation_right_associative() {
        // 2 ** 3 ** 4 == 2 ** (3 ** 4)
        let ast = in_template("2 ** 3 ** 4;");
        // [1]=2, [2]=3, [3]=4, [4]=Power(2,3), [5]=Power(1,4)
        assert_eq!(ast.expressions[4], ex(None, power(2, 3)));
        assert_eq!(ast.expressions[5], ex(None, power(1, 4)));
    }

    #[test]
    fn test_division() {
        let ast = in_template("6 / 2;");
        assert_eq!(ast.expressions[3], ex(None, div(1, 2)));
    }

    #[test]
    fn test_integer_division() {
        let ast = in_template("7 \\ 2;");
        assert_eq!(ast.expressions[3], ex(None, int_div(1, 2)));
    }

    #[test]
    fn test_modulus() {
        let ast = in_template("7 % 3;");
        assert_eq!(ast.expressions[3], ex(None, modulus(1, 2)));
    }

    #[test]
    fn test_subtraction() {
        let ast = in_template("5 - 3;");
        assert_eq!(ast.expressions[3], ex(None, sub(1, 2)));
    }

    #[test]
    fn test_shift_left() {
        let ast = in_template("1 << 4;");
        assert_eq!(ast.expressions[3], ex(None, shift_left(1, 2)));
    }

    #[test]
    fn test_shift_right() {
        let ast = in_template("16 >> 2;");
        assert_eq!(ast.expressions[3], ex(None, shift_right(1, 2)));
    }

    #[test]
    fn test_less_than() {
        let ast = in_template("1 < 2;");
        assert_eq!(ast.expressions[3], ex(None, lt(1, 2)));
    }

    #[test]
    fn test_less_than_or_equal_to() {
        let ast = in_template("1 <= 2;");
        assert_eq!(ast.expressions[3], ex(None, le(1, 2)));
    }

    #[test]
    fn test_greater_than() {
        let ast = in_template("2 > 1;");
        assert_eq!(ast.expressions[3], ex(None, gt(1, 2)));
    }

    #[test]
    fn test_greater_than_or_equal_to() {
        let ast = in_template("2 >= 1;");
        assert_eq!(ast.expressions[3], ex(None, ge(1, 2)));
    }

    #[test]
    fn test_equal_to() {
        let ast = in_template("1 == 1;");
        assert_eq!(ast.expressions[3], ex(None, compare_eq(1, 2)));
    }

    #[test]
    fn test_not_equal_to() {
        let ast = in_template("1 != 2;");
        assert_eq!(ast.expressions[3], ex(None, compare_ne(1, 2)));
    }

    #[test]
    fn test_logical_and() {
        let ast = in_template("x && y;");
        assert_eq!(ast.expressions[3], ex(None, logical_and(1, 2)));
    }

    #[test]
    fn test_logical_or() {
        let ast = in_template("x || y;");
        assert_eq!(ast.expressions[3], ex(None, logical_or(1, 2)));
    }

    #[test]
    fn test_logical_and_left_associative() {
        // x && y && z → (x && y) && z
        let ast = in_template("x && y && z;");
        assert_eq!(ast.expressions[3], ex(None, logical_and(1, 2)));
        assert_eq!(ast.expressions[5], ex(None, logical_and(3, 4)));
    }

    #[test]
    fn test_logical_or_left_associative() {
        // x || y || z → (x || y) || z
        let ast = in_template("x || y || z;");
        assert_eq!(ast.expressions[3], ex(None, logical_or(1, 2)));
        assert_eq!(ast.expressions[5], ex(None, logical_or(3, 4)));
    }

    #[test]
    fn test_logical_and_higher_precedence_than_or() {
        // x || y && z → x || (y && z)
        let ast = in_template("x || y && z;");
        assert_eq!(ast.expressions[4], ex(None, logical_and(2, 3)));
        assert_eq!(ast.expressions[5], ex(None, logical_or(1, 4)));
    }

    #[test]
    fn test_bitwise_and() {
        let ast = in_template("x & y;");
        assert_eq!(ast.expressions[3], ex(None, bitwise_and(1, 2)));
    }

    #[test]
    fn test_bitwise_or() {
        let ast = in_template("x | y;");
        assert_eq!(ast.expressions[3], ex(None, bitwise_or(1, 2)));
    }

    #[test]
    fn test_bitwise_xor() {
        let ast = in_template("x ^ y;");
        assert_eq!(ast.expressions[3], ex(None, bitwise_xor(1, 2)));
    }

    #[test]
    fn test_bitwise_and_left_associative() {
        // x & y & z → (x & y) & z
        let ast = in_template("x & y & z;");
        assert_eq!(ast.expressions[3], ex(None, bitwise_and(1, 2)));
        assert_eq!(ast.expressions[5], ex(None, bitwise_and(3, 4)));
    }

    #[test]
    fn test_bitwise_or_left_associative() {
        // x | y | z → (x | y) | z
        let ast = in_template("x | y | z;");
        assert_eq!(ast.expressions[3], ex(None, bitwise_or(1, 2)));
        assert_eq!(ast.expressions[5], ex(None, bitwise_or(3, 4)));
    }

    #[test]
    fn test_bitwise_xor_left_associative() {
        // x ^ y ^ z → (x ^ y) ^ z
        let ast = in_template("x ^ y ^ z;");
        assert_eq!(ast.expressions[3], ex(None, bitwise_xor(1, 2)));
        assert_eq!(ast.expressions[5], ex(None, bitwise_xor(3, 4)));
    }

    #[test]
    fn test_bitwise_and_higher_precedence_than_xor() {
        // x ^ y & z → x ^ (y & z)
        let ast = in_template("x ^ y & z;");
        assert_eq!(ast.expressions[4], ex(None, bitwise_and(2, 3)));
        assert_eq!(ast.expressions[5], ex(None, bitwise_xor(1, 4)));
    }

    #[test]
    fn test_bitwise_xor_higher_precedence_than_or() {
        // x | y ^ z → x | (y ^ z)
        let ast = in_template("x | y ^ z;");
        assert_eq!(ast.expressions[4], ex(None, bitwise_xor(2, 3)));
        assert_eq!(ast.expressions[5], ex(None, bitwise_or(1, 4)));
    }

    #[test]
    fn test_bitwise_and_higher_precedence_than_or() {
        // x | y & z → x | (y & z)
        let ast = in_template("x | y & z;");
        assert_eq!(ast.expressions[4], ex(None, bitwise_and(2, 3)));
        assert_eq!(ast.expressions[5], ex(None, bitwise_or(1, 4)));
    }

    #[test]
    fn test_variable_assignment_simple() {
        let ast = in_template("x = y;");
        assert_eq!(
            ast.expressions[3],
            ex(
                None,
                assign(ast::assign_expression::Type::AssignmentTypeSimple, 1, 2)
            )
        );
    }

    #[test]
    fn test_variable_assignment_compound() {
        use ast::assign_expression::Type::*;
        for (op, ty) in [
            ("+=", AssignmentTypeCompoundAdd),
            ("-=", AssignmentTypeCompoundSubtract),
            ("*=", AssignmentTypeCompoundMultiply),
            ("**=", AssignmentTypeCompoundPower),
            ("/=", AssignmentTypeCompoundDivide),
            ("\\=", AssignmentTypeCompoundDivideInteger),
            ("%=", AssignmentTypeCompoundModulus),
            ("&&=", AssignmentTypeCompoundLogicalAnd),
            ("||=", AssignmentTypeCompoundLogicalOr),
            ("&=", AssignmentTypeCompoundBitwiseAnd),
            ("|=", AssignmentTypeCompoundBitwiseOr),
            ("^=", AssignmentTypeCompoundBitwiseXor),
            ("<<=", AssignmentTypeCompoundShiftLeft),
            (">>=", AssignmentTypeCompoundShiftRight),
        ] {
            let src = format!("x {} y;", op);
            let ast = in_template(&src);
            assert_eq!(
                ast.expressions[3],
                ex(None, assign(ty, 1, 2)),
                "failed for operator {op}"
            );
        }
    }

    #[test]
    fn test_unconstrained_assign_right_to_left() {
        // a <-- b
        let ast = in_template("a <-- b;");
        assert_eq!(ast.expressions[3], ex(None, unconstrained_rtl(1, 2)));
    }

    #[test]
    fn test_unconstrained_assign_left_to_right() {
        // a --> b
        let ast = in_template("a --> b;");
        assert_eq!(ast.expressions[3], ex(None, unconstrained_ltr(1, 2)));
    }

    #[test]
    fn test_constrained_assign_left_to_right() {
        // a ==> b
        let ast = in_template("a ==> b;");
        assert_eq!(ast.expressions[3], ex(None, con_assign_ltr(1, 2)));
    }

    #[test]
    fn test_empty_statement() {
        let ast = in_template(";");
        // statements: [0]=sentinel, [1]=Empty, [2]=Block([1])
        assert_eq!(ast.statements[1], st(None, empty_st()));
    }

    #[test]
    fn test_if_statement_no_else() {
        let ast = in_template("if (x) { log x; }");
        // [1]=x(cond), [2]=x(body)
        // statements: [1]=Log{2}, [2]=Block([1]), [3]=If{cond:1, then:2, else:0}, [4]=Block([3])
        assert_eq!(ast.statements[3], st(None, if_st(1, 2, 0)));
    }

    #[test]
    fn test_if_statement_with_else() {
        let ast = in_template("if (x) { log x; } else { log y; }");
        // [1]=x, [2]=x(then), [3]=y(else)
        // stmts: [1]=Log{2}, [2]=Block([1]), [3]=Log{3}, [4]=Block([3]), [5]=If{1,2,4}, [6]=Block([5])
        assert_eq!(ast.statements[5], st(None, if_st(1, 2, 4)));
    }

    #[test]
    fn test_while_loop() {
        let ast = in_template("while (x) { log x; }");
        // [1]=x(cond), [2]=x(body)
        // stmts: [1]=Log{2}, [2]=Block([1]), [3]=While{1,2}, [4]=Block([3])
        assert_eq!(ast.statements[3], st(None, while_st(1, 2)));
    }

    #[test]
    fn test_do_while_loop() {
        let ast = in_template("do { log x; } while (x);");
        // exprs: [1]=x(body), [2]=x(cond)
        // stmts: [1]=Log{1}, [2]=Block([1]), [3]=DoWhile{body:2, cond:2}, [4]=Block([3])
        assert_eq!(ast.statements[3], st(None, do_while_st(2, 2)));
    }

    #[test]
    fn test_for_loop_empty() {
        let ast = in_template("for (;;) { break; }");
        // stmts: [1]=Empty(;), [2]=Break, [3]=Block([2]), [4]=For{init:1,cond:0,step:0,body:3}, [5]=Block([4])
        assert_eq!(ast.statements[4], st(None, for_st(1, 0, 0, 3)));
    }

    #[test]
    fn test_for_loop_expression_initializer() {
        let ast = in_template("for (i = 0; i < n; i = i + 1) { log i; }");
        // exprs: [1]=i, [2]=0, [3]=Assign(=,1,2) [init], [4]=i(cond-lhs), [5]=n, [6]=Lt(4,5)
        //        [7]=i(step-lhs), [8]=i(step-rhs1), [9]=1, [10]=Add(8,9), [11]=Assign(=,7,10) [step]
        //        [12]=i(body)
        // stmts: [1]=Expr{3}(init stmt), [2]=Log{12}, [3]=Block([2]), [4]=For{1,6,11,3}, [5]=Block([4])
        assert_eq!(ast.statements[4], st(None, for_st(1, 6, 11, 3)));
    }

    #[test]
    fn test_break_statement() {
        let ast = in_template("break;");
        assert_eq!(ast.statements[1], st(None, break_st()));
    }

    #[test]
    fn test_continue_statement() {
        let ast = in_template("continue;");
        assert_eq!(ast.statements[1], st(None, continue_st()));
    }

    #[test]
    fn test_return_statement() {
        let ast = in_template("return x;");
        assert_eq!(ast.expressions[1], ex(None, var("x")));
        assert_eq!(ast.statements[1], st(None, return_st(1)));
    }

    #[test]
    fn test_log_statement() {
        let ast = in_template("log x;");
        assert_eq!(ast.statements[1], st(None, log_st(1)));
    }

    #[test]
    fn test_assert_statement() {
        let ast = in_template("assert x;");
        assert_eq!(ast.statements[1], st(None, assert_st(1)));
    }

    #[test]
    fn test_declaration_var_no_init() {
        let ast = in_template("var v;");
        assert_eq!(ast.statements[1], st(None, var_decl_st("v")));
    }

    #[test]
    fn test_declaration_signal_output() {
        let ast = in_template("signal output y;");
        assert_eq!(ast.statements[1], st(None, sig_out_decl("y")));
    }

    #[test]
    fn test_declaration_component() {
        let ast = in_template("component comp;");
        assert_eq!(ast.statements[1], st(None, comp_decl_st("comp")));
    }

    #[test]
    fn test_declaration_with_dimensions() {
        let ast = in_template("var a[N][M];");
        // exprs: [1]=Variable("N"), [2]=Variable("M")
        // stmts: [1]=Declaration with dimensions [1,2]
        assert_eq!(
            ast.statements[1],
            st(
                None,
                ast::statement_node::Statement::Declaration(ast::DeclarationStatement {
                    r#type: ast::declaration_statement::Type::DeclarationTypeVariable as i32,
                    declarations: vec![ast::declaration_statement::Declaration {
                        modifier: ast::declaration_statement::Modifier::None as i32,
                        name: "a".into(),
                        dimensions: vec![1, 2],
                        initializer: 0,
                    }],
                })
            )
        );
    }

    #[test]
    fn test_declaration_multiple_names() {
        // var a, b, c; — three names in one declaration, none with initializers
        let ast = in_template("var a, b, c;");
        assert_eq!(
            ast.statements[1],
            st(
                None,
                ast::statement_node::Statement::Declaration(ast::DeclarationStatement {
                    r#type: ast::declaration_statement::Type::DeclarationTypeVariable as i32,
                    declarations: vec![
                        ast::declaration_statement::Declaration {
                            modifier: ast::declaration_statement::Modifier::None as i32,
                            name: "a".into(),
                            dimensions: vec![],
                            initializer: 0,
                        },
                        ast::declaration_statement::Declaration {
                            modifier: ast::declaration_statement::Modifier::None as i32,
                            name: "b".into(),
                            dimensions: vec![],
                            initializer: 0,
                        },
                        ast::declaration_statement::Declaration {
                            modifier: ast::declaration_statement::Modifier::None as i32,
                            name: "c".into(),
                            dimensions: vec![],
                            initializer: 0,
                        },
                    ],
                })
            )
        );
    }

    #[test]
    fn test_statements_fixture() {
        static SRC: &str = include_str!("../test/statements.starkom");
        parse("statements.starkom", SRC, Settings::default()).unwrap();
    }

    #[test]
    fn test_expressions_fixture() {
        static SRC: &str = include_str!("../test/expressions.starkom");
        parse("statements.starkom", SRC, Settings::default()).unwrap();
    }

    #[test]
    fn test_vitalik() {
        static VITALIK: &'static str = include_str!("../test/vitalik.starkom");
        assert_ast_eq(
            &parse("vitalik.starkom", VITALIK, Settings::default()).unwrap(),
            &ast::File {
                path: "vitalik.starkom".to_string(),
                line_starts: vec![],
                tokens: vec![],
                version: Some(ast::Version {
                    range: None,
                    major: 1,
                    minor: 0,
                    patch: 0,
                }),
                includes: vec![],
                definitions: vec![ast::Definition {
                    definition: Some(ast::definition::Definition::TemplateDefinition(
                        ast::TemplateDefinition {
                            range: None,
                            name: "Vitalik".into(),
                            params: vec![],
                            body_index: 7,
                        },
                    )),
                }],
                main_component: Some(ast::MainComponent {
                    range: None,
                    public_signals: vec![],
                    instantiation: 19,
                }),
                expressions: vec![
                    ast::ExpressionNode::default(), // [0] sentinel
                    ex(None, var("square")),        // [1] square
                    ex(None, var("x")),             // [2] x  (lhs of x*x)
                    ex(None, var("x")),             // [3] x  (rhs of x*x)
                    ex(None, mul(2, 3)),            // [4] x * x
                    ex(None, con_assign(1, 4)),     // [5] square <== x * x
                    ex(None, var("cube")),          // [6] cube
                    ex(None, var("square")),        // [7] square (lhs of square*x)
                    ex(None, var("x")),             // [8] x  (rhs of square*x)
                    ex(None, mul(7, 8)),            // [9] square * x
                    ex(None, con_assign(6, 9)),     // [10] cube <== square * x
                    ex(None, var("cube")),          // [11] cube
                    ex(None, var("x")),             // [12] x  (in cube+x)
                    ex(None, add(11, 12)),          // [13] cube + x
                    ex(None, num("5")),             // [14] 5
                    ex(None, add(13, 14)),          // [15] cube + x + 5
                    ex(None, num("35")),            // [16] 35
                    ex(None, con_eq(15, 16)),       // [17] cube + x + 5 === 35
                    ex(None, var("Vitalik")),       // [18] Vitalik
                    ex(
                        None,
                        ast::expression_node::Expression::PostfixChain(
                            // [19] Vitalik()
                            ast::PostfixChainExpression {
                                operand: 18,
                                postfix: vec![ast::PostfixExpression {
                                    postfix: Some(ast::postfix_expression::Postfix::Invocation(
                                        ast::postfix_expression::Invocation { arguments: vec![] },
                                    )),
                                }],
                            },
                        ),
                    ),
                ],
                statements: vec![
                    ast::StatementNode::default(), // [0] sentinel
                    st(
                        None,
                        sig_decl(ast::declaration_statement::Modifier::SignalTypeInput, "x"),
                    ), // [1] signal input x;
                    st(
                        None,
                        sig_decl(ast::declaration_statement::Modifier::None, "square"),
                    ), // [2] signal square;
                    st(
                        None,
                        sig_decl(ast::declaration_statement::Modifier::None, "cube"),
                    ), // [3] signal cube;
                    st(None, expr_stmt(5)),        // [4] square <== x * x;
                    st(None, expr_stmt(10)),       // [5] cube <== square * x;
                    st(None, expr_stmt(17)),       // [6] cube + x + 5 === 35;
                    st(
                        None,
                        ast::statement_node::Statement::Block(
                            // [7] { ... }
                            ast::Block {
                                statements: vec![1, 2, 3, 4, 5, 6],
                            },
                        ),
                    ),
                ],
            },
        );
    }

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

    #[test]
    fn test_vitalik_with_tokens() {
        static VITALIK: &'static str = include_str!("../test/vitalik.starkom");
        assert_ast_eq(
            &parse(
                "vitalik.starkom",
                VITALIK,
                Settings {
                    with_tokens: true,
                    with_ranges: false,
                },
            )
            .unwrap(),
            &ast::File {
                path: "vitalik.starkom".to_string(),
                line_starts: vec![],
                tokens: vec![
                    token_with_label(
                        0,
                        TokenType::TokenTypeSingleLineComment,
                        " This is the circuit from Vitalik's PLONK tutorial. See",
                    ),
                    token_with_label(
                        58,
                        TokenType::TokenTypeSingleLineComment,
                        " https://vitalik.eth.limo/general/2019/09/22/plonk.html#how-plonk-works",
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
                    token(327, TokenType::TokenTypeEndOfFile),
                ],
                version: Some(ast::Version {
                    range: None,
                    major: 1,
                    minor: 0,
                    patch: 0,
                }),
                includes: vec![],
                definitions: vec![ast::Definition {
                    definition: Some(ast::definition::Definition::TemplateDefinition(
                        ast::TemplateDefinition {
                            range: None,
                            name: "Vitalik".into(),
                            params: vec![],
                            body_index: 7,
                        },
                    )),
                }],
                main_component: Some(ast::MainComponent {
                    range: None,
                    public_signals: vec![],
                    instantiation: 19,
                }),
                expressions: vec![
                    ast::ExpressionNode::default(), // [0] sentinel
                    ex(None, var("square")),        // [1] square
                    ex(None, var("x")),             // [2] x  (lhs of x*x)
                    ex(None, var("x")),             // [3] x  (rhs of x*x)
                    ex(None, mul(2, 3)),            // [4] x * x
                    ex(None, con_assign(1, 4)),     // [5] square <== x * x
                    ex(None, var("cube")),          // [6] cube
                    ex(None, var("square")),        // [7] square (lhs of square*x)
                    ex(None, var("x")),             // [8] x  (rhs of square*x)
                    ex(None, mul(7, 8)),            // [9] square * x
                    ex(None, con_assign(6, 9)),     // [10] cube <== square * x
                    ex(None, var("cube")),          // [11] cube
                    ex(None, var("x")),             // [12] x  (in cube+x)
                    ex(None, add(11, 12)),          // [13] cube + x
                    ex(None, num("5")),             // [14] 5
                    ex(None, add(13, 14)),          // [15] cube + x + 5
                    ex(None, num("35")),            // [16] 35
                    ex(None, con_eq(15, 16)),       // [17] cube + x + 5 === 35
                    ex(None, var("Vitalik")),       // [18] Vitalik
                    ex(
                        None,
                        ast::expression_node::Expression::PostfixChain(
                            // [19] Vitalik()
                            ast::PostfixChainExpression {
                                operand: 18,
                                postfix: vec![ast::PostfixExpression {
                                    postfix: Some(ast::postfix_expression::Postfix::Invocation(
                                        ast::postfix_expression::Invocation { arguments: vec![] },
                                    )),
                                }],
                            },
                        ),
                    ),
                ],
                statements: vec![
                    ast::StatementNode::default(), // [0] sentinel
                    st(
                        None,
                        sig_decl(ast::declaration_statement::Modifier::SignalTypeInput, "x"),
                    ), // [1] signal input x;
                    st(
                        None,
                        sig_decl(ast::declaration_statement::Modifier::None, "square"),
                    ), // [2] signal square;
                    st(
                        None,
                        sig_decl(ast::declaration_statement::Modifier::None, "cube"),
                    ), // [3] signal cube;
                    st(None, expr_stmt(5)),        // [4] square <== x * x;
                    st(None, expr_stmt(10)),       // [5] cube <== square * x;
                    st(None, expr_stmt(17)),       // [6] cube + x + 5 === 35;
                    st(
                        None,
                        ast::statement_node::Statement::Block(
                            // [7] { ... }
                            ast::Block {
                                statements: vec![1, 2, 3, 4, 5, 6],
                            },
                        ),
                    ),
                ],
            },
        );
    }

    #[test]
    fn test_vitalik_with_ranges() {
        static VITALIK: &'static str = include_str!("../test/vitalik.starkom");
        assert_ast_eq(
            &parse(
                "vitalik.starkom",
                VITALIK,
                Settings {
                    with_tokens: false,
                    with_ranges: true,
                },
            )
            .unwrap(),
            &ast::File {
                path: "vitalik.starkom".to_string(),
                line_starts: vec![
                    0, 58, 132, 133, 155, 156, 177, 195, 196, 213, 228, 229, 249, 272, 273, 296,
                    298, 299, 327,
                ],
                tokens: vec![],
                version: Some(ast::Version {
                    range: r(133, 21),
                    major: 1,
                    minor: 0,
                    patch: 0,
                }),
                includes: vec![],
                definitions: vec![ast::Definition {
                    definition: Some(ast::definition::Definition::TemplateDefinition(
                        ast::TemplateDefinition {
                            range: r(156, 141),
                            name: "Vitalik".into(),
                            params: vec![],
                            body_index: 7,
                        },
                    )),
                }],
                main_component: Some(ast::MainComponent {
                    range: r(299, 27),
                    public_signals: vec![],
                    instantiation: 19,
                }),
                expressions: vec![
                    ast::ExpressionNode::default(),   // [0] sentinel
                    ex(r(231, 6), var("square")),     // [1] square
                    ex(r(242, 1), var("x")),          // [2] x  (lhs of x*x)
                    ex(r(246, 1), var("x")),          // [3] x  (rhs of x*x)
                    ex(r(242, 5), mul(2, 3)),         // [4] x * x
                    ex(r(231, 16), con_assign(1, 4)), // [5] square <== x * x
                    ex(r(251, 4), var("cube")),       // [6] cube
                    ex(r(260, 6), var("square")),     // [7] square (lhs of square*x)
                    ex(r(269, 1), var("x")),          // [8] x  (rhs of square*x)
                    ex(r(260, 10), mul(7, 8)),        // [9] square * x
                    ex(r(251, 19), con_assign(6, 9)), // [10] cube <== square * x
                    ex(r(275, 4), var("cube")),       // [11] cube
                    ex(r(282, 1), var("x")),          // [12] x  (in cube+x)
                    ex(r(275, 8), add(11, 12)),       // [13] cube + x
                    ex(r(286, 1), num("5")),          // [14] 5
                    ex(r(275, 12), add(13, 14)),      // [15] cube + x + 5
                    ex(r(292, 2), num("35")),         // [16] 35
                    ex(r(275, 19), con_eq(15, 16)),   // [17] cube + x + 5 === 35
                    ex(r(316, 7), var("Vitalik")),    // [18] Vitalik
                    ex(
                        r(316, 9),
                        ast::expression_node::Expression::PostfixChain(
                            // [19] Vitalik()
                            ast::PostfixChainExpression {
                                operand: 18,
                                postfix: vec![ast::PostfixExpression {
                                    postfix: Some(ast::postfix_expression::Postfix::Invocation(
                                        ast::postfix_expression::Invocation { arguments: vec![] },
                                    )),
                                }],
                            },
                        ),
                    ),
                ],
                statements: vec![
                    ast::StatementNode::default(), // [0] sentinel
                    st(
                        r(179, 15),
                        sig_decl(ast::declaration_statement::Modifier::SignalTypeInput, "x"),
                    ), // [1] signal input x;
                    st(
                        r(198, 14),
                        sig_decl(ast::declaration_statement::Modifier::None, "square"),
                    ), // [2] signal square;
                    st(
                        r(215, 12),
                        sig_decl(ast::declaration_statement::Modifier::None, "cube"),
                    ), // [3] signal cube;
                    st(r(231, 17), expr_stmt(5)),  // [4] square <== x * x;
                    st(r(251, 20), expr_stmt(10)), // [5] cube <== square * x;
                    st(r(275, 20), expr_stmt(17)), // [6] cube + x + 5 === 35;
                    st(
                        r(175, 122),
                        ast::statement_node::Statement::Block(
                            // [7] { ... }
                            ast::Block {
                                statements: vec![1, 2, 3, 4, 5, 6],
                            },
                        ),
                    ),
                ],
            },
        );
    }

    #[test]
    fn test_if_without_else_range_single_line_comment() {
        let file = in_template_ranges("if (x) { ; } // comment\n;");
        let if_statement = file
            .statements
            .iter()
            .find(|statement| {
                matches!(
                    statement.statement,
                    Some(ast::statement_node::Statement::IfStatement(_))
                )
            })
            .expect("if statement not found in pool");
        assert_eq!(
            if_statement.range,
            Some(ast::Range {
                offset: 37,
                length: 12
            })
        );
    }

    #[test]
    fn test_if_without_else_range_multi_line_comment() {
        let file = in_template_ranges("if (x) { ; } /* comment */\n;");
        let if_statement = file
            .statements
            .iter()
            .find(|statement| {
                matches!(
                    statement.statement,
                    Some(ast::statement_node::Statement::IfStatement(_))
                )
            })
            .expect("if statement not found in pool");
        assert_eq!(
            if_statement.range,
            Some(ast::Range {
                offset: 37,
                length: 12
            })
        );
    }

    #[test]
    fn test_if_without_else_range_mixed_trailing_content() {
        let file = in_template_ranges("if (x) { ; }\n// comment\n\n/* block */\n;");
        let if_statement = file
            .statements
            .iter()
            .find(|statement| {
                matches!(
                    statement.statement,
                    Some(ast::statement_node::Statement::IfStatement(_))
                )
            })
            .expect("if statement not found in pool");
        assert_eq!(
            if_statement.range,
            Some(ast::Range {
                offset: 37,
                length: 12
            })
        );
    }
}
