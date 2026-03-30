#[test]
fn separator_unescapes_common_sequences() {
    assert_eq!(wlr::util::unescape_separator("\\n"), "\n");
    assert_eq!(wlr::util::unescape_separator("\\t"), "\t");
    assert_eq!(wlr::util::unescape_separator("a\\\\b"), "a\\b");
    assert_eq!(wlr::util::unescape_separator("x\\qy"), "x\\qy");
    assert_eq!(wlr::util::unescape_separator(""), "");
}
