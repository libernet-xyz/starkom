use ff::PrimeField;
use std::fmt::Debug;
use std::sync::Arc;

pub trait Value<F: PrimeField>: Debug {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scalar<F: PrimeField> {
    value: F,
}

impl<F: PrimeField> Scalar<F> {
    pub fn new(value: F) -> Self {
        Self { value }
    }

    pub fn value(&self) -> F {
        self.value
    }
}

impl<F: PrimeField> Value<F> for Scalar<F> {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Boolean {
    value: bool,
}

impl Boolean {
    pub fn new(value: bool) -> Self {
        Self { value }
    }

    pub fn value(&self) -> bool {
        self.value
    }
}

impl<F: PrimeField> Value<F> for Boolean {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringValue {
    value: String,
}

impl StringValue {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    pub fn take(self) -> String {
        self.value
    }
}

impl<F: PrimeField> Value<F> for StringValue {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Signal {
    name: String,
}

impl Signal {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn take(self) -> String {
        self.name
    }
}

impl<F: PrimeField> Value<F> for Signal {}

#[derive(Debug, Clone)]
pub struct Array<F: PrimeField> {
    elements: Vec<Arc<dyn Value<F>>>,
}

impl<F: PrimeField> Array<F> {
    pub fn new(elements: Vec<Arc<dyn Value<F>>>) -> Self {
        Self { elements }
    }

    pub fn elements(&self) -> &[Arc<dyn Value<F>>] {
        self.elements.as_slice()
    }

    pub fn take(self) -> Vec<Arc<dyn Value<F>>> {
        self.elements
    }
}

impl<F: PrimeField> Value<F> for Array<F> {}

#[derive(Debug, Clone)]
pub struct Tuple<F: PrimeField> {
    elements: Vec<Arc<dyn Value<F>>>,
}

impl<F: PrimeField> Value<F> for Tuple<F> {}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO
}
