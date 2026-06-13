use crate::ast;
use anyhow::{Context, Result, anyhow};
use ff::PrimeField;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::sync::Arc;

/// A PLONK constraint expression.
///
/// The implementation guarantees that it's always in vanilla PLONK form:
///
///   ql * L + qr * R + qo * O + qm * L * R + qc
///
/// with L, R, and O being signals corresponding to the three gate terminations, and the five q*
/// factors being constant scalars.
pub(crate) trait Expression<F: PrimeField>: Debug {
    fn get_free_variables(&self) -> BTreeSet<String>;
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct Constant<F: PrimeField> {
    value: F,
}

impl<F: PrimeField> Expression<F> for Constant<F> {
    fn get_free_variables(&self) -> BTreeSet<String> {
        BTreeSet::default()
    }

    // TODO
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Variable {
    name: String,
}

impl<F: PrimeField> Expression<F> for Variable {
    fn get_free_variables(&self) -> BTreeSet<String> {
        BTreeSet::from([self.name.clone()])
    }

    // TODO
}

#[derive(Debug, Clone)]
pub(crate) struct Sum<F: PrimeField> {
    operands: Vec<Arc<dyn Expression<F>>>,
}

impl<F: PrimeField> Expression<F> for Sum<F> {
    fn get_free_variables(&self) -> BTreeSet<String> {
        self.operands.iter().fold(BTreeSet::default(), |set, item| {
            set.union(&item.get_free_variables())
                .map(|name| name.clone())
                .collect::<BTreeSet<String>>()
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Product<F: PrimeField> {
    operands: Vec<Arc<dyn Expression<F>>>,
}

impl<F: PrimeField> Expression<F> for Product<F> {
    fn get_free_variables(&self) -> BTreeSet<String> {
        self.operands.iter().fold(BTreeSet::default(), |set, item| {
            set.union(&item.get_free_variables())
                .map(|name| name.clone())
                .collect::<BTreeSet<String>>()
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Power<F: PrimeField> {
    operand: Arc<dyn Expression<F>>,
    exponent: F,
}

impl<F: PrimeField> Expression<F> for Power<F> {
    fn get_free_variables(&self) -> BTreeSet<String> {
        self.operand.get_free_variables()
    }
}

pub(crate) trait TryIntoExpression {
    fn try_into_expression<F: PrimeField>(
        &self,
        pool: &[ast::ExpressionNode],
    ) -> Result<Arc<dyn Expression<F>>>;
}

impl TryIntoExpression for ast::ExpressionNode {
    fn try_into_expression<F: PrimeField>(
        &self,
        pool: &[ast::ExpressionNode],
    ) -> Result<Arc<dyn Expression<F>>> {
        match self
            .expression
            .as_ref()
            .context("invalid expression node")?
        {
            ast::expression_node::Expression::BooleanLiteral(node) => {
                node.try_into_expression(pool)
            }
            ast::expression_node::Expression::NumericLiteral(node) => {
                node.try_into_expression(pool)
            }
            ast::expression_node::Expression::StringLiteral(node) => node.try_into_expression(pool),
            ast::expression_node::Expression::ArrayLiteral(node) => node.try_into_expression(pool),
            ast::expression_node::Expression::Variable(node) => node.try_into_expression(pool),
            _ => {
                // TODO
                todo!()
            }
        }
    }
}

impl TryIntoExpression for ast::BooleanLiteral {
    fn try_into_expression<F: PrimeField>(
        &self,
        _pool: &[ast::ExpressionNode],
    ) -> Result<Arc<dyn Expression<F>>> {
        Err(anyhow!("cannot use booleans in PLONK constraints"))
    }
}

impl TryIntoExpression for ast::NumericLiteral {
    fn try_into_expression<F: PrimeField>(
        &self,
        _pool: &[ast::ExpressionNode],
    ) -> Result<Arc<dyn Expression<F>>> {
        match ast::numeric_literal::Base::try_from(self.base)? {
            ast::numeric_literal::Base::NumberBase10 => {
                // TODO
                todo!()
            }
            ast::numeric_literal::Base::NumberBase16 => {
                // TODO
                todo!()
            }
            ast::numeric_literal::Base::NumberBase8 => {
                // TODO
                todo!()
            }
        }
    }
}

impl TryIntoExpression for ast::StringLiteral {
    fn try_into_expression<F: PrimeField>(
        &self,
        _pool: &[ast::ExpressionNode],
    ) -> Result<Arc<dyn Expression<F>>> {
        Err(anyhow!("cannot use strings in PLONK constraints"))
    }
}

impl TryIntoExpression for ast::ArrayLiteral {
    fn try_into_expression<F: PrimeField>(
        &self,
        _pool: &[ast::ExpressionNode],
    ) -> Result<Arc<dyn Expression<F>>> {
        Err(anyhow!("cannot use arrays in PLONK constraints"))
    }
}

impl TryIntoExpression for ast::VariableExpression {
    fn try_into_expression<F: PrimeField>(
        &self,
        _pool: &[ast::ExpressionNode],
    ) -> Result<Arc<dyn Expression<F>>> {
        Ok(Arc::new(Variable {
            name: self.name.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO
}
