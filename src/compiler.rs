use crate::ast;
use crate::values::{self, Value};
use anyhow::{Context, Result, anyhow};
use ff::PrimeField;
use std::sync::Arc;

trait EvaluateExpression<F: PrimeField> {
    fn evaluate(&self, ast: &ast::File) -> Result<Arc<dyn Value<F>>>;
}

impl<F: PrimeField> EvaluateExpression<F> for ast::ExpressionNode {
    fn evaluate(&self, ast: &ast::File) -> Result<Arc<dyn Value<F>>> {
        match self.expression.as_ref().context("invalid expression")? {
            ast::expression_node::Expression::BooleanLiteral(node) => node.evaluate(ast),
            ast::expression_node::Expression::NumericLiteral(node) => node.evaluate(ast),
            ast::expression_node::Expression::StringLiteral(node) => node.evaluate(ast),
            ast::expression_node::Expression::ArrayLiteral(node) => node.evaluate(ast),
            _ => {
                // TODO
                todo!()
            }
        }
    }
}

impl<F: PrimeField> EvaluateExpression<F> for ast::BooleanLiteral {
    fn evaluate(&self, _ast: &ast::File) -> Result<Arc<dyn Value<F>>> {
        Ok(Arc::new(values::Boolean::new(self.value)))
    }
}

impl<F: PrimeField> EvaluateExpression<F> for ast::NumericLiteral {
    fn evaluate(&self, ast: &ast::File) -> Result<Arc<dyn Value<F>>> {
        Ok(Arc::new(values::Scalar::new(
            match ast::numeric_literal::Base::try_from(self.base)
                .context("invalid base for numeric literal")?
            {
                ast::numeric_literal::Base::NumberBase10 => {
                    // TODO
                    todo!()
                }
                ast::numeric_literal::Base::NumberBase16 => {
                    self.value.parse().context("invalid scalar value")?
                }
                ast::numeric_literal::Base::NumberBase8 => {
                    // TODO
                    todo!()
                }
            },
        )))
    }
}

impl<F: PrimeField> EvaluateExpression<F> for ast::StringLiteral {
    fn evaluate(&self, _ast: &ast::File) -> Result<Arc<dyn Value<F>>> {
        // TODO: remove quotes and process escapes.
        Ok(Arc::new(values::StringValue::new(self.value.clone())))
    }
}

impl<F: PrimeField> EvaluateExpression<F> for ast::ArrayLiteral {
    fn evaluate(&self, ast: &ast::File) -> Result<Arc<dyn Value<F>>> {
        Ok(Arc::new(values::Array::new(
            self.elements
                .iter()
                .map(|&element_index| {
                    let element_index = element_index as usize;
                    if element_index >= ast.expressions.len() {
                        return Err(anyhow!("invalid expression index"));
                    }
                    EvaluateExpression::<F>::evaluate(&ast.expressions[element_index], ast)
                })
                .collect::<Result<Vec<Arc<dyn Value<F>>>>>()?,
        )))
    }
}

// TODO

#[cfg(test)]
mod tests {
    use super::*;

    // TODO
}
