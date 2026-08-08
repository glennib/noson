//! Command-line interface for `noson`: reads a JSON Schema from a file or
//! stdin and writes random values conforming to it to stdout.

use std::fs;
use std::io;
use std::io::Write;
use std::process::ExitCode;

use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::Value;

const HELP: &str = "\
noson - generate random JSON values from a JSON Schema

Usage: noson [OPTIONS] [SCHEMA]

Arguments:
  [SCHEMA]  Path to a schema file. '-' or omitted reads from stdin.

Options:
  -s, --seed <SEED>  RNG seed (u64) for reproducible output. When omitted, a
                     random seed is used and printed to stderr.
  -n, --count <N>    Number of values to generate [default: 1]. Values are
                     printed one per line.
  -p, --pretty       Pretty-print the generated values.
  -h, --help         Print this help and exit.
  -V, --version      Print version and exit.
";

const USAGE_HINT: &str = "Usage: noson [OPTIONS] [SCHEMA] (try 'noson --help')";

#[derive(Debug, PartialEq, Eq)]
struct Args {
    schema: Option<String>,
    seed: Option<u64>,
    count: u64,
    pretty: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum ParseOutcome {
    Run(Args),
    Help,
    Version,
}

/// Parses command-line arguments (excluding the program name).
///
/// Options accept their value either as the next argument (`--seed 42`,
/// `-s 42`) or attached with `=` (`--seed=42`, `-s=42`). A `--` ends option
/// parsing; everything after it is treated as the positional schema path.
fn parse_args(argv: impl IntoIterator<Item = String>) -> Result<ParseOutcome, String> {
    let mut schema: Option<String> = None;
    let mut seed: Option<u64> = None;
    let mut count: Option<u64> = None;
    let mut pretty = false;
    let mut positional_only = false;

    let mut argv = argv.into_iter();
    while let Some(arg) = argv.next() {
        if positional_only || arg == "-" || !arg.starts_with('-') {
            if schema.is_some() {
                return Err(format!("unexpected extra argument '{arg}'"));
            }
            schema = Some(arg);
            continue;
        }
        if arg == "--" {
            positional_only = true;
            continue;
        }
        let (name, inline_value) = match arg.split_once('=') {
            Some((name, value)) => (name.to_owned(), Some(value.to_owned())),
            None => (arg, None),
        };
        match name.as_str() {
            "-h" | "--help" => return Ok(ParseOutcome::Help),
            "-V" | "--version" => return Ok(ParseOutcome::Version),
            "-p" | "--pretty" => {
                if inline_value.is_some() {
                    return Err(format!("option '{name}' does not take a value"));
                }
                if pretty {
                    return Err(format!("option '{name}' given more than once"));
                }
                pretty = true;
            }
            "-s" | "--seed" => {
                let value = take_value(&name, inline_value, &mut argv)?;
                set_once(&mut seed, &name, parse_u64(&name, &value)?)?;
            }
            "-n" | "--count" => {
                let value = take_value(&name, inline_value, &mut argv)?;
                set_once(&mut count, &name, parse_u64(&name, &value)?)?;
            }
            _ => return Err(format!("unknown option '{name}'")),
        }
    }

    Ok(ParseOutcome::Run(Args {
        schema,
        seed,
        count: count.unwrap_or(1),
        pretty,
    }))
}

fn take_value(
    name: &str,
    inline: Option<String>,
    argv: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    inline
        .or_else(|| argv.next())
        .ok_or_else(|| format!("option '{name}' requires a value"))
}

fn parse_u64(name: &str, value: &str) -> Result<u64, String> {
    value.parse().map_err(|_| {
        format!("invalid value '{value}' for option '{name}': expected an unsigned integer")
    })
}

fn set_once<T>(slot: &mut Option<T>, name: &str, value: T) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("option '{name}' given more than once"));
    }
    *slot = Some(value);
    Ok(())
}

fn run(args: Args) -> Result<(), String> {
    let schema_text = match args.schema.as_deref() {
        None | Some("-") => io::read_to_string(io::stdin())
            .map_err(|error| format!("failed to read schema from stdin: {error}"))?,
        Some(path) => {
            fs::read_to_string(path).map_err(|error| format!("failed to read '{path}': {error}"))?
        }
    };
    let schema: Value = serde_json::from_str(&schema_text)
        .map_err(|error| format!("failed to parse schema as JSON: {error}"))?;

    let seed = args.seed.unwrap_or_else(|| {
        let seed = rand::rng().random();
        eprintln!("noson: seed {seed}");
        seed
    });
    let mut rng = StdRng::seed_from_u64(seed);

    let mut out = io::BufWriter::new(io::stdout().lock());
    for _ in 0..args.count {
        let value = noson::generate(&schema, &mut rng)
            .map_err(|error| format!("generation failed: {error}"))?;
        let rendered = if args.pretty {
            serde_json::to_string_pretty(&value)
        } else {
            serde_json::to_string(&value)
        }
        .map_err(|error| format!("failed to serialize value: {error}"))?;
        writeln!(out, "{rendered}").map_err(|error| format!("failed to write output: {error}"))?;
    }
    out.flush()
        .map_err(|error| format!("failed to write output: {error}"))?;
    Ok(())
}

fn main() -> ExitCode {
    match parse_args(std::env::args().skip(1)) {
        Ok(ParseOutcome::Help) => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Ok(ParseOutcome::Version) => {
            println!("noson {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(ParseOutcome::Run(args)) => match run(args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("noson: {message}");
                ExitCode::from(1)
            }
        },
        Err(message) => {
            eprintln!("noson: {message}");
            eprintln!("{USAGE_HINT}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use super::ParseOutcome;
    use super::parse_args;

    fn parse(args: &[&str]) -> Result<ParseOutcome, String> {
        parse_args(args.iter().map(|arg| (*arg).to_owned()))
    }

    fn parse_run(args: &[&str]) -> Args {
        match parse(args).unwrap() {
            ParseOutcome::Run(args) => args,
            other => panic!("expected Run, got {other:?}"),
        }
    }

    #[test]
    fn defaults() {
        let args = parse_run(&[]);
        assert_eq!(
            args,
            Args {
                schema: None,
                seed: None,
                count: 1,
                pretty: false,
            }
        );
    }

    #[test]
    fn value_flag_forms() {
        for argv in [
            &["--seed", "42"][..],
            &["--seed=42"],
            &["-s", "42"],
            &["-s=42"],
        ] {
            assert_eq!(parse_run(argv).seed, Some(42), "argv: {argv:?}");
        }
    }

    #[test]
    fn count_and_pretty() {
        let args = parse_run(&["-n", "5", "--pretty", "schema.json"]);
        assert_eq!(args.count, 5);
        assert!(args.pretty);
        assert_eq!(args.schema.as_deref(), Some("schema.json"));
    }

    #[test]
    fn dash_is_stdin_positional() {
        let args = parse_run(&["-"]);
        assert_eq!(args.schema.as_deref(), Some("-"));
    }

    #[test]
    fn double_dash_forces_positional() {
        let args = parse_run(&["--", "-s"]);
        assert_eq!(args.schema.as_deref(), Some("-s"));
        assert_eq!(args.seed, None);
    }

    #[test]
    fn help_and_version() {
        assert_eq!(parse(&["-h"]).unwrap(), ParseOutcome::Help);
        assert_eq!(parse(&["--help"]).unwrap(), ParseOutcome::Help);
        assert_eq!(parse(&["-V"]).unwrap(), ParseOutcome::Version);
        assert_eq!(parse(&["--version"]).unwrap(), ParseOutcome::Version);
        // Help wins even alongside other (even invalid) arguments.
        assert_eq!(
            parse(&["--seed=1", "-h", "--bogus"]).unwrap(),
            ParseOutcome::Help
        );
    }

    #[test]
    fn unknown_option() {
        assert!(parse(&["--bogus"]).unwrap_err().contains("unknown option"));
    }

    #[test]
    fn missing_value() {
        assert!(parse(&["--seed"]).unwrap_err().contains("requires a value"));
    }

    #[test]
    fn empty_inline_value() {
        assert!(parse(&["-s="]).unwrap_err().contains("invalid value"));
    }

    #[test]
    fn non_numeric_value() {
        assert!(
            parse(&["--count", "many"])
                .unwrap_err()
                .contains("invalid value")
        );
    }

    #[test]
    fn repeated_flags() {
        assert!(
            parse(&["--seed", "1", "-s", "2"])
                .unwrap_err()
                .contains("more than once")
        );
        assert!(
            parse(&["-p", "--pretty"])
                .unwrap_err()
                .contains("more than once")
        );
    }

    #[test]
    fn extra_positional() {
        assert!(
            parse(&["a.json", "b.json"])
                .unwrap_err()
                .contains("unexpected extra argument")
        );
    }

    #[test]
    fn flag_that_rejects_value() {
        assert!(
            parse(&["--pretty=yes"])
                .unwrap_err()
                .contains("does not take a value")
        );
    }
}
