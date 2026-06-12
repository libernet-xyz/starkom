use crate::utils::get_text_coordinates_from_offset;
use wasm_bindgen::prelude::*;

/// A Starkom compilation error.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Error {
    file_path: String,
    row: usize,
    column: usize,
    message: String,
}

impl Error {
    pub fn new(file_path: String, message: String, offset: usize, line_starts: &[usize]) -> Self {
        let (row, column) = get_text_coordinates_from_offset(offset, line_starts);
        Self {
            file_path,
            row,
            column,
            message,
        }
    }

    pub fn file_path_ref(&self) -> &str {
        self.file_path.as_str()
    }

    pub fn message_ref(&self) -> &str {
        self.message.as_str()
    }
}

#[wasm_bindgen]
impl Error {
    pub fn file_path(&self) -> String {
        self.file_path.clone()
    }

    pub fn row_number(&self) -> usize {
        self.row
    }

    pub fn column_number(&self) -> usize {
        self.column
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error() {
        let error = Error::new(
            "foo/bar.baz".to_string(),
            "lorem ipsum dolor".to_string(),
            42,
            &[0, 10, 20, 30, 40, 50, 60],
        );
        assert_eq!(error.file_path_ref(), "foo/bar.baz");
        assert_eq!(error.file_path(), "foo/bar.baz");
        assert_eq!(error.row_number(), 4);
        assert_eq!(error.column_number(), 2);
        assert_eq!(error.message_ref(), "lorem ipsum dolor");
        assert_eq!(error.message(), "lorem ipsum dolor");
    }
}
