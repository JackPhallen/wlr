use wlr::colors::{ColorArg, ColorSelection, FgColor, TargetColor};

#[test]
fn orange_truecolor_does_not_overlap_with_red_or_yellow() {
    let orange = FgColor::Rgb(255, 140, 0);

    assert!(TargetColor::Orange.matches(orange));
    assert!(!TargetColor::Red.matches(orange));
    assert!(!TargetColor::Yellow.matches(orange));
}

#[test]
fn yellow_truecolor_does_not_overlap_with_orange() {
    let yellow = FgColor::Rgb(255, 255, 0);

    assert!(TargetColor::Yellow.matches(yellow));
    assert!(!TargetColor::Orange.matches(yellow));
}

#[test]
fn violet_truecolor_does_not_overlap_with_blue() {
    let violet = FgColor::Rgb(148, 0, 211);

    assert!(TargetColor::Violet.matches(violet));
    assert!(!TargetColor::Blue.matches(violet));
}

#[test]
fn indexed_colors_map_to_single_expected_bucket_for_representative_values() {
    let orange = FgColor::Indexed(208);
    let violet = FgColor::Indexed(177);

    assert!(TargetColor::Orange.matches(orange));
    assert!(!TargetColor::Yellow.matches(orange));

    assert!(TargetColor::Violet.matches(violet));
    assert!(!TargetColor::Blue.matches(violet));
}

#[test]
fn all_selection_matches_any_non_default_foreground() {
    let selection = ColorSelection::All;

    assert!(selection.matches(FgColor::Indexed(1)));
    assert!(selection.matches(FgColor::Rgb(255, 255, 255)));
    assert!(!selection.matches(FgColor::Default));
}

#[test]
fn color_args_default_to_red_when_omitted() {
    let selection = ColorSelection::from(Vec::<ColorArg>::new());

    assert!(selection.matches(FgColor::Indexed(1)));
    assert!(!selection.matches(FgColor::Indexed(2)));
}

#[test]
fn color_args_can_match_multiple_colors() {
    let selection = ColorSelection::from(vec![ColorArg::Red, ColorArg::Violet]);

    assert!(selection.matches(FgColor::Indexed(1)));
    assert!(selection.matches(FgColor::Indexed(177)));
    assert!(!selection.matches(FgColor::Indexed(2)));
}

#[test]
fn all_overrides_specific_colors_when_mixed() {
    let selection = ColorSelection::from(vec![ColorArg::Red, ColorArg::All]);

    assert!(matches!(selection, ColorSelection::All));
}
