#[derive(::serde::Serialize)]
#[serde(transparent)]
pub struct BoundedArrays(::std::vec::Vec<::std::vec::Vec<::std::string::String>>);
impl ::std::ops::Deref for BoundedArrays {
    type Target = ::std::vec::Vec<::std::vec::Vec<::std::string::String>>;
    fn deref(&self) -> &::std::vec::Vec<::std::vec::Vec<::std::string::String>> {
        &self.0
    }
}
impl ::std::convert::From<BoundedArrays>
for ::std::vec::Vec<::std::vec::Vec<::std::string::String>> {
    fn from(value: BoundedArrays) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<::std::vec::Vec<::std::vec::Vec<::std::string::String>>>
for BoundedArrays {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::vec::Vec<::std::vec::Vec<::std::string::String>>,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.len() > 4usize {
            return Err("more than 4 items".into());
        }
        if value.len() < 2usize {
            return Err("fewer than 2 items".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for BoundedArrays {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(
                <::std::vec::Vec<
                    ::std::vec::Vec<::std::string::String>,
                >>::deserialize(deserializer)?,
            )
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for BoundedArrays {
    fn schema_name() -> ::std::string::String {
        "BoundedArrays".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <::std::vec::Vec<
            ::std::vec::Vec<::std::string::String>,
        > as ::schemars::JsonSchema>::json_schema(g)
            .into_object();
        schema.array().min_items = ::std::option::Option::Some(2u32);
        schema.array().max_items = ::std::option::Option::Some(4u32);
        schema.into()
    }
}
pub struct BoundedInnerTail(
    pub ::std::string::String,
    pub ::std::vec::Vec<BoundedStrings>,
);
impl ::serde::Serialize for BoundedInnerTail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for BoundedInnerTail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = BoundedInnerTail;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(BoundedInnerTail(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for BoundedInnerTail {
    fn schema_name() -> ::std::string::String {
        "BoundedInnerTail".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<::std::string::String>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("BoundedInnerTail".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(g.subschema_for::<BoundedStrings>()),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct BoundedOuterTail(pub ::std::string::String, pub BoundedArrays);
impl ::serde::Serialize for BoundedOuterTail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for BoundedOuterTail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = BoundedOuterTail;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(BoundedOuterTail(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for BoundedOuterTail {
    fn schema_name() -> ::std::string::String {
        "BoundedOuterTail".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<::std::string::String>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("BoundedOuterTail".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::vec::Vec<::std::string::String>>(),
                        ),
                    ),
                    max_items: Some(5u32),
                    min_items: Some(3u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct BoundedStringTail(pub ::std::string::String, pub BoundedStrings);
impl ::serde::Serialize for BoundedStringTail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for BoundedStringTail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = BoundedStringTail;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(BoundedStringTail(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for BoundedStringTail {
    fn schema_name() -> ::std::string::String {
        "BoundedStringTail".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<::std::string::String>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("BoundedStringTail".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::string::String>(),
                        ),
                    ),
                    max_items: Some(5u32),
                    min_items: Some(3u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
#[derive(::serde::Serialize)]
#[serde(transparent)]
pub struct BoundedStrings(::std::vec::Vec<::std::string::String>);
impl ::std::ops::Deref for BoundedStrings {
    type Target = ::std::vec::Vec<::std::string::String>;
    fn deref(&self) -> &::std::vec::Vec<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<BoundedStrings> for ::std::vec::Vec<::std::string::String> {
    fn from(value: BoundedStrings) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<::std::vec::Vec<::std::string::String>> for BoundedStrings {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::vec::Vec<::std::string::String>,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.len() > 4usize {
            return Err("more than 4 items".into());
        }
        if value.len() < 2usize {
            return Err("fewer than 2 items".into());
        }
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for BoundedStrings {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(
                <::std::vec::Vec<::std::string::String>>::deserialize(deserializer)?,
            )
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for BoundedStrings {
    fn schema_name() -> ::std::string::String {
        "BoundedStrings".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let mut schema = <::std::vec::Vec<
            ::std::string::String,
        > as ::schemars::JsonSchema>::json_schema(g)
            .into_object();
        schema.array().min_items = ::std::option::Option::Some(2u32);
        schema.array().max_items = ::std::option::Option::Some(4u32);
        schema.into()
    }
}
#[derive(::serde::Serialize)]
#[serde(transparent)]
pub struct ContainsStrings(::std::vec::Vec<::std::string::String>);
impl ::std::ops::Deref for ContainsStrings {
    type Target = ::std::vec::Vec<::std::string::String>;
    fn deref(&self) -> &::std::vec::Vec<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<ContainsStrings> for ::std::vec::Vec<::std::string::String> {
    fn from(value: ContainsStrings) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<::std::vec::Vec<::std::string::String>>
for ContainsStrings {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::vec::Vec<::std::string::String>,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        #[::jsonschema::validator(
            schema = "{\"contains\":{\"const\":\"needle\"}}",
            draft = Draft7
        )]
        struct Schema;
        let json = ::serde_json::to_value(&value).map_err(|e| e.to_string())?;
        Schema::validate(&json).map_err(|e| e.to_string())?;
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for ContainsStrings {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(
                <::std::vec::Vec<::std::string::String>>::deserialize(deserializer)?,
            )
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for ContainsStrings {
    fn schema_name() -> ::std::string::String {
        "ContainsStrings".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let inner = g.subschema_for::<::std::vec::Vec<::std::string::String>>();
        let constraint = ::serde_json::from_str::<
            ::schemars::schema::Schema,
        >("{\"contains\":{\"const\":\"needle\"}}")
            .unwrap();
        ::schemars::schema::Schema::Object(::schemars::schema::SchemaObject {
            subschemas: ::std::option::Option::Some(
                ::std::boxed::Box::new(::schemars::schema::SubschemaValidation {
                    all_of: ::std::option::Option::Some(::std::vec![inner, constraint,]),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        })
    }
}
pub struct ContainsTail(pub ::std::string::String, pub ContainsStrings);
impl ContainsTail {
    /// Translate a suffix schema into constraints on its enclosing array.
    ///
    /// The caller must enforce the array type, validate the positional
    /// prefix, and require at least `prefix_len` elements. Only constraints
    /// on the suffix as a whole are shifted; element schemas and references
    /// within them are preserved.
    ///
    /// The second result is true for an exact translation. False means
    /// suffix uniqueness or a boolean combination was relaxed, so the
    /// result must not be negated or used to count matching alternatives.
    ///
    /// # Errors
    ///
    /// For a nonempty prefix, sequence-level references must resolve to
    /// available Schemars definitions and must not form a reference cycle.
    /// Referenced false is also rejected: Schemars uses false while building
    /// a definition and cannot distinguish it from a finished false target.
    /// Intentional false schemas must be provided inline.
    fn __typespace_prefix_array_schema(
        schema: ::schemars::schema::Schema,
        prefix_len: u32,
        g: &::schemars::r#gen::SchemaGenerator,
    ) -> ::std::result::Result<
        (::schemars::schema::Schema, bool),
        ::std::string::String,
    > {
        use ::schemars::r#gen::SchemaGenerator;
        use ::schemars::schema::{
            InstanceType, Schema, SchemaObject, SingleOrVec, SubschemaValidation,
        };
        fn try_prefix_array_schema(
            schema: Schema,
            offset: u32,
            g: &SchemaGenerator,
            seen: &mut ::std::collections::BTreeSet<::std::string::String>,
        ) -> ::std::result::Result<(Schema, bool), ::std::string::String> {
            if offset == 0 {
                return Ok((schema, true));
            }
            let Schema::Object(mut schema) = schema else {
                return Ok((schema, true));
            };
            if let Some(reference) = &schema.reference {
                return prefix_array_reference(reference, offset, g, seen);
            }
            if !schema.has_type(InstanceType::Array) {
                return Ok((Schema::Bool(false), true));
            }
            schema.instance_type = None;
            schema.metadata = None;
            let mut exact = true;
            if let Some(subschemas) = schema.subschemas.as_mut() {
                exact = prefix_subschemas(subschemas, offset, g, seen)?;
            }
            prefix_array_literals(&mut schema, offset);
            exact &= prefix_array_validation(&mut schema, offset);
            Ok((schema.into(), exact))
        }
        /// Resolve a sequence-level reference and translate its target. The visited
        /// set tracks the active reference chain, so reused targets remain valid.
        fn prefix_array_reference(
            reference: &str,
            offset: u32,
            g: &SchemaGenerator,
            seen: &mut ::std::collections::BTreeSet<::std::string::String>,
        ) -> ::std::result::Result<(Schema, bool), ::std::string::String> {
            if !seen.insert(reference.to_string()) {
                return Err(
                    format!("sequence schema reference cycle through `{reference}`"),
                );
            }
            let result = match g.dereference(&Schema::new_ref(reference.to_string())) {
                None => {
                    Err(
                        format!(
                            "sequence schema reference `{reference}` does not resolve to an \
                         available Schemars definition; only registered targets under \
                         `{}` are supported and must be ready before the remainder schema returns",
                            g.settings().definitions_path,
                        ),
                    )
                }
                Some(Schema::Bool(false)) => {
                    Err(
                        format!(
                            "sequence schema reference `{reference}` resolves to false, which \
                         may still be in progress; provide an intentional false schema inline",
                        ),
                    )
                }
                Some(target) => try_prefix_array_schema(target.clone(), offset, g, seen),
            };
            seen.remove(reference);
            result
        }
        /// Translate logical branches that constrain the suffix itself. Relaxed
        /// branches remain safe under allOf/anyOf; not, oneOf and the condition of
        /// if/then/else require exact predicates to avoid rejecting valid arrays.
        fn prefix_subschemas(
            subschemas: &mut SubschemaValidation,
            offset: u32,
            g: &SchemaGenerator,
            seen: &mut ::std::collections::BTreeSet<::std::string::String>,
        ) -> ::std::result::Result<bool, ::std::string::String> {
            let mut exact = true;
            for list in [&mut subschemas.all_of, &mut subschemas.any_of]
                .into_iter()
                .flatten()
            {
                for sub in list {
                    let (translated, complete) = try_prefix_array_schema(
                        sub.clone(),
                        offset,
                        g,
                        seen,
                    )?;
                    *sub = translated;
                    exact &= complete;
                }
            }
            if let Some(alternatives) = subschemas.one_of.take() {
                let alternatives = alternatives
                    .into_iter()
                    .map(|sub| try_prefix_array_schema(sub, offset, g, seen))
                    .collect::<::std::result::Result<::std::vec::Vec<_>, _>>()?;
                if alternatives.iter().all(|(_, complete)| *complete) {
                    subschemas.one_of = Some(
                        alternatives.into_iter().map(|(sub, _)| sub).collect(),
                    );
                } else {
                    exact = false;
                }
            }
            if let Some(sub) = subschemas.not.take() {
                let (translated, complete) = try_prefix_array_schema(
                    *sub,
                    offset,
                    g,
                    seen,
                )?;
                if complete {
                    subschemas.not = Some(::std::boxed::Box::new(translated));
                } else {
                    exact = false;
                }
            }
            if let Some(condition) = subschemas.if_schema.take() {
                let (condition, complete) = try_prefix_array_schema(
                    *condition,
                    offset,
                    g,
                    seen,
                )?;
                if complete {
                    subschemas.if_schema = Some(::std::boxed::Box::new(condition));
                    for sub in [&mut subschemas.then_schema, &mut subschemas.else_schema]
                        .into_iter()
                        .flatten()
                    {
                        let (translated, complete) = try_prefix_array_schema(
                            (**sub).clone(),
                            offset,
                            g,
                            seen,
                        )?;
                        **sub = translated;
                        exact &= complete;
                    }
                } else {
                    subschemas.then_schema = None;
                    subschemas.else_schema = None;
                    exact = false;
                }
            }
            Ok(exact)
        }
        /// Compare const/enum values with the suffix, leaving the prefix free.
        /// Non-array literals cannot match a suffix and contribute no alternative.
        fn prefix_array_literals(schema: &mut SchemaObject, offset: u32) {
            for values in [
                schema.const_value.take().map(|v| vec![v]),
                schema.enum_values.take(),
            ]
                .into_iter()
                .flatten()
            {
                let alternatives = values
                    .into_iter()
                    .filter_map(|value| {
                        let items = value.as_array()?;
                        let len = offset.saturating_add(items.len() as u32);
                        let mut fields = vec![Schema::Bool(true); offset as usize];
                        fields
                            .extend(
                                items
                                    .iter()
                                    .map(|value| {
                                        SchemaObject {
                                            const_value: Some(value.clone()),
                                            ..::std::default::Default::default()
                                        }
                                            .into()
                                    }),
                            );
                        let mut alternative = SchemaObject::default();
                        let array = alternative.array();
                        array.items = Some(SingleOrVec::Vec(fields));
                        array.min_items = Some(len);
                        array.max_items = Some(len);
                        Some(alternative.into())
                    })
                    .collect::<::std::vec::Vec<Schema>>();
                let constraint = if alternatives.is_empty() {
                    Schema::Bool(false)
                } else {
                    let mut constraint = SchemaObject::default();
                    constraint.subschemas().any_of = Some(alternatives);
                    constraint.into()
                };
                schema
                    .subschemas()
                    .all_of
                    .get_or_insert_with(::std::vec::Vec::new)
                    .push(constraint);
            }
        }
        /// Shift array positions and lengths without translating item schemas.
        /// The result is false when suffix uniqueness had to be relaxed. Contains
        /// is expressed separately so a matching prefix item cannot satisfy it.
        fn prefix_array_validation(schema: &mut SchemaObject, offset: u32) -> bool {
            let mut exact = true;
            let mut contains = None;
            if let Some(array) = schema.array.as_mut() {
                match array.items.take() {
                    Some(SingleOrVec::Single(item)) => {
                        array.items = Some(
                            SingleOrVec::Vec(vec![Schema::Bool(true); offset as usize]),
                        );
                        array.additional_items = Some(item);
                    }
                    Some(SingleOrVec::Vec(items)) => {
                        let mut fields = vec![Schema::Bool(true); offset as usize];
                        fields.extend(items);
                        array.items = Some(SingleOrVec::Vec(fields));
                    }
                    None => array.additional_items = None,
                }
                array.min_items = array.min_items.map(|n| n.saturating_add(offset));
                array.max_items = array.max_items.map(|n| n.saturating_add(offset));
                contains = array.contains.take();
                if array.unique_items.take() == Some(true) {
                    exact = false;
                }
            }
            if let Some(contains) = contains {
                let mut not_contains = SchemaObject::default();
                not_contains.subschemas().not = Some(contains);
                let mut no_match = SchemaObject::default();
                let array = no_match.array();
                array.items = Some(
                    SingleOrVec::Vec(vec![Schema::Bool(true); offset as usize]),
                );
                array.additional_items = Some(
                    ::std::boxed::Box::new(not_contains.into()),
                );
                let mut constraint = SchemaObject::default();
                constraint.subschemas().not = Some(
                    ::std::boxed::Box::new(no_match.into()),
                );
                schema
                    .subschemas()
                    .all_of
                    .get_or_insert_with(::std::vec::Vec::new)
                    .push(constraint.into());
            }
            exact
        }
        try_prefix_array_schema(
            schema,
            prefix_len,
            g,
            &mut ::std::collections::BTreeSet::new(),
        )
    }
}
impl ::serde::Serialize for ContainsTail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for ContainsTail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = ContainsTail;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(ContainsTail(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for ContainsTail {
    fn schema_name() -> ::std::string::String {
        "ContainsTail".to_string()
    }
    /// # Panics
    ///
    /// When shifting a remainder schema past a nonempty prefix,
    /// panics if a sequence-level reference is unavailable in the
    /// schema generator, forms a cycle, or targets a false
    /// definition. Schemars also uses
    /// false for in-progress definitions, so intentional false
    /// schemas must be supplied inline. Referenced targets must be
    /// ready before the remainder's JsonSchema method returns.
    /// Only references managed by the schema generator are supported.
    /// References within element schemas are preserved unchanged.
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<::std::string::String>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("ContainsTail".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::string::String>(),
                        ),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            subschemas: Some(
                ::std::boxed::Box::new(::schemars::schema::SubschemaValidation {
                    all_of: Some(
                        vec![
                            ContainsTail::__typespace_prefix_array_schema(::serde_json::from_str("{\"contains\":{\"const\":\"needle\"}}")
                            .unwrap(), 1u32, g,).unwrap_or_else(| error |
                            panic!("cannot generate schema for `{}`: {error}", < Self as
                            ::schemars::JsonSchema > ::schema_name(),)).0,
                        ],
                    ),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct NestedTail(
    pub ::std::string::String,
    pub ::std::vec::Vec<::std::vec::Vec<::std::string::String>>,
);
impl ::serde::Serialize for NestedTail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for NestedTail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = NestedTail;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(NestedTail(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for NestedTail {
    fn schema_name() -> ::std::string::String {
        "NestedTail".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<::std::string::String>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("NestedTail".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::vec::Vec<::std::string::String>>(),
                        ),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
#[derive(::serde::Deserialize, ::serde::Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub children: ::std::vec::Vec<Node>,
}
#[derive(::serde::Serialize)]
#[serde(transparent)]
pub struct PatternedString(::std::string::String);
impl ::std::ops::Deref for PatternedString {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<PatternedString> for ::std::string::String {
    fn from(value: PatternedString) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<::std::string::String> for PatternedString {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        #[::jsonschema::validator(schema = "{\"pattern\":\"^a\"}", draft = Draft7)]
        struct Schema;
        let json = ::serde_json::to_value(&value).map_err(|e| e.to_string())?;
        Schema::validate(&json).map_err(|e| e.to_string())?;
        Ok(Self(value))
    }
}
impl<'de> ::serde::Deserialize<'de> for PatternedString {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<::std::string::String>::deserialize(deserializer)?)
            .map_err(|e| { <D::Error as ::serde::de::Error>::custom(e.to_string()) })
    }
}
impl ::schemars::JsonSchema for PatternedString {
    fn schema_name() -> ::std::string::String {
        "PatternedString".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let inner = g.subschema_for::<::std::string::String>();
        let constraint = ::serde_json::from_str::<
            ::schemars::schema::Schema,
        >("{\"pattern\":\"^a\"}")
            .unwrap();
        ::schemars::schema::Schema::Object(::schemars::schema::SchemaObject {
            subschemas: ::std::option::Option::Some(
                ::std::boxed::Box::new(::schemars::schema::SubschemaValidation {
                    all_of: ::std::option::Option::Some(::std::vec![inner, constraint,]),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        })
    }
}
pub struct PatternedTail(
    pub ::std::string::String,
    pub ::std::vec::Vec<PatternedString>,
);
impl ::serde::Serialize for PatternedTail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for PatternedTail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = PatternedTail;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(PatternedTail(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for PatternedTail {
    fn schema_name() -> ::std::string::String {
        "PatternedTail".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<::std::string::String>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("PatternedTail".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(g.subschema_for::<PatternedString>()),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct PlainTail(
    pub ::std::string::String,
    pub ::std::vec::Vec<::std::string::String>,
);
impl ::serde::Serialize for PlainTail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for PlainTail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = PlainTail;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(PlainTail(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for PlainTail {
    fn schema_name() -> ::std::string::String {
        "PlainTail".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<::std::string::String>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("PlainTail".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            g.subschema_for::<::std::string::String>(),
                        ),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
pub struct RecursiveTail(pub ::std::string::String, pub ::std::vec::Vec<Node>);
impl ::serde::Serialize for RecursiveTail {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use ::serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.0)?;
        self.1.serialize(::json_serde::FlattenedSequenceSerializer::new(&mut seq))?;
        seq.end()
    }
}
impl<'de> ::serde::Deserialize<'de> for RecursiveTail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> ::serde::de::Visitor<'de> for Visitor {
            type Value = RecursiveTail;
            fn expecting(
                &self,
                formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                formatter.write_str("a sequence")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::SeqAccess<'de>,
            {
                let field_0 = seq
                    .next_element()?
                    .ok_or_else(|| ::serde::de::Error::invalid_length(
                        0usize,
                        &"a tuple of size 1 or more",
                    ))?;
                let rest = ::serde::Deserialize::deserialize(
                    ::json_serde::FlattenedSequenceDeserializer::new(&mut seq),
                )?;
                Ok(RecursiveTail(field_0, rest))
            }
        }
        deserializer.deserialize_seq(Visitor)
    }
}
impl ::schemars::JsonSchema for RecursiveTail {
    fn schema_name() -> ::std::string::String {
        "RecursiveTail".to_string()
    }
    fn json_schema(
        g: &mut ::schemars::r#gen::SchemaGenerator,
    ) -> ::schemars::schema::Schema {
        let fields = [g.subschema_for::<::std::string::String>()].into_iter().collect();
        ::schemars::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(::schemars::schema::Metadata {
                    title: Some("RecursiveTail".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                ::schemars::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(::schemars::schema::InstanceType::Array),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(::schemars::schema::ArrayValidation {
                    items: Some(::schemars::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(g.subschema_for::<Node>()),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
/// Error types.
pub mod error {
    /// Error from a `TryFrom` or `FromStr` implementation.
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(
            &self,
            f: &mut ::std::fmt::Formatter<'_>,
        ) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
