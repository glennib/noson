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
keywords 0.2.0 still documents as unsupported (`contains`, `not`,
`dependentSchemas`, `patternProperties` enforcement). Duplicate drafts
(schemas differing only in `description` strings) are dropped at harvest
time. INDEX.md numbers come from 1000 fixed seeds per schema; the
[corpus test](../corpus.rs) runs 200.

## Status

**noson 0.2.0 resolved every failure class from the original harvests** —
all 47 pre-0.2.0 schemas are green at 1000 seeds each: `pattern`, the
`email`/`uri`/`uuid`/`hostname`/`ipv4`/`ipv6` formats, `uniqueItems`,
count-aware `minProperties`/`maxProperties` with map-entry synthesis,
array-form `type`, `multipleOf`, `prefixItems`, `dependentRequired`,
conjunctive sibling composition for `oneOf`/`anyOf`/`$ref`, and
`if`/`then`/`else`.

The current frontier is much smaller: **2 of 74 schemas fail**, both on
the same keyword.

## Priorities

### 1. `contains` (2 files, ~60–70% invalid)

The assistant uses `contains` for "at least one element must …" phrasing
— here, "at least one image must be marked as the cover":

```json
{ "type": "array", "minItems": 1,
  "items": { …photo object… },
  "contains": { "type": "object", "required": ["cover"],
                "properties": { "cover": { "const": true } } } }
```

noson generates items from `items` alone; whether any of them satisfies
the `contains` sub-schema is luck (a coin-flip optional `cover` flag that
must also land on `true`).

- Corpus: `64-gallery`, `72-gallery` (pinned in `EXPECTED_FAILURES`).
- Sketch: after generating the array, check whether any element satisfies
  `contains` (structural check, or reuse the conjunctive-merge machinery):
  if not, regenerate one random slot from the merge of `items` and
  `contains`. `minContains`/`maxContains` generalize this to a target
  count — pick the count first, generate that many slots from the merged
  schema and the rest from `items` (for `maxContains`, retrying slots
  that accidentally satisfy `contains`).

### 2. Latent tripwires — green today, by luck

These pass every seed but only because random output almost never
collides with the constraint. They become real the moment generation gets
richer, so keep them in the corpus:

- **`not`** (`73-display_name`): `"not": {"enum": ["unknown", "untitled"]}`
  on a random string — the collision probability is ~0. Honest support is
  generate-and-reject with bounded retries; worth doing cheaply while at
  it, since `not` against `enum`/`const` is the shape the assistant
  actually produces.
- **`minContains` + `if`/`then` interplay** (`68-highlight_quotes`):
  passes because default strings are short; flips if default string
  lengths ever grow. Covered by the `contains` work above.
- **`patternProperties`/`propertyNames` on declared properties**
  (`66-custom_meta`, `61-external_refs`): synthesis honors these when
  *creating* map entries, but declared `properties` are not checked
  against them. Only relevant once a schema declares properties that
  violate its own `patternProperties` — pathological, low priority.

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
deleted in the same change, and that deletion is the review artifact.
Currently listed: `64-gallery`, `72-gallery`.

Per-keyword unit tests live in `src/lib.rs` alongside the generator;
edge-case suggestions for the `contains` work: `contains` with
`maxItems` smaller than `minContains` (→ error), `contains` disjoint
from `items` (→ error or honest failure), `maxContains: 0` (≈ `not`
on items).

**Determinism discipline.** Everything runs on fixed seed ranges — a
failure reproduces exactly with `StdRng::seed_from_u64(seed)` and the
schema from the corpus file. The rarest failure rate observed across all
harvests was 4/100, so 200 seeds catch the probabilistic classes
reliably.

**Refreshing the corpus.** A throwaway harness in the consuming
application produced this directory and can regenerate it: fresh drafts
via the LLM assistant, or a re-evaluation mode that re-runs the fixed
seeds over the stored schemas against whatever noson/jsonschema the
application currently links — that is how the numbers in INDEX.md were
produced. Drafts vary run to run (the model is not seeded); evaluations
for a given schema are deterministic. Worth a re-run after `contains`
lands, to hunt the frontier beyond it.
