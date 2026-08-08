# noson

A Rust library that generates example JSON data from a JSON Schema.

Given a JSON Schema as a `serde_json::Value`, `noson` produces a random JSON
value that validates against it.

## Installation

As a library:

```sh
cargo add noson
```

As a CLI:

```sh
cargo install noson
```

## Usage

The caller provides the random number generator, so output is reproducible when
a seeded RNG is used.

```rust
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": {
        "name": { "type": "string", "minLength": 1, "maxLength": 20 },
        "age": { "type": "integer", "minimum": 0, "maximum": 120 },
        "active": { "type": "boolean" }
    },
    "required": ["name", "age"]
});

let mut rng = StdRng::seed_from_u64(42);
let example = noson::generate(&schema, &mut rng).unwrap();

// example is a random JSON value that validates against the schema
println!("{}", serde_json::to_string_pretty(&example).unwrap());
```

Required properties are always included; optional properties are included with
50% probability. Recursive schemas are supported up to a nesting depth of 16.

## CLI

The crate ships a `noson` binary that reads a schema from a file or stdin and
writes random values to stdout:

```sh
noson schema.json                    # one random value from a file
cat schema.json | noson              # ... from stdin
noson --seed 42 -n 100 schema.json   # 100 reproducible values, one per line
noson --pretty schema.json           # pretty-printed output
```

When `--seed` is omitted, a random seed is used and printed to stderr so the
output can be reproduced. See `noson --help` for all options.

## Supported Schema Features

- **Types**: `null`, `boolean`, `string`, `integer`, `number`, `object`, `array`
- **Type unions**: `"type": ["string", "null"]` (array form of `type`) -- one
  type is picked uniformly, and sibling constraints apply to it
- **Constraints**: `minimum`/`maximum`, `exclusiveMinimum`/`exclusiveMaximum`,
  `minLength`/`maxLength`, `minItems`/`maxItems`
- **Multiples**: `multipleOf` -- a random multiple within the bounds, erroring
  when the range contains none
- **Tuples**: `prefixItems` -- the first slots are generated positionally from
  the prefix schemas, later slots from `items`; `items: false` caps the length
  at the tuple size
- **Array uniqueness**: `uniqueItems` -- collisions are retried a bounded
  number of times per slot, erroring when `minItems` distinct items cannot be
  found
- **Array containment**: `contains`/`minContains`/`maxContains` --
  `minContains` slots (one by default) are generated from the merge of
  `contains` and the schema the slot would otherwise use, so they satisfy
  `contains` by construction, and the generated length always leaves room for
  them. A `contains` that cannot be merged with any slot's own schema is
  reported as conflicting; `minContains: 0` waives the requirement.
  `maxContains` caps how many elements may match: a slot whose candidate would
  exceed the cap is redrawn, and one that cannot avoid matching leaves the array
  short (never below `minItems`) or, when even that is impossible, reports the
  conflict. Matches the satisfaction check behind `not` cannot decide are not
  counted, so `maxContains` over such a `contains` is best-effort
- **Object sizing**: `minProperties`/`maxProperties` -- property selection is
  count-aware; when the declared properties cannot reach `minProperties`,
  extra entries are synthesized from `patternProperties`, `propertyNames`, or
  `additionalProperties`
- **Dependencies**: `dependentRequired` -- present properties pull in their
  transitive dependents
- **Pattern**: `pattern` -- a random string is generated from the regex.
  `pattern` takes precedence over `format`. Samples are redrawn until one also
  satisfies `minLength`/`maxLength`. Patterns the generator cannot handle
  (invalid regexes, classes matching nothing) silently fall back to
  unconstrained string generation, so the result may not satisfy the pattern.
- **Format**: `date-time`, `date`, `time`, `duration`, `uuid`, `email`, `uri`,
  `hostname`, `ipv4`, `ipv6`
- **Enum / Const**: `enum`, `const`
- **Composition**: `allOf`, `anyOf`, `oneOf` -- combined conjunctively with
  sibling keywords: a random `anyOf`/`oneOf` branch is merged with the rest
  of the schema, retrying other branches on conflict
- **Negation**: `not` -- the rest of the schema generates candidates, and ones
  the negated sub-schema accepts are redrawn, up to a bounded number of
  attempts; conjoined negations forbid the union of what each forbids. The
  satisfaction check behind this is partial, and a candidate it cannot decide is
  accepted: `not` against `enum`, `const`, `type`, `required`, `properties`, the
  length/count/numeric bounds and nested `allOf`/`anyOf`/`oneOf`/`$ref` is
  enforced exactly, while a `not` over `pattern`, `format`, or any other keyword
  is best-effort
- **Conditionals**: `if`/`then`/`else` -- the then branch is generated by
  forcing the `if` condition; the else branch by pinning a discriminator
  property to the complement of the `if` values when derivable
- **References**: `$ref` resolved against `$defs` / `definitions`; sibling
  keywords are merged conjunctively (2020-12 semantics)
- **Boolean schemas**: `true` (any value) and `false` (error)

## Not Supported

The following JSON Schema features are **not** currently handled. Schemas using
them will either be silently ignored (the keyword has no effect on generation)
or, in the case of external `$ref`, return an error.

- **String**: `contentEncoding`, `contentMediaType`
- **Object**: `unevaluatedProperties`; `patternProperties` and `propertyNames`
  are only consulted when synthesizing extra entries -- they are not enforced
  on declared `properties`
- **Array**: `additionalItems`, `unevaluatedItems`
- **Dependencies**: `dependentSchemas`
- **References**: external `$ref` (http/file URIs), `$dynamicRef`, `$anchor`

## Roadmap

- **Builder/configuration API** -- customizable string length ranges, array
  sizes, max recursion depth
- **`dependentSchemas`**
- **External `$ref`** resolution (http/file references)

## Development

This project was developed with the assistance of LLMs (Claude).
