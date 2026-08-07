//! Generate random JSON values that conform to a [JSON Schema](https://json-schema.org/).
//!
//! `noson` takes a JSON Schema represented as a [`serde_json::Value`] and
//! produces a random JSON value that validates against it. The caller supplies
//! the random number generator, making output reproducible when a seeded RNG is
//! used.
//!
//! # Example
//!
//! ```
//! use rand::SeedableRng;
//! use rand::rngs::StdRng;
//! use serde_json::json;
//!
//! let schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "name": { "type": "string", "minLength": 1, "maxLength": 20 },
//!         "age":  { "type": "integer", "minimum": 0, "maximum": 120 }
//!     },
//!     "required": ["name", "age"]
//! });
//!
//! let mut rng = StdRng::seed_from_u64(42);
//! let value = noson::generate(&schema, &mut rng).unwrap();
//! assert!(value.get("name").unwrap().is_string());
//! assert!(value.get("age").unwrap().is_i64());
//! ```
//!
//! # Supported Schema Features
//!
//! - **Types**: `null`, `boolean`, `string`, `integer`, `number`, `object`,
//!   `array`
//! - **Constraints**: `minimum`/`maximum`,
//!   `exclusiveMinimum`/`exclusiveMaximum`, `minLength`/`maxLength`,
//!   `minItems`/`maxItems`
//! - **Pattern**: `pattern` — a random string is generated from the regex.
//!   `pattern` takes precedence over `format`, and `minLength`/`maxLength` are
//!   ignored when it applies. Patterns the generator cannot handle (invalid
//!   regexes, classes matching nothing) silently fall back to unconstrained
//!   string generation, so the result may not satisfy the pattern.
//! - **Format**: `date-time`, `date`, `time`, `duration`
//! - **Enum / Const**: `enum`, `const`
//! - **Composition**: `allOf`, `anyOf`, `oneOf`
//! - **References**: `$ref` resolved against `$defs` / `definitions`
//! - **Boolean schemas**: `true` (any value) and `false` (error)
//!
//! # Not Supported
//!
//! The following JSON Schema features are **not** currently handled. Schemas
//! using them will either be silently ignored (the keyword has no effect on
//! generation) or, in the case of external `$ref`, return an error.
//!
//! - **String**: `contentEncoding`, `contentMediaType`
//! - **Numeric**: `multipleOf`
//! - **Object**: `additionalProperties`, `patternProperties`, `propertyNames`,
//!   `minProperties`, `maxProperties`, `unevaluatedProperties`
//! - **Array**: `prefixItems`, `additionalItems`, `contains`, `minContains`,
//!   `maxContains`, `uniqueItems`, `unevaluatedItems`
//! - **Composition**: `not`, `if`/`then`/`else`
//! - **Dependencies**: `dependentRequired`, `dependentSchemas`
//! - **References**: external `$ref` (http/file URIs), `$dynamicRef`, `$anchor`
//! - **Type unions**: `"type": ["string", "null"]` (array form of `type`)

mod error;
mod generate;
mod xeger;

pub use error::Error;
pub use generate::generate;

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use serde_json::Value;
    use serde_json::json;

    use crate::generate;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn generate_and_validate(schema: &Value) {
        let mut rng = seeded_rng();
        let result = generate(schema, &mut rng).expect("generation should succeed");
        assert!(
            jsonschema::is_valid(schema, &result),
            "generated value does not validate against schema.\nschema: {schema}\nvalue: {result}"
        );
    }

    fn generate_and_validate_n(schema: &Value, n: usize) {
        let mut rng = seeded_rng();
        for i in 0..n {
            let result = generate(schema, &mut rng).expect("generation should succeed");
            assert!(
                jsonschema::is_valid(schema, &result),
                "sample {i} does not validate.\nschema: {schema}\nvalue: {result}"
            );
        }
    }

    // ── Scalars ──

    #[test]
    fn test_null() {
        let schema = json!({"type": "null"});
        generate_and_validate(&schema);
    }

    #[test]
    fn test_boolean() {
        let schema = json!({"type": "boolean"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_basic() {
        let schema = json!({"type": "string"});
        generate_and_validate(&schema);
    }

    #[test]
    fn test_string_min_max_length() {
        let schema = json!({"type": "string", "minLength": 5, "maxLength": 10});
        let mut rng = seeded_rng();
        for _ in 0..50 {
            let result = generate(&schema, &mut rng).unwrap();
            let s = result.as_str().unwrap();
            assert!(
                s.len() >= 5 && s.len() <= 10,
                "string length {} out of range [5, 10]: {s}",
                s.len()
            );
        }
        generate_and_validate_n(&schema, 50);
    }

    #[test]
    fn test_integer_basic() {
        let schema = json!({"type": "integer"});
        generate_and_validate(&schema);
    }

    #[test]
    fn test_integer_min_max() {
        let schema = json!({"type": "integer", "minimum": 10, "maximum": 20});
        let mut rng = seeded_rng();
        for _ in 0..50 {
            let result = generate(&schema, &mut rng).unwrap();
            let n = result.as_i64().unwrap();
            assert!((10..=20).contains(&n), "integer {n} out of range [10, 20]");
        }
        generate_and_validate_n(&schema, 50);
    }

    #[test]
    fn test_number_basic() {
        let schema = json!({"type": "number"});
        generate_and_validate(&schema);
    }

    #[test]
    fn test_number_min_max() {
        let schema = json!({"type": "number", "minimum": 1.5, "maximum": 3.5});
        let mut rng = seeded_rng();
        for _ in 0..50 {
            let result = generate(&schema, &mut rng).unwrap();
            let n = result.as_f64().unwrap();
            assert!(
                (1.5..=3.5).contains(&n),
                "number {n} out of range [1.5, 3.5]"
            );
        }
        generate_and_validate_n(&schema, 50);
    }

    // ── Enum / Const ──

    #[test]
    fn test_const() {
        let schema = json!({"const": 42});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).unwrap();
        assert_eq!(result, json!(42));
        generate_and_validate(&schema);
    }

    #[test]
    fn test_const_string() {
        let schema = json!({"const": "hello"});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).unwrap();
        assert_eq!(result, json!("hello"));
        generate_and_validate(&schema);
    }

    #[test]
    fn test_enum() {
        let schema = json!({"enum": [1, "two", true, null]});
        let variants: Vec<Value> = vec![json!(1), json!("two"), json!(true), json!(null)];
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let result = generate(&schema, &mut rng).unwrap();
            assert!(
                variants.contains(&result),
                "enum result {result} not in variants"
            );
        }
        generate_and_validate_n(&schema, 100);
    }

    // ── Objects ──

    #[test]
    fn test_object_basic() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"},
                "optional_field": {"type": "boolean"}
            },
            "required": ["name", "age"]
        });
        let mut rng = seeded_rng();
        for _ in 0..20 {
            let result = generate(&schema, &mut rng).unwrap();
            let obj = result.as_object().unwrap();
            assert!(obj.contains_key("name"), "missing required field 'name'");
            assert!(obj.contains_key("age"), "missing required field 'age'");
        }
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_object_all_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string"},
                "b": {"type": "integer"},
                "c": {"type": "boolean"}
            },
            "required": ["a", "b", "c"]
        });
        let mut rng = seeded_rng();
        for _ in 0..20 {
            let result = generate(&schema, &mut rng).unwrap();
            let obj = result.as_object().unwrap();
            assert!(obj.contains_key("a"));
            assert!(obj.contains_key("b"));
            assert!(obj.contains_key("c"));
        }
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_object_nested() {
        let schema = json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"}
                    },
                    "required": ["street", "city"]
                }
            },
            "required": ["address"]
        });
        generate_and_validate_n(&schema, 20);
    }

    // ── Arrays ──

    #[test]
    fn test_array_basic() {
        let schema = json!({"type": "array", "items": {"type": "integer"}});
        generate_and_validate(&schema);
    }

    #[test]
    fn test_array_min_max_items() {
        let schema = json!({
            "type": "array",
            "items": {"type": "integer"},
            "minItems": 2,
            "maxItems": 5
        });
        let mut rng = seeded_rng();
        for _ in 0..50 {
            let result = generate(&schema, &mut rng).unwrap();
            let arr = result.as_array().unwrap();
            assert!(
                arr.len() >= 2 && arr.len() <= 5,
                "array length {} out of range [2, 5]",
                arr.len()
            );
        }
        generate_and_validate_n(&schema, 50);
    }

    #[test]
    fn test_array_of_objects() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "integer"},
                    "name": {"type": "string"}
                },
                "required": ["id", "name"]
            },
            "minItems": 1,
            "maxItems": 3
        });
        generate_and_validate_n(&schema, 20);
    }

    // ── Composition ──

    #[test]
    fn test_any_of() {
        let schema = json!({"anyOf": [{"type": "string"}, {"type": "integer"}]});
        generate_and_validate_n(&schema, 50);
    }

    #[test]
    fn test_any_of_all_variants_generated() {
        let schema =
            json!({"anyOf": [{"type": "string"}, {"type": "integer"}, {"type": "boolean"}]});
        let mut rng = seeded_rng();
        let mut saw_string = false;
        let mut saw_integer = false;
        let mut saw_boolean = false;
        for _ in 0..200 {
            let result = generate(&schema, &mut rng).unwrap();
            match &result {
                Value::String(_) => saw_string = true,
                Value::Number(n) if n.is_i64() => saw_integer = true,
                Value::Bool(_) => saw_boolean = true,
                other => panic!("unexpected variant: {other}"),
            }
        }
        assert!(saw_string, "anyOf never generated a string variant");
        assert!(saw_integer, "anyOf never generated an integer variant");
        assert!(saw_boolean, "anyOf never generated a boolean variant");
    }

    #[test]
    fn test_enum_all_variants_generated() {
        let schema = json!({"enum": [1, "two", true, null]});
        let mut rng = seeded_rng();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            let result = generate(&schema, &mut rng).unwrap();
            seen.insert(result.to_string());
        }
        assert!(seen.contains("1"), "enum never generated 1");
        assert!(seen.contains("\"two\""), "enum never generated \"two\"");
        assert!(seen.contains("true"), "enum never generated true");
        assert!(seen.contains("null"), "enum never generated null");
    }

    #[test]
    fn test_one_of() {
        let schema = json!({"oneOf": [{"type": "string"}, {"type": "integer"}]});
        generate_and_validate_n(&schema, 50);
    }

    #[test]
    fn test_all_of() {
        let schema = json!({
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                },
                {
                    "type": "object",
                    "properties": {
                        "age": {"type": "integer"}
                    },
                    "required": ["age"]
                }
            ]
        });
        let mut rng = seeded_rng();
        for _ in 0..20 {
            let result = generate(&schema, &mut rng).unwrap();
            let obj = result.as_object().unwrap();
            assert!(obj.contains_key("name"));
            assert!(obj.contains_key("age"));
        }
        // Note: jsonschema validation with allOf works on the generated merged
        // result
    }

    // ── $ref ──

    #[test]
    fn test_ref_basic() {
        let schema = json!({
            "$defs": {
                "Name": {"type": "string", "minLength": 1, "maxLength": 20}
            },
            "type": "object",
            "properties": {
                "name": {"$ref": "#/$defs/Name"}
            },
            "required": ["name"]
        });
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_ref_not_found() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"$ref": "#/$defs/NonExistent"}
            },
            "required": ["name"]
        });
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng);
        assert!(result.is_err());
    }

    #[test]
    fn test_ref_nested() {
        let schema = json!({
            "$defs": {
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"}
                    },
                    "required": ["street", "city"]
                }
            },
            "type": "object",
            "properties": {
                "home": {"$ref": "#/$defs/Address"},
                "work": {"$ref": "#/$defs/Address"}
            },
            "required": ["home"]
        });
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_ref_resolves_to_correct_type() {
        // Verify that $ref actually produces values matching the referenced
        // definition, not just "any valid JSON".
        let schema = json!({
            "$defs": {
                "PositiveInt": {"type": "integer", "minimum": 1, "maximum": 1000}
            },
            "type": "object",
            "properties": {
                "count": {"$ref": "#/$defs/PositiveInt"}
            },
            "required": ["count"]
        });
        let mut rng = seeded_rng();
        for _ in 0..50 {
            let result = generate(&schema, &mut rng).unwrap();
            let count = result.get("count").expect("missing 'count'");
            let n = count.as_i64().expect("count should be integer");
            assert!(
                (1..=1000).contains(&n),
                "ref-resolved value {n} doesn't match PositiveInt constraints"
            );
        }
    }

    #[test]
    fn test_ref_definitions_legacy() {
        // "definitions" (draft-07 style) should work the same as "$defs"
        let schema = json!({
            "definitions": {
                "Color": {"type": "string", "enum": ["red", "green", "blue"]}
            },
            "type": "object",
            "properties": {
                "color": {"$ref": "#/definitions/Color"}
            },
            "required": ["color"]
        });
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).unwrap();
        let color = result.get("color").unwrap().as_str().unwrap();
        assert!(
            ["red", "green", "blue"].contains(&color),
            "unexpected color: {color}"
        );
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_ref_external_returns_error() {
        // External $ref (http/file URLs) are not supported and should
        // return a RefNotFound error rather than panicking.
        let schema = json!({
            "type": "object",
            "properties": {
                "item": {"$ref": "https://example.com/schemas/item.json"}
            },
            "required": ["item"]
        });
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng);
        assert!(result.is_err(), "external $ref should return an error");
        let err = result.unwrap_err();
        assert!(
            matches!(err, crate::Error::RefNotFound { .. }),
            "expected RefNotFound, got: {err}"
        );
    }

    // ── Edge cases ──

    #[test]
    fn test_empty_object_schema() {
        let schema = json!({"type": "object"});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).unwrap();
        assert!(result.is_object());
        generate_and_validate(&schema);
    }

    #[test]
    fn test_empty_array_schema() {
        let schema = json!({"type": "array"});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).unwrap();
        assert!(result.is_array());
        generate_and_validate(&schema);
    }

    #[test]
    fn test_string_zero_length() {
        let schema = json!({"type": "string", "minLength": 0, "maxLength": 0});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).unwrap();
        assert_eq!(result, json!(""));
        generate_and_validate(&schema);
    }

    #[test]
    fn test_integer_equal_min_max() {
        let schema = json!({"type": "integer", "minimum": 5, "maximum": 5});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).unwrap();
        assert_eq!(result, json!(5));
        generate_and_validate(&schema);
    }

    #[test]
    fn test_boolean_schema_true() {
        let schema = json!(true);
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng);
        assert!(result.is_ok());
    }

    #[test]
    fn test_boolean_schema_false() {
        let schema = json!(false);
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng);
        assert!(result.is_err());
    }

    // ── Integration (bulk validation) ──

    #[test]
    fn test_person_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1, "maxLength": 50},
                "age": {"type": "integer", "minimum": 0, "maximum": 150},
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"},
                        "zip": {"type": "string", "minLength": 5, "maxLength": 10}
                    },
                    "required": ["street", "city"]
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "minItems": 0,
                    "maxItems": 5
                }
            },
            "required": ["name", "age"]
        });
        generate_and_validate_n(&schema, 100);
    }

    #[test]
    fn test_product_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1, "maxLength": 100},
                "price": {"type": "number", "minimum": 0.01, "maximum": 99999.99},
                "categories": {
                    "type": "array",
                    "items": {"type": "string"},
                    "minItems": 1,
                    "maxItems": 5
                },
                "inStock": {"type": "boolean"}
            },
            "required": ["name", "price", "categories", "inStock"]
        });
        generate_and_validate_n(&schema, 100);
    }

    // ── Format ──

    fn validate_with_formats(schema: &Value, instance: &Value) -> bool {
        jsonschema::options()
            .should_validate_formats(true)
            .build(schema)
            .expect("valid schema")
            .is_valid(instance)
    }

    #[test]
    fn test_string_format_date_time() {
        let schema = json!({"type": "string", "format": "date-time"});
        let mut rng = seeded_rng();
        for i in 0..100 {
            let result = generate(&schema, &mut rng).expect("generation should succeed");
            assert!(
                validate_with_formats(&schema, &result),
                "sample {i} does not validate as date-time.\nvalue: {result}"
            );
        }
    }

    #[test]
    fn test_string_format_date_time_structure() {
        let schema = json!({"type": "string", "format": "date-time"});
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let result = generate(&schema, &mut rng).unwrap();
            let s = result.as_str().unwrap();
            // Expected format: YYYY-MM-DDThh:mm:ssZ
            assert!(s.ends_with('Z'), "should end with Z: {s}");
            let parts: Vec<&str> = s.trim_end_matches('Z').split('T').collect();
            assert_eq!(parts.len(), 2, "should have date and time parts: {s}");
            let date_parts: Vec<i32> = parts[0].split('-').map(|p| p.parse().unwrap()).collect();
            let time_parts: Vec<i32> = parts[1].split(':').map(|p| p.parse().unwrap()).collect();
            assert!(
                (1970..=2099).contains(&date_parts[0]),
                "year out of range: {s}"
            );
            assert!((1..=12).contains(&date_parts[1]), "month out of range: {s}");
            assert!((1..=31).contains(&date_parts[2]), "day out of range: {s}");
            assert!((0..=23).contains(&time_parts[0]), "hour out of range: {s}");
            assert!(
                (0..=59).contains(&time_parts[1]),
                "minute out of range: {s}"
            );
            assert!(
                (0..=59).contains(&time_parts[2]),
                "second out of range: {s}"
            );
        }
    }

    #[test]
    fn test_string_format_date_time_feb_leap_year() {
        let schema = json!({"type": "string", "format": "date-time"});
        let mut rng = seeded_rng();
        for _ in 0..2000 {
            let result = generate(&schema, &mut rng).unwrap();
            let s = result.as_str().unwrap();
            let parts: Vec<&str> = s.trim_end_matches('Z').split('T').collect();
            let date_parts: Vec<i32> = parts[0].split('-').map(|p| p.parse().unwrap()).collect();
            let year = date_parts[0];
            let month = date_parts[1];
            let day = date_parts[2];
            if month == 2 {
                let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                let max_day = if is_leap { 29 } else { 28 };
                assert!(
                    day <= max_day,
                    "Feb day {day} exceeds max {max_day} for year {year}: {s}"
                );
            }
        }
    }

    #[test]
    fn test_string_format_date() {
        let schema = json!({"type": "string", "format": "date"});
        let mut rng = seeded_rng();
        for i in 0..100 {
            let result = generate(&schema, &mut rng).expect("generation should succeed");
            assert!(
                validate_with_formats(&schema, &result),
                "sample {i} does not validate as date.\nvalue: {result}"
            );
        }
    }

    #[test]
    fn test_string_format_date_structure() {
        let schema = json!({"type": "string", "format": "date"});
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let result = generate(&schema, &mut rng).unwrap();
            let s = result.as_str().unwrap();
            let parts: Vec<i32> = s.split('-').map(|p| p.parse().unwrap()).collect();
            assert_eq!(parts.len(), 3, "should have 3 date parts: {s}");
            assert!((1970..=2099).contains(&parts[0]), "year out of range: {s}");
            assert!((1..=12).contains(&parts[1]), "month out of range: {s}");
            assert!((1..=31).contains(&parts[2]), "day out of range: {s}");
        }
    }

    #[test]
    fn test_string_format_time() {
        let schema = json!({"type": "string", "format": "time"});
        let mut rng = seeded_rng();
        for i in 0..100 {
            let result = generate(&schema, &mut rng).expect("generation should succeed");
            assert!(
                validate_with_formats(&schema, &result),
                "sample {i} does not validate as time.\nvalue: {result}"
            );
        }
    }

    #[test]
    fn test_string_format_time_structure() {
        let schema = json!({"type": "string", "format": "time"});
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let result = generate(&schema, &mut rng).unwrap();
            let s = result.as_str().unwrap();
            assert!(s.ends_with('Z'), "should end with Z: {s}");
            let parts: Vec<i32> = s
                .trim_end_matches('Z')
                .split(':')
                .map(|p| p.parse().unwrap())
                .collect();
            assert_eq!(parts.len(), 3, "should have 3 time parts: {s}");
            assert!((0..=23).contains(&parts[0]), "hour out of range: {s}");
            assert!((0..=59).contains(&parts[1]), "minute out of range: {s}");
            assert!((0..=59).contains(&parts[2]), "second out of range: {s}");
        }
    }

    #[test]
    fn test_string_format_duration() {
        let schema = json!({"type": "string", "format": "duration"});
        let mut rng = seeded_rng();
        let validator = jsonschema::options()
            .should_validate_formats(true)
            .with_draft(jsonschema::Draft::Draft201909)
            .build(&schema)
            .expect("valid schema");
        for i in 0..100 {
            let result = generate(&schema, &mut rng).expect("generation should succeed");
            assert!(
                validator.is_valid(&result),
                "sample {i} does not validate as duration.\nvalue: {result}"
            );
        }
    }

    #[test]
    fn test_string_format_duration_structure() {
        let schema = json!({"type": "string", "format": "duration"});
        let mut rng = seeded_rng();
        for _ in 0..100 {
            let result = generate(&schema, &mut rng).unwrap();
            let s = result.as_str().unwrap();
            assert!(s.starts_with('P'), "duration should start with P: {s}");
            assert!(s.len() > 1, "duration should have components after P: {s}");
        }
    }

    #[test]
    fn test_string_format_unknown_falls_back() {
        let schema = json!({"type": "string", "format": "unknown-format"});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng);
        assert!(result.is_ok(), "unknown format should not error");
        assert!(result.unwrap().is_string());
    }

    #[test]
    fn test_string_format_date_time_ignores_length_constraints() {
        let schema =
            json!({"type": "string", "format": "date-time", "minLength": 1000, "maxLength": 2000});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).expect("generation should succeed");
        let s = result.as_str().unwrap();
        // date-time format should take precedence, producing a ~20 char string
        assert!(
            s.len() < 100,
            "format should override length constraints, got len {}",
            s.len()
        );
        assert!(validate_with_formats(
            &json!({"type": "string", "format": "date-time"}),
            &result
        ));
    }

    // ── Pattern ──

    #[test]
    fn test_string_pattern_literal() {
        let schema = json!({"type": "string", "pattern": "^abc$"});
        generate_and_validate(&schema);
    }

    #[test]
    fn test_string_pattern_unanchored() {
        // A full match trivially satisfies `pattern`'s unanchored search.
        let schema = json!({"type": "string", "pattern": "foo"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_date_like() {
        let schema = json!({"type": "string", "pattern": r"^\d{4}-\d{2}-\d{2}$"});
        generate_and_validate_n(&schema, 50);
    }

    #[test]
    fn test_string_pattern_alternation() {
        let schema = json!({"type": "string", "pattern": "^(red|green|blue)$"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_char_class() {
        let schema = json!({"type": "string", "pattern": "^[a-f0-9]{8}$"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_negated_class() {
        let schema = json!({"type": "string", "pattern": "^[^0-9]{3}$"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_perl_classes() {
        let schema = json!({"type": "string", "pattern": r"^\w+\s\d+$"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_case_insensitive() {
        let schema = json!({"type": "string", "pattern": "(?i)^abc$"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_repetition_operators() {
        let schema = json!({"type": "string", "pattern": "^a+b*c?$"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_bounded_repetition() {
        let schema = json!({"type": "string", "pattern": "^[ab]{2,5}$"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_unicode_class() {
        let schema = json!({"type": "string", "pattern": r"^\p{L}{3}$"});
        generate_and_validate_n(&schema, 20);
    }

    #[test]
    fn test_string_pattern_beats_format() {
        let schema = json!({"type": "string", "pattern": r"^\d{3}$", "format": "date"});
        generate_and_validate(&schema);
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).expect("generation should succeed");
        let s = result.as_str().unwrap();
        assert_eq!(s.chars().count(), 3, "pattern should win over format: {s}");
        assert!(
            s.chars().all(|c| c.is_ascii_digit()),
            "pattern should win over format: {s}"
        );
    }

    #[test]
    fn test_string_pattern_ignores_length_constraints() {
        // Documented precedence: when pattern generation succeeds,
        // minLength/maxLength are ignored (so this value legitimately
        // violates minLength and the validation oracle is not used).
        let schema = json!({"type": "string", "pattern": "^ab$", "minLength": 10});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng).expect("generation should succeed");
        assert_eq!(result.as_str().unwrap(), "ab");
    }

    #[test]
    fn test_string_pattern_invalid_regex_falls_back() {
        // jsonschema cannot compile this schema, so no validation oracle;
        // generation should fall back to an unconstrained string.
        let schema = json!({"type": "string", "pattern": "(unclosed"});
        let mut rng = seeded_rng();
        let result = generate(&schema, &mut rng);
        assert!(result.is_ok(), "invalid pattern should not error");
        assert!(result.unwrap().is_string());
    }

    #[test]
    fn test_string_pattern_determinism() {
        let schema = json!({"type": "string", "pattern": r"^[a-z]{5}-\d{3}$"});
        let mut rng_a = seeded_rng();
        let mut rng_b = seeded_rng();
        let a = generate(&schema, &mut rng_a).expect("generation should succeed");
        let b = generate(&schema, &mut rng_b).expect("generation should succeed");
        assert_eq!(a, b);
    }

    // ── Integration (bulk validation) ──

    #[test]
    fn test_schema_with_refs() {
        let schema = json!({
            "$defs": {
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string", "minLength": 1},
                        "city": {"type": "string", "minLength": 1},
                        "country": {"type": "string", "minLength": 1}
                    },
                    "required": ["street", "city", "country"]
                },
                "ContactInfo": {
                    "type": "object",
                    "properties": {
                        "email": {"type": "string"},
                        "phone": {"type": "string"}
                    },
                    "required": ["email"]
                }
            },
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1, "maxLength": 50},
                "homeAddress": {"$ref": "#/$defs/Address"},
                "workAddress": {"$ref": "#/$defs/Address"},
                "contact": {"$ref": "#/$defs/ContactInfo"}
            },
            "required": ["name", "homeAddress", "contact"]
        });
        generate_and_validate_n(&schema, 100);
    }
}
