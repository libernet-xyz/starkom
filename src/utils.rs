/// Returns the row & column numbers from a given byte offset within a source file.
///
/// The coordinates are reconstructed with a simple binary search thanks to the line starts
/// information.
///
/// Both coordinates are zero-based, e.g. row 0 is the first row.
pub fn get_text_coordinates_from_offset(offset: usize, line_starts: &[usize]) -> (usize, usize) {
    let mut i = 0;
    let mut j = line_starts.len();
    while j > i + 1 {
        let k = i + ((j - i) >> 1);
        if offset < line_starts[k] {
            j = k;
        } else if offset > line_starts[k] {
            i = k;
        } else {
            return (k, 0);
        }
    }
    (i, offset - line_starts[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(get_text_coordinates_from_offset(0, &[0]), (0, 0));
        assert_eq!(get_text_coordinates_from_offset(1, &[0]), (0, 1));
        assert_eq!(get_text_coordinates_from_offset(2, &[0]), (0, 2));
    }

    #[test]
    fn test_one_line() {
        assert_eq!(get_text_coordinates_from_offset(0, &[0, 1]), (0, 0));
        assert_eq!(get_text_coordinates_from_offset(1, &[0, 1]), (1, 0));
        assert_eq!(get_text_coordinates_from_offset(2, &[0, 1]), (1, 1));
        assert_eq!(get_text_coordinates_from_offset(3, &[0, 1]), (1, 2));
        assert_eq!(get_text_coordinates_from_offset(0, &[0, 2]), (0, 0));
        assert_eq!(get_text_coordinates_from_offset(1, &[0, 2]), (0, 1));
        assert_eq!(get_text_coordinates_from_offset(2, &[0, 2]), (1, 0));
        assert_eq!(get_text_coordinates_from_offset(3, &[0, 2]), (1, 1));
        assert_eq!(get_text_coordinates_from_offset(4, &[0, 2]), (1, 2));
        assert_eq!(get_text_coordinates_from_offset(0, &[0, 3]), (0, 0));
        assert_eq!(get_text_coordinates_from_offset(1, &[0, 3]), (0, 1));
        assert_eq!(get_text_coordinates_from_offset(2, &[0, 3]), (0, 2));
        assert_eq!(get_text_coordinates_from_offset(3, &[0, 3]), (1, 0));
        assert_eq!(get_text_coordinates_from_offset(4, &[0, 3]), (1, 1));
        assert_eq!(get_text_coordinates_from_offset(5, &[0, 3]), (1, 2));
    }

    #[test]
    fn test_two_lines() {
        let lines = [0, 7, 12];
        assert_eq!(get_text_coordinates_from_offset(0, &lines), (0, 0));
        assert_eq!(get_text_coordinates_from_offset(1, &lines), (0, 1));
        assert_eq!(get_text_coordinates_from_offset(2, &lines), (0, 2));
        assert_eq!(get_text_coordinates_from_offset(3, &lines), (0, 3));
        assert_eq!(get_text_coordinates_from_offset(4, &lines), (0, 4));
        assert_eq!(get_text_coordinates_from_offset(5, &lines), (0, 5));
        assert_eq!(get_text_coordinates_from_offset(6, &lines), (0, 6));
        assert_eq!(get_text_coordinates_from_offset(7, &lines), (1, 0));
        assert_eq!(get_text_coordinates_from_offset(8, &lines), (1, 1));
        assert_eq!(get_text_coordinates_from_offset(9, &lines), (1, 2));
        assert_eq!(get_text_coordinates_from_offset(10, &lines), (1, 3));
        assert_eq!(get_text_coordinates_from_offset(11, &lines), (1, 4));
        assert_eq!(get_text_coordinates_from_offset(12, &lines), (2, 0));
        assert_eq!(get_text_coordinates_from_offset(13, &lines), (2, 1));
        assert_eq!(get_text_coordinates_from_offset(14, &lines), (2, 2));
    }
}
