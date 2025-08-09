pub mod access;
pub mod define;
pub mod execute;
pub mod insert;
pub mod remove;
pub mod upsert;

use crate::{
    mockmake::Mockmaker,
    schemasync::table::TableConfig,
    types::{FieldType, StructConfig, StructField},
};
use rand::Rng;
use serde::Serialize;
use serde_json::Value;

pub enum QueryType {
    Create,
    Update,
}

/// Check if a field is nullable (wrapped in Option)
fn is_nullable_field(field: &StructField) -> bool {
    matches!(&field.field_type, FieldType::Option(_))
}

/// Check if a field is a nullable partial struct that needs conditional wrapping
fn is_nullable_partial_struct(field: &StructField, original_table: Option<&TableConfig>) -> bool {
    // Check if this is a struct field (partial update)
    if let FieldType::Struct(_) = &field.field_type {
        // Find the original field definition to check if it's nullable
        if let Some(table) = original_table {
            if let Some(original_field) = table
                .struct_config
                .fields
                .iter()
                .find(|f| f.field_name == field.field_name)
            {
                // Check if the original field type is Option<...>
                return is_nullable_field(original_field);
            }
        }
    }
    false
}

/// Check if any field needs null-preserving conditional logic
fn needs_null_preservation(field: &StructField, original_table: Option<&TableConfig>) -> bool {
    // Direct nullable check
    if is_nullable_field(field) {
        return true;
    }

    // Check for nullable partial structs
    if is_nullable_partial_struct(field, original_table) {
        return true;
    }

    false
}

impl Mockmaker {
    pub fn generate_server_only_inline(
        &self,
        gen_details: &TableConfig,
        struct_config: &StructConfig,
    ) -> String {
        let mut assignments = Vec::new();
        for table_field in &struct_config.fields {
            let val = self.generate_field_value_with_format(
                table_field,
                gen_details,
                None, // table_name
                None, // id_index
            );
            assignments.push(format!("{}: {val}", table_field.field_name));
        }
        // Surreal accepts JSON-like objects with unquoted keys:
        format!("{{ {} }}", assignments.join(", "))
    }
}

pub fn random_string(len: usize) -> String {
    use rand::distr::Alphanumeric;
    let mut rng = rand::rng();
    (0..len)
        .map(|_| rng.sample(&Alphanumeric) as char)
        .collect()
}

/// Generate a CREATE or UPDATE query for SurrealDB using a given schema definition and object
/// corresponding to the `table_schema` and the fields in `object`.
pub fn generate_query<T: Serialize>(
    query_type: QueryType,
    table_config: &TableConfig,
    object: &T,
    explicit_id: Option<String>,
) -> String {
    // Convert the input struct to a serde_json::Value so we can introspect field values by name.
    let value = serde_json::to_value(object).expect("Failed to serialize object to JSON Value");
    let mut record_id = if explicit_id.is_some() {
        explicit_id.clone().unwrap()
    } else {
        "".to_owned()
    };
    // Build the content body by iterating over schema fields and pulling matching JSON values.
    let mut content_parts = Vec::new();
    for field in &table_config.struct_config.fields {
        // We look up the JSON for this named field. If it's missing in the JSON,
        // you might want to skip it or handle defaults:
        if let Some(field_value) = value.get(&field.field_name) {
            if &field.field_name == "id" && explicit_id.is_none() {
                record_id = to_surreal_string(&field.field_type, field_value);
            } else if field.edge_config.is_none() {
                let surreal_string = to_surreal_string(&field.field_type, field_value);
                // SurrealDB wants e.g. name: 'John', or skills: ['Rust','Go','JavaScript'],
                // so we piece that together here:
                let part = format!("{}: {}", field.field_name, surreal_string);
                content_parts.push(part);
            }
        }
    }

    // Join all "field: value" pairs, then build the final update statement.
    let content_body = content_parts.join(", ");
    match query_type {
        QueryType::Create => {
            format!("CREATE {} CONTENT {{ {} }};", record_id, content_body)
        }
        QueryType::Update => {
            format!("UPDATE {} CONTENT {{ {} }};", record_id, content_body)
        }
    }
}

/// Convert a JSON value (already extracted from our struct) into the SurrealDB
/// syntax, guided by a FieldType.  Strings get single quotes in SurrealDB,
/// numeric/bool remain unquoted, arrays get bracketed, etc. This function
/// includes the special logic for EvenframeRecordId (no quotes).
fn to_surreal_string(field_type: &FieldType, value: &Value) -> String {
    match field_type {
        FieldType::String | FieldType::Char => {
            // Surreal uses single quotes for string literals. We assume `value` is a string:
            let s = value.as_str().unwrap_or_default();
            format!("'{}'", escape_single_quotes(s))
        }
        FieldType::Bool => {
            // Surreal booleans are unquoted true/false
            if value.as_bool().unwrap_or(false) {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }

        FieldType::Other(_) => value.to_string(),
        // For numeric types, rely on the JSON representation to yield
        // an unquoted string (e.g. "123", "3.14")
        FieldType::Decimal => {
            if value.is_string() {
                // Decimal values might come as strings to preserve precision
                value.as_str().unwrap_or("0.0").to_string()
            } else if value.is_number() {
                value.to_string()
            } else {
                "0.0".to_string()
            }
        }
        FieldType::F32
        | FieldType::F64
        | FieldType::OrderedFloat(_)
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
        | FieldType::Usize => {
            if value.is_number() {
                value.to_string() // e.g. 42, 3.14, etc.
            } else {
                "0".to_string()
            }
        }
        FieldType::Unit => {
            // A unit type might be stored as Surreal `NONE` or simply null,
            // but you can choose your own representation:
            "null".to_string()
        }
        FieldType::EvenframeRecordId => {
            let id_string = value.as_str().unwrap_or_default();
            id_string.to_string()
        }
        FieldType::DateTime => {
            // DateTime values should be ISO 8601 strings in JSON
            // SurrealDB requires datetime values to be prefixed with d''
            if let Some(s) = value.as_str() {
                format!("d'{}'", escape_single_quotes(s))
            } else {
                // Fallback to current time if not a string
                format!("d'{}'", chrono::Utc::now().to_rfc3339())
            }
        }
        FieldType::Duration => {
            // Duration values are stored as nanoseconds (i64)
            // SurrealDB uses duration::from::nanos() to convert
            if let Some(nanos) = value.as_i64() {
                format!("duration::from::nanos({})", nanos)
            } else if let Some(nanos) = value.as_u64() {
                format!("duration::from::nanos({})", nanos)
            } else {
                // Default to 0 nanoseconds
                "duration::from::nanos(0)".to_string()
            }
        }
        FieldType::Timezone => {
            // Timezone values are stored as IANA timezone strings
            // SurrealDB stores them as regular strings
            if let Some(s) = value.as_str() {
                format!("'{}'", escape_single_quotes(s))
            } else {
                // Default to UTC
                "'UTC'".to_string()
            }
        }
        // Vec<T> -> a bracketed list [...], each element converted recursively
        FieldType::Vec(inner_type) => {
            if let Some(array) = value.as_array() {
                let items: Vec<String> = array
                    .iter()
                    .map(|item_value| to_surreal_string(inner_type, item_value))
                    .collect();
                format!("[{}]", items.join(", "))
            } else {
                "[]".to_string()
            }
        }
        // Option<T> -> either null or the T type, recursively
        FieldType::Option(inner_type) => {
            if value.is_null() {
                "null".to_string()
            } else {
                to_surreal_string(inner_type, value)
            }
        }
        // Tuple(Vec<FieldType>) -> e.g. (val1, val2, val3)
        FieldType::Tuple(field_types) => {
            if let Some(arr) = value.as_array() {
                let mut parts = Vec::new();
                for (sub_ftype, sub_val) in field_types.iter().zip(arr.iter()) {
                    let s = to_surreal_string(sub_ftype, sub_val);
                    parts.push(s);
                }
                format!("({})", parts.join(", "))
            } else {
                "()".to_string()
            }
        }
        // Struct(Vec<(String, FieldType)>) -> a nested Surreal object { key: val, key2: val2, ... }
        FieldType::Struct(fields) => {
            if let Some(obj) = value.as_object() {
                let mut pairs = Vec::new();
                for (sub_field_name, sub_field_type) in fields {
                    if let Some(sub_val) = obj.get(sub_field_name) {
                        let s = to_surreal_string(sub_field_type, sub_val);
                        pairs.push(format!("{}: {}", sub_field_name, s));
                    }
                }
                format!("{{ {} }}", pairs.join(", "))
            } else {
                "{}".to_string()
            }
        }
        // Surreal doesn't have a direct "map literal" syntax (like JSON), so
        // you might represent it as an object { key: val }, or store as a JSON doc:
        FieldType::HashMap(key_type, value_type) => {
            if let Some(obj) = value.as_object() {
                let mut pairs = Vec::new();
                // We assume key_type is a string-like or something convertible:
                for (k, v) in obj {
                    let key_str = match &**key_type {
                        FieldType::String | FieldType::Char | FieldType::Other(_) => {
                            format!("'{}'", escape_single_quotes(k))
                        }
                        _ => k.clone(),
                    };
                    let val_str = to_surreal_string(value_type, v);
                    pairs.push(format!("{}: {}", key_str, val_str));
                }
                format!("{{ {} }}", pairs.join(", "))
            } else {
                "{}".to_string()
            }
        }
        FieldType::BTreeMap(key_type, value_type) => {
            if let Some(obj) = value.as_object() {
                let mut pairs = Vec::new();
                // We assume key_type is a string-like or something convertible:
                for (k, v) in obj {
                    let key_str = match &**key_type {
                        FieldType::String | FieldType::Char | FieldType::Other(_) => {
                            format!("'{}'", escape_single_quotes(k))
                        }
                        _ => k.clone(),
                    };
                    let val_str = to_surreal_string(value_type, v);
                    pairs.push(format!("{}: {}", key_str, val_str));
                }
                format!("{{ {} }}", pairs.join(", "))
            } else {
                "{}".to_string()
            }
        }
        // RecordLink means it's effectively a "link by ID" or a nested object to store inline.
        FieldType::RecordLink(inner_ftype) => {
            // Two typical shapes in the JSON:
            // 1) { "Id": "company:acme" } => store unquoted "company:acme"
            // 2) { "Object": { ... } } => store nested struct { ... }
            //
            // Or you might store a plain string: "company:acme"
            if value.is_string() {
                // If the user puts the link as a direct string in the JSON, e.g. "company:acme"
                let link_string = value
                    .as_str()
                    .expect("Record link value should not be None");
                link_string.to_string()
            } else if let Some(obj) = value.as_object() {
                // Check for "Id" => string
                if let Some(id_value) = obj.get("Id") {
                    if let Some(id_str) = id_value.as_str() {
                        format!("r{}", id_str.to_string())
                    } else {
                        "null".to_string()
                    }
                } else if let Some(obj_value) = obj.get("Object") {
                    // If it's an inline object, handle recursively
                    to_surreal_string(inner_ftype, obj_value)
                } else {
                    // Possibly your JSON structure is different. Fallback is to treat the entire object as nested.
                    to_surreal_string(inner_ftype, value)
                }
            } else {
                // If it's not an object/string, fallback:
                "null".to_string()
            }
        }
    }
}

/// Simple helper to escape single quotes in a string, because Surreal uses single quotes
/// for strings. This just replaces `'` with `\'`. Adjust as needed for robust escaping.
fn escape_single_quotes(s: &str) -> String {
    s.replace('\'', "\\'")
}
