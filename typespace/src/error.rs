use crate::TypespaceTrait;

/// Errors that arise from an invalid type graph provided to the
/// [`TypespaceBuilder`](crate::TypespaceBuilder).
#[derive(Debug, thiserror::Error)]
pub enum TypespaceError<Id>
where
    Id: std::fmt::Debug + std::fmt::Display,
{
    /// A type was inserted with an ID already in use by another type.
    #[error("a type with the id `{type_id}` has already been inserted")]
    DuplicateTypeId {
        /// The ID of the duplicate insertion.
        type_id: Id,
    },

    /// A type refers to a child type ID for which no type was inserted.
    #[error(
        "the type with id `{type_id}` references the id `{child_id}` \
         for which there is no type"
    )]
    UnknownTypeId {
        /// The ID of the type containing the dangling reference.
        type_id: Id,
        /// The referenced ID for which no type exists.
        child_id: Id,
    },

    /// A floating-point type is required to implement traits that
    /// floating-point types cannot implement, for example because it is
    /// used--directly or transitively--as a map key.
    #[error(
        "the float type `{name}` with id `{type_id}` is required to \
         implement {missing:?}, which floating-point types cannot implement"
    )]
    FloatTraits {
        /// The ID of the offending float type.
        type_id: Id,
        /// The Rust name of the float type (e.g. `f64`).
        name: String,
        /// The required traits that floating-point types cannot implement.
        missing: Vec<TypespaceTrait>,
    },

    /// A JSON value type is required to implement traits that
    /// `serde_json::Value` does not implement, for example because it is
    /// used--directly or transitively--as a map key.
    #[error(
        "the JSON value type with id `{type_id}` is required to implement \
         {missing:?}, which `serde_json::Value` does not implement"
    )]
    JsonValueTraits {
        /// The ID of the offending JSON value type.
        type_id: Id,
        /// The required traits that `serde_json::Value` does not implement.
        missing: Vec<TypespaceTrait>,
    },
}
