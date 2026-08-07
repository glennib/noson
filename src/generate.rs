use std::fmt::Write as _;

use rand::Rng;
use rand::RngExt;
use serde_json::Map;
use serde_json::Value;

use crate::error::Error;
use crate::xeger;

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

    // Composition keywords ($ref, allOf, anyOf, oneOf) are conjunctive with
    // their sibling keywords, so they are resolved by merging before any
    // other keyword is considered.
    if COMPOSITION_KEYWORDS.iter().any(|k| obj.contains_key(*k)) {
        return generate_composed(ctx, obj, rng);
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

    // Dispatch on "type"
    if let Some(Value::String(type_name)) = obj.get("type") {
        return generate_typed(ctx, obj, type_name, rng);
    }

    // "type" as an array of type names — pick one uniformly, then generate
    // with the full schema so sibling constraints (minimum, maxLength, …)
    // still apply to the picked type.
    if let Some(Value::Array(type_names)) = obj.get("type") {
        if type_names.is_empty() {
            return Err(Error::InvalidSchema {
                message: "type array must have at least one entry".into(),
            });
        }
        let idx = rng.random_range(0..type_names.len());
        let Some(type_name) = type_names[idx].as_str() else {
            return Err(Error::InvalidSchema {
                message: format!(
                    "type array entries must be strings, got {}",
                    type_names[idx]
                ),
            });
        };
        return generate_typed(ctx, obj, type_name, rng);
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

fn generate_typed(
    ctx: &Context,
    obj: &Map<String, Value>,
    type_name: &str,
    rng: &mut impl Rng,
) -> Result<Value, Error> {
    match type_name {
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
    }
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

/// Samples drawn from a `pattern` before concluding its length constraints
/// cannot be met.
const PATTERN_LENGTH_RETRIES: usize = 64;

fn generate_string(obj: &Map<String, Value>, rng: &mut impl Rng) -> Result<Value, Error> {
    let min_len = obj.get("minLength").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let max_len_constraint = obj
        .get("maxLength")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    if let Some(max_len) = max_len_constraint
        && min_len > max_len
    {
        return Err(Error::ConflictingConstraints {
            message: format!("minLength ({min_len}) > maxLength ({max_len})"),
        });
    }

    // `pattern` is an assertion keyword, so it takes precedence over
    // `format` (annotation-only by default). Samples are redrawn until one
    // also satisfies `minLength`/`maxLength` (length in code points, per the
    // spec); when none does, the constraints conflict. Unsupported patterns
    // fall through to unconstrained generation.
    if let Some(Value::String(pattern)) = obj.get("pattern") {
        let mut pattern_supported = false;
        for _ in 0..PATTERN_LENGTH_RETRIES {
            let Some(s) = xeger::generate_matching(pattern, rng) else {
                break;
            };
            pattern_supported = true;
            let len = s.chars().count();
            if len >= min_len && max_len_constraint.is_none_or(|max| len <= max) {
                return Ok(Value::String(s));
            }
        }
        if pattern_supported {
            return Err(Error::ConflictingConstraints {
                message: format!(
                    "no string matching pattern `{pattern}` satisfied minLength/maxLength in \
                     {PATTERN_LENGTH_RETRIES} attempts"
                ),
            });
        }
    }

    if let Some(Value::String(format)) = obj.get("format")
        && let Some(value) = generate_formatted_string(format, rng)?
    {
        return Ok(value);
    }

    let max_len = max_len_constraint.unwrap_or(min_len.max(10));
    let len = rng.random_range(min_len..=max_len);
    Ok(Value::String(random_alphanumeric_string(rng, len)))
}

fn generate_formatted_string(format: &str, rng: &mut impl Rng) -> Result<Option<Value>, Error> {
    match format {
        "date-time" => generate_date_time(rng).map(Some),
        "date" => generate_date(rng).map(Some),
        "time" => generate_time(rng).map(Some),
        "duration" => generate_duration(rng).map(Some),
        "uuid" => Ok(Some(generate_uuid(rng))),
        "email" => Ok(Some(generate_email(rng))),
        "uri" => Ok(Some(generate_uri(rng))),
        "hostname" => Ok(Some(Value::String(generate_hostname(rng)))),
        "ipv4" => Ok(Some(generate_ipv4(rng))),
        "ipv6" => Ok(Some(generate_ipv6(rng))),
        _ => Ok(None),
    }
}

/// A random version-4 UUID. Bytes come from the caller's RNG (not
/// `Uuid::new_v4`, which draws OS entropy) so output stays reproducible
/// under a seeded generator.
fn generate_uuid(rng: &mut impl Rng) -> Value {
    let uuid = uuid::Builder::from_random_bytes(rng.random()).into_uuid();
    Value::String(uuid.to_string())
}

/// A random lowercase alphanumeric label starting with a letter — safe as a
/// hostname label, email local part, or URI path segment.
fn random_label(rng: &mut impl Rng, max_len: usize) -> String {
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const ALPHANUMERIC: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let len = rng.random_range(1..=max_len);
    let mut s = String::with_capacity(len);
    s.push(LETTERS[rng.random_range(0..LETTERS.len())] as char);
    for _ in 1..len {
        s.push(ALPHANUMERIC[rng.random_range(0..ALPHANUMERIC.len())] as char);
    }
    s
}

fn generate_hostname(rng: &mut impl Rng) -> String {
    let labels = rng.random_range(1..=3usize);
    let mut s = String::new();
    for i in 0..labels {
        if i > 0 {
            s.push('.');
        }
        s.push_str(&random_label(rng, 10));
    }
    s
}

fn generate_email(rng: &mut impl Rng) -> Value {
    let local = random_label(rng, 10);
    let domain = generate_hostname(rng);
    Value::String(format!("{local}@{domain}"))
}

fn generate_uri(rng: &mut impl Rng) -> Value {
    let scheme = if rng.random_bool(0.5) {
        "https"
    } else {
        "http"
    };
    let host = generate_hostname(rng);
    let mut uri = format!("{scheme}://{host}");
    for _ in 0..rng.random_range(0..=2usize) {
        uri.push('/');
        uri.push_str(&random_label(rng, 8));
    }
    Value::String(uri)
}

fn generate_ipv4(rng: &mut impl Rng) -> Value {
    let octets: [u8; 4] = rng.random();
    Value::String(format!(
        "{}.{}.{}.{}",
        octets[0], octets[1], octets[2], octets[3]
    ))
}

/// A random IPv6 address in full uncompressed form (eight groups), which
/// sidesteps the `::` compression rules.
fn generate_ipv6(rng: &mut impl Rng) -> Value {
    let groups: [u16; 8] = rng.random();
    let mut s = String::new();
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            s.push(':');
        }
        let _ = write!(s, "{group:x}");
    }
    Value::String(s)
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

    // The RFC 3339 appendix A grammar chains duration units without gaps:
    // the date part must be one of Y, YM, YMD, M, MD, D and the time part one
    // of H, HM, HMS, M, MS, S. Emitting each part as a contiguous run of
    // units makes the output valid by construction.
    const DATE_UNITS: [(char, u32); 3] = [('Y', 10), ('M', 11), ('D', 30)];
    const TIME_UNITS: [(char, u32); 3] = [('H', 23), ('M', 59), ('S', 59)];

    fn push_run(s: &mut String, units: &[(char, u32)], rng: &mut impl Rng) {
        let start = rng.random_range(0..units.len());
        let len = rng.random_range(1..=units.len() - start);
        for &(unit, max) in &units[start..start + len] {
            let value = rng.random_range(0..=max);
            s.push_str(&format!("{value}{unit}"));
        }
    }

    let mut s = String::from("P");
    match rng.random_range(0..3u8) {
        0 => push_run(&mut s, &DATE_UNITS, rng),
        1 => {
            s.push('T');
            push_run(&mut s, &TIME_UNITS, rng);
        }
        _ => {
            push_run(&mut s, &DATE_UNITS, rng);
            s.push('T');
            push_run(&mut s, &TIME_UNITS, rng);
        }
    }
    Ok(Value::String(s))
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

    if let Some(m_value) = obj.get("multipleOf") {
        let Some(m) = m_value.as_f64().filter(|m| *m > 0.0) else {
            return Err(Error::InvalidSchema {
                message: format!("multipleOf must be a positive number, got {m_value}"),
            });
        };
        let Some(step) = integer_step(m) else {
            return Err(Error::ConflictingConstraints {
                message: format!("no integer is a multiple of {m}"),
            });
        };
        let k_min = min.div_euclid(step) + i64::from(min.rem_euclid(step) != 0);
        let k_max = max.div_euclid(step);
        if k_min > k_max {
            return Err(Error::ConflictingConstraints {
                message: format!("no multiple of {m} in [{min}, {max}]"),
            });
        }
        let k = rng.random_range(k_min..=k_max);
        return Ok(Value::Number(serde_json::Number::from(k * step)));
    }

    let val = rng.random_range(min..=max);
    Ok(Value::Number(serde_json::Number::from(val)))
}

/// The smallest positive integer that is a multiple of `m` — the stride for
/// integer generation under `multipleOf`. Handles fractional `m` (2.5 → 5);
/// `None` when no small integer multiple exists.
fn integer_step(m: f64) -> Option<i64> {
    (1..=1000).find_map(|j| {
        let candidate = m * f64::from(j);
        ((candidate - candidate.round()).abs() < 1e-9).then_some(candidate.round() as i64)
    })
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

    if let Some(m_value) = obj.get("multipleOf") {
        let Some(m) = m_value.as_f64().filter(|m| *m > 0.0) else {
            return Err(Error::InvalidSchema {
                message: format!("multipleOf must be a positive number, got {m_value}"),
            });
        };
        // Work in step space: pick an integer k and emit k*m.
        let mut k_min = (min / m).ceil();
        let mut k_max = (max / m).floor();
        if min_exclusive && k_min * m <= min {
            k_min += 1.0;
        }
        if max_exclusive && k_max * m >= max {
            k_max -= 1.0;
        }
        if k_min > k_max {
            return Err(Error::ConflictingConstraints {
                message: format!("no multiple of {m} in the range ({min}, {max})"),
            });
        }
        let k = rng.random_range(k_min as i64..=k_max as i64);
        let val = snap_to_step_decimals(k as f64 * m, m_value);
        return match serde_json::Number::from_f64(val) {
            Some(n) => Ok(Value::Number(n)),
            None => Err(Error::ConflictingConstraints {
                message: format!("multiple of {m} ({val}) is not representable"),
            }),
        };
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

/// The binary product `k*m` can land an ulp away from the exact decimal
/// product (6 * 0.1 = 0.6000000000000001), which validators using exact
/// rational arithmetic on the decimal representation reject. Rounding to
/// the decimal precision of `multipleOf`'s own representation recovers the
/// exact product.
fn snap_to_step_decimals(val: f64, m_value: &Value) -> f64 {
    let repr = m_value.to_string();
    if repr.contains(['e', 'E']) {
        return val;
    }
    let Some(dot) = repr.find('.') else {
        return val;
    };
    let decimals = repr.len() - dot - 1;
    format!("{val:.decimals$}").parse().unwrap_or(val)
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

/// How the schema constrains properties beyond the declared `properties`.
enum ExtraValues<'a> {
    /// `additionalProperties: false`
    Forbidden,
    /// `additionalProperties` absent or `true`
    Any,
    /// `additionalProperties` is a schema
    Schema(&'a Value),
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

    let properties = match obj.get("properties") {
        Some(Value::Object(props)) => Some(props),
        _ => None,
    };

    let mut required: Vec<&str> = match obj.get("required") {
        Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).collect(),
        _ => Vec::new(),
    };

    let dependent_required = match obj.get("dependentRequired") {
        None => None,
        Some(Value::Object(map)) => {
            for (name, deps) in map {
                let is_string_array =
                    matches!(deps, Value::Array(deps) if deps.iter().all(Value::is_string));
                if !is_string_array {
                    return Err(Error::InvalidSchema {
                        message: format!(
                            "dependentRequired entries must be arrays of strings, got {deps} for \
                             `{name}`"
                        ),
                    });
                }
            }
            Some(map)
        }
        Some(other) => {
            return Err(Error::InvalidSchema {
                message: format!("dependentRequired must be an object, got {other}"),
            });
        }
    };

    // A required property drags in its transitive dependentRequired closure,
    // so the closure is folded into `required` before the count checks.
    if let Some(deps_map) = dependent_required {
        let mut i = 0;
        while i < required.len() {
            if let Some(Value::Array(deps)) = deps_map.get(required[i]) {
                for dep in deps.iter().filter_map(Value::as_str) {
                    if !required.contains(&dep) {
                        required.push(dep);
                    }
                }
            }
            i += 1;
        }
    }

    let min_props = obj
        .get("minProperties")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let max_props = obj
        .get("maxProperties")
        .and_then(Value::as_u64)
        .map_or(usize::MAX, |v| v as usize);

    if min_props > max_props {
        return Err(Error::ConflictingConstraints {
            message: format!("minProperties ({min_props}) > maxProperties ({max_props})"),
        });
    }
    if required.len() > max_props {
        return Err(Error::ConflictingConstraints {
            message: format!(
                "required has {} properties, maxProperties is {max_props}",
                required.len()
            ),
        });
    }

    let pattern_properties: Vec<(&String, &Value)> = match obj.get("patternProperties") {
        Some(Value::Object(patterns)) => patterns.iter().collect(),
        _ => Vec::new(),
    };
    let extra_values = match obj.get("additionalProperties") {
        None | Some(Value::Bool(true)) => ExtraValues::Any,
        Some(Value::Bool(false)) => ExtraValues::Forbidden,
        Some(schema @ Value::Object(_)) => ExtraValues::Schema(schema),
        Some(other) => {
            return Err(Error::InvalidSchema {
                message: format!("additionalProperties must be a boolean or schema, got {other}"),
            });
        }
    };

    let mut optional: Vec<(&String, &Value)> = properties
        .map(|props| {
            props
                .iter()
                .filter(|(key, _)| !required.contains(&key.as_str()))
                .collect()
        })
        .unwrap_or_default();

    let declared_total = required.len() + optional.len();
    let target_min = required.len().max(min_props);
    let can_synthesize =
        !pattern_properties.is_empty() || !matches!(extra_values, ExtraValues::Forbidden);

    // Extra (undeclared) properties are synthesized only when the declared
    // ones cannot reach the minimum count; the headroom above `target_min`
    // gives map-shaped schemas some size variety.
    let target_max = if declared_total >= target_min {
        max_props.min(declared_total)
    } else if can_synthesize {
        max_props.min(target_min + 3)
    } else {
        return Err(Error::ConflictingConstraints {
            message: format!(
                "minProperties ({min_props}) exceeds the {declared_total} declared properties and \
                 no additionalProperties/patternProperties schema allows more"
            ),
        });
    };
    let target = rng.random_range(target_min..=target_max);

    let mut result = Map::new();

    for name in &required {
        let value = generate_property_value(&child_ctx, properties, &extra_values, name, rng)?;
        result.insert((*name).to_string(), value);
    }

    // Optional declared properties fill toward the target, sampled without
    // replacement. A selected property drags in its missing transitive
    // dependentRequired closure; candidates whose closure does not fit
    // within `maxProperties` are skipped.
    while result.len() < target && !optional.is_empty() {
        let idx = rng.random_range(0..optional.len());
        let (name, prop_schema) = optional.swap_remove(idx);
        if result.contains_key(name.as_str()) {
            continue;
        }
        let dependents = missing_dependents(dependent_required, name, &result);
        if result.len() + 1 + dependents.len() > max_props {
            continue;
        }
        let value = generate_value(&child_ctx, prop_schema, rng)?;
        result.insert(name.clone(), value);
        for dep in dependents {
            let value = generate_property_value(&child_ctx, properties, &extra_values, dep, rng)?;
            result.insert(dep.to_string(), value);
        }
    }

    while result.len() < target && can_synthesize {
        let inserted = synthesize_extra_property(
            &child_ctx,
            obj,
            &pattern_properties,
            &extra_values,
            &mut result,
            rng,
        )?;
        if !inserted {
            break;
        }
    }

    if result.len() < min_props {
        return Err(Error::ConflictingConstraints {
            message: format!(
                "could not synthesize enough distinct property names to reach minProperties \
                 ({min_props})"
            ),
        });
    }

    // Synthesized names can themselves trigger dependentRequired, so a final
    // closure pass runs over everything present.
    if let Some(deps_map) = dependent_required {
        loop {
            let mut missing: Vec<&str> = Vec::new();
            for name in result.keys() {
                if let Some(Value::Array(deps)) = deps_map.get(name) {
                    for dep in deps.iter().filter_map(Value::as_str) {
                        if !result.contains_key(dep) && !missing.contains(&dep) {
                            missing.push(dep);
                        }
                    }
                }
            }
            if missing.is_empty() {
                break;
            }
            if result.len() + missing.len() > max_props {
                return Err(Error::ConflictingConstraints {
                    message: format!(
                        "dependentRequired needs {missing:?}, which would exceed maxProperties \
                         ({max_props})"
                    ),
                });
            }
            for dep in missing {
                let value =
                    generate_property_value(&child_ctx, properties, &extra_values, dep, rng)?;
                result.insert(dep.to_string(), value);
            }
        }
    }

    Ok(Value::Object(result))
}

/// Generate a value for a property that must be present: from its declared
/// schema when it has one, otherwise from `additionalProperties`.
fn generate_property_value(
    ctx: &Context,
    properties: Option<&Map<String, Value>>,
    extra_values: &ExtraValues,
    name: &str,
    rng: &mut impl Rng,
) -> Result<Value, Error> {
    match properties.and_then(|props| props.get(name)) {
        Some(prop_schema) => generate_value(ctx, prop_schema, rng),
        None => match extra_values {
            ExtraValues::Schema(schema) => generate_value(ctx, schema, rng),
            ExtraValues::Any => Ok(generate_random_simple(rng)),
            ExtraValues::Forbidden => Err(Error::ConflictingConstraints {
                message: format!(
                    "property `{name}` must be present but is not declared in properties and \
                     additionalProperties is false"
                ),
            }),
        },
    }
}

/// The transitive `dependentRequired` closure that selecting `candidate`
/// would pull in: dependents (of the candidate, and of each other) that are
/// not already present.
fn missing_dependents<'a>(
    dependent_required: Option<&'a Map<String, Value>>,
    candidate: &str,
    present: &Map<String, Value>,
) -> Vec<&'a str> {
    let Some(deps_map) = dependent_required else {
        return Vec::new();
    };
    let mut needed: Vec<&'a str> = Vec::new();
    let mut pending = vec![candidate];
    while let Some(name) = pending.pop() {
        let Some(Value::Array(deps)) = deps_map.get(name) else {
            continue;
        };
        for dep in deps.iter().filter_map(Value::as_str) {
            if dep != candidate && !present.contains_key(dep) && !needed.contains(&dep) {
                needed.push(dep);
                pending.push(dep);
            }
        }
    }
    needed
}

/// Attempts per synthesized property before concluding the name space is
/// exhausted.
const EXTRA_NAME_RETRIES: usize = 16;

/// Synthesize one undeclared property into `result`. The name comes from a
/// `patternProperties` regex when one exists (its value schema is then the
/// one that validator applies), otherwise from the `propertyNames` schema,
/// otherwise a random alphanumeric string; the value comes from the
/// matching `patternProperties` schema or from `additionalProperties`.
/// Returns `false` when no fresh name was found.
fn synthesize_extra_property(
    ctx: &Context,
    obj: &Map<String, Value>,
    pattern_properties: &[(&String, &Value)],
    extra_values: &ExtraValues,
    result: &mut Map<String, Value>,
    rng: &mut impl Rng,
) -> Result<bool, Error> {
    for _ in 0..EXTRA_NAME_RETRIES {
        let (name, value_schema) = if pattern_properties.is_empty() {
            let name = if let Some(names_schema) = obj.get("propertyNames") {
                generate_property_name(ctx, names_schema, rng)?
            } else {
                let len = rng.random_range(4..=10);
                random_alphanumeric_string(rng, len)
            };
            let value_schema = match extra_values {
                ExtraValues::Schema(schema) => Some(*schema),
                _ => None,
            };
            (name, value_schema)
        } else {
            let idx = rng.random_range(0..pattern_properties.len());
            let (pattern, schema) = pattern_properties[idx];
            match xeger::generate_matching(pattern, rng) {
                Some(name) => (name, Some(schema)),
                None => continue,
            }
        };
        if result.contains_key(&name) {
            continue;
        }
        let value = match value_schema {
            Some(schema) => generate_value(ctx, schema, rng)?,
            None => generate_random_simple(rng),
        };
        result.insert(name, value);
        return Ok(true);
    }
    Ok(false)
}

/// Generate a property name from a `propertyNames` schema. Keys are always
/// strings, so `type: string` is implied when the schema does not state it.
fn generate_property_name(
    ctx: &Context,
    names_schema: &Value,
    rng: &mut impl Rng,
) -> Result<String, Error> {
    let mut merged = match names_schema {
        Value::Bool(true) => Map::new(),
        Value::Bool(false) => {
            return Err(Error::ConflictingConstraints {
                message: "propertyNames is false, no property name is valid".into(),
            });
        }
        Value::Object(o) => o.clone(),
        other => {
            return Err(Error::InvalidSchema {
                message: format!("propertyNames must be a boolean or schema, got {other}"),
            });
        }
    };
    merged
        .entry("type")
        .or_insert_with(|| Value::String("string".into()));
    match generate_value(ctx, &Value::Object(merged), rng)? {
        Value::String(s) => Ok(s),
        other => Err(Error::InvalidSchema {
            message: format!("propertyNames schema generated a non-string name: {other}"),
        }),
    }
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

    let prefix_items: &[Value] = match obj.get("prefixItems") {
        None => &[],
        Some(Value::Array(schemas)) => schemas,
        Some(other) => {
            return Err(Error::InvalidSchema {
                message: format!("prefixItems must be an array, got {other}"),
            });
        }
    };

    let min_items = obj.get("minItems").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let mut max_items = obj
        .get("maxItems")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    // `items: false` rejects every element beyond the prefix, so the tuple
    // length caps the count.
    if obj.get("items") == Some(&Value::Bool(false)) {
        if min_items > prefix_items.len() {
            return Err(Error::ConflictingConstraints {
                message: format!(
                    "minItems ({min_items}) exceeds the {} prefixItems and items is false",
                    prefix_items.len()
                ),
            });
        }
        max_items = Some(max_items.map_or(prefix_items.len(), |m| m.min(prefix_items.len())));
    }
    let max_items = max_items.unwrap_or_else(|| min_items.max(prefix_items.len()).max(3));

    if min_items > max_items {
        return Err(Error::ConflictingConstraints {
            message: format!("minItems ({min_items}) > maxItems ({max_items})"),
        });
    }

    // Arrays shorter than the prefix are valid but make poor examples, so
    // the count covers the full tuple whenever the bounds allow.
    let count_min = min_items.max(prefix_items.len().min(max_items));
    let count = rng.random_range(count_min..=max_items);

    let item_schema = obj.get("items").cloned().unwrap_or(Value::Bool(true));

    if obj
        .get("uniqueItems")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return generate_unique_array(
            &child_ctx,
            prefix_items,
            &item_schema,
            min_items,
            count,
            rng,
        );
    }

    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let slot_schema = prefix_items.get(i).unwrap_or(&item_schema);
        items.push(generate_value(&child_ctx, slot_schema, rng)?);
    }

    Ok(Value::Array(items))
}

/// Attempts per array slot before concluding the item space is exhausted.
const UNIQUE_ITEM_RETRIES: usize = 16;

/// Generate `count` distinct items, retrying each slot on collision.
/// Slots within the prefix draw from their positional `prefixItems` schema,
/// the rest from `item_schema`. When a slot cannot be filled, any length at
/// or above `min_items` is still valid, so the array is returned short;
/// below `min_items` the constraints are unsatisfiable.
fn generate_unique_array(
    ctx: &Context,
    prefix_items: &[Value],
    item_schema: &Value,
    min_items: usize,
    count: usize,
    rng: &mut impl Rng,
) -> Result<Value, Error> {
    let mut items: Vec<Value> = Vec::with_capacity(count);
    while items.len() < count {
        let slot_schema = prefix_items.get(items.len()).unwrap_or(item_schema);
        let mut filled = false;
        for _ in 0..UNIQUE_ITEM_RETRIES {
            let candidate = generate_value(ctx, slot_schema, rng)?;
            if !items.contains(&candidate) {
                items.push(candidate);
                filled = true;
                break;
            }
        }
        if !filled {
            if items.len() >= min_items {
                break;
            }
            return Err(Error::ConflictingConstraints {
                message: format!(
                    "uniqueItems: found only {} distinct items, minItems is {min_items}",
                    items.len()
                ),
            });
        }
    }
    Ok(Value::Array(items))
}

const COMPOSITION_KEYWORDS: [&str; 7] = ["$ref", "allOf", "anyOf", "oneOf", "if", "then", "else"];

const CONDITIONAL_KEYWORDS: [&str; 3] = ["if", "then", "else"];

/// Generate from a schema whose top level contains composition keywords.
///
/// Keywords at one schema level are conjunctive, so the sibling keywords,
/// the resolved `$ref` target, all `allOf` members, one picked branch each
/// from `anyOf` and `oneOf`, and one `if`/`then`/`else` branch are merged
/// into a single schema, which is then generated from. When a picked branch
/// cannot be merged or its merged schema fails to generate, the remaining
/// branch combinations are tried before giving up with the last error.
fn generate_composed(
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

    // Sibling keywords form the base schema.
    let mut base = Map::new();
    for (key, value) in obj {
        if !COMPOSITION_KEYWORDS.contains(&key.as_str()) {
            base.insert(key.clone(), value.clone());
        }
    }

    if let Some(reference) = obj.get("$ref") {
        let Value::String(reference) = reference else {
            return Err(Error::InvalidSchema {
                message: format!("$ref must be a string, got {reference}"),
            });
        };
        let resolved = resolve_ref(ctx.root, reference)?;
        merge_schema(&child_ctx, &mut base, resolved)?;
    }

    if let Some(members) = obj.get("allOf") {
        let Value::Array(members) = members else {
            return Err(Error::InvalidSchema {
                message: format!("allOf must be an array, got {members}"),
            });
        };
        for member in members {
            merge_schema(&child_ctx, &mut base, member)?;
        }
    }

    let any_of = branch_list(obj, "anyOf")?;
    let one_of = branch_list(obj, "oneOf")?;

    // A random branch is picked from each of anyOf/oneOf and from
    // if/then/else; on failure the remaining combinations are tried in
    // cyclic order. The conditional is the outermost dimension so every
    // anyOf/oneOf combination is exhausted before its other branch is tried.
    let (cond_branches, cond_start) = conditional_branches(&base, obj, rng);
    let any_count = any_of.map_or(1, <[Value]>::len);
    let one_count = one_of.map_or(1, <[Value]>::len);
    let any_start = any_of.map_or(0, |b| rng.random_range(0..b.len()));
    let one_start = one_of.map_or(0, |b| rng.random_range(0..b.len()));

    let mut last_err = None;
    for cond_offset in 0..cond_branches.len() {
        let cond_idx = (cond_start + cond_offset) % cond_branches.len();
        for one_offset in 0..one_count {
            for any_offset in 0..any_count {
                let mut merged = base.clone();
                let mut merge_result = Ok(());
                for addition in &cond_branches[cond_idx] {
                    merge_result = merge_schema(&child_ctx, &mut merged, addition);
                    if merge_result.is_err() {
                        break;
                    }
                }
                if merge_result.is_ok()
                    && let Some(branches) = any_of
                {
                    let idx = (any_start + any_offset) % branches.len();
                    merge_result = merge_schema(&child_ctx, &mut merged, &branches[idx]);
                }
                if merge_result.is_ok()
                    && let Some(branches) = one_of
                {
                    let idx = (one_start + one_offset) % branches.len();
                    merge_result = merge_schema(&child_ctx, &mut merged, &branches[idx]);
                }
                let result = merge_result
                    .and_then(|()| generate_value(&child_ctx, &Value::Object(merged), rng));
                match result {
                    Ok(value) => return Ok(value),
                    Err(err) => last_err = Some(err),
                }
            }
        }
    }
    Err(last_err.expect("at least one branch combination is always attempted"))
}

/// The `if`/`then`/`else` branch alternatives for [`generate_composed`]'s
/// combination loop: each branch is a list of schemas to merge, and the
/// second element of the return value is the branch to try first.
///
/// The then branch merges `if` and `then` — forcing the condition makes
/// `then` apply and `else` moot, so it is sound by construction. The else
/// branch merges `else` plus a negation of `if` when [`negate_if`] derives
/// one; only then is it eligible as the starting branch (picked by coin
/// flip, for variety). Without a derivable negation the else branch is
/// reached only after every then combination failed — the failure itself is
/// the signal that `if` cannot hold together with the rest of the schema,
/// so a base-conforming value is overwhelmingly likely to fail `if` and
/// fall under `else`. The corpus and unit tests validate this with a real
/// validator.
///
/// Without `if`, or with `if` but neither `then` nor `else`, the trio
/// asserts nothing: a single empty branch is returned.
fn conditional_branches(
    base: &Map<String, Value>,
    obj: &Map<String, Value>,
    rng: &mut impl Rng,
) -> (Vec<Vec<Value>>, usize) {
    let Some(if_schema) = obj.get("if") else {
        return (vec![Vec::new()], 0);
    };
    let then_schema = obj.get("then");
    let else_schema = obj.get("else");
    if then_schema.is_none() && else_schema.is_none() {
        return (vec![Vec::new()], 0);
    }

    let mut branches = Vec::new();
    if if_schema != &Value::Bool(false) {
        let mut additions = vec![if_schema.clone()];
        additions.extend(then_schema.cloned());
        branches.push(additions);
    }
    let mut else_is_start_candidate = false;
    if if_schema != &Value::Bool(true) {
        let mut additions = Vec::new();
        if if_schema == &Value::Bool(false) {
            // Nothing fails a false schema harder; the branch always applies.
            else_is_start_candidate = true;
        } else if let Some(negation) = negate_if(base, if_schema) {
            additions.push(negation);
            else_is_start_candidate = true;
        }
        additions.extend(else_schema.cloned());
        branches.push(additions);
    }

    let start = if branches.len() == 2 && else_is_start_candidate {
        rng.random_range(0..branches.len())
    } else {
        0
    };
    (branches, start)
}

/// Derive a schema that forces `¬if`, for discriminator-shaped conditions:
/// `if` constrains a single property to a `const`/`enum`, and the base
/// schema declares that property with an `enum` whose complement is
/// non-empty. The negation pins the property to the complement and requires
/// its presence — an absent property satisfies `if` vacuously, so presence
/// with an excluded value is what guarantees the condition fails. Any other
/// shape returns `None`.
fn negate_if(base: &Map<String, Value>, if_schema: &Value) -> Option<Value> {
    let if_obj = if_schema.as_object()?;
    if !if_obj
        .keys()
        .all(|k| matches!(k.as_str(), "properties" | "required"))
    {
        return None;
    }
    let props = if_obj.get("properties")?.as_object()?;
    if props.len() != 1 {
        return None;
    }
    let (name, prop_schema) = props.iter().next()?;
    let prop_schema = prop_schema.as_object()?;
    if !prop_schema.keys().all(|k| {
        matches!(
            k.as_str(),
            "const" | "enum" | "title" | "description" | "$comment"
        )
    }) {
        return None;
    }
    let excluded: Vec<&Value> = if let Some(value) = prop_schema.get("const") {
        vec![value]
    } else if let Some(Value::Array(variants)) = prop_schema.get("enum") {
        variants.iter().collect()
    } else {
        return None;
    };

    let base_variants = base
        .get("properties")?
        .as_object()?
        .get(name)?
        .as_object()?
        .get("enum")?
        .as_array()?;
    let complement: Vec<Value> = base_variants
        .iter()
        .filter(|v| !excluded.contains(v))
        .cloned()
        .collect();
    if complement.is_empty() {
        return None;
    }

    let mut prop = Map::new();
    prop.insert("enum".into(), Value::Array(complement));
    let mut properties = Map::new();
    properties.insert(name.clone(), Value::Object(prop));
    let mut negation = Map::new();
    negation.insert("properties".into(), Value::Object(properties));
    negation.insert(
        "required".into(),
        Value::Array(vec![Value::String(name.clone())]),
    );
    Some(Value::Object(negation))
}

fn branch_list<'a>(
    obj: &'a Map<String, Value>,
    keyword: &str,
) -> Result<Option<&'a [Value]>, Error> {
    match obj.get(keyword) {
        None => Ok(None),
        Some(Value::Array(branches)) if branches.is_empty() => Err(Error::InvalidSchema {
            message: format!("{keyword} must have at least one sub-schema"),
        }),
        Some(Value::Array(branches)) => Ok(Some(branches)),
        Some(other) => Err(Error::InvalidSchema {
            message: format!("{keyword} must be an array, got {other}"),
        }),
    }
}

/// Merge `addition` (a `$ref` target, `allOf` member, or picked
/// `anyOf`/`oneOf` branch) into `target`, keyword by keyword, following
/// conjunctive semantics. Errors when keywords cannot be combined.
fn merge_schema(
    ctx: &Context,
    target: &mut Map<String, Value>,
    addition: &Value,
) -> Result<(), Error> {
    let add_obj = match addition {
        Value::Bool(true) => return Ok(()),
        Value::Bool(false) => {
            return Err(Error::InvalidSchema {
                message: "false schema rejects all values".into(),
            });
        }
        Value::Object(o) => o,
        other => {
            return Err(Error::InvalidSchema {
                message: format!("schema must be a boolean or object, got {other}"),
            });
        }
    };

    // `if`/`then`/`else` are only meaningful as a group, so the trio moves
    // whole: onto the target's top level when those slots are free (the next
    // generate_value pass resolves it there), otherwise deferred into allOf
    // like a second anyOf/oneOf would be. A bare `then`/`else` without `if`
    // asserts nothing and is dropped.
    if add_obj.contains_key("if") {
        let trio: Map<String, Value> = CONDITIONAL_KEYWORDS
            .iter()
            .filter_map(|k| add_obj.get(*k).map(|v| ((*k).to_string(), v.clone())))
            .collect();
        if CONDITIONAL_KEYWORDS.iter().any(|k| target.contains_key(*k)) {
            push_all_of_member(target, Value::Object(trio));
        } else {
            for (key, value) in trio {
                target.insert(key, value);
            }
        }
    }

    for (key, value) in add_obj {
        if CONDITIONAL_KEYWORDS.contains(&key.as_str()) {
            continue;
        }
        merge_keyword(ctx, target, key, value)?;
    }
    Ok(())
}

fn merge_keyword(
    ctx: &Context,
    target: &mut Map<String, Value>,
    key: &str,
    value: &Value,
) -> Result<(), Error> {
    match key {
        // Nested composition: $ref resolves immediately (depth-guarded, so
        // cyclic references terminate); allOf members merge recursively.
        "$ref" => {
            if ctx.depth >= ctx.max_depth {
                return Err(Error::MaxDepthExceeded);
            }
            let child_ctx = Context {
                root: ctx.root,
                depth: ctx.depth + 1,
                max_depth: ctx.max_depth,
            };
            let Value::String(reference) = value else {
                return Err(Error::InvalidSchema {
                    message: format!("$ref must be a string, got {value}"),
                });
            };
            let resolved = resolve_ref(ctx.root, reference)?;
            merge_schema(&child_ctx, target, resolved)
        }
        "allOf" => {
            let Value::Array(members) = value else {
                return Err(Error::InvalidSchema {
                    message: format!("allOf must be an array, got {value}"),
                });
            };
            for member in members {
                merge_schema(ctx, target, member)?;
            }
            Ok(())
        }
        // anyOf/oneOf need a branch pick, so they stay unresolved: kept
        // top-level if that slot is free, otherwise deferred into allOf.
        // Either way the next generate_value pass picks them up.
        "anyOf" | "oneOf" => {
            if target.contains_key(key) {
                let mut wrapper = Map::new();
                wrapper.insert(key.into(), value.clone());
                push_all_of_member(target, Value::Object(wrapper));
            } else {
                target.insert(key.into(), value.clone());
            }
            Ok(())
        }
        "type" => merge_type(target, value),
        "properties" => merge_sub_schema_map(target, key, value),
        "required" => merge_required(target, value),
        "enum" => merge_enum(target, value),
        "const" => {
            // A const outside a sibling enum would win the generation
            // dispatch and silently violate the enum.
            if let Some(Value::Array(variants)) = target.get("enum")
                && !variants.contains(value)
            {
                return Err(Error::ConflictingConstraints {
                    message: format!(
                        "const {value} is not in enum {}",
                        Value::Array(variants.clone())
                    ),
                });
            }
            match target.get("const") {
                None => {
                    target.insert(key.into(), value.clone());
                    Ok(())
                }
                Some(existing) if existing == value => Ok(()),
                Some(existing) => Err(Error::ConflictingConstraints {
                    message: format!("conflicting const values: {existing} vs {value}"),
                }),
            }
        }
        "items" => merge_wrapping_all_of(target, key, value),
        "minimum" | "exclusiveMinimum" | "minLength" | "minItems" | "minProperties"
        | "minContains" => merge_bound(target, key, value, BoundKind::Lower),
        "maximum" | "exclusiveMaximum" | "maxLength" | "maxItems" | "maxProperties"
        | "maxContains" => merge_bound(target, key, value, BoundKind::Upper),
        // Annotations don't affect generation; the first one seen wins.
        "title" | "description" | "default" | "examples" | "$comment" | "$schema" | "$id"
        | "$defs" | "definitions" | "deprecated" | "readOnly" | "writeOnly" => {
            if !target.contains_key(key) {
                target.insert(key.into(), value.clone());
            }
            Ok(())
        }
        // Any other keyword merges only when the values agree; differing
        // values would need keyword-specific semantics, so fail honestly
        // rather than generate silently invalid output.
        _ => match target.get(key) {
            None => {
                target.insert(key.into(), value.clone());
                Ok(())
            }
            Some(existing) if existing == value => Ok(()),
            Some(existing) => Err(Error::ConflictingConstraints {
                message: format!(
                    "cannot merge differing values for `{key}`: {existing} vs {value}"
                ),
            }),
        },
    }
}

fn push_all_of_member(target: &mut Map<String, Value>, member: Value) {
    // `allOf` in additions is merged away immediately, so an `allOf` key on
    // `target` can only have been created here and is always an array.
    match target.get_mut("allOf") {
        Some(Value::Array(members)) => members.push(member),
        _ => {
            target.insert("allOf".into(), Value::Array(vec![member]));
        }
    }
}

fn merge_type(target: &mut Map<String, Value>, value: &Value) -> Result<(), Error> {
    fn type_set(value: &Value) -> Result<Vec<&str>, Error> {
        match value {
            Value::String(s) => Ok(vec![s.as_str()]),
            Value::Array(entries) => entries
                .iter()
                .map(|e| {
                    e.as_str().ok_or_else(|| Error::InvalidSchema {
                        message: format!("type array entries must be strings, got {e}"),
                    })
                })
                .collect(),
            other => Err(Error::InvalidSchema {
                message: format!("type must be a string or array of strings, got {other}"),
            }),
        }
    }

    let Some(existing) = target.get("type").cloned() else {
        target.insert("type".into(), value.clone());
        return Ok(());
    };

    let a = type_set(&existing)?;
    let b = type_set(value)?;
    let mut intersection: Vec<&str> = Vec::new();
    for t in &a {
        let common = if b.contains(t) {
            Some(*t)
        } else if (*t == "integer" && b.contains(&"number"))
            || (*t == "number" && b.contains(&"integer"))
        {
            // integer is a subtype of number
            Some("integer")
        } else {
            None
        };
        if let Some(common) = common
            && !intersection.contains(&common)
        {
            intersection.push(common);
        }
    }

    match intersection.as_slice() {
        [] => Err(Error::ConflictingConstraints {
            message: format!("no common type between {existing} and {value}"),
        }),
        [single] => {
            target.insert("type".into(), Value::String((*single).into()));
            Ok(())
        }
        several => {
            let entries = several.iter().map(|t| Value::String((*t).into())).collect();
            target.insert("type".into(), Value::Array(entries));
            Ok(())
        }
    }
}

/// Merge a map of named sub-schemas (`properties`): entries new to `target`
/// are inserted; entries present on both sides combine as
/// `{"allOf": [existing, addition]}`, which a later pass merges.
fn merge_sub_schema_map(
    target: &mut Map<String, Value>,
    key: &str,
    value: &Value,
) -> Result<(), Error> {
    let Value::Object(additions) = value else {
        return Err(Error::InvalidSchema {
            message: format!("{key} must be an object, got {value}"),
        });
    };

    match target.get_mut(key) {
        None => {
            target.insert(key.into(), value.clone());
            Ok(())
        }
        Some(Value::Object(existing)) => {
            for (name, addition) in additions {
                match existing.get(name) {
                    None => {
                        existing.insert(name.clone(), addition.clone());
                    }
                    Some(current) if current == addition => {}
                    Some(current) => {
                        let combined = Value::Array(vec![current.clone(), addition.clone()]);
                        let mut wrapper = Map::new();
                        wrapper.insert("allOf".into(), combined);
                        existing.insert(name.clone(), Value::Object(wrapper));
                    }
                }
            }
            Ok(())
        }
        Some(other) => Err(Error::InvalidSchema {
            message: format!("{key} must be an object, got {other}"),
        }),
    }
}

/// Merge a single sub-schema keyword (`items`) by wrapping both sides in
/// `{"allOf": [existing, addition]}` when they differ.
fn merge_wrapping_all_of(
    target: &mut Map<String, Value>,
    key: &str,
    value: &Value,
) -> Result<(), Error> {
    match target.get(key) {
        None => {
            target.insert(key.into(), value.clone());
        }
        Some(existing) if existing == value => {}
        Some(existing) => {
            let combined = Value::Array(vec![existing.clone(), value.clone()]);
            let mut wrapper = Map::new();
            wrapper.insert("allOf".into(), combined);
            target.insert(key.into(), Value::Object(wrapper));
        }
    }
    Ok(())
}

fn merge_required(target: &mut Map<String, Value>, value: &Value) -> Result<(), Error> {
    let Value::Array(additions) = value else {
        return Err(Error::InvalidSchema {
            message: format!("required must be an array, got {value}"),
        });
    };

    match target.get_mut("required") {
        None => {
            target.insert("required".into(), value.clone());
            Ok(())
        }
        Some(Value::Array(existing)) => {
            for addition in additions {
                if !existing.contains(addition) {
                    existing.push(addition.clone());
                }
            }
            Ok(())
        }
        Some(other) => Err(Error::InvalidSchema {
            message: format!("required must be an array, got {other}"),
        }),
    }
}

fn merge_enum(target: &mut Map<String, Value>, value: &Value) -> Result<(), Error> {
    let Value::Array(additions) = value else {
        return Err(Error::InvalidSchema {
            message: format!("enum must be an array, got {value}"),
        });
    };

    let merged = match target.get("enum").cloned() {
        None => additions.clone(),
        Some(Value::Array(existing)) => {
            let intersection: Vec<Value> = existing
                .into_iter()
                .filter(|variant| additions.contains(variant))
                .collect();
            if intersection.is_empty() {
                return Err(Error::ConflictingConstraints {
                    message: "enum intersection is empty".into(),
                });
            }
            intersection
        }
        Some(existing) => {
            return Err(Error::InvalidSchema {
                message: format!("enum must be an array, got {existing}"),
            });
        }
    };

    // A sibling const outside the enum would win the generation dispatch
    // and silently violate it.
    if let Some(const_value) = target.get("const")
        && !merged.contains(const_value)
    {
        return Err(Error::ConflictingConstraints {
            message: format!(
                "const {const_value} is not in enum {}",
                Value::Array(merged)
            ),
        });
    }

    target.insert("enum".into(), Value::Array(merged));
    Ok(())
}

enum BoundKind {
    Lower,
    Upper,
}

/// Conjunction of two bounds keeps the stricter one: the larger lower bound
/// or the smaller upper bound.
fn merge_bound(
    target: &mut Map<String, Value>,
    key: &str,
    value: &Value,
    kind: BoundKind,
) -> Result<(), Error> {
    let Some(addition) = value.as_f64() else {
        return Err(Error::InvalidSchema {
            message: format!("{key} must be a number, got {value}"),
        });
    };

    let existing_is_stricter = match target.get(key).and_then(serde_json::Value::as_f64) {
        Some(existing) => match kind {
            BoundKind::Lower => existing >= addition,
            BoundKind::Upper => existing <= addition,
        },
        None => false,
    };
    if !existing_is_stricter {
        target.insert(key.into(), value.clone());
    }
    Ok(())
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
