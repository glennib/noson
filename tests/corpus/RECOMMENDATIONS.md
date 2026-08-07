# noson capability recommendations

Hand-written companion to the generated [INDEX.md](INDEX.md).

**Where this corpus comes from:** a real deployment — a content-metadata
service whose operators define *attribute types*, each optionally
constrained by a JSON Schema. An LLM assistant (`gemini-3.5-flash`)
drafts those schemas from natural-language descriptions, and noson powers
a "generate example" button that fabricates sample values against the
drafted schema. The service validates stored values with `format`
assertion enabled, so noson's output has to satisfy that same validator.

The corpus holds 47 schemas: 30 from a first harvest plus 17 distinct
schemas from a second one (drafts differing only in `description` strings
were dropped). Each schema is evaluated with 100 seeded `noson::generate`
runs (seeds `0..100`), validated with
`jsonschema::options().should_validate_formats(true).should_ignore_unknown_formats(false)`,
against noson 0.1.5 and jsonschema 0.49.6.

25 of 47 schemas produce invalid values; none produce generation errors —
every failure is *silently invalid output*, the worst mode for an
example-generation feature.

noson 0.1.5 already fixed one class: the first harvest's `video_meta`
failed 4% on grammar-invalid durations (`P8Y19DT24S`) under 0.1.4;
re-evaluated under 0.1.5 it is green.

## Why the validator config matters

The consuming service asserts `format` at validation time and rejects
unknown formats at compile time. noson's own test suite only asserts
formats in dedicated format tests. Corpus-driven testing in noson should
adopt the stricter config, otherwise one of the largest failure classes
below (non-date formats) is invisible.

## Priorities

Ordered by observed frequency × severity in the corpus.

### 1. `pattern` (10 files, 87–100% invalid)

The assistant reaches for regexes constantly — hex colors, slugs, locale
codes, prefixed ids. Random alphanumerics essentially never match.

- Corpus: `04-brand_color`, `23-locale`, `44-locale`, `12-author_slug`,
  `37-author_slug`, `03-topic_tags`, `32-topic_tags`,
  `15-related_content`, `38-related_content`, `27-canonical_url`.
- Sketch: `rand_regex` (already on the noson roadmap) built on
  `regex-syntax`. Gotchas: strip `^`/`$` anchors before feeding
  `rand_regex` (it rejects them); define precedence when `pattern` and
  `minLength`/`maxLength` coexist (suggest: generate from pattern, retry a
  few times if length constraints are violated, then error honestly).

### 2. Composition keywords must combine with sibling keywords (1 file, 93% invalid — but structural)

`40-push_message` drafts a discriminated object as *sibling* keywords:

```json
{ "type": "object", "properties": {…}, "required": ["channel", "text"],
  "oneOf": [ {"properties": {"channel": {"const": "sms"}, …}}, … ] }
```

noson's dispatch returns the `oneOf` branch result alone, ignoring the
sibling `type`/`properties`/`required` — it emits `{}` (the branch's own
properties are all optional), which violates `required` *and* matches
both branches, violating `oneOf` itself. JSON Schema keywords at one
level are conjunctive.

- Sketch: when `oneOf`/`anyOf`/`$ref` appear with sibling schema
  keywords, generate from the merge of the picked branch and the sibling
  schema (reuse/extend the `allOf` merge machinery). For `oneOf`
  specifically, the exactly-one semantics may additionally require a
  validate-and-retry pass across branches. This is an architectural fix
  worth doing early — it changes `generate_value`'s dispatch from
  "first keyword wins" to "combine".

### 3. `format` generators: `email`, `uri`, `uuid` (5 files, 100% invalid)

The assistant's instructions explicitly allow `email`, `uri`, `uuid`,
`regex`, `ipv4`, `ipv6`, `hostname` — and the service asserts them.
noson's fallback (random alphanumeric string) fails every time.

- Corpus: `26-contact_email`, `27-canonical_url`, `28-story_uuid`,
  `07-teaser_media`, `34-teaser_media`.
- Sketch: hand-rolled generators are small and dependency-free. `uuid`:
  16 random bytes, set version/variant bits, hex-format. `email`/`uri`/
  `hostname`: compose from safe alphabets (`[a-z0-9]` labels, fixed
  schemes). `ipv4`/`ipv6`: trivial. This also shrinks what the `pattern`
  work has to cover.

### 4. `uniqueItems` (5 files, 27–100% invalid)

Collisions are near-certain when the item space is small — an enum-items
array with `minItems: 1` collides constantly.

- Corpus: `03-topic_tags`, `32-topic_tags`, `13-content_warnings`,
  `15-related_content`, `38-related_content`.
- Sketch: when `uniqueItems: true`, dedupe into a set (serialized
  `Value` as key), retrying each slot up to K times; if `minItems` is
  unreachable (item space smaller than `minItems`), return
  `ConflictingConstraints`. For `enum` items a smarter path is sampling
  without replacement.

### 5. `minProperties` / `maxProperties` (+ `additionalProperties` as schema)

The incident that motivated this corpus (`00-emotions`, 49% invalid;
`30-emotions`, 76%): many optional properties, `maxProperties: 3`, and
the independent 50%-per-property coin flips blow the cap.
`14-ab_variants` (100% invalid) is the map-shaped variant:
`additionalProperties: {…}` with `minProperties: 1` — noson emits `{}`
because it only ever generates declared properties.

- Corpus: `00-emotions`, `30-emotions`, `14-ab_variants`.
- Sketch: make property selection count-aware instead of per-property coin
  flips: start from required properties (error if they alone exceed
  `maxProperties`), pick a target count in
  `[max(minProperties, required), min(maxProperties, available)]`, then
  sample that many optionals. When declared properties can't reach
  `minProperties` and `additionalProperties`/`patternProperties` is a
  schema, synthesize extra properties: generate names (random, or from the
  `patternProperties` regex / `propertyNames` schema once `pattern` support
  exists) and values from the sub-schema. That single mechanism covers
  `minProperties`, `maxProperties`, `additionalProperties`-as-schema, and
  most of `patternProperties`.

### 6. `type` as an array (2 files, 80% invalid)

`{"type": ["integer", "null"]}` is the assistant's natural encoding of
"nullable X". noson ignores non-string `type` entirely and emits random
simple values.

- Corpus: `06-review_score`, `33-review_score`.
- Sketch: pick one entry uniformly, then delegate to the existing
  single-type path. Small, isolated change in `generate_value`.

### 7. `multipleOf` (1 file, 94% invalid)

- Corpus: `05-price_ore`.
- Sketch: for integers, sample `k` in `[ceil(min/m), floor(max/m)]` and
  emit `k*m`. For floats, do the same in step space and multiply, keeping
  an eye on FP error (validate `k*m` divides cleanly; nudge if not).

### 8. `prefixItems` (2 files, 92% invalid)

Tuple-shaped arrays — `[lat, lon]` — get plain `items` treatment.

- Corpus: `09-geo_point`, `35-geo_point`.
- Sketch: generate positionally from `prefixItems[i]` for the first
  `prefixItems.len()` slots, then from `items` for the rest.
  `minItems`/`maxItems` interact: the count range must include the tuple
  length when `items: false`.

### 9. `dependentRequired` (1 file, 23% invalid)

- Corpus: `10-publish_window` (generates `end` without `start`).
- Sketch: after the optional-property selection, take the transitive
  closure over `dependentRequired` — while any present key's dependents
  are missing, add them (respecting `maxProperties`; error on conflict).

### 10. `if` / `then` / `else` — deferred, but pinned

`17-push_message` carries an sms-implies-`maxLength: 160` conditional and
scores 0/100 invalid **by luck**: noson's default strings are ≤10 chars.
Keep it in the corpus as a tripwire; a real fix (generate against
`allOf: [schema, then]` after forcing the `if` condition, or
generate-and-check with retries) can wait until the above land. Note that
the *same* attribute drafted a second time (`40-push_message`) used
sibling-`oneOf` instead and fails hard — #2 covers that encoding.

Same latent-luck caveat applies to `08-site_visibility`,
`18-external_refs` and `41-external_refs` (`patternProperties` /
`propertyNames` maps that pass because noson emits no extra properties at
all) — they become *real* once #5 starts synthesizing map entries, so
implement #5's name generation against the `patternProperties` regex from
day one.

## Suggested way to get started

1. **Land the corpus + test harness** (below) with today's failures pinned
   as expected. This makes every subsequent step observable and prevents
   regressions on the 22 green schemas.
2. Implement in priority order. Each step flips specific corpus files
   from the expected-failures list to green — that flip *is* the review
   artifact for the PR.
3. Revisit the deferred items (#10) once map-entry synthesis exists.

Item #6 (`type` arrays) is the best first code change: smallest surface,
no new dependencies, and a good warm-up for the dispatch structure in
`generate_value`. Item #2 (sibling-keyword composition) is the most
architectural and worth designing before the keyword-local items pile
more cases onto the current "first keyword wins" dispatch.

## Testing strategy

**Corpus integration test.** Add `tests/corpus.rs` reading this
directory:

- Iterate every `*.json` file, extract the `schema` field.
- For each schema, generate with `StdRng::seed_from_u64(seed)` for seeds
  `0..200` and validate with the strict config:
  `jsonschema::options().should_validate_formats(true).should_ignore_unknown_formats(false)`.
- Keep an explicit expected-failures list (e.g. a `const` in the test or
  `corpus/expected-failures.txt`) naming the files that currently fail
  (the 25 FAIL rows in INDEX.md). The test asserts both directions: files
  *not* on the list must be 100% valid, and files *on* the list must
  still fail — so a capability landing forces the corresponding list
  entry to be removed in the same change, and a regression on a green
  schema fails loudly.
- The embedded `evaluation` blocks in the corpus files are provenance from
  the harvest runs (counts, example failures, validator errors) — useful
  when debugging a red file, not something the test needs to re-assert.

**Per-keyword unit tests.** Mirror the existing `src/lib.rs` style
(`generate_and_validate_n` with a seeded RNG): each new keyword gets
focused tests including its edge cases (`maxProperties` < required count →
error; `uniqueItems` with an enum smaller than `minItems` → error;
`pattern` + `maxLength` interaction; `prefixItems` + `minItems`; `oneOf`
with sibling `required`).

**Determinism discipline.** Everything runs on fixed seed ranges — a
failure reproduces exactly with `StdRng::seed_from_u64(seed)` and the
schema from the corpus file. Resist unseeded RNGs in tests; 200 fixed
seeds catch the probabilistic failures in this corpus reliably (the rarest
observed rate was 4/100, in a class since fixed).

**Refreshing the corpus.** A throwaway harness in the consuming
application produced this directory and can regenerate it: fresh drafts
via the LLM assistant, or a re-evaluation mode that re-runs the fixed
seeds over the stored schemas against whatever noson/jsonschema the
application currently links — that is how the numbers in INDEX.md were
produced. Drafts vary run to run (the model is not seeded); evaluations
for a given schema are deterministic. Worth a re-run after several
capabilities land, to harvest fresh schemas that exercise the *new*
frontier.
