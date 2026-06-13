use crate::values::Value;
use anyhow::Result;
use ff::PrimeField;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Context<F: PrimeField> {
    values: BTreeMap<String, Vec<Arc<dyn Value<F>>>>,
}

impl<F: PrimeField> Context<F> {
    pub fn top(name: &str) -> Result<&Arc<dyn Value<F>>> {
        // TODO
        todo!()
    }

    pub fn push(&mut self, name: String, value: Arc<dyn Value<F>>) -> usize {
        match self.values.get_mut(&name) {
            Some(values) => {
                let index = values.len();
                values.push(value);
                index
            }
            None => {
                self.values.insert(name, vec![value]);
                0
            }
        }
    }

    pub fn pop(&mut self, name: &str) -> Arc<dyn Value<F>> {
        self.values.get_mut(name).unwrap().pop().unwrap()
    }
}

#[derive(Debug)]
pub struct Frame<'a, F: PrimeField> {
    context: &'a mut Context<F>,
    name: String,
    index: usize,
}

impl<'a, F: PrimeField> Frame<'a, F> {
    pub fn new(context: &'a mut Context<F>, name: String, value: Arc<dyn Value<F>>) -> Self {
        let index = context.push(name.clone(), value);
        Self {
            context,
            name,
            index,
        }
    }
}

impl<'a, F: PrimeField> Drop for Frame<'a, F> {
    fn drop(&mut self) {
        self.context.pop(self.name.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO
}
