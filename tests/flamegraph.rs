#[macro_use]
extern crate pretty_assertions;

extern crate inferno;

use inferno::flamegraph::{self, BackgroundColor, Direction, Options, Palette, PaletteMap};
use log::Level;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Cursor};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

fn test_flamegraph(
    input_file: &str,
    expected_result_file: &str,
    mut options: Options,
) -> quick_xml::Result<()> {
    // Always pretty print XML to make it easier to find differences when tests fail.
    options.pretty_xml = true;
    // Never include static JavaScript in tests so we don't have to have it duplicated
    // in all of the test files.
    options.no_javascript = true;
    let r = File::open(input_file).unwrap();
    let expected_len = fs::metadata(expected_result_file).unwrap().len() as usize;
    let mut result = Cursor::new(Vec::with_capacity(expected_len));
    let return_value = flamegraph::from_reader(options, r, &mut result);
    let expected = BufReader::new(File::open(expected_result_file).unwrap());
    result.set_position(0);
    compare_results(result, expected, expected_result_file);
    return_value
}

fn test_flamegraph_multiple_files(
    input_files: Vec<String>,
    expected_result_file: &str,
    mut options: Options,
) -> quick_xml::Result<()> {
    // Always pretty print XML to make it easier to find differences when tests fail.
    options.pretty_xml = true;
    // Never include static JavaScript in tests so we don't have to have it duplicated
    // in all of the test files.
    options.no_javascript = true;
    let mut readers: Vec<File> = Vec::with_capacity(input_files.len());
    for infile in input_files.iter() {
        let r = File::open(infile).map_err(quick_xml::Error::Io)?;
        readers.push(r);
    }
    let expected_len = fs::metadata(expected_result_file).unwrap().len() as usize;
    let mut result = Cursor::new(Vec::with_capacity(expected_len));
    let return_value = flamegraph::from_readers(options, readers, &mut result);
    let expected = BufReader::new(File::open(expected_result_file).unwrap());
    result.set_position(0);
    compare_results(result, expected, expected_result_file);
    return_value
}

fn compare_results<R, E>(result: R, mut expected: E, expected_file: &str)
where
    R: BufRead,
    E: BufRead,
{
    let mut buf = String::new();
    let mut line_num = 1;
    for line in result.lines() {
        if expected.read_line(&mut buf).unwrap() == 0 {
            panic!(
                "\noutput has more lines than expected result file: {}",
                expected_file
            );
        }
        assert_eq!(
            line.unwrap(),
            buf.trim_end(),
            "\n{}:{}",
            expected_file,
            line_num
        );
        buf.clear();
        line_num += 1;
    }

    if expected.read_line(&mut buf).unwrap() > 0 {
        panic!(
            "\n{} has more lines than output, beginning at line: {}",
            expected_file, line_num
        )
    }
}

fn test_flamegraph_logs<F>(input_file: &str, asserter: F)
where
    F: Fn(&Vec<testing_logger::CapturedLog>),
{
    test_flamegraph_logs_with_options(input_file, asserter, Default::default());
}

fn test_flamegraph_logs_with_options<F>(input_file: &str, asserter: F, options: flamegraph::Options)
where
    F: Fn(&Vec<testing_logger::CapturedLog>),
{
    testing_logger::setup();
    let r = File::open(input_file).unwrap();
    let sink = io::sink();
    let _ = flamegraph::from_reader(options, r, sink);
    testing_logger::validate(asserter);
}

#[test]
fn flamegraph_colors_java() {
    let input_file = "./flamegraph/test/results/perf-java-stacks-01-collapsed-all.txt";
    let expected_result_file = "./tests/data/flamegraph/colors/java.svg";

    let options = flamegraph::Options {
        colors: Palette::from_str("java").unwrap(),
        bgcolors: Some(BackgroundColor::from_str("blue").unwrap()),
        hash: true,
        ..Default::default()
    };

    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_colors_js() {
    let input_file = "./flamegraph/test/results/perf-js-stacks-01-collapsed-all.txt";
    let expected_result_file = "./tests/data/flamegraph/colors/js.svg";

    let options = flamegraph::Options {
        colors: Palette::from_str("js").unwrap(),
        bgcolors: Some(BackgroundColor::from_str("green").unwrap()),
        hash: true,
        ..Default::default()
    };

    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_differential() {
    let input_file =
        "./tests/data/flamegraph/differential/perf-cycles-instructions-01-collapsed-all-diff.txt";
    let expected_result_file = "./tests/data/flamegraph/differential/diff.svg";
    test_flamegraph(input_file, expected_result_file, Default::default()).unwrap();
}

#[test]
fn flamegraph_differential_negated() {
    let input_file =
        "./tests/data/flamegraph/differential/perf-cycles-instructions-01-collapsed-all-diff.txt";
    let expected_result_file = "./tests/data/flamegraph/differential/diff-negated.svg";
    let options = Options {
        negate_differentials: true,
        ..Default::default()
    };
    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_factor() {
    let input_file = "./flamegraph/test/results/perf-vertx-stacks-01-collapsed-all.txt";
    let expected_result_file = "./tests/data/flamegraph/factor/factor-2.5.svg";
    let options = Options {
        factor: 2.5,
        hash: true,
        ..Default::default()
    };
    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_nameattr() {
    let input_file = "./flamegraph/test/results/perf-cycles-instructions-01-collapsed-all.txt";
    let expected_result_file = "./tests/data/flamegraph/nameattr/nameattr.svg";
    let nameattr_file = "./tests/data/flamegraph/nameattr/nameattr.txt";

    let options = flamegraph::Options {
        hash: true,
        func_frameattrs: flamegraph::FuncFrameAttrsMap::from_file(&PathBuf::from(nameattr_file))
            .unwrap(),
        ..Default::default()
    };

    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_should_warn_about_fractional_samples() {
    test_flamegraph_logs(
        "./tests/data/flamegraph/fractional-samples/fractional.txt",
        |captured_logs| {
            let nwarnings = captured_logs
                .into_iter()
                .filter(|log| {
                    log.body
                        .starts_with("The input data has fractional sample counts")
                        && log.level == Level::Warn
                })
                .count();
            assert_eq!(
                nwarnings, 1,
                "fractional samples warning logged {} times, but should be logged exactly once",
                nwarnings
            );
        },
    );
}

#[test]
fn flamegraph_should_not_warn_about_zero_fractional_samples() {
    test_flamegraph_logs(
        "./tests/data/flamegraph/fractional-samples/zero-fractionals.txt",
        |captured_logs| {
            let nwarnings = captured_logs
                .into_iter()
                .filter(|log| {
                    log.body
                        .starts_with("The input data has fractional sample counts")
                        && log.level == Level::Warn
                })
                .count();
            assert_eq!(
                nwarnings, 0,
                "warning about fractional samples not expected"
            );
        },
    );
}

#[test]
fn flamegraph_should_not_warn_about_fractional_sample_with_tricky_stack() {
    test_flamegraph_logs(
        "./tests/data/flamegraph/fractional-samples/tricky-stack.txt",
        |captured_logs| {
            let nwarnings = captured_logs
                .into_iter()
                .filter(|log| {
                    log.body
                        .starts_with("The input data has fractional sample counts")
                        && log.level == Level::Warn
                })
                .count();
            assert_eq!(
                nwarnings, 0,
                "warning about fractional samples not expected"
            );
        },
    );
}

#[test]
fn flamegraph_palette_map() {
    let input_file = "./flamegraph/test/results/perf-vertx-stacks-01-collapsed-all.txt";
    let expected_result_file = "./tests/data/flamegraph/palette-map/consistent-palette.svg";
    let palette_file = "./tests/data/flamegraph/palette-map/palette.map";
    let mut palette_map = load_palette_map_file(palette_file);

    let options = flamegraph::Options {
        palette_map: Some(&mut palette_map),
        ..Default::default()
    };

    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_should_warn_about_bad_input_lines() {
    test_flamegraph_logs(
        "./tests/data/flamegraph/bad-lines/bad-lines.txt",
        |captured_logs| {
            let nwarnings = captured_logs
                .into_iter()
                .filter(|log| {
                    log.body.starts_with("Ignored")
                        && log.body.ends_with(" lines with invalid format")
                        && log.level == Level::Warn
                })
                .count();
            assert_eq!(
                nwarnings, 1,
                "bad lines warning logged {} times, but should be logged exactly once",
                nwarnings
            );
        },
    );
}

#[test]
fn flamegraph_should_warn_about_empty_input() {
    test_flamegraph_logs("./tests/data/flamegraph/empty/empty.txt", |captured_logs| {
        let nwarnings = captured_logs
            .into_iter()
            .filter(|log| log.body == "No stack counts found" && log.level == Level::Error)
            .count();
        assert_eq!(
            nwarnings, 1,
            "no stack counts error logged {} times, but should be logged exactly once",
            nwarnings
        );
    });
}

#[test]
fn flamegraph_empty_input() {
    let input_file = "./tests/data/flamegraph/empty/empty.txt";
    let expected_result_file = "./tests/data/flamegraph/empty/empty.svg";
    assert!(test_flamegraph(input_file, expected_result_file, Default::default()).is_err());
}

#[test]
fn flamegraph_unsorted_multiple_input_files() {
    let input_files = vec![
        "./tests/data/flamegraph/multiple-inputs/perf-vertx-stacks-01-collapsed-all-unsorted-1.txt"
            .to_string(),
        "./tests/data/flamegraph/multiple-inputs/perf-vertx-stacks-01-collapsed-all-unsorted-2.txt"
            .to_string(),
    ];
    let expected_result_file =
        "./tests/data/flamegraph/multiple-inputs/perf-vertx-stacks-01-collapsed-all.svg";
    let options = Options {
        hash: true,
        ..Default::default()
    };
    test_flamegraph_multiple_files(input_files, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_should_prune_narrow_blocks() {
    let input_file = "./tests/data/flamegraph/narrow-blocks/narrow-blocks.txt";
    let expected_result_file = "./tests/data/flamegraph/narrow-blocks/narrow-blocks.svg";

    let options = flamegraph::Options {
        hash: true,
        ..Default::default()
    };

    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_inverted() {
    let input_file = "./flamegraph/test/results/perf-vertx-stacks-01-collapsed-all.txt";
    let expected_result_file = "./tests/data/flamegraph/inverted/inverted.svg";

    let options = flamegraph::Options {
        hash: true,
        title: "Icicle Graph".to_string(),
        direction: Direction::Inverted,
        ..Default::default()
    };

    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_grey_frames() {
    let input_file = "./tests/data/flamegraph/grey-frames/grey-frames.txt";
    let expected_result_file = "./tests/data/flamegraph/grey-frames/grey-frames.svg";

    let options = flamegraph::Options {
        hash: true,
        ..Default::default()
    };

    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

#[test]
fn flamegraph_example_perf_stacks() {
    let input_file = "./tests/data/collapse-perf/results/example-perf-stacks-collapsed.txt";
    let expected_result_file =
        "./tests/data/flamegraph/example-perf-stacks/example-perf-stacks.svg";
    let palette_file = "./tests/data/flamegraph/example-perf-stacks/palette.map";
    let mut palette_map = load_palette_map_file(palette_file);

    let options = flamegraph::Options {
        palette_map: Some(&mut palette_map),
        ..Default::default()
    };

    test_flamegraph(input_file, expected_result_file, options).unwrap();
}

fn load_palette_map_file(palette_file: &str) -> PaletteMap {
    let path = Path::new(palette_file);
    PaletteMap::load_from_file_or_empty(&path).unwrap()
}

#[test]
fn flamegraph_cli() {
    let input_file = "./flamegraph/test/results/perf-vertx-stacks-01-collapsed-all.txt";
    let expected_file =
        "./tests/data/flamegraph/multiple-inputs/perf-vertx-stacks-01-collapsed-all.svg";

    // Test with file passed in
    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("inferno-flamegraph")
        .arg("--")
        .arg("--pretty-xml")
        .arg("--no-javascript")
        .arg("--hash")
        .arg(input_file)
        .output()
        .expect("failed to execute process");
    let expected = BufReader::new(File::open(expected_file).unwrap());
    compare_results(Cursor::new(output.stdout), expected, expected_file);

    // Test with STDIN
    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("inferno-flamegraph")
        .arg("--")
        .arg("--pretty-xml")
        .arg("--no-javascript")
        .arg("--hash")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");
    let mut input = BufReader::new(File::open(input_file).unwrap());
    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
    io::copy(&mut input, stdin).unwrap();
    let output = child.wait_with_output().expect("Failed to read stdout");
    let expected = BufReader::new(File::open(expected_file).unwrap());
    compare_results(Cursor::new(output.stdout), expected, expected_file);

    // Test with multiple files passed in
    let input_file_part1 =
        "./tests/data/flamegraph/multiple-inputs/perf-vertx-stacks-01-collapsed-all-unsorted-1.txt";
    let input_file_part2 =
        "./tests/data/flamegraph/multiple-inputs/perf-vertx-stacks-01-collapsed-all-unsorted-2.txt";
    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("inferno-flamegraph")
        .arg("--")
        .arg("--pretty-xml")
        .arg("--no-javascript")
        .arg("--hash")
        .arg(input_file_part1)
        .arg(input_file_part2)
        .output()
        .expect("failed to execute process");
    let expected = BufReader::new(File::open(expected_file).unwrap());
    compare_results(Cursor::new(output.stdout), expected, expected_file);
}
