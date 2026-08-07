use thiserror::Error;

/// Errors that can occur during JSON value generation.
#[derive(Debug, Error)]
pub enum Error {
    /// The schema specifies a `type` value that this library does not handle.
    #[error("unsupported type: {type_name}")]
    UnsupportedType { type_name: String },

    /// The schema is structurally invalid (e.g. a non-object, non-boolean
    /// value, an empty `enum` array, or a `false` boolean schema).
    #[error("invalid schema: {message}")]
    InvalidSchema { message: String },

    /// Recursive generation exceeded the maximum nesting depth (default 16).
    #[error("maximum depth exceeded")]
    MaxDepthExceeded,

    /// A `$ref` pointer could not be resolved against the root schema.
    /// This includes references to missing definitions and external URIs.
    #[error("$ref not found: {reference}")]
    RefNotFound { reference: String },

    /// Constraints are impossible to satisfy, either within one schema
    /// (e.g. `minLength` > `maxLength`) or when merging composition
    /// keywords (e.g. conflicting `type` values across `allOf` members).
    #[error("conflicting constraints: {message}")]
    ConflictingConstraints { message: String },
}
