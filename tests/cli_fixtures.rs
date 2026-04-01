use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use wlr::ansi::ColorLineFilter;
use wlr::colors::{ColorArg, ColorSelection};
use wlr::emitter::FilterConfig;

fn fixture(path: &str) -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

fn run_binary(color: &str, input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wlr"))
        .args(["--color", color])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child.stdin.take().unwrap().write_all(input).unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "binary exited with {:?}", output.status);
    output.stdout
}

fn run_binary_args(args: &[&str], input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wlr"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    use std::io::Write;
    child.stdin.take().unwrap().write_all(input).unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "binary exited with {:?}", output.status);
    output.stdout
}

#[test]
fn red_fixture_matches_expected_output() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/red.expected");
    assert_eq!(run_binary("red", &input), expected);
}

#[test]
fn green_fixture_matches_expected_output() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/green.expected");
    assert_eq!(run_binary("green", &input), expected);
}

#[test]
fn orange_fixture_matches_expected_output() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/orange.expected");
    assert_eq!(run_binary("orange", &input), expected);
}

#[test]
fn violet_fixture_matches_expected_output() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/violet.expected");
    assert_eq!(run_binary("violet", &input), expected);
}

#[test]
fn filter_keeps_state_across_chunk_boundaries() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/red.expected");

    let mut filter = ColorLineFilter::new(
        ColorSelection::from(vec![ColorArg::Red]),
        FilterConfig {
            separator: b"\n".to_vec(),
            before: 0,
            after: 0,
        },
    );
    let mut out = Vec::new();

    for chunk in input.chunks(3) {
        filter.process_bytes(chunk, &mut out).unwrap();
    }
    filter.finish(&mut out).unwrap();

    assert_eq!(out, expected);
}

#[test]
fn default_color_is_red_when_flag_is_omitted() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/red.expected");

    assert_eq!(run_binary_args(&[], &input), expected);
}

#[test]
fn multiple_color_flags_match_multiple_buckets() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let red = fixture("tests/fixtures/red.no_separator.expected");
    let violet = fixture("tests/fixtures/violet.expected");

    let mut expected = red;
    expected.extend(violet);

    assert_eq!(
        run_binary_args(&["--color", "red", "--color", "violet", "--separator", ""], &input),
        expected
    );
}

#[test]
fn all_color_matches_every_non_default_colored_line() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/all.expected");

    assert_eq!(run_binary_args(&["--color", "all", "--separator", ""], &input), expected);
}

#[test]
fn default_separator_inserts_blank_line_between_matching_blocks() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/red.expected");

    assert_eq!(run_binary("red", &input), expected);
}

#[test]
fn empty_separator_disables_block_spacing() {
    let input = fixture("tests/fixtures/mixed_input.ansi");
    let expected = fixture("tests/fixtures/red.no_separator.expected");

    assert_eq!(
        run_binary_args(&["--color", "red", "--separator", ""], &input),
        expected
    );
}

#[test]
fn after_context_stops_before_next_matching_blob_and_starts_new_section() {
    let input = fixture("tests/fixtures/context_input.ansi");
    let expected = fixture("tests/fixtures/context_after.expected");

    assert_eq!(
        run_binary_args(&["--color", "red", "--after", "10"], &input),
        expected
    );
}

#[test]
fn overlapping_before_and_after_context_merges_sections() {
    let input = fixture("tests/fixtures/context_input.ansi");
    let expected = fixture("tests/fixtures/context_overlap.expected");

    assert_eq!(
        run_binary_args(&["--color", "red", "--before", "5", "--after", "5"], &input),
        expected
    );
}

#[test]
fn before_context_clips_cleanly_at_start_of_file() {
    let input = fixture("tests/fixtures/context_before_edge_input.ansi");
    let expected = fixture("tests/fixtures/context_before_edge.expected");

    assert_eq!(
        run_binary_args(&["--color", "red", "--before", "5", "--after", "0"], &input),
        expected
    );
}
