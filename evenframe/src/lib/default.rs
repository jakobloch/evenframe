use super::schemasync::*;
use crate::types::{FieldType, StructConfig, StructField, TaggedUnion, VariantData};
use convert_case::{Case, Casing};
use rand::{rng, seq::IndexedRandom};
use std::collections::HashMap;

pub fn field_type_to_default_value(
    field_type: &FieldType,
    structs: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
) -> String {
    match field_type {
        FieldType::String | FieldType::Char => r#""""#.to_string(),
        FieldType::Bool => "false".to_string(),
        FieldType::DateTime => r#""2024-01-01T00:00:00Z""#.to_string(),
        FieldType::Duration => "0".to_string(), // nanoseconds
        FieldType::Timezone => r#""UTC""#.to_string(), // IANA timezone string
        FieldType::Unit => "undefined".to_string(),
        FieldType::Decimal => r#""0""#.to_string(),
        FieldType::OrderedFloat(inner) => field_type_to_default_value(inner, structs, enums),
        FieldType::F32
        | FieldType::F64
        | FieldType::I8
        | FieldType::I16
        | FieldType::I32
        | FieldType::I64
        | FieldType::I128
        | FieldType::Isize
        | FieldType::U8
        | FieldType::U16
        | FieldType::U32
        | FieldType::U64
        | FieldType::U128
        | FieldType::Usize => "0".to_string(),
        FieldType::EvenframeRecordId => "''".to_string(),
        FieldType::Tuple(inner_types) => {
            let tuple_defaults: Vec<String> = inner_types
                .iter()
                .map(|ty| field_type_to_default_value(ty, structs, enums))
                .collect();
            format!("[{}]", tuple_defaults.join(", "))
        }
        FieldType::Struct(fields) => {
            let fields_str = fields
                .iter()
                .map(|(name, ftype)| {
                    format!(
                        "{}: {}",
                        name.to_case(Case::Camel),
                        field_type_to_default_value(ftype, structs, enums)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {} }}", fields_str)
        }
        FieldType::Option(_) => {
            // You can decide whether to produce `null` or `undefined` or something else.
            // For TypeScript, `null` is a more direct representation of "no value."
            "null".to_string()
        }
        FieldType::Vec(_) => {
            // Returns an empty array as the default
            // but recursively if you wanted an "example entry" you could do:
            //   format!("[{}]", field_type_to_default_value(inner, structs, enums))
            "[]".to_string()
        }
        FieldType::HashMap(_, _) => {
            // Return an empty object as default
            "{}".to_string()
        }

        FieldType::BTreeMap(_, _) => {
            // Return an empty object as default
            "{}".to_string()
        }

        FieldType::RecordLink(_) => {
            // Could produce "null" or "0" depending on your usage pattern.
            // We'll pick "null" for "unlinked".
            "''".to_string()
        }
        FieldType::Other(name) => {
            // 1) If this is an enum, pick a random variant.
            // 2) Otherwise if it matches a known table, produce a default object for that table.
            // 3) If neither, fall back to 'undefined'.

            // First check for an enum of this name
            if let Some(enum_schema) = enums.values().find(|e| e.enum_name == *name) {
                let mut rng = rng();
                if let Some(chosen_variant) = enum_schema.variants.choose(&mut rng) {
                    // If the variant has data, generate a default for it.
                    if let Some(variant_data) = &chosen_variant.data {
                        let variant_data_field_type = match variant_data {
                            VariantData::InlineStruct(inline_struct) => {
                                &FieldType::Other(inline_struct.name.clone())
                            }
                            VariantData::DataStructureRef(field_type) => field_type,
                        };
                        let data_default =
                            field_type_to_default_value(variant_data_field_type, structs, enums);
                        return format!("{}", data_default);
                    } else {
                        // A variant without data
                        return format!("'{}'", chosen_variant.name);
                    }
                } else {
                    // If no variants, fallback to undefined
                    return "undefined".to_string();
                }
            }

            if let Some(struct_config) = structs
                .values()
                .find(|sc| sc.name.to_case(Case::Pascal) == name.to_case(Case::Pascal))
            {
                // We treat this similarly to a struct:
                let fields_str = struct_config
                    .fields
                    .iter()
                    .map(|table_field| {
                        format!(
                            "{}: {}",
                            table_field.field_name.to_case(Case::Camel),
                            field_type_to_default_value(&table_field.field_type, structs, enums)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", fields_str)
            } else {
                // Not an enum or known table
                "undefined".to_string()
            }
        }
    }
}

/// Generate default values for SurrealDB queries (CREATE/UPDATE statements)
pub fn field_type_to_surql_default(
    field_name: &String,
    table_name: &String,
    field_type: &FieldType,
    enums: &HashMap<String, TaggedUnion>,
    app_structs: &HashMap<String, StructConfig>,
    persistable_structs: &HashMap<String, TableConfig>,
) -> String {
    match field_type {
        FieldType::String | FieldType::Char => "\'\'".to_string(),
        FieldType::Bool => "false".to_string(),
        FieldType::DateTime => {
            // Generate current timestamp in SurrealDB datetime format
            "d'2024-01-01T00:00:00Z'".to_string()
        }
        FieldType::Duration => {
            // Default duration of 0 nanoseconds
            "duration::from::nanos(0)".to_string()
        }
        FieldType::Timezone => {
            // Default timezone UTC
            "'UTC'".to_string()
        }
        FieldType::Unit => "NULL".to_string(),
        FieldType::Decimal => "0.00dec".to_string(),
        FieldType::OrderedFloat(_inner) => "0.0f".to_string(), // OrderedFloat is treated as float
        FieldType::F32 | FieldType::F64 => "0.0f".to_string(),
        FieldType::I8
        | FieldType::I16
        | FieldType::I32
        | FieldType::I64
        | FieldType::I128
        | FieldType::Isize
        | FieldType::U8
        | FieldType::U16
        | FieldType::U32
        | FieldType::U64
        | FieldType::U128
        | FieldType::Usize => "0".to_string(),
        FieldType::EvenframeRecordId => {
            // For id fields, let SurrealDB auto-generate
            if field_name == "id" {
                "NONE".to_string()
            } else {
                "NONE".to_string()
            }
        }
        FieldType::Tuple(inner_types) => {
            let tuple_defaults: Vec<String> = inner_types
                .iter()
                .map(|ty| {
                    field_type_to_surql_default(
                        field_name,
                        table_name,
                        ty,
                        enums,
                        app_structs,
                        persistable_structs,
                    )
                })
                .collect();
            format!("[{}]", tuple_defaults.join(", "))
        }
        FieldType::Struct(fields) => {
            let fields_str = fields
                .iter()
                .map(|(name, ftype)| {
                    format!(
                        "{}: {}",
                        name.to_case(Case::Snake), // SurrealDB typically uses snake_case
                        field_type_to_surql_default(
                            field_name,
                            table_name,
                            ftype,
                            enums,
                            app_structs,
                            persistable_structs
                        )
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {} }}", fields_str)
        }
        FieldType::Option(_) => "NULL".to_string(),
        FieldType::Vec(_) => "[]".to_string(),
        FieldType::HashMap(_, _) | FieldType::BTreeMap(_, _) => "{}".to_string(),
        FieldType::RecordLink(_) => "NULL".to_string(),
        FieldType::Other(name) => {
            // Check if it's an enum
            if let Some(enum_schema) = enums.values().find(|e| e.enum_name == *name) {
                let chosen_variant = &enum_schema.variants[0];
                if let Some(variant_data) = &chosen_variant.data {
                    let variant_data_field_type = match variant_data {
                        VariantData::InlineStruct(inline_struct) => {
                            &FieldType::Other(inline_struct.name.clone())
                        }
                        VariantData::DataStructureRef(field_type) => field_type,
                    };
                    // For enum with data, return the data's default
                    field_type_to_surql_default(
                        field_name,
                        table_name,
                        variant_data_field_type,
                        enums,
                        app_structs,
                        persistable_structs,
                    )
                } else {
                    // For simple enum variant
                    format!("'{}'", chosen_variant.name)
                }
            }
            // Check if it's a struct
            else if let Some(struct_config) = app_structs
                .values()
                .find(|sc| sc.name.to_case(Case::Pascal) == name.to_case(Case::Pascal))
            {
                let fields_str = struct_config
                    .fields
                    .iter()
                    .map(|table_field| {
                        format!(
                            "{}: {}",
                            table_field.field_name.to_case(Case::Snake),
                            field_type_to_surql_default(
                                &table_field.field_name,
                                table_name,
                                &table_field.field_type,
                                enums,
                                app_structs,
                                persistable_structs
                            )
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", fields_str)
            }
            // Check if it's a persistable struct (table reference)
            else if persistable_structs.get(name).is_some() {
                // For record links to other tables, default to NULL
                "NULL".to_string()
            } else {
                "NULL".to_string()
            }
        }
    }
}

pub fn field_type_to_surreal_type(
    field_name: &String,
    table_name: &String,
    field_type: &FieldType,
    enums: &HashMap<String, TaggedUnion>,
    app_structs: &HashMap<String, StructConfig>,
    persistable_structs: &HashMap<String, TableConfig>,
) -> (String, bool, Option<String>) {
    match field_type {
        FieldType::String | FieldType::Char => ("string".to_string(), false, None),
        FieldType::Bool => ("bool".to_string(), false, None),
        FieldType::DateTime => ("datetime".to_string(), false, None),
        FieldType::Duration => ("duration".to_string(), false, None),
        FieldType::Timezone => ("string".to_string(), false, None),
        FieldType::Decimal => ("decimal".to_string(), false, None),
        FieldType::OrderedFloat(_inner) => ("float".to_string(), false, None), // OrderedFloat is treated as float
        FieldType::F32 | FieldType::F64 => ("float".to_string(), false, None),
        FieldType::I8
        | FieldType::I16
        | FieldType::I32
        | FieldType::I64
        | FieldType::I128
        | FieldType::Isize
        | FieldType::U8
        | FieldType::U16
        | FieldType::U32
        | FieldType::U64
        | FieldType::U128
        | FieldType::Usize => ("int".to_string(), false, None),
        FieldType::EvenframeRecordId => {
            let type_str = if field_name == "id" {
                format!("record<{}>", table_name)
            } else {
                "record<any>".to_string()
            };
            (type_str, false, None)
        }
        FieldType::Unit => ("any".to_string(), false, None),
        FieldType::HashMap(_key, value) => {
            let (value_type, _, _) = field_type_to_surreal_type(
                field_name,
                table_name,
                value,
                enums,
                app_structs,
                persistable_structs,
            );
            ("object".to_string(), true, Some(value_type))
        }
        FieldType::BTreeMap(_key, value) => {
            let (value_type, _, _) = field_type_to_surreal_type(
                field_name,
                table_name,
                value,
                enums,
                app_structs,
                persistable_structs,
            );
            ("object".to_string(), true, Some(value_type))
        }
        FieldType::RecordLink(inner) => {
            let (inner_type, needs_wildcard, wildcard_type) = field_type_to_surreal_type(
                field_name,
                table_name,
                inner,
                enums,
                app_structs,
                persistable_structs,
            );
            (inner_type, needs_wildcard, wildcard_type)
        }
        FieldType::Other(name) => {
            // If this type name is defined as an enum, output its union literal.
            if let Some(enum_def) = enums.get(name) {
                let variants: Vec<String> = enum_def
                    .variants
                    .iter()
                    .map(|v| {
                        if let Some(variant_data) = &v.data {
                            let variant_data_field_type = match variant_data {
                                VariantData::InlineStruct(inline_struct) => {
                                    &FieldType::Other(inline_struct.name.clone())
                                }
                                VariantData::DataStructureRef(field_type) => field_type,
                            };
                            let (variant_type, _, _) = field_type_to_surreal_type(
                                field_name,
                                table_name,
                                variant_data_field_type,
                                &enums,
                                &app_structs,
                                &persistable_structs,
                            );
                            variant_type
                        } else {
                            format!("{}", v.name)
                        }
                    })
                    .collect();
                (variants.join(" | "), false, None)
            } else if let Some(app_struct) = app_structs.get(name) {
                let field_defs: Vec<String> = app_struct
                    .fields
                    .iter()
                    .map(|f: &StructField| {
                        let (field_type, _, _) = field_type_to_surreal_type(
                            &f.field_name,
                            table_name,
                            &f.field_type,
                            enums,
                            app_structs,
                            persistable_structs,
                        );
                        format!("{}: {}", f.field_name, field_type)
                    })
                    .collect();

                (format!("{{ {} }}", field_defs.join(", ")), false, None)
            } else if persistable_structs.get(name).is_some() {
                (
                    format!("record<{}>", name.to_case(Case::Snake)),
                    false,
                    None,
                )
            } else {
                (name.clone(), false, None)
            }
        }
        FieldType::Option(inner) => {
            let (inner_type, needs_wildcard, wildcard_type) = field_type_to_surreal_type(
                field_name,
                table_name,
                inner,
                enums,
                app_structs,
                persistable_structs,
            );
            (
                format!("null | {}", inner_type),
                needs_wildcard,
                wildcard_type,
            )
        }
        FieldType::Vec(inner) => {
            let (inner_type, _, _) = field_type_to_surreal_type(
                field_name,
                table_name,
                inner,
                enums,
                app_structs,
                persistable_structs,
            );
            (format!("array<{}>", inner_type), false, None)
        }
        FieldType::Tuple(inner_types) => {
            let inner: Vec<String> = inner_types
                .iter()
                .map(|t| {
                    let (inner_type, _, _) = field_type_to_surreal_type(
                        field_name,
                        table_name,
                        t,
                        enums,
                        app_structs,
                        persistable_structs,
                    );
                    inner_type
                })
                .collect();
            // (SurrealDB does not have a dedicated tuple type so we wrap it as an array)
            (format!("array<{}>", inner.join(", ")), false, None)
        }
        FieldType::Struct(fields) => {
            let field_defs: Vec<String> = fields
                .iter()
                .map(|(name, t)| {
                    let (field_type, _, _) = field_type_to_surreal_type(
                        field_name,
                        table_name,
                        t,
                        enums,
                        app_structs,
                        persistable_structs,
                    );
                    format!("{}: {}", name, field_type)
                })
                .collect();
            (format!("{{ {} }}", field_defs.join(", ")), false, None)
        }
    }
}
