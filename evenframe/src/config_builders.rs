use crate::workspace_scanner::WorkspaceScanner;
use convert_case::{Case, Casing};
use evenframe::{
    derive::attributes::{
        parse_mock_data_attribute, parse_relation_attribute, parse_table_validators,
    },
    mockmake::{coordinate::Coordination, MockGenerationConfig},
    schemasync::table::TableConfig,
    schemasync::{DefineConfig, EdgeConfig, PermissionsConfig},
    types::{FieldType, StructConfig, StructField, TaggedUnion, Variant, VariantData},
    validator::Validator,
};
use std::collections::HashMap;
use std::fs;
use syn::{
    parse_file, Attribute, Expr, ExprArray, ExprLit, Fields, Item, ItemEnum, ItemStruct, Lit, Meta,
};
use tracing::{debug, info, trace, warn};

pub fn build_all_configs() -> (
    HashMap<String, TaggedUnion>,
    HashMap<String, TableConfig>,
    HashMap<String, StructConfig>,
) {
    debug!("Starting build_all_configs");
    let mut enum_configs = HashMap::new();
    let mut table_configs = HashMap::new();
    let mut struct_configs = HashMap::new();

    debug!("Creating workspace scanner");
    // Scan the workspace for all Evenframe types
    let scanner = WorkspaceScanner::new();
    let types = match scanner.scan_for_evenframe_types() {
        Ok(types) => {
            info!("Found {} Evenframe types", types.len());
            types
        }
        Err(e) => {
            warn!("Error scanning workspace: {}", e);
            return (HashMap::new(), HashMap::new(), HashMap::new());
        }
    };

    debug!("Grouping types by file");
    // Group types by file for efficient parsing
    let mut types_by_file: HashMap<String, Vec<_>> = HashMap::new();
    for evenframe_type in types {
        types_by_file
            .entry(evenframe_type.file_path.clone())
            .or_insert_with(Vec::new)
            .push(evenframe_type);
    }
    debug!("Grouped into {} files", types_by_file.len());

    debug!("Starting first pass: parsing structs and enums");
    // First pass: Parse all structs and enums to build struct_configs
    for (file_path, file_types) in &types_by_file {
        trace!("Processing file: {}", file_path);
        let content = match fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Error reading file {}: {}", file_path, e);
                continue;
            }
        };

        let syntax = match parse_file(&content) {
            Ok(syntax) => syntax,
            Err(e) => {
                warn!("Error parsing file {}: {}", file_path, e);
                continue;
            }
        };
        trace!("Parsed {} items from {}", syntax.items.len(), file_path);

        // Process each item in the file
        for item in syntax.items {
            match item {
                Item::Struct(item_struct) => {
                    // Check if this struct is in our list of Evenframe types
                    if let Some(evenframe_type) = file_types
                        .iter()
                        .find(|t| t.name == item_struct.ident.to_string())
                    {
                        if let Some(config) = parse_struct_config(&item_struct) {
                            struct_configs.insert(config.name.clone(), config.clone());

                            if evenframe_type.has_id_field {
                                // Build table config immediately (like before)
                                let name = config.name.to_case(Case::Snake);

                                // We don't have all struct_configs yet, so we can't parse coordination rules properly
                                // But that's okay - we'll just parse them without full resolution for now
                                let table_config = TableConfig {
                                    struct_config: config.clone(),
                                    relation: parse_relation_attribute(&item_struct.attrs).ok().flatten(),
                                    permissions: PermissionsConfig::parse(&item_struct.attrs).ok().flatten(),
                                    mock_generation_config: parse_mock_data_attribute(&item_struct.attrs)
                                        .ok()
                                        .flatten()
                                        .map(|(n, _overrides, _coords)| MockGenerationConfig {
                                            n,
                                            table_level_override: None,
                                            coordination_rules: vec![], // Empty for now, like it was before
                                            preserve_unchanged: false,
                                            preserve_modified: false,
                                            batch_size: 1000,
                                            regenerate_fields: vec!["updated_at".to_string(), "created_at".to_string()],
                                            preservation_mode:
                                               evenframe::schemasync::compare::PreservationMode::default(),
                                        }),
                                };
                                table_configs.insert(name, table_config);
                            }
                        }
                    }
                }
                Item::Enum(item_enum) => {
                    // Check if this enum is in our list of Evenframe types
                    if file_types
                        .iter()
                        .any(|t| t.name == item_enum.ident.to_string())
                    {
                        if let Some(tagged_union) = parse_enum_config(&item_enum) {
                            enum_configs
                                .insert(tagged_union.enum_name.clone(), tagged_union.clone());

                            // Also extract inline structs from enum variants
                            for variant in &tagged_union.variants {
                                if let Some(VariantData::InlineStruct(ref inline_struct)) =
                                    variant.data
                                {
                                    struct_configs
                                        .insert(inline_struct.name.clone(), inline_struct.clone());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    info!(
        "First pass complete. Found {} struct configs, {} enum configs, {} table configs",
        struct_configs.len(),
        enum_configs.len(),
        table_configs.len()
    );

    (enum_configs, table_configs, struct_configs)
}

fn parse_struct_config(item_struct: &ItemStruct) -> Option<StructConfig> {
    let name = item_struct.ident.to_string();
    let mut fields = Vec::new();

    if let Fields::Named(ref fields_named) = item_struct.fields {
        for field in &fields_named.named {
            let field_name = field.ident.as_ref()?.to_string();
            let field_name = field_name.trim_start_matches("r#").to_string();

            // Parse field type directly to FieldType
            let field_type = FieldType::parse_syn_ty(&field.ty);

            // Parse attributes using the derive module's parsers
            let edge_config = EdgeConfig::parse(field).ok().flatten();
            let define_config = DefineConfig::parse(field).ok().flatten();

            // Parse format - simplified for now
            let format = None;

            // Parse validators - simplified for now
            let validators = vec![];

            fields.push(StructField {
                field_name,
                field_type,
                edge_config,
                define_config,
                format,
                validators,
                always_regenerate: false,
            });
        }
    }

    let table_validators = parse_table_validators(&item_struct.attrs)
        .ok()
        .unwrap_or_default();

    Some(StructConfig {
        name: name, // Keep original name, don't convert to snake_case
        fields,
        validators: table_validators
            .into_iter()
            .map(|v| {
                Validator::StringValidator(evenframe::validator::StringValidator::StringEmbedded(v))
            })
            .collect(),
    })
}

fn parse_enum_config(item_enum: &ItemEnum) -> Option<TaggedUnion> {
    let enum_name = item_enum.ident.to_string();
    let mut variants = Vec::new();

    for variant in &item_enum.variants {
        let variant_name = variant.ident.to_string();

        let data = match &variant.fields {
            Fields::Unit => None,
            Fields::Unnamed(fields) => {
                if fields.unnamed.is_empty() {
                    None
                } else if fields.unnamed.len() == 1 {
                    let field = &fields.unnamed[0];
                    let field_type = FieldType::parse_syn_ty(&field.ty);
                    Some(VariantData::DataStructureRef(field_type))
                } else {
                    let field_types: Vec<_> = fields
                        .unnamed
                        .iter()
                        .map(|f| FieldType::parse_syn_ty(&f.ty))
                        .collect();
                    Some(VariantData::DataStructureRef(FieldType::Tuple(field_types)))
                }
            }
            Fields::Named(fields) => {
                let mut struct_fields = Vec::new();
                for field in &fields.named {
                    let field_name = field.ident.as_ref()?.to_string();
                    let field_name = field_name.trim_start_matches("r#").to_string();
                    let field_type = FieldType::parse_syn_ty(&field.ty);

                    struct_fields.push(StructField {
                        field_name,
                        field_type,
                        edge_config: None,
                        define_config: None,
                        format: None,
                        validators: vec![],
                        always_regenerate: false,
                    });
                }

                Some(VariantData::InlineStruct(StructConfig {
                    name: variant_name.clone(),
                    fields: struct_fields,
                    validators: vec![],
                }))
            }
        };

        variants.push(Variant {
            name: variant_name,
            data,
        });
    }

    Some(TaggedUnion {
        enum_name,
        variants,
    })
}

/// Parse coordination rules from mock_data attribute, resolving dot notation
fn _parse_coordination_rules(
    attrs: &[Attribute],
    struct_configs: &HashMap<String, StructConfig>,
) -> Vec<Coordination> {
    let mut coordination_rules = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("mock_data") {
            if let Ok(metas) = attr.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            ) {
                for meta in metas {
                    if let Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("coordinate") {
                            if let Expr::Array(ExprArray { elems, .. }) = nv.value {
                                for elem in elems {
                                    if let Some(rule) =
                                        parse_coordination_expr(&elem, struct_configs)
                                    {
                                        coordination_rules.push(rule);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    coordination_rules
}

/// Parse a single coordination expression
fn parse_coordination_expr(
    expr: &Expr,
    struct_configs: &HashMap<String, StructConfig>,
) -> Option<Coordination> {
    if let Expr::Call(call) = expr {
        if let Expr::Path(path) = &*call.func {
            if let Some(segment) = path.path.segments.last() {
                let func_name = segment.ident.to_string();

                match func_name.as_str() {
                    "InitializeEqual" => {
                        if let Some(Expr::Array(arr)) = call.args.first() {
                            let mut field_names = Vec::new();
                            for elem in &arr.elems {
                                if let Expr::Lit(ExprLit {
                                    lit: Lit::Str(s), ..
                                }) = elem
                                {
                                    let resolved_field =
                                        resolve_field_path(&s.value(), struct_configs);
                                    field_names.push(resolved_field);
                                }
                            }
                            if !field_names.is_empty() {
                                return Some(Coordination::InitializeEqual(field_names));
                            }
                        }
                    }
                    // Add other coordination types as needed
                    _ => {}
                }
            }
        }
    }
    None
}

/// Resolve a field path that may contain dot notation
/// e.g., "recurrence_rule.recurrence_begins" validates that recurrence_rule is an Option<RecurrenceRule>
/// and that RecurrenceRule has a field recurrence_begins
fn resolve_field_path(path: &str, _struct_configs: &HashMap<String, StructConfig>) -> String {
    // For simple fields without dots, just return as-is
    if !path.contains('.') {
        return path.to_string();
    }

    // Split the path into segments
    let segments: Vec<&str> = path.split('.').collect();
    if segments.len() != 2 {
        // For now, only support one level of nesting
        debug!("Complex nested path '{}' not fully validated", path);
        return path.to_string();
    }

    // The field name is the first segment (e.g., "recurrence_rule")
    // The nested field is the second segment (e.g., "recurrence_begins")
    // We would validate this if we had access to the parent struct's fields
    // For now, trust that the user provided a valid path

    path.to_string()
}

pub fn merge_tables_and_objects(
    tables: &HashMap<String, TableConfig>,
    objects: &HashMap<String, StructConfig>,
) -> HashMap<String, StructConfig> {
    let mut struct_configs = objects.clone();

    // Extract StructConfig from each TableConfig and merge into struct_configs
    for (name, table_config) in tables {
        struct_configs.insert(name.clone(), table_config.struct_config.clone());
    }

    struct_configs
}
