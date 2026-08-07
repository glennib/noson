//! Generate random strings matching a regex, for the JSON Schema `pattern`
//! keyword.
//!
//! The generator walks the regex's compiled
//! [HIR](regex_syntax::hir::Hir) and produces a random *full* match. JSON
//! Schema's `pattern` keyword uses unanchored search semantics, so a full
//! match trivially satisfies it.
//!
//! [`generate_matching`] returns `None` for patterns it cannot handle
//! (invalid syntax, classes that match nothing, non-ASCII byte classes, or
//! patterns whose minimum match exceeds an internal length budget); the
//! caller is expected to fall back to unconstrained string generation.
//!
//! Known caveat: zero-width assertions (anchors, word boundaries) generate
//! nothing. Anchors at the pattern's edges (`^...$`) are then satisfied by
//! construction, but mid-pattern assertions like `a$b` or `foo\bbar` may
//! produce strings that do not match.

use rand::Rng;
use rand::RngExt;
use regex_syntax::hir::Class;
use regex_syntax::hir::ClassBytes;
use regex_syntax::hir::ClassUnicode;
use regex_syntax::hir::ClassUnicodeRange;
use regex_syntax::hir::Hir;
use regex_syntax::hir::HirKind;
use regex_syntax::hir::Literal;

/// Extra repetitions above `min` allowed when sampling unbounded (`+`, `*`,
/// `{n,}`) and wide bounded (`{n,m}`) repetitions. Exact counts (`{n}`) are
/// unaffected because the clamp never drops below `min`.
const MAX_REPEAT_EXTRA: u32 = 8;

/// Output size budget in bytes. Patterns whose *minimum* match exceeds this
/// (e.g. `(a{255}){255}`) bail out rather than produce huge strings, since
/// honoring `min` is required for a valid match.
const MAX_TOTAL_BYTES: usize = 4096;

/// Generates a random string that fully matches `pattern`, or `None` if the
/// pattern is unsupported.
pub(crate) fn generate_matching(pattern: &str, rng: &mut impl Rng) -> Option<String> {
    let hir = regex_syntax::parse(pattern).ok()?;
    let mut out = String::new();
    generate_hir(&hir, rng, &mut out)?;
    Some(out)
}

fn generate_hir(hir: &Hir, rng: &mut impl Rng, out: &mut String) -> Option<()> {
    if out.len() > MAX_TOTAL_BYTES {
        return None;
    }
    match hir.kind() {
        HirKind::Empty => Some(()),
        HirKind::Literal(Literal(bytes)) => {
            // Valid UTF-8 under the parser's default utf8(true) mode.
            out.push_str(str::from_utf8(bytes).ok()?);
            Some(())
        }
        HirKind::Class(Class::Unicode(class)) => {
            out.push(sample_unicode_class(class, rng)?);
            Some(())
        }
        HirKind::Class(Class::Bytes(class)) => {
            out.push(sample_ascii_byte_class(class, rng)?);
            Some(())
        }
        // Zero-width assertions emit nothing; see the module docs for the
        // mid-pattern caveat.
        HirKind::Look(_) => Some(()),
        HirKind::Repetition(rep) => {
            // `greedy` is ignored: greediness affects matching, not the
            // matched language.
            let max = rep
                .max
                .unwrap_or(u32::MAX)
                .min(rep.min.saturating_add(MAX_REPEAT_EXTRA));
            let n = rng.random_range(rep.min..=max);
            for _ in 0..n {
                generate_hir(&rep.sub, rng, out)?;
            }
            Some(())
        }
        HirKind::Capture(capture) => generate_hir(&capture.sub, rng, out),
        HirKind::Concat(subs) => {
            for sub in subs {
                generate_hir(sub, rng, out)?;
            }
            Some(())
        }
        HirKind::Alternation(subs) => {
            let idx = rng.random_range(0..subs.len());
            generate_hir(&subs[idx], rng, out)
        }
    }
}

/// Number of Unicode scalar values in a range, correcting for ranges that
/// straddle the surrogate gap: regex-syntax merges ranges across
/// U+D800..=U+DFFF (e.g. `.` compiles to the single range
/// `'\0'..='\u{10FFFF}'`), and [`ClassUnicodeRange::len`] counts the 2048
/// surrogate codepoints, so it cannot be used for sampling.
fn range_char_count(range: &ClassUnicodeRange) -> u32 {
    let start = u32::from(range.start());
    let end = u32::from(range.end());
    let mut count = end - start + 1;
    if start < 0xD800 && end > 0xDFFF {
        count -= 0x800;
    }
    count
}

/// Number of ASCII characters in a range (ranges are ascending, so clipping
/// the end to 0x7F suffices).
fn ascii_char_count(range: &ClassUnicodeRange) -> u32 {
    let start = u32::from(range.start());
    if start > 0x7F {
        return 0;
    }
    u32::from(range.end()).min(0x7F) - start + 1
}

fn sample_unicode_class(class: &ClassUnicode, rng: &mut impl Rng) -> Option<char> {
    // Prefer the class's ASCII subset when it has one. JSON Schema validators
    // use ECMA-262 regex semantics, where `\d`/`\w`/`\s` are ASCII-only,
    // while the HIR expands them to their full Unicode sets; sampling ASCII
    // keeps the output valid under both interpretations.
    let ascii_total: u32 = class.ranges().iter().map(ascii_char_count).sum();
    if ascii_total > 0 {
        let mut idx = rng.random_range(0..ascii_total);
        for range in class.ranges() {
            let count = ascii_char_count(range);
            if idx < count {
                return char::from_u32(u32::from(range.start()) + idx);
            }
            idx -= count;
        }
        unreachable!("index within total count")
    }

    // Two-step uniform sample: total count, then index -> char. Ranges are
    // disjoint and there are at most ~0x10F800 scalar values, so u32 sums
    // cannot overflow.
    let total: u32 = class.ranges().iter().map(range_char_count).sum();
    if total == 0 {
        // An empty class (e.g. `[a&&b]`) matches nothing.
        return None;
    }
    let mut idx = rng.random_range(0..total);
    for range in class.ranges() {
        let count = range_char_count(range);
        if idx < count {
            let start = u32::from(range.start());
            let mut cp = start + idx;
            if start < 0xD800 && cp >= 0xD800 {
                cp += 0x800;
            }
            return char::from_u32(cp);
        }
        idx -= count;
    }
    unreachable!("index within total count")
}

/// Byte classes only arise from `(?-u:...)` sub-patterns. They are accepted
/// only when every range is ASCII (where bytes and chars coincide);
/// otherwise the pattern is unsupported.
fn sample_ascii_byte_class(class: &ClassBytes, rng: &mut impl Rng) -> Option<char> {
    let ranges = class.ranges();
    if ranges.is_empty() || ranges.iter().any(|r| r.end() > 0x7F) {
        return None;
    }
    let total: u32 = ranges
        .iter()
        .map(|r| u32::from(r.end()) - u32::from(r.start()) + 1)
        .sum();
    let mut idx = rng.random_range(0..total);
    for range in ranges {
        let count = u32::from(range.end()) - u32::from(range.start()) + 1;
        if idx < count {
            return Some(char::from(range.start() + idx as u8));
        }
        idx -= count;
    }
    unreachable!("index within total count")
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::MAX_REPEAT_EXTRA;
    use super::generate_matching;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn round_trip() {
        // Every generated string must fully match the regex it was
        // generated from, under the regex crate's own (Unicode) semantics.
        // Mid-pattern zero-width assertions are the documented exception
        // and are deliberately absent here.
        let patterns = [
            "^abc$",
            "foo",
            r"^\d{4}-\d{2}-\d{2}$",
            "^(red|green|blue)$",
            "^[a-f0-9]{8}$",
            "^[^0-9]{3}$",
            r"^\w+\s\d+$",
            "(?i)^abc$",
            "^a+b*c?$",
            "^[ab]{2,5}$",
            r"^\p{L}{3}$",
            r"^\p{Han}{2}$",
            r"[\x{1000}-\x{FFFFF}]",
            ".+",
            r"^[a-z]{2,5}-\d+(\.\d{2})?$",
            r"^((ab|cd)+x?){1,3}$",
            "^héllo (wörld|mønd)$",
            "",
        ];
        let mut rng = seeded_rng();
        for pattern in patterns {
            let re = regex::Regex::new(&format!("^(?:{pattern})$")).expect("valid pattern");
            for i in 0..50 {
                let s = generate_matching(pattern, &mut rng)
                    .unwrap_or_else(|| panic!("pattern {pattern:?} should generate"));
                assert!(
                    re.is_match(&s),
                    "sample {i} of pattern {pattern:?} does not match: {s:?}"
                );
            }
        }
    }

    #[test]
    fn empty_class_returns_none() {
        assert_eq!(generate_matching("[a&&b]", &mut seeded_rng()), None);
    }

    #[test]
    fn invalid_pattern_returns_none() {
        assert_eq!(generate_matching("(", &mut seeded_rng()), None);
    }

    #[test]
    fn non_utf8_pattern_returns_none() {
        assert_eq!(
            generate_matching(r"(?-u)[\x80-\xff]", &mut seeded_rng()),
            None
        );
    }

    #[test]
    fn anchors_and_word_boundaries_are_zero_width() {
        assert_eq!(
            generate_matching(r"^\bfoo\b$", &mut seeded_rng()).as_deref(),
            Some("foo")
        );
    }

    #[test]
    fn unbounded_repetition_is_capped() {
        let mut rng = seeded_rng();
        for _ in 0..200 {
            let s = generate_matching("^a+$", &mut rng).expect("should generate");
            assert!(
                (1..=1 + MAX_REPEAT_EXTRA as usize).contains(&s.len()),
                "unexpected length {}",
                s.len()
            );
        }
    }

    #[test]
    fn wide_bounded_repetition_is_clamped() {
        let mut rng = seeded_rng();
        for _ in 0..200 {
            let s = generate_matching("^a{2,1000}$", &mut rng).expect("should generate");
            assert!(
                (2..=2 + MAX_REPEAT_EXTRA as usize).contains(&s.len()),
                "unexpected length {}",
                s.len()
            );
        }
    }

    #[test]
    fn exact_repetition_is_honored() {
        let s = generate_matching("^a{40}$", &mut seeded_rng()).expect("should generate");
        assert_eq!(s.len(), 40);
    }

    #[test]
    fn length_budget_bails() {
        assert_eq!(generate_matching("(a{255}){255}", &mut seeded_rng()), None);
    }

    #[test]
    fn dot_samples_valid_chars() {
        let mut rng = seeded_rng();
        for _ in 0..1000 {
            let s = generate_matching(".", &mut rng).expect("should generate");
            assert_eq!(s.chars().count(), 1, "expected one char, got {s:?}");
        }
    }

    #[test]
    fn ascii_subset_is_preferred() {
        // `\d` expands to all Unicode digits in the HIR, but ECMA-262 regex
        // semantics (used by JSON Schema validators) make it ASCII-only.
        let mut rng = seeded_rng();
        for _ in 0..200 {
            let s = generate_matching(r"\d", &mut rng).expect("should generate");
            assert!(
                s.chars().all(|c| c.is_ascii_digit()),
                "expected ASCII digit, got {s:?}"
            );
        }
    }

    #[test]
    fn surrogate_straddling_class_samples_valid_chars() {
        // This range has no ASCII subset and is stored merged across the
        // surrogate gap U+D800..=U+DFFF, exercising the gap-skipping
        // arithmetic.
        let mut rng = seeded_rng();
        for _ in 0..1000 {
            let s = generate_matching(r"[\x{1000}-\x{FFFFF}]", &mut rng).expect("should generate");
            let c = s.chars().next().expect("one char");
            assert_eq!(s.chars().count(), 1, "expected one char, got {s:?}");
            assert!(
                ('\u{1000}'..='\u{FFFFF}').contains(&c),
                "char out of range: {c:?}"
            );
        }
    }

    #[test]
    fn non_ascii_literal() {
        assert_eq!(
            generate_matching("^héllo$", &mut seeded_rng()).as_deref(),
            Some("héllo")
        );
    }
}
