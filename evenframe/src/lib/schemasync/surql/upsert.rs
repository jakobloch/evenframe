use crate::{coordinate::TableInsertsState, mockmake::Mockmaker, schemasync::table::TableConfig};
use convert_case::{Case, Casing};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};

impl Mockmaker {
    pub fn generate_upsert_statements(
        &self,
        table_name: &str,
        table_config: &TableConfig,
    ) -> String {
        info!(table_name = %table_name, "Generating upsert statements for table");
        debug!("Table config: {:?}", table_config);
        let mut output = String::new();
        let config = self
            .tables
            .get(table_name)
            .expect("TableConfig was not found");

        // Step 1: Parse Coordination Rules
        let gen_context = TableInsertsState::new(config, &self.schemasync_config.mock_gen_config);

        let n = config
            .mock_generation_config
            .as_ref()
            .map(|c| c.n)
            .unwrap_or(self.schemasync_config.mock_gen_config.default_record_count);

        // Step 2: Generate coordinated values for all records
        let coordinated_values = self.generate_coordinated_values(
            table_name,
            table_config,
            gen_context.coordination_group,
        );

        // Step 3: Generate UPSERT statements for each record
        for i in 0..n {
            let mut field_assignments = Vec::new();
            let mut processed_fields: HashSet<String> = HashSet::new();

            // Determine the record ID
            let record_id = if let Some(ids) = self.id_map.get(table_name) {
                if i < ids.len() {
                    ids[i].clone()
                } else {
                    format!("{}:{}", table_name.to_case(Case::Snake), i + 1)
                }
            } else {
                format!("{}:{}", table_name.to_case(Case::Snake), i + 1)
            };

            // First, add coordinated values if any exist for this record
            if i < coordinated_values.len() {
                for (field_name, value) in &coordinated_values[i] {
                    processed_fields.insert(field_name.clone());
                    // Quote string values properly
                    let quoted_value = if value.parse::<f64>().is_ok()
                        || value == "true"
                        || value == "false"
                        || value == "null"
                    {
                        value.clone()
                    } else {
                        format!("'{}'", value)
                    };

                    // Check if this field is nullable by looking it up in the struct fields
                    let field_def = table_config
                        .struct_config
                        .fields
                        .iter()
                        .find(|f| f.field_name == *field_name);

                    let needs_conditional = if let Some(field) = field_def {
                        super::needs_null_preservation(field, self.tables.get(table_name))
                    } else {
                        false
                    };

                    if needs_conditional {
                        // Wrap in conditional to preserve NULL state
                        field_assignments.push(format!(
                            "{}: (IF {} != NULL THEN {} ELSE NULL END)",
                            field_name, field_name, quoted_value
                        ));
                    } else {
                        field_assignments.push(format!("{}: {}", field_name, quoted_value));
                    }
                }
            }

            // Then, process remaining fields that weren't coordinated
            for table_field in &table_config.struct_config.fields {
                if !processed_fields.contains(&table_field.field_name) {
                    if table_field.edge_config.is_none()
                        || (table_field.define_config.is_some()
                            && !table_field.define_config.as_ref().unwrap().should_skip)
                    {
                        // Skip readonly fields
                        if let Some(ref define_config) = table_field.define_config {
                            if let Some(true) = define_config.readonly {
                                continue;
                            }
                        }
                        let field_val = if let Some(coord_value) = coordinated_values
                            .get(i)
                            .and_then(|cv| cv.get(&table_field.field_name))
                        {
                            // Use coordinated value if available
                            match &table_field.field_type {
                                crate::types::FieldType::DateTime => {
                                    format!("d'{}'", coord_value)
                                }
                                crate::types::FieldType::String => {
                                    format!("'{}'", coord_value)
                                }
                                _ => coord_value.clone(),
                            }
                        } else {
                            // Pass coordinated values for nested fields
                            let field_coordinated_values = if i < coordinated_values.len() {
                                &coordinated_values[i]
                            } else {
                                &HashMap::new()
                            };

                            self.generate_field_value_with_format_and_coordination(
                                table_field,
                                table_config,
                                Some(&table_name.to_string()),
                                field_coordinated_values,
                                Some(i),
                            )
                        };

                        // Check if this field needs null preservation
                        let needs_conditional = super::needs_null_preservation(
                            &table_field,
                            self.tables.get(table_name),
                        );

                        if needs_conditional {
                            // Wrap in conditional to preserve NULL state
                            field_assignments.push(format!(
                                "{}: (IF {} != NULL THEN {} ELSE NULL END)",
                                table_field.field_name, table_field.field_name, field_val
                            ));
                        } else {
                            field_assignments
                                .push(format!("{}: {field_val}", table_field.field_name));
                        }
                    }
                }
            }

            let fields_str = field_assignments.join(", ");

            // Generate UPSERT statement with MERGE for each record
            if table_config.relation.is_some() {
                // For relation tables, we need special handling
                output.push_str(&format!("UPSERT {record_id} MERGE {{ {fields_str} }};\n"));
            } else {
                // For regular tables, use UPSERT with MERGE
                output.push_str(&format!("UPSERT {record_id} MERGE {{ {fields_str} }};\n"));
            }
        }

        output
    }
}
