// Copyright 2026 The Libernet Team
// SPDX-License-Identifier: Apache-2.0

use error::Error;
use prost::{self, Message};
use wasm_bindgen::prelude::*;

pub mod error;
pub mod lexer;
pub mod parser;
pub mod utils;

pub mod ast {
    include!(concat!(env!("OUT_DIR"), "/starkom.ast.v1.rs"));
}

impl ast::Token {
    pub fn token_type(&self) -> ast::token::Type {
        ast::token::Type::try_from(self.r#type).unwrap_or_default()
    }
}

#[wasm_bindgen]
pub struct TextCoordinates {
    pub row: usize,
    pub col: usize,
}

#[wasm_bindgen]
pub fn get_text_coordinates_from_offset(offset: usize, line_starts: &[usize]) -> TextCoordinates {
    let (row, col) = utils::get_text_coordinates_from_offset(offset, line_starts);
    TextCoordinates { row, col }
}

#[wasm_bindgen]
pub fn parse(path: &str, source: &str, with_ranges: bool) -> Result<Vec<u8>, Error> {
    let ast = parser::parse(path, source, with_ranges)?;
    Ok(ast.encode_to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_text_coordinates_helper(offset: usize, line_starts: &[usize]) -> (usize, usize) {
        let coordinates = get_text_coordinates_from_offset(offset, line_starts);
        (coordinates.row, coordinates.col)
    }

    #[test]
    fn test_text_coordinates() {
        let lines = [0, 7, 12];
        assert_eq!(get_text_coordinates_helper(0, &lines), (0, 0));
        assert_eq!(get_text_coordinates_helper(1, &lines), (0, 1));
        assert_eq!(get_text_coordinates_helper(2, &lines), (0, 2));
        assert_eq!(get_text_coordinates_helper(3, &lines), (0, 3));
        assert_eq!(get_text_coordinates_helper(4, &lines), (0, 4));
        assert_eq!(get_text_coordinates_helper(5, &lines), (0, 5));
        assert_eq!(get_text_coordinates_helper(6, &lines), (0, 6));
        assert_eq!(get_text_coordinates_helper(7, &lines), (1, 0));
        assert_eq!(get_text_coordinates_helper(8, &lines), (1, 1));
        assert_eq!(get_text_coordinates_helper(9, &lines), (1, 2));
        assert_eq!(get_text_coordinates_helper(10, &lines), (1, 3));
        assert_eq!(get_text_coordinates_helper(11, &lines), (1, 4));
        assert_eq!(get_text_coordinates_helper(12, &lines), (2, 0));
        assert_eq!(get_text_coordinates_helper(13, &lines), (2, 1));
        assert_eq!(get_text_coordinates_helper(14, &lines), (2, 2));
    }

    #[test]
    fn test_parse() {
        static VITALIK: &'static str = include_str!("../test/vitalik.starkom");
        let ast = parser::parse("vitalik.starkom", VITALIK, true).unwrap();
        let encoded = parse("vitalik.starkom", VITALIK, true).unwrap();
        assert_eq!(ast::File::decode(encoded.as_slice()).unwrap(), ast);
    }
}
