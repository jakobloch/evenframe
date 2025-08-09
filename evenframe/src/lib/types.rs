use crate::{
    default::field_type_to_surql_default,
    format::Format,
    schemasync::{table::generate_assert_from_validators, DefineConfig, EdgeConfig, TableConfig},
    validator::Validator,
    wrappers::EvenframeRecordId,
};
use convert_case::{Case, Casing};
use core::fmt;
use quote::{quote, ToTokens};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use syn::Type as SynType;

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
pub enum FieldType {
    String,
    Char,
    Bool,
    Unit,
    F32,
    F64,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    EvenframeRecordId,
    DateTime,
    Duration,
    Timezone,
    Decimal,
    OrderedFloat(Box<FieldType>), // Wraps F32 or F64
    Tuple(Vec<FieldType>),
    Struct(Vec<(String, FieldType)>),
    Option(Box<FieldType>),
    Vec(Box<FieldType>),
    HashMap(Box<FieldType>, Box<FieldType>),
    BTreeMap(Box<FieldType>, Box<FieldType>),
    RecordLink(Box<FieldType>),
    Other(String),
}

impl ToTokens for FieldType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            FieldType::String => tokens.extend(quote! { FieldType::String }),
            FieldType::Char => tokens.extend(quote! { FieldType::Char }),
            FieldType::Bool => tokens.extend(quote! { FieldType::Bool }),
            FieldType::F32 => tokens.extend(quote! { FieldType::F32 }),
            FieldType::F64 => tokens.extend(quote! { FieldType::F64 }),
            FieldType::I8 => tokens.extend(quote! { FieldType::I8 }),
            FieldType::I16 => tokens.extend(quote! { FieldType::I16 }),
            FieldType::I32 => tokens.extend(quote! { FieldType::I32 }),
            FieldType::I64 => tokens.extend(quote! { FieldType::I64 }),
            FieldType::I128 => tokens.extend(quote! { FieldType::I128 }),
            FieldType::Isize => tokens.extend(quote! { FieldType::Isize }),
            FieldType::U8 => tokens.extend(quote! { FieldType::U8 }),
            FieldType::U16 => tokens.extend(quote! { FieldType::U16 }),
            FieldType::U32 => tokens.extend(quote! { FieldType::U32 }),
            FieldType::U64 => tokens.extend(quote! { FieldType::U64 }),
            FieldType::U128 => tokens.extend(quote! { FieldType::U128 }),
            FieldType::Usize => tokens.extend(quote! { FieldType::Usize }),
            FieldType::EvenframeRecordId => tokens.extend(quote! { FieldType::EvenframeRecordId }),
            FieldType::DateTime => tokens.extend(quote! { FieldType::DateTime }),
            FieldType::Duration => tokens.extend(quote! { FieldType::Duration }),
            FieldType::Timezone => tokens.extend(quote! { FieldType::Timezone }),
            FieldType::Decimal => tokens.extend(quote! { FieldType::Decimal }),
            FieldType::Unit => tokens.extend(quote! { FieldType::Unit }),
            FieldType::OrderedFloat(inner) => {
                tokens.extend(quote! {
                    FieldType::OrderedFloat(Box::new(#inner))
                });
            }
            FieldType::Other(s) => {
                // Wrap the string in a LitStr so it becomes a literal token.
                let lit = syn::LitStr::new(s, proc_macro2::Span::call_site());
                tokens.extend(quote! { FieldType::Other(#lit.to_string()) });
            }
            FieldType::Option(inner) => {
                tokens.extend(quote! {
                    FieldType::Option(Box::new(#inner))
                });
            }
            FieldType::Vec(inner) => {
                tokens.extend(quote! {
                    FieldType::Vec(Box::new(#inner))
                });
            }
            FieldType::Tuple(types) => {
                tokens.extend(quote! {
                    FieldType::Tuple(vec![#(#types),*])
                });
            }
            FieldType::Struct(fields) => {
                let field_tokens = fields.iter().map(|(fname, fty)| {
                    let lit = syn::LitStr::new(fname, proc_macro2::Span::call_site());
                    quote! { (#lit.to_string(), #fty) }
                });
                tokens.extend(quote! {
                    FieldType::Struct(vec![#(#field_tokens),*])
                });
            }
            FieldType::HashMap(key, value) => tokens.extend(quote! {
            FieldType::HashMap(Box::new(#key),Box::new(#value) ) }),
            FieldType::BTreeMap(key, value) => tokens.extend(quote! {
            FieldType::BTreeMap(Box::new(#key),Box::new(#value) ) }),
            FieldType::RecordLink(inner) => tokens.extend(quote! {
            FieldType::RecordLink(Box::new(#inner)) }),
        }
    }
}

impl FieldType {
    pub fn parse_syn_ty(ty: &SynType) -> FieldType {
        match ty {
            // Handle simple paths (e.g. "String", "bool", etc.)
            SynType::Path(type_path) => {
                // If there's a single segment, we check for known identifiers.
                if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                    let segment = type_path.path.segments.first().unwrap();
                    let ident = &segment.ident;

                    // Check if this segment has generic arguments
                    if let syn::PathArguments::AngleBracketed(ref args) = segment.arguments {
                        // Handle generic types like HashMap<K, V>, Vec<T>, Option<T>, etc.
                        match ident.to_string().as_str() {
                            "HashMap" | "BTreeMap" => {
                                if args.args.len() == 2 {
                                    if let (
                                        syn::GenericArgument::Type(key_ty),
                                        syn::GenericArgument::Type(val_ty),
                                    ) = (&args.args[0], &args.args[1])
                                    {
                                        let key_parsed = Self::parse_syn_ty(key_ty);
                                        let val_parsed = Self::parse_syn_ty(val_ty);
                                        if ident == "HashMap" {
                                            return FieldType::HashMap(
                                                Box::new(key_parsed),
                                                Box::new(val_parsed),
                                            );
                                        } else {
                                            return FieldType::BTreeMap(
                                                Box::new(key_parsed),
                                                Box::new(val_parsed),
                                            );
                                        }
                                    }
                                }
                            }
                            "Vec" => {
                                if args.args.len() == 1 {
                                    if let syn::GenericArgument::Type(inner_ty) = &args.args[0] {
                                        return FieldType::Vec(Box::new(Self::parse_syn_ty(
                                            inner_ty,
                                        )));
                                    }
                                }
                            }
                            "Option" => {
                                if args.args.len() == 1 {
                                    if let syn::GenericArgument::Type(inner_ty) = &args.args[0] {
                                        return FieldType::Option(Box::new(Self::parse_syn_ty(
                                            inner_ty,
                                        )));
                                    }
                                }
                            }
                            "RecordLink" => {
                                if args.args.len() == 1 {
                                    if let syn::GenericArgument::Type(inner_ty) = &args.args[0] {
                                        return FieldType::RecordLink(Box::new(
                                            Self::parse_syn_ty(inner_ty),
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // No generic arguments, check for simple types
                    match ident.to_string().as_str() {
                        "String" => FieldType::String,
                        "char" => FieldType::Char,
                        "bool" => FieldType::Bool,
                        "f32" => FieldType::F32,
                        "f64" => FieldType::F64,
                        "i8" => FieldType::I8,
                        "i16" => FieldType::I16,
                        "i32" => FieldType::I32,
                        "i64" => FieldType::I64,
                        "i128" => FieldType::I128,
                        "isize" => FieldType::Isize,
                        "u8" => FieldType::U8,
                        "u16" => FieldType::U16,
                        "u32" => FieldType::U32,
                        "u64" => FieldType::U64,
                        "u128" => FieldType::U128,
                        "usize" => FieldType::Usize,
                        "EvenframeRecordId" => FieldType::EvenframeRecordId,
                        "DateTime" => FieldType::DateTime,
                        "Duration" => FieldType::Duration,
                        "Tz" => FieldType::Timezone,
                        "Decimal" => FieldType::Decimal,
                        "()" => FieldType::Unit,
                        _ => {
                            // Convert the type into a string and remove all whitespace.
                            let type_str: String = type_path
                                .to_token_stream()
                                .to_string()
                                .chars()
                                .filter(|c| !c.is_whitespace())
                                .collect();

                            // Look for the generic delimiters.
                            if let Some(start) = type_str.find('<') {
                                if let Some(end) = type_str.rfind('>') {
                                    let outer = &type_str[..start];
                                    let inner = &type_str[start + 1..end];

                                    if outer == "Option" {
                                        if let Ok(inner_ty) = syn::parse_str::<SynType>(inner) {
                                            let inner_parsed = Self::parse_syn_ty(&inner_ty);
                                            FieldType::Option(Box::new(inner_parsed))
                                        } else {
                                            FieldType::Other(type_str)
                                        }
                                    } else if outer == "Vec" {
                                        if let Ok(inner_ty) = syn::parse_str::<SynType>(inner) {
                                            let inner_parsed = Self::parse_syn_ty(&inner_ty);
                                            FieldType::Vec(Box::new(inner_parsed))
                                        } else {
                                            FieldType::Other(type_str)
                                        }
                                    } else if outer == "RecordLink" {
                                        if let Ok(inner_ty) = syn::parse_str::<SynType>(inner) {
                                            let inner_parsed = Self::parse_syn_ty(&inner_ty);
                                            FieldType::RecordLink(Box::new(inner_parsed))
                                        } else {
                                            FieldType::Other(type_str)
                                        }
                                    } else if outer == "DateTime" {
                                        // Handle DateTime<Utc> and similar types
                                        FieldType::DateTime
                                    } else if outer == "Duration" {
                                        // Handle Duration and similar types
                                        FieldType::Duration
                                    } else {
                                        FieldType::Other(type_str)
                                    }
                                } else {
                                    FieldType::Other(type_str)
                                }
                            } else {
                                FieldType::Other(type_str)
                            }
                        }
                    }
                } else {
                    // For complex type paths, fallback to using their string representation.
                    let type_str = type_path.to_token_stream().to_string();
                    FieldType::Other(type_str)
                }
            }
            // For tuple types, recursively convert each element.
            SynType::Tuple(tuple) => {
                let elems = tuple
                    .elems
                    .iter()
                    .map(|elem| Self::parse_syn_ty(elem))
                    .collect();
                FieldType::Tuple(elems)
            }
            // Fallback for any other type.
            _ => {
                // Convert the type into a string and remove all whitespace.
                let type_str: String = ty
                    .to_token_stream()
                    .to_string()
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();

                // Look for the generic delimiters.
                if let Some(start) = type_str.find('<') {
                    if let Some(end) = type_str.rfind('>') {
                        let outer = &type_str[..start];
                        let inner = &type_str[start + 1..end];

                        if outer == "Option" {
                            if let Ok(inner_ty) = syn::parse_str::<SynType>(inner) {
                                let inner_parsed = Self::parse_syn_ty(&inner_ty);
                                FieldType::Option(Box::new(inner_parsed))
                            } else {
                                FieldType::Other(type_str)
                            }
                        } else if outer == "Vec" {
                            if let Ok(inner_ty) = syn::parse_str::<SynType>(inner) {
                                let inner_parsed = Self::parse_syn_ty(&inner_ty);
                                FieldType::Vec(Box::new(inner_parsed))
                            } else {
                                FieldType::Other(type_str)
                            }
                        } else if outer == "RecordLink" {
                            if let Ok(inner_ty) = syn::parse_str::<SynType>(inner) {
                                let inner_parsed = Self::parse_syn_ty(&inner_ty);
                                FieldType::RecordLink(Box::new(inner_parsed))
                            } else {
                                FieldType::Other(type_str)
                            }
                        } else if outer == "HashMap" || outer == "BTreeMap" {
                            // Parse HashMap<K, V> or BTreeMap<K, V>
                            if let Some(comma_pos) = inner.find(',') {
                                let key_str = inner[..comma_pos].trim();
                                let value_str = inner[comma_pos + 1..].trim();
                                if let (Ok(key_ty), Ok(value_ty)) = (
                                    syn::parse_str::<SynType>(key_str),
                                    syn::parse_str::<SynType>(value_str),
                                ) {
                                    let key_parsed = Self::parse_syn_ty(&key_ty);
                                    let value_parsed = Self::parse_syn_ty(&value_ty);
                                    if outer == "HashMap" {
                                        FieldType::HashMap(
                                            Box::new(key_parsed),
                                            Box::new(value_parsed),
                                        )
                                    } else {
                                        FieldType::BTreeMap(
                                            Box::new(key_parsed),
                                            Box::new(value_parsed),
                                        )
                                    }
                                } else {
                                    FieldType::Other(type_str)
                                }
                            } else {
                                FieldType::Other(type_str)
                            }
                        } else {
                            FieldType::Other(type_str)
                        }
                    } else {
                        FieldType::Other(type_str)
                    }
                } else {
                    FieldType::Other(type_str)
                }
            }
        }
    }

    pub fn parse_type_str(type_str: &str) -> FieldType {
        // Remove whitespace for consistent parsing
        let clean_str = type_str
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();

        // Check for simple primitive types first
        match clean_str.as_str() {
            "String" => FieldType::String,
            "char" => FieldType::Char,
            "bool" => FieldType::Bool,
            "f32" => FieldType::F32,
            "f64" => FieldType::F64,
            "i8" => FieldType::I8,
            "i16" => FieldType::I16,
            "i32" => FieldType::I32,
            "i64" => FieldType::I64,
            "i128" => FieldType::I128,
            "isize" => FieldType::Isize,
            "u8" => FieldType::U8,
            "u16" => FieldType::U16,
            "u32" => FieldType::U32,
            "u64" => FieldType::U64,
            "u128" => FieldType::U128,
            "usize" => FieldType::Usize,
            "EvenframeRecordId" => FieldType::EvenframeRecordId,
            "DateTime" => FieldType::DateTime,
            "Duration" => FieldType::Duration,
            "Tz" => FieldType::Timezone,
            "Decimal" => FieldType::Decimal,
            "()" => FieldType::Unit,
            _ => {
                // Check for generic types like Option<T> or Vec<T>
                if let Some(start) = clean_str.find('<') {
                    if let Some(end) = clean_str.rfind('>') {
                        let outer = &clean_str[..start];
                        let inner = &clean_str[start + 1..end];

                        match outer {
                            "Option" => {
                                let inner_type = Self::parse_type_str(inner);
                                FieldType::Option(Box::new(inner_type))
                            }
                            "Vec" => {
                                let inner_type = Self::parse_type_str(inner);
                                FieldType::Vec(Box::new(inner_type))
                            }
                            "DateTime" => FieldType::DateTime,
                            "Duration" => FieldType::Duration,
                            _ => FieldType::Other(clean_str),
                        }
                    } else {
                        // Malformed generic type (missing closing '>')
                        FieldType::Other(clean_str)
                    }
                } else if clean_str.starts_with('(') && clean_str.ends_with(')') {
                    // Handle tuple types
                    // Strip the outer parentheses
                    let inner = &clean_str[1..clean_str.len() - 1];

                    // Split by commas, but handle nested generics carefully
                    let mut elements = Vec::new();
                    let mut current = String::new();
                    let mut depth = 0;

                    for c in inner.chars() {
                        match c {
                            '<' => {
                                depth += 1;
                                current.push(c);
                            }
                            '>' => {
                                depth -= 1;
                                current.push(c);
                            }
                            ',' if depth == 0 => {
                                if !current.is_empty() {
                                    elements.push(Self::parse_type_str(&current));
                                    current.clear();
                                }
                            }
                            _ => current.push(c),
                        }
                    }

                    // Don't forget the last element
                    if !current.is_empty() {
                        elements.push(Self::parse_type_str(&current));
                    }

                    FieldType::Tuple(elements)
                } else {
                    // Unknown or complex type
                    FieldType::Other(clean_str)
                }
            }
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::String => write!(f, "String"),
            FieldType::Char => write!(f, "Char"),
            FieldType::Bool => write!(f, "Bool"),
            FieldType::Unit => write!(f, "Unit"),
            FieldType::F32 => write!(f, "F32"),
            FieldType::F64 => write!(f, "F64"),
            FieldType::I8 => write!(f, "I8"),
            FieldType::I16 => write!(f, "I16"),
            FieldType::I32 => write!(f, "I32"),
            FieldType::I64 => write!(f, "I64"),
            FieldType::I128 => write!(f, "I128"),
            FieldType::Isize => write!(f, "Isize"),
            FieldType::U8 => write!(f, "U8"),
            FieldType::U16 => write!(f, "U16"),
            FieldType::U32 => write!(f, "U32"),
            FieldType::U64 => write!(f, "U64"),
            FieldType::U128 => write!(f, "U128"),
            FieldType::Usize => write!(f, "Usize"),
            FieldType::EvenframeRecordId => write!(f, "EvenframeRecordId"),
            FieldType::DateTime => write!(f, "DateTime"),
            FieldType::Duration => write!(f, "Duration"),
            FieldType::Timezone => write!(f, "Timezone"),
            FieldType::Decimal => write!(f, "Decimal"),
            FieldType::OrderedFloat(inner) => write!(f, "OrderedFloat<{}>", inner),
            FieldType::Tuple(types) => {
                write!(f, "Tuple(")?;
                let mut first = true;
                for field_type in types {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field_type)?;
                    first = false;
                }
                write!(f, ")")
            }
            FieldType::Struct(fields) => {
                write!(f, "Struct(")?;
                let mut first = true;
                for (name, field_type) in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, field_type)?;
                    first = false;
                }
                write!(f, ")")
            }
            FieldType::Option(inner) => write!(f, "Option({})", inner),
            FieldType::Vec(inner) => write!(f, "Vec({})", inner),
            FieldType::HashMap(key, value) => write!(f, "HashMap({}, {})", key, value),
            FieldType::BTreeMap(key, value) => write!(f, "BTreeMap({}, {})", key, value),
            FieldType::RecordLink(inner) => write!(f, "RecordLink({})", inner),
            FieldType::Other(name) => write!(f, "{}", name),
        }
    }
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
        // Helper to convert a FieldType to its SurrealDB type string.
        // Returns (type_string, needs_wildcard_field, wildcard_type)
        fn field_type_to_surql_type(
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
                FieldType::F32 | FieldType::F64 => ("float".to_string(), false, None),
                FieldType::OrderedFloat(_inner) => ("float".to_string(), false, None),
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
                    let (value_type, _, _) = field_type_to_surql_type(
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
                    let (value_type, _, _) = field_type_to_surql_type(
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
                    // RecordLink should always point to a record type
                    match inner.as_ref() {
                        FieldType::Other(type_name) => {
                            // Convert type name to snake_case for table name
                            (format!("record<{}>", type_name.to_case(Case::Snake)), false, None)
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
                                            &FieldType::Other(inline_struct.name.clone())
                                        }
                                        VariantData::DataStructureRef(field_type) => field_type,
                                    };
                                    let (variant_type, _, _) = field_type_to_surql_type(
                                        field_name,
                                        table_name,
                                        variant_data_field_type,
                                        &enums,
                                        &app_structs,
                                        &persistable_structs,
                                    );
                                    variant_type
                                } else {
                                    format!("\"{}\"", variant.name)
                                }
                            })
                            .collect();
                        (variants.join(" | "), false, None)
                    } else if let Some(app_struct) = app_structs.get(name) {
                        let field_defs: Vec<String> = app_struct
                            .fields
                            .iter()
                            .map(|f: &StructField| {
                                let (field_type, _, _) = field_type_to_surql_type(
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
                    } else if persistable_structs.get(&name.to_case(Case::Snake)).is_some() {
                        (format!("record<{}>", name.to_case(Case::Snake)), false, None)
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
    pub name: String,
    pub fields: Vec<StructField>,
    pub validators: Vec<Validator>,
}
