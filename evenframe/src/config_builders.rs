use crate::workspace_scanner::WorkspaceScanner;
use helpers::evenframe::{
    schemasync::table::TableConfig,
    types::{StructConfig, StructField, TaggedUnion, Variant, VariantData, FieldType},
    schemasync::{EdgeConfig, DefineConfig, PermissionsConfig},
    validator::Validator,
    mockmake::MockGenerationConfig,
    derive::{
        attributes::{parse_mock_data_attribute, parse_relation_attribute, parse_table_validators},
    },
};
use std::collections::HashMap;
use std::fs;
use syn::{parse_file, Item, ItemStruct, ItemEnum, Fields};

pub fn build_all_configs() -> (
    HashMap<String, TaggedUnion>,
    HashMap<String, TableConfig>,
    HashMap<String, StructConfig>,
) {
    let mut enum_configs = HashMap::new();
    let mut table_configs = HashMap::new();
    let mut struct_configs = HashMap::new();

    // Scan the workspace for all Evenframe types
    let scanner = WorkspaceScanner::new();
    let types = match scanner.scan_for_evenframe_types() {
        Ok(types) => types,
        Err(e) => {
            eprintln!("Error scanning workspace: {}", e);
            return (HashMap::new(), HashMap::new(), HashMap::new());
        }
    };

    // Group types by file for efficient parsing
    let mut types_by_file: HashMap<String, Vec<_>> = HashMap::new();
    for evenframe_type in types {
        types_by_file.entry(evenframe_type.file_path.clone())
            .or_insert_with(Vec::new)
            .push(evenframe_type);
    }

    // Parse each file and extract configs
    for (file_path, file_types) in types_by_file {
        let content = match fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file {}: {}", file_path, e);
                continue;
            }
        };

        let syntax = match parse_file(&content) {
            Ok(syntax) => syntax,
            Err(e) => {
                eprintln!("Error parsing file {}: {}", file_path, e);
                continue;
            }
        };

        // Process each item in the file
        for item in syntax.items {
            match item {
                Item::Struct(item_struct) => {
                    // Check if this struct is in our list of Evenframe types
                    if let Some(evenframe_type) = file_types.iter().find(|t| t.name == item_struct.ident.to_string()) {
                        if let Some(config) = parse_struct_config(&item_struct) {
                            if evenframe_type.has_id_field {
                                // It's a persistable struct with table config
                                let table_config = TableConfig {
                                    struct_config: config.clone(),
                                    relation: parse_relation_attribute(&item_struct.attrs).ok().flatten(),
                                    permissions: PermissionsConfig::parse(&item_struct.attrs).ok().flatten(),
                                    mock_generation_config: parse_mock_data_attribute(&item_struct.attrs).ok().flatten().map(|(n, _overrides, _coords)| {
                                        MockGenerationConfig {
                                            n,
                                            table_level_override: None,
                                            coordination_rules: vec![],
                                            preserve_unchanged: false,
                                            preserve_modified: false,
                                            batch_size: 1000,
                                            regenerate_fields: vec!["updated_at".to_string(), "created_at".to_string()],
                                            preservation_mode: helpers::evenframe::schemasync::compare::PreservationMode::default(),
                                        }
                                    }),
                                };
                                let name = helpers::case::to_snake_case(&config.name);
                                table_configs.insert(name, table_config);
                            } else {
                                // It's an app struct - keep original name for lookup
                                struct_configs.insert(config.name.clone(), config);
                            }
                        }
                    }
                }
                Item::Enum(item_enum) => {
                    // Check if this enum is in our list of Evenframe types
                    if file_types.iter().any(|t| t.name == item_enum.ident.to_string()) {
                        if let Some(tagged_union) = parse_enum_config(&item_enum) {
                            enum_configs.insert(tagged_union.enum_name.clone(), tagged_union.clone());
                            
                            // Also extract inline structs from enum variants
                            for variant in &tagged_union.variants {
                                if let Some(VariantData::InlineStruct(ref inline_struct)) = variant.data {
                                    struct_configs.insert(inline_struct.name.clone(), inline_struct.clone());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

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

    let table_validators = parse_table_validators(&item_struct.attrs).ok().unwrap_or_default();
    
    Some(StructConfig {
        name: name,  // Keep original name, don't convert to snake_case
        fields,
        validators: table_validators.into_iter().map(|v| {
            Validator::StringValidator(helpers::evenframe::validator::StringValidator::StringEmbedded(v))
        }).collect(),
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
                    let field_types: Vec<_> = fields.unnamed.iter()
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