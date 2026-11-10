pub struct First(pub u32, pub super::InlineArraySchema);
impl First {
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
        schema: super::schema_facade::schema::Schema,
        prefix_len: u32,
        g: &super::schema_facade::r#gen::SchemaGenerator,
    ) -> ::std::result::Result<
        (super::schema_facade::schema::Schema, bool),
        ::std::string::String,
    > {
        use super::schema_facade::r#gen::SchemaGenerator;
        use super::schema_facade::schema::{
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
impl super::schema_facade::JsonSchema for First {
    fn schema_name() -> ::std::string::String {
        "First".to_string()
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
        g: &mut super::schema_facade::r#gen::SchemaGenerator,
    ) -> super::schema_facade::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        super::schema_facade::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::Metadata {
                    title: Some("First".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                super::schema_facade::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(
                        super::schema_facade::schema::InstanceType::Array,
                    ),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::ArrayValidation {
                    items: Some(super::schema_facade::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            super::schema_facade::schema::Schema::Bool(true),
                        ),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            subschemas: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::SubschemaValidation {
                    all_of: Some(
                        vec![
                            { let remainder = < super::InlineArraySchema as
                            super::schema_facade::JsonSchema > ::json_schema(g);
                            First::__typespace_prefix_array_schema(remainder, 1u32, g,)
                            .unwrap_or_else(| error |
                            panic!("cannot generate schema for `{}`: {error}", < Self as
                            super::schema_facade::JsonSchema > ::schema_name(),)).0 },
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
pub struct FirstAgain(pub ::std::string::String, pub u32, pub super::InlineArraySchema);
impl super::schema_facade::JsonSchema for FirstAgain {
    fn schema_name() -> ::std::string::String {
        "FirstAgain".to_string()
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
        g: &mut super::schema_facade::r#gen::SchemaGenerator,
    ) -> super::schema_facade::schema::Schema {
        let fields = [
            g.subschema_for::<::std::string::String>(),
            g.subschema_for::<u32>(),
        ]
            .into_iter()
            .collect();
        super::schema_facade::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::Metadata {
                    title: Some("FirstAgain".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                super::schema_facade::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(
                        super::schema_facade::schema::InstanceType::Array,
                    ),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::ArrayValidation {
                    items: Some(super::schema_facade::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            super::schema_facade::schema::Schema::Bool(true),
                        ),
                    ),
                    min_items: Some(2u32),
                    ..::std::default::Default::default()
                }),
            ),
            subschemas: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::SubschemaValidation {
                    all_of: Some(
                        vec![
                            { let remainder = < super::InlineArraySchema as
                            super::schema_facade::JsonSchema > ::json_schema(g);
                            First::__typespace_prefix_array_schema(remainder, 2u32, g,)
                            .unwrap_or_else(| error |
                            panic!("cannot generate schema for `{}`: {error}", < Self as
                            super::schema_facade::JsonSchema > ::schema_name(),)).0 },
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
pub struct Second(pub u32, pub super::InlineArraySchema);
impl Second {
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
        schema: super::schema_facade::schema::Schema,
        prefix_len: u32,
        g: &super::schema_facade::r#gen::SchemaGenerator,
    ) -> ::std::result::Result<
        (super::schema_facade::schema::Schema, bool),
        ::std::string::String,
    > {
        use super::schema_facade::r#gen::SchemaGenerator;
        use super::schema_facade::schema::{
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
impl super::schema_facade::JsonSchema for Second {
    fn schema_name() -> ::std::string::String {
        "Second".to_string()
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
        g: &mut super::schema_facade::r#gen::SchemaGenerator,
    ) -> super::schema_facade::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        super::schema_facade::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::Metadata {
                    title: Some("Second".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                super::schema_facade::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(
                        super::schema_facade::schema::InstanceType::Array,
                    ),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::ArrayValidation {
                    items: Some(super::schema_facade::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            super::schema_facade::schema::Schema::Bool(true),
                        ),
                    ),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            subschemas: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::SubschemaValidation {
                    all_of: Some(
                        vec![
                            { let remainder = < super::InlineArraySchema as
                            super::schema_facade::JsonSchema > ::json_schema(g);
                            Second::__typespace_prefix_array_schema(remainder, 1u32, g,)
                            .unwrap_or_else(| error |
                            panic!("cannot generate schema for `{}`: {error}", < Self as
                            super::schema_facade::JsonSchema > ::schema_name(),)).0 },
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
pub struct SecondAgain(pub bool, pub bool, pub bool, pub super::InlineArraySchema);
impl super::schema_facade::JsonSchema for SecondAgain {
    fn schema_name() -> ::std::string::String {
        "SecondAgain".to_string()
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
        g: &mut super::schema_facade::r#gen::SchemaGenerator,
    ) -> super::schema_facade::schema::Schema {
        let fields = [
            g.subschema_for::<bool>(),
            g.subschema_for::<bool>(),
            g.subschema_for::<bool>(),
        ]
            .into_iter()
            .collect();
        super::schema_facade::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::Metadata {
                    title: Some("SecondAgain".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                super::schema_facade::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(
                        super::schema_facade::schema::InstanceType::Array,
                    ),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::ArrayValidation {
                    items: Some(super::schema_facade::schema::SingleOrVec::Vec(fields)),
                    additional_items: Some(
                        ::std::boxed::Box::new(
                            super::schema_facade::schema::Schema::Bool(true),
                        ),
                    ),
                    min_items: Some(3u32),
                    ..::std::default::Default::default()
                }),
            ),
            subschemas: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::SubschemaValidation {
                    all_of: Some(
                        vec![
                            { let remainder = < super::InlineArraySchema as
                            super::schema_facade::JsonSchema > ::json_schema(g);
                            Second::__typespace_prefix_array_schema(remainder, 3u32, g,)
                            .unwrap_or_else(| error |
                            panic!("cannot generate schema for `{}`: {error}", < Self as
                            super::schema_facade::JsonSchema > ::schema_name(),)).0 },
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
#[allow(non_camel_case_types)]
pub struct __typespace_prefix_array_schema(pub u32);
impl super::schema_facade::JsonSchema for __typespace_prefix_array_schema {
    fn schema_name() -> ::std::string::String {
        "__typespace_prefix_array_schema".to_string()
    }
    fn json_schema(
        g: &mut super::schema_facade::r#gen::SchemaGenerator,
    ) -> super::schema_facade::schema::Schema {
        let fields = [g.subschema_for::<u32>()].into_iter().collect();
        super::schema_facade::schema::SchemaObject {
            metadata: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::Metadata {
                    title: Some("__typespace_prefix_array_schema".to_string()),
                    ..::std::default::Default::default()
                }),
            ),
            instance_type: Some(
                super::schema_facade::schema::SingleOrVec::Single(
                    ::std::boxed::Box::new(
                        super::schema_facade::schema::InstanceType::Array,
                    ),
                ),
            ),
            array: Some(
                ::std::boxed::Box::new(super::schema_facade::schema::ArrayValidation {
                    items: Some(super::schema_facade::schema::SingleOrVec::Vec(fields)),
                    max_items: Some(1u32),
                    min_items: Some(1u32),
                    ..::std::default::Default::default()
                }),
            ),
            ..::std::default::Default::default()
        }
            .into()
    }
}
