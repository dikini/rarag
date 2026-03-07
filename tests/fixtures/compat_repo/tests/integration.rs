use compat_repo::doc_example_sum;

#[test]
fn integration_sum_smoke() {
    assert_eq!(doc_example_sum(3, 4), 7);
}
