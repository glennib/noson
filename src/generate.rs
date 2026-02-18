use rand::Rng;
use rand::RngExt;
use serde_json::Map;
use serde_json::Value;

use crate::error::Error;

struct Context<'a> {
    root: &'a Value,
    depth: usize,
    max_depth: usize,
}

/// Generate a random JSON value that conforms to the given JSON Schema.
///
/// `schema` must be a JSON Schema represented as a [`serde_json::Value`] --
/// typically an object (e.g. `{"type": "string"}`), but boolean schemas
/// (`true` / `false`) are also accepted.
///
/// The caller provides the RNG, so output is reproducible when a seeded
/// generator is used.
///
/// # Returns
///
/// A random [`serde_json::Value`] that satisfies the schema, or an [`Error`]
/// if the schema is invalid, contains unresolvable `$ref` pointers,
/// has conflicting constraints, or exceeds the maximum recursion depth of 16.
pub fn generate(schema: &Value, rng: &mut impl Rng) -> Result<Value, Error> {
    let ctx = Context {
        root: schema,
        depth: 0,
        max_depth: 16,
    };
    generate_value(&ctx, schema, rng)
}

fn generate_value(ctx: &Context, schema: &Value, rng: &mut impl Rng) -> Result<Value, Error> {
    // Boolean schemas
    if let Some(b) = schema.as_bool() {
        return if b {
            Ok(generate_random_simple(rng))
        } else {
            Err(Error::InvalidSchema {
                message: "false schema rejects all values".into(),
            })
        };
    }

    let obj = match schema.as_object() {
        Some(o) => o,
        None => {
            return Err(Error::InvalidSchema {
                message: format!("schema must be a boolean or object, got {}", schema),
            });
        }
    };

    // Empty object schema — generate a random simple value
    if obj.is_empty() {
        return Ok(generate_random_simple(rng));
    }

    // const
    if let Some(val) = obj.get("const") {
        return Ok(val.clone());
    }

    // enum
    if let Some(Value::Array(variants)) = obj.get("enum") {
        if variants.is_empty() {
            return Err(Error::InvalidSchema {
                message: "enum must have at least one variant".into(),
            });
        }
        let idx = rng.random_range(0..variants.len());
        return Ok(variants[idx].clone());
    }

    // $ref
    if let Some(Value::String(reference)) = obj.get("$ref") {
        return generate_ref(ctx, reference, rng);
    }

    // allOf
    if let Some(Value::Array(sub_schemas)) = obj.get("allOf") {
        return generate_all_of(ctx, sub_schemas, rng);
    }

    // anyOf
    if let Some(Value::Array(sub_schemas)) = obj.get("anyOf") {
        if sub_schemas.is_empty() {
            return Err(Error::InvalidSchema {
                message: "anyOf must have at least one sub-schema".into(),
            });
        }
        let idx = rng.random_range(0..sub_schemas.len());
        return generate_value(ctx, &sub_schemas[idx], rng);
    }

    // oneOf
    if let Some(Value::Array(sub_schemas)) = obj.get("oneOf") {
        if sub_schemas.is_empty() {
            return Err(Error::InvalidSchema {
                message: "oneOf must have at least one sub-schema".into(),
            });
        }
        let idx = rng.random_range(0..sub_schemas.len());
        return generate_value(ctx, &sub_schemas[idx], rng);
    }

    // Dispatch on "type"
    if let Some(Value::String(type_name)) = obj.get("type") {
        return match type_name.as_str() {
            "null" => Ok(Value::Null),
            "boolean" => Ok(Value::Bool(rng.random_bool(0.5))),
            "string" => generate_string(obj, rng),
            "integer" => generate_integer(obj, rng),
            "number" => generate_number(obj, rng),
            "object" => generate_object(ctx, obj, rng),
            "array" => generate_array(ctx, obj, rng),
            other => Err(Error::UnsupportedType {
                type_name: other.into(),
            }),
        };
    }

    // No type but has properties → treat as object
    if obj.contains_key("properties") {
        return generate_object(ctx, obj, rng);
    }

    // No type but has items → treat as array
    if obj.contains_key("items") {
        return generate_array(ctx, obj, rng);
    }

    // No type info at all — generate random simple value
    Ok(generate_random_simple(rng))
}

fn generate_random_simple(rng: &mut impl Rng) -> Value {
    match rng.random_range(0..4u8) {
        0 => Value::Null,
        1 => Value::Bool(rng.random_bool(0.5)),
        2 => Value::Number(serde_json::Number::from(rng.random_range(0..100))),
        _ => {
            let len = rng.random_range(1..=10);
            Value::String(random_alphanumeric_string(rng, len))
        }
    }
}

fn random_alphanumeric_string(rng: &mut impl Rng, len: usize) -> String {
    (0..len)
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .collect()
}

fn generate_string(obj: &Map<String, Value>, rng: &mut impl Rng) -> Result<Value, Error> {
    if let Some(Value::String(format)) = obj.get("format")
        && let Some(value) = generate_formatted_string(format, rng)?
    {
        return Ok(value);
    }

    let min_len = obj.get("minLength").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let max_len = obj
        .get("maxLength")
        .and_then(|v| v.as_u64())
        .unwrap_or(min_len.max(10) as u64) as usize;

    if min_len > max_len {
        return Err(Error::ConflictingConstraints {
            message: format!("minLength ({min_len}) > maxLength ({max_len})"),
        });
    }

    let len = rng.random_range(min_len..=max_len);
    Ok(Value::String(random_alphanumeric_string(rng, len)))
}

fn generate_formatted_string(format: &str, rng: &mut impl Rng) -> Result<Option<Value>, Error> {
    match format {
        "date-time" => generate_date_time(rng).map(Some),
        "date" => generate_date(rng).map(Some),
        "time" => generate_time(rng).map(Some),
        "duration" => generate_duration(rng).map(Some),
        _ => Ok(None),
    }
}

fn generate_date_time(rng: &mut impl Rng) -> Result<Value, Error> {
    let year = rng.random_range(1970..=2099i16);
    let month = rng.random_range(1..=12i8);
    let max_day = jiff::civil::Date::new(year, month, 1)
        .expect("valid date")
        .days_in_month();
    let day = rng.random_range(1..=max_day);
    let hour = rng.random_range(0..=23i8);
    let minute = rng.random_range(0..=59i8);
    let second = rng.random_range(0..=59i8);
    let dt = jiff::civil::DateTime::new(year, month, day, hour, minute, second, 0)
        .expect("valid datetime");
    Ok(Value::String(format!("{dt}Z")))
}

fn generate_date(rng: &mut impl Rng) -> Result<Value, Error> {
    let year = rng.random_range(1970..=2099i16);
    let month = rng.random_range(1..=12i8);
    let max_day = jiff::civil::Date::new(year, month, 1)
        .expect("valid date")
        .days_in_month();
    let day = rng.random_range(1..=max_day);
    let date = jiff::civil::Date::new(year, month, day).expect("valid date");
    Ok(Value::String(format!("{date}")))
}

fn generate_time(rng: &mut impl Rng) -> Result<Value, Error> {
    let hour = rng.random_range(0..=23i8);
    let minute = rng.random_range(0..=59i8);
    let second = rng.random_range(0..=59i8);
    let time = jiff::civil::Time::new(hour, minute, second, 0).expect("valid time");
    Ok(Value::String(format!("{time}Z")))
}

fn generate_duration(rng: &mut impl Rng) -> Result<Value, Error> {
    let use_weeks = rng.random_bool(0.5);
    if use_weeks {
        let weeks = rng.random_range(1..=52u32);
        return Ok(Value::String(format!("P{weeks}W")));
    }

    // Mixed duration: randomly include components, ensuring at least one is present
    loop {
        let years = if rng.random_bool(0.3) {
            Some(rng.random_range(0..=10u32))
        } else {
            None
        };
        let months = if rng.random_bool(0.3) {
            Some(rng.random_range(0..=11u32))
        } else {
            None
        };
        let days = if rng.random_bool(0.3) {
            Some(rng.random_range(0..=30u32))
        } else {
            None
        };
        let hours = if rng.random_bool(0.3) {
            Some(rng.random_range(0..=23u32))
        } else {
            None
        };
        let minutes = if rng.random_bool(0.3) {
            Some(rng.random_range(0..=59u32))
        } else {
            None
        };
        let seconds = if rng.random_bool(0.3) {
            Some(rng.random_range(0..=59u32))
        } else {
            None
        };

        let has_any = years.is_some()
            || months.is_some()
            || days.is_some()
            || hours.is_some()
            || minutes.is_some()
            || seconds.is_some();
        if !has_any {
            continue;
        }

        let mut s = String::from("P");
        if let Some(y) = years {
            s.push_str(&format!("{y}Y"));
        }
        if let Some(m) = months {
            s.push_str(&format!("{m}M"));
        }
        if let Some(d) = days {
            s.push_str(&format!("{d}D"));
        }

        let has_time = hours.is_some() || minutes.is_some() || seconds.is_some();
        if has_time {
            s.push('T');
            if let Some(h) = hours {
                s.push_str(&format!("{h}H"));
            }
            if let Some(min) = minutes {
                s.push_str(&format!("{min}M"));
            }
            if let Some(sec) = seconds {
                s.push_str(&format!("{sec}S"));
            }
        }

        return Ok(Value::String(s));
    }
}

fn generate_integer(obj: &Map<String, Value>, rng: &mut impl Rng) -> Result<Value, Error> {
    let min = get_integer_bound(obj, "minimum", "exclusiveMinimum", true);
    let max = get_integer_bound(obj, "maximum", "exclusiveMaximum", false);

    let min = min.unwrap_or(0);
    let max = max.unwrap_or(min.max(0) + 100);

    if min > max {
        return Err(Error::ConflictingConstraints {
            message: format!("effective minimum ({min}) > effective maximum ({max})"),
        });
    }

    let val = rng.random_range(min..=max);
    Ok(Value::Number(serde_json::Number::from(val)))
}

fn get_integer_bound(
    obj: &Map<String, Value>,
    inclusive_key: &str,
    exclusive_key: &str,
    is_min: bool,
) -> Option<i64> {
    if let Some(val) = obj.get(exclusive_key).and_then(|v| v.as_i64()) {
        return Some(if is_min { val + 1 } else { val - 1 });
    }
    if let Some(val) = obj.get(exclusive_key).and_then(|v| v.as_f64()) {
        return Some(if is_min {
            val.ceil() as i64
        } else {
            (val - 1.0).ceil() as i64
        });
    }
    obj.get(inclusive_key).and_then(|v| v.as_i64())
}

fn generate_number(obj: &Map<String, Value>, rng: &mut impl Rng) -> Result<Value, Error> {
    let (min, min_exclusive) = get_number_bound(obj, "minimum", "exclusiveMinimum");
    let (max, max_exclusive) = get_number_bound(obj, "maximum", "exclusiveMaximum");

    let min = min.unwrap_or(0.0);
    let max = max.unwrap_or(if min > 0.0 { min + 100.0 } else { 100.0 });

    if min > max || (min == max && (min_exclusive || max_exclusive)) {
        return Err(Error::ConflictingConstraints {
            message: format!("effective minimum ({min}) > effective maximum ({max})"),
        });
    }

    let val = rng.random_range(min..=max);

    // Clamp away from exclusive bounds with a tiny epsilon
    let val = if min_exclusive && val == min {
        min + f64::EPSILON
    } else {
        val
    };
    let val = if max_exclusive && val == max {
        max - f64::EPSILON
    } else {
        val
    };

    match serde_json::Number::from_f64(val) {
        Some(n) => Ok(Value::Number(n)),
        None => Ok(Value::Number(serde_json::Number::from(0))),
    }
}

fn get_number_bound(
    obj: &Map<String, Value>,
    inclusive_key: &str,
    exclusive_key: &str,
) -> (Option<f64>, bool) {
    if let Some(val) = obj.get(exclusive_key).and_then(|v| v.as_f64()) {
        return (Some(val), true);
    }
    if let Some(val) = obj.get(inclusive_key).and_then(|v| v.as_f64()) {
        return (Some(val), false);
    }
    (None, false)
}

fn generate_object(
    ctx: &Context,
    obj: &Map<String, Value>,
    rng: &mut impl Rng,
) -> Result<Value, Error> {
    if ctx.depth >= ctx.max_depth {
        return Err(Error::MaxDepthExceeded);
    }

    let child_ctx = Context {
        root: ctx.root,
        depth: ctx.depth + 1,
        max_depth: ctx.max_depth,
    };

    let mut result = Map::new();

    let required: Vec<&str> = match obj.get("required") {
        Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).collect(),
        _ => Vec::new(),
    };

    if let Some(Value::Object(properties)) = obj.get("properties") {
        for (key, prop_schema) in properties {
            let is_required = required.iter().any(|r| r == key);
            if is_required || rng.random_bool(0.5) {
                let val = generate_value(&child_ctx, prop_schema, rng)?;
                result.insert(key.clone(), val);
            }
        }
    }

    Ok(Value::Object(result))
}

fn generate_array(
    ctx: &Context,
    obj: &Map<String, Value>,
    rng: &mut impl Rng,
) -> Result<Value, Error> {
    if ctx.depth >= ctx.max_depth {
        return Err(Error::MaxDepthExceeded);
    }

    let child_ctx = Context {
        root: ctx.root,
        depth: ctx.depth + 1,
        max_depth: ctx.max_depth,
    };

    let min_items = obj.get("minItems").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let max_items = obj
        .get("maxItems")
        .and_then(|v| v.as_u64())
        .unwrap_or(min_items.max(3) as u64) as usize;

    if min_items > max_items {
        return Err(Error::ConflictingConstraints {
            message: format!("minItems ({min_items}) > maxItems ({max_items})"),
        });
    }

    let count = rng.random_range(min_items..=max_items);

    let item_schema = obj.get("items").cloned().unwrap_or(Value::Bool(true));

    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(generate_value(&child_ctx, &item_schema, rng)?);
    }

    Ok(Value::Array(items))
}

fn generate_ref(ctx: &Context, reference: &str, rng: &mut impl Rng) -> Result<Value, Error> {
    if ctx.depth >= ctx.max_depth {
        return Err(Error::MaxDepthExceeded);
    }

    let child_ctx = Context {
        root: ctx.root,
        depth: ctx.depth + 1,
        max_depth: ctx.max_depth,
    };

    let resolved = resolve_ref(ctx.root, reference)?;
    generate_value(&child_ctx, resolved, rng)
}

fn resolve_ref<'a>(root: &'a Value, reference: &str) -> Result<&'a Value, Error> {
    // Parse references like "#/$defs/Foo" or "#/definitions/Bar"
    let path = reference.strip_prefix('#').unwrap_or(reference);
    let path = path.strip_prefix('/').unwrap_or(path);

    let mut current = root;
    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }
        match current {
            Value::Object(obj) => match obj.get(segment) {
                Some(val) => current = val,
                None => {
                    return Err(Error::RefNotFound {
                        reference: reference.into(),
                    });
                }
            },
            _ => {
                return Err(Error::RefNotFound {
                    reference: reference.into(),
                });
            }
        }
    }

    // If we never moved (e.g. "#" or empty ref), return root
    Ok(current)
}

fn generate_all_of(
    ctx: &Context,
    sub_schemas: &[Value],
    rng: &mut impl Rng,
) -> Result<Value, Error> {
    if sub_schemas.is_empty() {
        return Ok(generate_random_simple(rng));
    }

    // Merge all sub-schemas into a single object schema
    let mut merged_properties = Map::new();
    let mut merged_required: Vec<String> = Vec::new();
    let mut merged_type: Option<String> = None;

    for sub in sub_schemas {
        let sub_obj = match sub.as_object() {
            Some(o) => o,
            None => {
                // Boolean true is a no-op; false rejects
                if sub.as_bool() == Some(false) {
                    return Err(Error::AllOfFailed {
                        message: "allOf contains false schema".into(),
                    });
                }
                continue;
            }
        };

        if let Some(Value::Object(props)) = sub_obj.get("properties") {
            for (k, v) in props {
                merged_properties.insert(k.clone(), v.clone());
            }
        }

        if let Some(Value::Array(req)) = sub_obj.get("required") {
            for r in req.iter().filter_map(|v| v.as_str()) {
                if !merged_required.iter().any(|existing| existing == r) {
                    merged_required.push(r.to_string());
                }
            }
        }

        if let Some(Value::String(t)) = sub_obj.get("type") {
            if let Some(ref existing) = merged_type {
                if existing != t {
                    return Err(Error::AllOfFailed {
                        message: format!("conflicting types in allOf: {existing} vs {t}"),
                    });
                }
            } else {
                merged_type = Some(t.clone());
            }
        }
    }

    // Build merged schema
    let mut merged = Map::new();
    if let Some(t) = merged_type {
        merged.insert("type".into(), Value::String(t));
    } else if !merged_properties.is_empty() {
        merged.insert("type".into(), Value::String("object".into()));
    }
    if !merged_properties.is_empty() {
        merged.insert("properties".into(), Value::Object(merged_properties));
    }
    if !merged_required.is_empty() {
        merged.insert(
            "required".into(),
            Value::Array(merged_required.into_iter().map(Value::String).collect()),
        );
    }

    generate_value(ctx, &Value::Object(merged), rng)
}
