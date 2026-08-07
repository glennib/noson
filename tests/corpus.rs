//! Corpus integration test.
//!
//! Runs `noson::generate` against every schema in `tests/corpus/`
//! (AI-drafted schemas harvested from a real deployment, see
//! `tests/corpus/RECOMMENDATIONS.md`) and validates the output with the
//! consuming service's strict validator configuration: `format` asserted,
//! unknown formats rejected.
//!
//! Files listed in [`EXPECTED_FAILURES`] are known to produce invalid output;
//! everything else must be valid for every seed. Both directions are
//! asserted, so landing a capability that turns a file green forces the
//! corresponding line to be deleted from the list in the same change — that
//! deletion is the review artifact. A failure reproduces exactly with
//! `StdRng::seed_from_u64(seed)` and the schema from the corpus file.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::Value;

/// Fresh RNG per seed, matching the corpus harvest regime — one
/// `StdRng::seed_from_u64(seed)` per sample, not N draws from one RNG.
const SEEDS: u64 = 200;

/// Failure examples kept per file for the panic report.
const MAX_EXAMPLES: usize = 3;

/// Corpus files (stems, without `.json`) that currently produce invalid
/// output. One line per file, with the schema feature(s) responsible.
///
/// When a file goes green, `expected_failures_still_fail` names it and the
/// fix is to delete its line here.
const EXPECTED_FAILURES: &[&str] = &[
    "09-geo_point",      // prefixItems
    "10-publish_window", // dependentRequired
    "23-locale",         // pattern ignores maxLength
    "35-geo_point",      // prefixItems
];

struct FailureExample {
    seed: u64,
    outcome: String,
}

struct FileReport {
    stem: String,
    invalid: usize,
    gen_errors: usize,
    examples: Vec<FailureExample>,
}

impl FileReport {
    fn is_green(&self) -> bool {
        self.invalid == 0 && self.gen_errors == 0
    }
}

fn corpus_files() -> Vec<PathBuf> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus");
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .expect("corpus directory should exist")
        .map(|entry| entry.expect("corpus directory should be readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no corpus files found in {dir}");
    files
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .expect("corpus file should have a stem")
        .to_string_lossy()
        .into_owned()
}

/// Generates [`SEEDS`] samples for the schema in the given corpus file and
/// validates each with the strict configuration.
fn evaluate(path: &Path) -> FileReport {
    let stem = stem(path);
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let file: Value =
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("failed to parse {stem}: {e}"));
    let schema = file
        .get("schema")
        .unwrap_or_else(|| panic!("{stem} has no `schema` field"));
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(false)
        .build(schema)
        .unwrap_or_else(|e| panic!("failed to build validator for {stem}: {e}"));

    let mut report = FileReport {
        stem,
        invalid: 0,
        gen_errors: 0,
        examples: Vec::new(),
    };
    for seed in 0..SEEDS {
        let mut rng = StdRng::seed_from_u64(seed);
        match noson::generate(schema, &mut rng) {
            Ok(value) => {
                if !validator.is_valid(&value) {
                    report.invalid += 1;
                    if report.examples.len() < MAX_EXAMPLES {
                        let errors: Vec<String> = validator
                            .iter_errors(&value)
                            .map(|e| e.to_string())
                            .collect();
                        report.examples.push(FailureExample {
                            seed,
                            outcome: format!("value {value} -- {}", errors.join("; ")),
                        });
                    }
                }
            }
            Err(e) => {
                report.gen_errors += 1;
                if report.examples.len() < MAX_EXAMPLES {
                    report.examples.push(FailureExample {
                        seed,
                        outcome: format!("generation error: {e}"),
                    });
                }
            }
        }
    }
    report
}

fn format_report(report: &FileReport) -> String {
    let mut out = format!(
        "{}: {}/{SEEDS} invalid, {}/{SEEDS} generation errors\n",
        report.stem, report.invalid, report.gen_errors
    );
    for example in &report.examples {
        writeln!(out, "  seed {}: {}", example.seed, example.outcome).unwrap();
    }
    out
}

/// Every corpus file not in [`EXPECTED_FAILURES`] must generate valid values
/// for all seeds.
#[test]
fn green_corpus_files_stay_green() {
    let regressions: Vec<String> = corpus_files()
        .iter()
        .filter(|path| !EXPECTED_FAILURES.contains(&stem(path).as_str()))
        .map(|path| evaluate(path))
        .filter(|report| !report.is_green())
        .map(|report| format_report(&report))
        .collect();
    assert!(
        regressions.is_empty(),
        "corpus files failed that are not in EXPECTED_FAILURES — either a regression, or a \
         new/improved capability whose files must be added to (or were forgotten from) the \
         list:\n\n{}",
        regressions.join("\n")
    );
}

/// Every file in [`EXPECTED_FAILURES`] must still fail for at least one
/// seed, so a capability landing forces its list entries to be removed in
/// the same change.
#[test]
fn expected_failures_still_fail() {
    let files = corpus_files();
    let stems: Vec<String> = files.iter().map(|path| stem(path)).collect();

    let unknown: Vec<&&str> = EXPECTED_FAILURES
        .iter()
        .filter(|entry| !stems.iter().any(|s| s == **entry))
        .collect();
    assert!(
        unknown.is_empty(),
        "EXPECTED_FAILURES entries do not match any corpus file: {unknown:?}"
    );

    let now_green: Vec<String> = files
        .iter()
        .filter(|path| EXPECTED_FAILURES.contains(&stem(path).as_str()))
        .map(|path| evaluate(path))
        .filter(FileReport::is_green)
        .map(|report| report.stem)
        .collect();
    assert!(
        now_green.is_empty(),
        "these corpus files are green now — remove them from EXPECTED_FAILURES: {now_green:?}"
    );
}
