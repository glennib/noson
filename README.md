# noson

A Rust library that generates example JSON data from a JSON Schema.

Given a JSON Schema as a `serde_json::Value`, `noson` produces a random JSON
value that validates against it.

## Installation

```sh
cargo add noson
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

## Supported Schema Features

- **Types**: `null`, `boolean`, `string`, `integer`, `number`, `object`, `array`
- **Constraints**: `minimum`/`maximum`, `exclusiveMinimum`/`exclusiveMaximum`,
  `minLength`/`maxLength`, `minItems`/`maxItems`
- **Enum / Const**: `enum`, `const`
- **Composition**: `allOf`, `anyOf`, `oneOf`
- **References**: `$ref` resolved against `$defs` / `definitions`
- **Boolean schemas**: `true` (any value) and `false` (error)

## Not Supported

The following JSON Schema features are **not** currently handled. Schemas using
them will either be silently ignored (the keyword has no effect on generation)
or, in the case of external `$ref`, return an error.

- **String**: `pattern`, `format`, `contentEncoding`, `contentMediaType`
- **Numeric**: `multipleOf`
- **Object**: `additionalProperties`, `patternProperties`, `propertyNames`,
  `minProperties`, `maxProperties`, `unevaluatedProperties`
- **Array**: `prefixItems`, `additionalItems`, `contains`, `minContains`,
  `maxContains`, `uniqueItems`, `unevaluatedItems`
- **Composition**: `not`, `if`/`then`/`else`
- **Dependencies**: `dependentRequired`, `dependentSchemas`
- **References**: external `$ref` (http/file URIs), `$dynamicRef`, `$anchor`
- **Type unions**: `"type": ["string", "null"]` (array form of `type`)

## Roadmap

- **Builder/configuration API** -- customizable string length ranges, array
  sizes, max recursion depth
- **`format` generators** -- `date-time`, `email`, `uri`, `uuid`, etc.
- **`pattern` support** -- regex-based string generation via `rand_regex`
- **`additionalProperties`** generation
- **`prefixItems`**, `contains`
- **`minProperties`** / `maxProperties`
- **`multipleOf`**
- **`if`/`then`/`else`**
- **`not`**
- **`dependentRequired`** / `dependentSchemas`
- **External `$ref`** resolution (http/file references)

## Development

This project was developed with the assistance of LLMs (Claude).
