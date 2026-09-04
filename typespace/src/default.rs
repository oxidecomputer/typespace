// Copyright 2026 Oxide Computer Company

use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    build::{self, StructPropertySerde, StructPropertyState, Type},
    error::Error,
};

pub(crate) fn check_default<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    value: &serde_json::Value,
    id: Id,
) -> Result<(), Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let ty = types.get(&id).unwrap();
    default_impl(types, Mode::Check, id, value).map(|_| ())
}

pub(crate) fn generate_default<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    value: &serde_json::Value,
    id: Id,
) -> TokenStream
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    default_impl(types, Mode::Generate, id, value)
        .expect("an error should not be possible post-validation")
        .expect("a value should be generated with Mode::Generate")
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Mode {
    Check,
    Generate,
}
impl Mode {
    fn then<F>(&self, generate: F) -> Option<TokenStream>
    where
        F: FnOnce() -> TokenStream,
    {
        match self {
            Mode::Check => None,
            Mode::Generate => Some(generate()),
        }
    }
}

fn default_impl<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    id: Id,
    value: &serde_json::Value,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let ty = types.get(&id).unwrap();
    match ty {
        Type::Enum(enum_info) => default_impl_enum(types, mode, enum_info, value, id),
        Type::Struct(struct_info) => default_impl_struct(types, mode, struct_info, value, id),
        Type::UnitStruct(unit_struct) => {
            default_impl_unit_struct(types, mode, unit_struct, value, id)
        }
        Type::TupleStruct(tuple_struct) => {
            default_impl_tuple_struct(types, mode, tuple_struct, value, id)
        }
        Type::NewtypeStruct(newtype_struct) => todo!(),
        Type::TypeAlias(type_alias) => todo!(),
        Type::Native(native) => todo!(),

        // Note that this doesn't mean "optiona"; it means that null is also
        // permitted as a value.
        Type::Option(type_id) => {
            if value.is_null() {
                Ok(mode.then(|| quote! { None }))
            } else {
                let inner = default_impl(types, mode, type_id.clone(), value)?;
                Ok(inner.map(|inner| quote! { Some(#inner) }))
            }
        }
        Type::Box(type_id) => {
            let inner = default_impl(types, mode, type_id.clone(), value)?;
            // TODO 9/4/2026
            // We need Settings to know what to render here...
            Ok(inner.map(|inner| quote! { Box::new(#inner) }))
        }
        Type::Vec(_) => todo!(),
        Type::Map(_, _) => todo!(),
        Type::Set(_) => todo!(),
        Type::Array(_, _) => todo!(),
        Type::Tuple(items) => todo!(),

        Type::Unit => {
            if value.is_null() {
                Ok(mode.then(|| quote! { () }))
            } else {
                Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "unit type default value must be null".to_string(),
                })
            }
        }

        Type::Boolean => {
            if let Some(v) = value.as_bool() {
                Ok(mode.then(|| quote! { #v }))
            } else {
                Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "expected a boolean".to_string(),
                })
            }
        }
        Type::Integer(itype) => {
            let Some(n) = value.as_number() else {
                return Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "expected an integer".to_string(),
                });
            };

            // TODO figure out non-zero types
            if itype.starts_with("xxx") {
                let type_path = syn::parse_str::<syn::TypePath>(itype).unwrap();
                let num = proc_macro2::Literal::from_str(value.to_string().as_str()).unwrap();

                Ok(mode.then(|| {
                    quote! {
                        #type_path::new(#num).unwrap()
                    }
                }))
            } else {
                let val = match proc_macro2::Literal::from_str(&format!("{}_{}", value, itype)) {
                    Ok(v) => v,
                    Err(_) => unreachable!(),
                };
                Ok(mode.then(|| TokenStream::from(proc_macro2::TokenTree::from(val))))
            }
        }
        Type::Float(_) => todo!(),
        Type::String => {
            if let Some(s) = value.as_str() {
                Ok(mode.then(|| quote! { #s.to_string()}))
            } else {
                Err(Error::InvalidDefault {
                    value: value.clone(),
                    id: id.clone(),
                    reason: "expected a string".to_string(),
                })
            }
        }
        Type::JsonValue => Ok(mode.then(|| json_value_to_token_stream(value))),
        Type::Never => todo!(),
    }
}

fn default_impl_struct<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    struct_info: &build::Struct<Id>,
    value: &serde_json::Value,
    id: Id,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let map = value.as_object().ok_or_else(|| Error::InvalidDefault {
        value: value.clone(),
        id: id.clone(),
        reason: "expected JSON object".to_string(),
    })?;

    // Examine local (non-flattened) properties first, then descend into
    // flattened properties with the value. If deny_unknown_fields is set,
    // things may be weird.

    let (local_properties, flattened_properties): (Vec<_>, Vec<_>) = struct_info
        .properties
        .iter()
        .partition(|prop| !matches!(prop.json_name, StructPropertySerde::Flatten));

    let local_properties = local_properties
        .into_iter()
        .map(|prop| {
            let name = match &prop.json_name {
                StructPropertySerde::None => prop.rust_name.as_str(),
                StructPropertySerde::Rename(rename) => rename.as_str(),
                StructPropertySerde::Flatten => unreachable!(),
            };
            (name, prop)
        })
        .collect::<BTreeMap<_, _>>();

    let keys = local_properties
        .keys()
        .map(|k| *k)
        .chain(map.keys().map(|k| k.as_str()))
        .collect::<BTreeSet<_>>();

    let mut extra_keys = Vec::new();
    let mut rendered_properties = Vec::new();

    for key in keys {
        let prop_info = local_properties.get(key);
        let prop_value = map.get(key);

        match (prop_info, prop_value) {
            (None, None) => unreachable!(),
            (None, Some(_)) => extra_keys.push(key),
            (Some(prop_info), None) => {
                if prop_info.state != StructPropertyState::Optional {
                    return Err(Error::InvalidDefault {
                        value: value.clone(),
                        id: id.clone(),
                        reason: format!("missing required property {}", key),
                    });
                }
            }
            (Some(prop_info), Some(prop_value)) => {
                let prop_id = &prop_info.type_id;
                let xxx =
                    default_impl(types, mode, prop_id.clone(), prop_value)?.map(|prop_value| {
                        let prop_ident = format_ident!("{}", prop_info.rust_name);
                        quote! {
                            #prop_ident: #prop_value
                        }
                    });

                if mode == Mode::Generate {
                    rendered_properties
                        .push(xxx.expect("a value should be generated with Mode::Generate"));
                }
            }
        }
    }

    assert!(extra_keys.is_empty());
    assert!(flattened_properties.is_empty());

    Ok(mode.then(|| {
        let struct_ident = format_ident!("{}", struct_info.common.name.as_ref().unwrap());
        quote! {
            #struct_ident {
                #( #rendered_properties, )*
            }
        }
    }))
}

fn default_impl_enum<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    enum_info: &build::Enum<Id>,
    value: &serde_json::Value,
    id: Id,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    let type_name = enum_info.common.name.as_ref().unwrap();
    match enum_info.tag_type.as_ref().unwrap() {
        build::EnumTagType::External => {
            default_impl_enum_external(types, mode, type_name, enum_info, value, id)
        }
        build::EnumTagType::Internal { tag } => {
            default_impl_enum_internal(types, mode, enum_info, tag, value, id)
        }
        build::EnumTagType::Adjacent { tag, content } => {
            default_impl_enum_adjacent(types, mode, enum_info, tag, content, value, id)
        }
        build::EnumTagType::Untagged => {
            default_impl_enum_untagged(types, mode, enum_info, value, id)
        }
    }
}

fn default_impl_enum_external<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    type_name: &str,
    enum_info: &build::Enum<Id>,
    value: &serde_json::Value,
    id: Id,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    if let Some(variant_name) = value.as_str() {
        let variant = enum_info
            .variants
            .iter()
            .find(|variant| {
                // TODO 9/4/2026
                // This is kind of wrong; we're going to need to represent the json
                // serialization name and use that.
                let name = variant.rename.as_ref().unwrap_or(&variant.rust_name);
                variant_name == name
            })
            .ok_or_else(|| Error::InvalidDefault {
                value: value.clone(),
                id: id.clone(),
                reason: format!("variant {} not found in enum", variant_name),
            })?;

        let var_ident = format_ident!("{}", variant.rust_name);
        let type_ident = format_ident!("{}", type_name);
        Ok(mode.then(|| quote! { #type_ident::#var_ident }))
    } else {
        todo!()
    }
}

fn default_impl_enum_internal<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    enum_info: &build::Enum<Id>,
    tag: &str,
    value: &serde_json::Value,
    id: Id,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    todo!()
}

fn default_impl_enum_adjacent<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    enum_info: &build::Enum<Id>,
    tag: &str,
    content: &str,
    value: &serde_json::Value,
    id: Id,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    todo!()
}

fn default_impl_enum_untagged<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    enum_info: &build::Enum<Id>,
    value: &serde_json::Value,
    id: Id,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    todo!()
}

fn default_impl_tuple_struct<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    tuple_struct: &build::TupleStruct<Id>,
    value: &serde_json::Value,
    id: Id,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    todo!()
}

fn default_impl_unit_struct<Id>(
    types: &BTreeMap<Id, Type<Id>>,
    mode: Mode,
    unit_struct: &build::UnitStruct,
    value: &serde_json::Value,
    id: Id,
) -> Result<Option<TokenStream>, Error<Id>>
where
    Id: Clone + Ord + std::fmt::Debug + std::fmt::Display,
{
    if value == &unit_struct.repr {
        Ok(mode.then(|| json_value_to_token_stream(value)))
    } else {
        Err(Error::InvalidDefault {
            value: value.clone(),
            id,
            reason: format!("unit struct default value must be {}", unit_struct.repr),
        })
    }
}

fn json_value_to_token_stream(value: &serde_json::Value) -> TokenStream {
    match value {
        serde_json::Value::Null => todo!(),
        serde_json::Value::Bool(_) => todo!(),
        serde_json::Value::Number(number) => todo!(),
        serde_json::Value::String(_) => todo!(),
        serde_json::Value::Array(values) => todo!(),
        serde_json::Value::Object(map) => todo!(),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_null() {
        let mut types = BTreeMap::new();
        let id = "unit".to_string();
        types.insert(id.clone(), Type::<String>::Unit);

        let value = serde_json::json! { null };

        check_default(&types, &value, id.clone()).unwrap();
        let code = generate_default(&types, &value, id);

        assert_eq!(code.to_string(), "()");
    }
}
