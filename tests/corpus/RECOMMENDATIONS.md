# noson capability recommendations

Hand-written companion to the generated [INDEX.md](INDEX.md).

**Where this corpus comes from:** a real deployment — a content-metadata
service whose operators define *attribute types*, each optionally
constrained by a JSON Schema. An LLM assistant (`gemini-3.5-flash`)
drafts those schemas from natural-language descriptions, and noson powers
a "generate example" button that fabricates sample values against the
drafted schema. The service validates stored values with `format`
assertion enabled, so noson's output has to satisfy that same validator:
`jsonschema::options().should_validate_formats(true).should_ignore_unknown_formats(false)`.

The corpus holds 74 schemas. Files `00-46` are the harvests that drove
the noson 0.2.0 capability work; `47-73` are distinct schemas from three
later harvests against 0.2.0, including entries written to bait the
keywords 0.2.0 documented as unsupported (`contains`, `not`,
`dependentSchemas`, `patternProperties` enforcement). Duplicate drafts
(schemas differing only in `description` strings) are dropped at harvest
time. INDEX.md numbers come from 1000 fixed seeds per schema; the
[corpus test](../corpus.rs) runs 200.

## Status

**Every schema in the corpus is green**, and `EXPECTED_FAILURES` in the
corpus test is empty.

noson 0.2.0 resolved every failure class from the original harvests:
`pattern`, the `email`/`uri`/`uuid`/`hostname`/`ipv4`/`ipv6` formats,
`uniqueItems`, count-aware `minProperties`/`maxProperties` with map-entry
synthesis, array-form `type`, `multipleOf`, `prefixItems`,
`dependentRequired`, conjunctive sibling composition for
`oneOf`/`anyOf`/`$ref`, and `if`/`then`/`else`.

The `contains` frontier the later harvests exposed — `64-gallery` and
`72-gallery`, 594/1000 and 696/1000 invalid — is closed too, along with
both latent tripwires the same harvests planted:

- **`contains`/`minContains`**: `minContains` slots (one by default) are
  generated from the merge of `contains` and the schema the slot would
  otherwise use, so they satisfy it by construction, and the generated
  length always leaves room for them. That length fix is what makes
  `72-gallery` work — it has no `minItems`, so the satisfying element is
  the only thing forcing a non-empty array.
- **`maxContains`**: a slot whose candidate would exceed the cap is
  redrawn; one that cannot avoid matching leaves the array short, and only
  an array that cannot even reach `minItems` reports a conflict.
- **`not`**: the rest of the schema generates candidates and ones the
  negated sub-schema accepts are redrawn, on a deliberately partial
  three-valued satisfaction check (`Satisfies::{Yes, No, Unknown}`).

**INDEX.md is stale for `64-gallery` and `72-gallery`** — their rows still
show the pre-fix counts, because the numbers come from the consuming
application's harness (see *Refreshing the corpus* below), not from this
repository.

## Remaining gaps

### 1. `patternProperties`/`propertyNames` on declared properties

`66-custom_meta`, `61-external_refs`: synthesis honors these when
*creating* map entries, but declared `properties` are not checked against
them. Only relevant once a schema declares properties that violate its own
`patternProperties` — pathological, low priority. Green today and not by
luck: neither schema declares `properties` at all, so both are pure map
shapes where synthesis is the only path to an entry.

### 2. Best-effort corners of what is supported

Neither shape appears in the corpus, and each is documented as
best-effort rather than silently wrong:

- `not` and `maxContains` rest on a satisfaction check that covers
  `const`, `enum`, `type`, `required`, `properties`, `items`/`prefixItems`,
  `uniqueItems`, the length/count/numeric bounds, integer `multipleOf`,
  the conditional trio, `$ref`, and nested `allOf`/`anyOf`/`oneOf`/`not`.
  A `not: {pattern: …}` or `not: {format: …}` is undecidable — `pattern`
  would need a regex engine — and undecidable candidates are accepted. If
  a harvest ever produces one, the fix is to decide it, not to loosen the
  contract: `pattern` could reuse the [xeger](../../src/xeger.rs) parser
  for a match check.
- Two `contains` sub-schemas at one schema level still merge under the
  identical-or-conflict rule, so differing ones are reported as
  conflicting rather than both satisfied. Reachable wherever merging
  brings two together: `allOf` members that each carry one, a `$ref`
  target and a sibling, or a `then` branch injecting one into a base that
  already has it.

### 3. Not yet observed in the wild

`dependentSchemas`, `unevaluatedProperties`/`unevaluatedItems`,
`contentEncoding`/`contentMediaType`, external `$ref`. Several harvests
(including deliberately bait-shaped prompts — e.g. the `discount` entry
was phrased to elicit `dependentSchemas`) never produced them; the
assistant prefers `oneOf` for conditional shapes. Suggest leaving these
until a harvest actually surfaces one.

## Testing strategy (as implemented)

The [corpus test](../corpus.rs) runs every `*.json` here for 200 fixed
seeds under the strict validator config and asserts both directions
against `EXPECTED_FAILURES`: green files must stay green, listed files
must still fail — so landing a capability forces its list entry to be
deleted in the same change, and that deletion is the review artifact. The
list is currently empty, which makes the test a pure regression guard.

Per-keyword unit tests live in `src/lib.rs` alongside the generator. Two
of them stand in for corpus files that cannot prove anything, because the
schemas pass either way: `test_array_contains_from_if_then_corpus_highlight_quotes`
covers the `68-highlight_quotes` shape (`if`/`then` injecting `contains`
and `minContains: 2`, which passed only because default strings are
short), and `test_not_enum_is_never_generated` covers `73-display_name`
(`not: {enum: [...]}` on a random string, where a collision is
vanishingly unlikely).

**Determinism discipline.** Everything runs on fixed seed ranges — a
failure reproduces exactly with `StdRng::seed_from_u64(seed)` and the
schema from the corpus file. The rarest failure rate observed across all
harvests was 4/100, so 200 seeds catch the probabilistic classes
reliably. Capability work that changes how arrays draw their items
perturbs the sample sequence of every array-shaped schema, so a green run
at 200 seeds after such a change is a fresh sample rather than a stable
one: raise `SEEDS` locally (2000 takes about five seconds) and confirm
before committing. Keeping new machinery from drawing at all when its
keyword is absent — as the `contains` work does — is what keeps the
committed 200-seed run comparable across changes.

**Refreshing the corpus.** A throwaway harness in the consuming
application produced this directory and can regenerate it: fresh drafts
via the LLM assistant, or a re-evaluation mode that re-runs the fixed
seeds over the stored schemas against whatever noson/jsonschema the
application currently links — that is how the numbers in INDEX.md were
produced. Drafts vary run to run (the model is not seeded); evaluations
for a given schema are deterministic. With the frontier closed, the useful
next step is a re-run: it refreshes the two stale rows and, more
importantly, hunts for a failure class no harvest has produced yet.
