/// Adds two numbers together.
///
/// ```rust
/// use compat_repo::doc_example_sum;
///
/// assert_eq!(doc_example_sum(2, 3), 5);
/// ```
pub fn doc_example_sum(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::doc_example_sum;

    #[test]
    fn unit_sum_smoke() {
        assert_eq!(doc_example_sum(1, 1), 2);
    }
}
