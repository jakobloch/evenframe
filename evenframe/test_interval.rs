use std::collections::HashMap;
use helpers::evenframe::schemasync::{
    types::{FieldType, TaggedUnion, Variant},
    table::{TableConfig, TableField, StructConfig},
    surql::generate_field_value,
};

fn main() {
    // Create a simple enum
    let mut enums = HashMap::new();
    
    let interval_enum = TaggedUnion {
        enum_name: "Interval".to_string(),
        variants: vec![
            Variant {
                name: "DailyInterval".to_string(),
                data: Some(FieldType::Other("DailyRecurrenceRule".to_string())),
            },
            Variant {
                name: "WeeklyInterval".to_string(), 
                data: Some(FieldType::Other("WeeklyRecurrenceRule".to_string())),
            },
        ],
    };
    
    enums.insert("Interval".to_string(), interval_enum);
    
    // Create empty hashmaps for other params
    let tables = HashMap::new();
    let structs = HashMap::new();
    let id_map = HashMap::new();
    
    // Create a dummy table config
    let table_config = TableConfig {
        struct_config: StructConfig {
            name: "test".to_string(),
            fields: vec![],
        },
        relation: None,
        permissions: None,
        mock_generation_config: None,
    };
    
    // Test generating a field value for Interval
    let field_name = "interval".to_string();
    let result = generate_field_value(
        &table_config,
        &field_name,
        Some(&"test".to_string()),
        &FieldType::Other("Interval".to_string()),
        &tables,
        &structs,
        &enums,
        &id_map,
        Some(0),
        1,
    );
    
    println!("Generated value: {}", result);
}