mod field_type;

use crate::{
    default::field_type_to_surql_default,
    format::Format,
    schemasync::{table::generate_assert_from_validators, DefineConfig, EdgeConfig, TableConfig},
    validator::Validator,
    wrappers::EvenframeRecordId,
};
use convert_case::{Case, Casing};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub use crate::types::field_type::FieldType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaggedUnion {
    pub enum_name: String,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum RecordLink<T> {
    Id(EvenframeRecordId),
    Object(T),
}

impl<'de, T> Deserialize<'de> for RecordLink<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<RecordLink<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the input into a serde_json::Value so we can attempt multiple deserializations.
        let value = Value::deserialize(deserializer)?;

        // Try to deserialize into EvenframeRecordId (the Id variant)
        let id_attempt = EvenframeRecordId::deserialize(value.clone());
        // Try to deserialize into T (the Object variant)
        let obj_attempt = T::deserialize(value);

        match (id_attempt, obj_attempt) {
            // If the Id variant works and the Object variant fails, choose Id.
            (Ok(id), Err(_)) => Ok(RecordLink::Id(id)),
            // If the Object variant works and the Id variant fails, choose Object.
            (Err(_), Ok(obj)) => Ok(RecordLink::Object(obj)),
            // If both variants succeed, it's ambiguous.
            (Ok(_), Ok(_)) => Err(serde::de::Error::custom(
                "Ambiguous value: it matches both RecordLink::Id and RecordLink::Object",
            )),
            // If both attempts fail, combine their error messages.
            (Err(err_id), Err(err_obj)) => Err(serde::de::Error::custom(format!(
                "Failed to deserialize RecordLink. Tried Id variant: {}. Tried Object variant: {}.",
                err_id, err_obj
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub data: Option<VariantData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariantData {
    InlineStruct(StructConfig),
    DataStructureRef(FieldType),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructField {
    pub field_name: String,
    pub field_type: FieldType,
    pub edge_config: Option<EdgeConfig>,
    pub define_config: Option<DefineConfig>,
    pub format: Option<Format>,
    pub validators: Vec<Validator>,
    pub always_regenerate: bool,
}

impl StructField {
    pub fn generate_define_statement(
        &self,
        enums: HashMap<String, TaggedUnion>,
        app_structs: HashMap<String, StructConfig>,
        persistable_structs: HashMap<String, TableConfig>,
        table_name: &String,
    ) -> String {
        use std::collections::HashSet;
        
        // Wrapper function that initializes visited_types
        fn field_type_to_surql_type(
            field_name: &String,
            table_name: &String,
            field_type: &FieldType,
            enums: &HashMap<String, TaggedUnion>,
            app_structs: &HashMap<String, StructConfig>,
            persistable_structs: &HashMap<String, TableConfig>,
        ) -> (String, bool, Option<String>) {
            let mut visited_types = HashSet::new();
            field_type_to_surql_type_impl(
                field_name,
                table_name,
                field_type,
                enums,
                app_structs,
                persistable_structs,
                &mut visited_types,
            )
        }
        
        // Helper to convert a FieldType to its SurrealDB type string.
        // Returns (type_string, needs_wildcard_field, wildcard_type)
        fn field_type_to_surql_type_impl(
            field_name: &String,
            table_name: &String,
            field_type: &FieldType,
            enums: &HashMap<String, TaggedUnion>,
            app_structs: &HashMap<String, StructConfig>,
            persistable_structs: &HashMap<String, TableConfig>,
            visited_types: &mut HashSet<String>,
        ) -> (String, bool, Option<String>) {
            match field_type {
                FieldType::String | FieldType::Char => ("string".to_string(), false, None),
                FieldType::Bool => ("bool".to_string(), false, None),
                FieldType::DateTime => ("datetime".to_string(), false, None),
                FieldType::Duration => ("duration".to_string(), false, None),
                FieldType::Timezone => ("string".to_string(), false, None),
                FieldType::Decimal => ("decimal".to_string(), false, None),
                FieldType::F32 | FieldType::F64 => ("float".to_string(), false, None),
                FieldType::OrderedFloat(_inner) => {
                    tracing::debug!("Converting OrderedFloat to float for field {}", field_name);
                    ("float".to_string(), false, None)
                }
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
                    let (value_type, _, _) = field_type_to_surql_type_impl(
                        field_name,
                        table_name,
                        value,
                        enums,
                        app_structs,
                        persistable_structs,
                        visited_types,
                    );
                    ("object".to_string(), true, Some(value_type))
                }
                FieldType::BTreeMap(_key, value) => {
                    let (value_type, _, _) = field_type_to_surql_type_impl(
                        field_name,
                        table_name,
                        value,
                        enums,
                        app_structs,
                        persistable_structs,
                        visited_types,
                    );
                    ("object".to_string(), true, Some(value_type))
                }
                FieldType::RecordLink(inner) => {
                    // RecordLink should always point to a record type
                    match inner.as_ref() {
                        FieldType::Other(type_name) => {
                            // Convert type name to snake_case for table name
                            (
                                format!("record<{}>", type_name.to_case(Case::Snake)),
                                false,
                                None,
                            )
                        }
                        _ => {
                            // For other inner types, treat as before
                            let (inner_type, needs_wildcard, wildcard_type) =
                                field_type_to_surql_type(
                                    field_name,
                                    table_name,
                                    inner,
                                    enums,
                                    app_structs,
                                    persistable_structs,
                                );
                            (inner_type, needs_wildcard, wildcard_type)
                        }
                    }
                }
                FieldType::Other(name) => {
                    // If this type name is defined as an enum, output its union literal.
                    if let Some(enum_def) = enums.get(name) {
                        let variants: Vec<String> = enum_def
                            .variants
                            .iter()
                            .map(|variant| {
                                if let Some(variant_data) = &variant.data {
                                    let variant_data_field_type = match variant_data {
                                        VariantData::InlineStruct(inline_struct) => {
                                            &FieldType::Other(inline_struct.struct_name.clone())
                                        }
                                        VariantData::DataStructureRef(field_type) => field_type,
                                    };
                                    let (variant_type, _, _) = field_type_to_surql_type(
                                        field_name,
                                        table_name,
                                        variant_data_field_type,
                                        enums,
                                        app_structs,
                                        persistable_structs,
                                    );
                                    variant_type
                                } else {
                                    format!("\"{}\"", variant.name)
                                }
                            })
                            .collect();
                        (variants.join(" | "), false, None)
                    } else if let Some(app_struct) = app_structs.get(name) {
                        tracing::debug!("Processing app_struct {} for field {}", name, field_name);

                        // Check if we've already visited this type to prevent infinite recursion
                        if visited_types.contains(name) {
                            tracing::debug!(
                                "Detected circular reference to type {}, using 'object' type",
                                name
                            );
                            return ("object".to_string(), false, None);
                        }
                        
                        // Add this type to visited set
                        visited_types.insert(name.clone());

                        let field_defs: Vec<String> = app_struct
                            .fields
                            .iter()
                            .map(|f: &StructField| {
                                // Also check for recursive fields within the struct
                                if let FieldType::Other(ref field_type_name) = f.field_type {
                                    if field_type_name == name {
                                        tracing::debug!(
                                            "  Struct field {} is self-referential, using 'object'",
                                            f.field_name
                                        );
                                        return format!("{}: object", f.field_name);
                                    }
                                }

                                let (field_type, _, _) = field_type_to_surql_type(
                                    &f.field_name,
                                    table_name,
                                    &f.field_type,
                                    enums,
                                    app_structs,
                                    persistable_structs,
                                );
                                tracing::debug!(
                                    "  Struct field {}: {:?} -> {}",
                                    f.field_name,
                                    &f.field_type,
                                    field_type
                                );
                                format!("{}: {}", f.field_name, field_type)
                            })
                            .collect();

                        (format!("{{ {} }}", field_defs.join(", ")), false, None)
                    } else if persistable_structs
                        .get(&name.to_case(Case::Snake))
                        .is_some()
                    {
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
                    let (inner_type, needs_wildcard, wildcard_type) = field_type_to_surql_type(
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
                    let (inner_type, _, _) = field_type_to_surql_type(
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
                            let (inner_type, _, _) = field_type_to_surql_type(
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
                            let (field_type, _, _) = field_type_to_surql_type(
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

        // Begin building the statement.
        let mut stmt = format!(
            "DEFINE FIELD OVERWRITE {} ON TABLE {}",
            self.field_name, table_name
        );

        // Determine the type clause and check if we need a wildcard field
        let (type_str, needs_wildcard, wildcard_type) = if let Some(ref def) = self.define_config {
            if def.should_skip {
                ("".to_string(), false, None)
            } else if let Some(ref data_type) = def.data_type {
                tracing::warn!(
                    "Field {} has data_type override: {}",
                    self.field_name,
                    data_type
                );
                (data_type.clone(), false, None)
            } else {
                field_type_to_surql_type(
                    &self.field_name,
                    table_name,
                    &self.field_type,
                    &enums,
                    &app_structs,
                    &persistable_structs,
                )
            }
        } else {
            field_type_to_surql_type(
                &self.field_name,
                table_name,
                &self.field_type,
                &enums,
                &app_structs,
                &persistable_structs,
            )
        };

        if let Some(ref def) = self.define_config {
            if def.flexible.is_some() && def.flexible.unwrap() {
                stmt.push_str(" FLEXIBLE");
            }
        }

        if !type_str.is_empty() {
            stmt.push_str(&format!(" TYPE {}", type_str));
        }

        // Handle DEFAULT clause.
        if let Some(ref def) = self.define_config {
            if let Some(ref def_val) = def.default {
                // Use DEFAULT ALWAYS if default_always is provided.
                let always = if def.default_always.is_some() {
                    " ALWAYS"
                } else {
                    ""
                };
                stmt.push_str(&format!(" DEFAULT{} {}", always, def_val));
            } else {
                stmt.push_str(&format!(
                    " DEFAULT {}",
                    field_type_to_surql_default(
                        &self.field_name,
                        table_name,
                        &self.field_type,
                        &enums,
                        &app_structs,
                        &persistable_structs,
                    )
                ));
            }

            // Append READONLY if set.
            if def.readonly.unwrap_or(false) {
                stmt.push_str(" READONLY");
            }

            // Append VALUE clause.
            if let Some(ref val) = def.value {
                stmt.push_str(&format!(" VALUE {}", val));
            }

            // Append ASSERT clause.
            if let Some(ref assertion) = def.assert {
                stmt.push_str(&format!(" ASSERT {}", assertion));
                let assert_clause = generate_assert_from_validators(&self.validators, "$value");
                if !assert_clause.is_empty() {
                    stmt.push_str(&format!(" AND {}", assert_clause));
                }
            } else {
                let assert_clause = generate_assert_from_validators(&self.validators, "$value");
                if !assert_clause.is_empty() {
                    stmt.push_str(&format!(" ASSERT {}", assert_clause));
                }
            }

            // Append PERMISSIONS clause if any permission is defined.
            let mut perms = Vec::new();
            if let Some(ref sel) = def.select_permissions {
                perms.push(format!("FOR select {}", sel));
            }
            if let Some(ref cre) = def.create_permissions {
                perms.push(format!("FOR create {}", cre));
            }
            if let Some(ref upd) = def.update_permissions {
                perms.push(format!("FOR update {}", upd));
            }
            if !perms.is_empty() {
                stmt.push_str(&format!(" PERMISSIONS {}", perms.join(" ")));
            }
        }
        stmt.push_str(";\n");

        // Add wildcard field definition for HashMap/BTreeMap
        if needs_wildcard {
            if let Some(wildcard_value_type) = wildcard_type {
                stmt.push_str(&format!(
                    "DEFINE FIELD {}.* ON TABLE {} TYPE {};\n",
                    self.field_name, table_name, wildcard_value_type
                ));
            }
        }

        stmt
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructConfig {
    pub struct_name: String,
    pub fields: Vec<StructField>,
    pub validators: Vec<Validator>,
}
