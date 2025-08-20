// Schemasync Merge - SurrealDB Native Implementation with Data Preservation
// This module provides a simplified schema synchronization system
// that leverages SurrealDB's native export/import functionality

pub mod import;

pub use crate::schemasync::mockmake::MockGenerationConfig;
use crate::{
    EvenframeError, Result, compare, evenframe_log,
    schemasync::{
        TableConfig,
        config::{PerformanceConfig, SchemasyncMockGenConfig},
        surql::access::setup_access_definitions,
    },
    types::{FieldType, StructConfig, StructField, TaggedUnion, VariantData},
};
pub use import::SchemaImporter;
use import::{AccessDefinition, FieldDefinition, ObjectType, SchemaDefinition, TableDefinition};
use quote::{ToTokens, quote};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use surrealdb::engine::local::{Db, Mem};
use surrealdb::{Surreal, engine::remote::http::Client};
use tracing;

#[derive(Debug)]
pub struct Comparator {
    db: Surreal<Client>,
    schemasync_config: crate::schemasync::config::SchemasyncConfig,

    // Runtime state
    remote_schema: Option<Surreal<Db>>,
    new_schema: Option<Surreal<Db>>,
    access_query: String,
    remote_schema_string: String,
    new_schema_string: String,
    schema_changes: Option<SchemaChanges>,
}

impl Comparator {
    pub fn new(
        db: Surreal<Client>,
        schemasync_config: crate::schemasync::config::SchemasyncConfig,
    ) -> Self {
        Self {
            db,
            schemasync_config,
            remote_schema: None,
            new_schema: None,
            access_query: String::new(),
            remote_schema_string: String::new(),
            new_schema_string: String::new(),
            schema_changes: None,
        }
    }

    pub async fn run(mut self) -> Result<Self> {
        tracing::info!("Starting Comparator pipeline");

        tracing::debug!("Setting up schemas");
        self.setup_schemas().await?;

        tracing::debug!("Setting up access definitions");
        self.setup_access().await?;

        tracing::debug!("Exporting schemas for comparison");
        self.export_schemas().await?;

        tracing::debug!("Comparing schemas");
        self.compare_schemas().await?;

        tracing::info!("Comparator pipeline completed successfully");
        Ok(self)
    }

    /// Setup backup and create in-memory schemas
    async fn setup_schemas(&mut self) -> Result<()> {
        tracing::trace!("Creating backup and in-memory schemas");
        let (remote_schema, new_schema) = setup_backup_and_schemas(&self.db).await?;
        self.remote_schema = Some(remote_schema);
        self.new_schema = Some(new_schema);
        tracing::trace!("Schemas setup complete");
        Ok(())
    }

    /// Setup access definitions
    async fn setup_access(&mut self) -> Result<()> {
        tracing::trace!("Setting up access definitions");
        let new_schema = self.new_schema.as_ref().unwrap();
        self.access_query = setup_access_definitions(new_schema, &self.schemasync_config).await?;
        tracing::trace!(
            access_query_length = self.access_query.len(),
            "Access query generated"
        );
        Ok(())
    }

    /// Export schemas for comparison
    async fn export_schemas(&mut self) -> Result<()> {
        tracing::trace!("Exporting schemas");
        let remote_schema = self.remote_schema.as_ref().unwrap();
        let new_schema = self.new_schema.as_ref().unwrap();

        let (remote_schema_string, new_schema_string) =
            export_schemas(remote_schema, new_schema).await?;

        tracing::trace!(
            remote_schema_size = remote_schema_string.len(),
            new_schema_size = new_schema_string.len(),
            "Schemas exported"
        );

        self.remote_schema_string = remote_schema_string;
        self.new_schema_string = new_schema_string;
        Ok(())
    }

    /// Compare schemas to find changes
    async fn compare_schemas(&mut self) -> Result<()> {
        tracing::trace!("Starting schema comparison");
        let changes = compare_schemas(
            &self.db,
            &self.remote_schema_string,
            &self.new_schema_string,
        )
        .await?;

        tracing::info!(
            new_tables = changes.new_tables.len(),
            removed_tables = changes.removed_tables.len(),
            modified_tables = changes.modified_tables.len(),
            "Schema changes detected"
        );

        self.schema_changes = Some(changes);
        Ok(())
    }

    // Getters for Mockmaker to access the results
    pub fn get_new_schema(&self) -> Option<&Surreal<Db>> {
        self.new_schema.as_ref()
    }

    pub fn get_access_query(&self) -> &str {
        &self.access_query
    }

    pub fn get_schema_changes(&self) -> Option<&compare::SchemaChanges> {
        self.schema_changes.as_ref()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub enum PreservationMode {
    /// No preservation - generate all new data
    None,
    #[default]
    /// Smart preservation - preserve unchanged fields, regenerate modified fields
    Smart,
    /// Full preservation - preserve all existing data, only generate for new fields
    Full,
}

impl ToTokens for PreservationMode {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variant_tokens = match self {
            PreservationMode::None => {
                quote! { ::evenframe::schemasync::compare::PreservationMode::None }
            }
            PreservationMode::Smart => {
                quote! { ::evenframe::schemasync::compare::PreservationMode::Smart }
            }
            PreservationMode::Full => {
                quote! { ::evenframe::schemasync::compare::PreservationMode::Full }
            }
        };
        tokens.extend(variant_tokens);
    }
}

/// Main entry point for Schemasync Merge functionality
pub struct Merger<'a> {
    pub client: &'a Surreal<Client>,
    pub default_mock_gen_config: SchemasyncMockGenConfig,
    pub performance: PerformanceConfig,
}

impl<'a> Merger<'a> {
    /// Create a new Merger instance
    pub async fn new(
        client: &'a Surreal<Client>,
        default_mock_gen_config: SchemasyncMockGenConfig,
        performance: PerformanceConfig,
    ) -> Result<Self> {
        Ok(Self {
            client,
            default_mock_gen_config,
            performance,
        })
    }

    /// Import schema from production database
    pub async fn import_schema_from_db(&self) -> Result<import::SchemaDefinition> {
        tracing::debug!("Importing schema from production database");
        let importer = SchemaImporter::new(self.client);
        let schema = importer.import_schema_only().await?;
        tracing::debug!(
            tables = schema.tables.len(),
            edges = schema.edges.len(),
            accesses = schema.accesses.len(),
            "Schema imported"
        );
        Ok(schema)
    }

    /// Generate schema from Rust structs
    pub fn generate_schema_from_structs(
        &self,
        tables: &HashMap<String, TableConfig>,
    ) -> Result<import::SchemaDefinition> {
        tracing::debug!(
            table_count = tables.len(),
            "Generating schema from Rust structs"
        );
        let schema = import::SchemaDefinition::from_table_configs(tables)?;
        tracing::debug!(
            tables = schema.tables.len(),
            edges = schema.edges.len(),
            "Schema generated from structs"
        );
        Ok(schema)
    }

    pub fn compare_schemas(
        &self,
        old: &import::SchemaDefinition,
        new: &import::SchemaDefinition,
    ) -> Result<SchemaChanges> {
        tracing::debug!("Comparing schemas using legacy method");
        Comparator::compare(old, new)
    }

    /// Export mock data to file
    pub async fn export_mock_data(&self, _file_path: &str) -> Result<()> {
        // Implementation will use the generated statements
        // and write them to the specified file
        todo!("Implement export_mock_data")
    }

    /// Generate preserved data for a specific table
    pub async fn generate_preserved_data(
        &self,
        table_name: &str,
        table_config: &TableConfig,
        mock_config: MockGenerationConfig,
        existing_records: Vec<serde_json::Value>,
        target_count: usize,
        schema_changes: Option<&SchemaChanges>,
    ) -> Vec<serde_json::Value> {
        use serde_json::Value;

        // Determine how many records to preserve vs generate
        let existing_count = existing_records.len();
        let mut result = Vec::new();

        match mock_config.preservation_mode {
            PreservationMode::None => {
                // No preservation - generate all new data
                result = self.generate_new_records(table_name, table_config, target_count);
            }
            PreservationMode::Smart => {
                // Smart preservation - keep unchanged fields, regenerate specified fields
                if existing_count > 0 {
                    // Determine which fields need regeneration
                    let mut fields_to_regenerate = mock_config.regenerate_fields.clone();

                    // If schema changes are provided, add fields that changed
                    if let Some(changes) = schema_changes {
                        // Get fields that need regeneration based on schema changes
                        let schema_fields_needing_generation =
                            changes.get_fields_needing_generation(table_name);

                        // If all fields need generation (new table), regenerate everything
                        if schema_fields_needing_generation.contains(&"*".to_string()) {
                            // Generate all new records for new tables
                            result =
                                self.generate_new_records(table_name, table_config, target_count);
                            return result;
                        }

                        // Add schema-detected fields to the regeneration list
                        for field in schema_fields_needing_generation {
                            if !fields_to_regenerate.contains(&field) {
                                fields_to_regenerate.push(field);
                            }
                        }
                    }

                    for mut record in existing_records {
                        // Regenerate specified fields
                        if let Value::Object(ref mut map) = record {
                            // First, add any new fields that don't exist in the record
                            for field in &table_config.struct_config.fields {
                                if !map.contains_key(&field.field_name) {
                                    // This is a new field, generate value
                                    let new_value = Self::generate_field_value(field, table_config);
                                    map.insert(field.field_name.clone(), new_value);
                                }
                            }

                            // Then, regenerate fields that need it
                            for field_name in &fields_to_regenerate {
                                if let Some(field) = table_config
                                    .struct_config
                                    .fields
                                    .iter()
                                    .find(|f| &f.field_name == field_name)
                                {
                                    // Generate new value for this field
                                    let new_value = Self::generate_field_value(field, table_config);
                                    map.insert(field_name.clone(), new_value);
                                }
                            }
                        }

                        result.push(record);
                    }

                    // Generate additional records if needed
                    if target_count > existing_count {
                        let additional = self.generate_new_records(
                            table_name,
                            table_config,
                            target_count - existing_count,
                        );
                        result.extend(additional);
                    }
                } else {
                    // No existing data or preservation disabled
                    result = self.generate_new_records(table_name, table_config, target_count);
                }
            }
            PreservationMode::Full => {
                // Full preservation - keep all data, only add new fields
                if existing_count > 0 {
                    // Check if target count is less than existing count
                    if target_count < existing_count {
                        eprintln!(
                            "\n⚠️  WARNING: Full preservation mode with data reduction detected!"
                        );
                        eprintln!(
                            "   Table '{}' has {} existing records but target count is set to {}",
                            table_name, existing_count, target_count
                        );
                        eprintln!(
                            "   This will DELETE {} records!",
                            existing_count - target_count
                        );
                        eprintln!("\n   Options:");
                        eprintln!(
                            "   1. Change the target count (n) to {} or higher to preserve all records",
                            existing_count
                        );
                        eprintln!("   2. Use Smart preservation mode instead of Full");
                        eprintln!(
                            "   3. Set preservation_mode to None if you want to regenerate all data"
                        );
                        eprintln!(
                            "\n   In a production environment, this would require user confirmation."
                        );
                        eprintln!(
                            "   For now, proceeding with target count of {} records.\n",
                            target_count
                        );

                        for mut record in existing_records {
                            // Only add new fields that don't exist
                            if let Value::Object(ref mut map) = record {
                                for field in &table_config.struct_config.fields {
                                    if !map.contains_key(&field.field_name) {
                                        // This is a new field, generate value
                                        let new_value =
                                            Self::generate_field_value(field, table_config);
                                        map.insert(field.field_name.clone(), new_value);
                                    }
                                }
                            }

                            result.push(record);
                        }
                    } else {
                        // Normal case: preserve all existing records
                        for mut record in existing_records {
                            // Only add new fields that don't exist
                            if let Value::Object(ref mut map) = record {
                                for field in &table_config.struct_config.fields {
                                    if !map.contains_key(&field.field_name) {
                                        // This is a new field, generate value
                                        let new_value =
                                            Self::generate_field_value(field, table_config);
                                        map.insert(field.field_name.clone(), new_value);
                                    }
                                }
                            }

                            result.push(record);
                        }

                        // Generate additional records if needed
                        if target_count > existing_count {
                            let additional = self.generate_new_records(
                                table_name,
                                table_config,
                                target_count - existing_count,
                            );
                            result.extend(additional);
                        }
                    }
                } else {
                    result = self.generate_new_records(table_name, table_config, target_count);
                }
            }
        }

        result
    }

    /// Generate new records for a table
    fn generate_new_records(
        &self,
        _table_name: &str,
        table_config: &TableConfig,
        count: usize,
    ) -> Vec<serde_json::Value> {
        use serde_json::Value;

        let mut records = Vec::new();

        for _ in 0..count {
            let mut record = serde_json::Map::new();

            // Generate values for each field
            for field in &table_config.struct_config.fields {
                let value = Self::generate_field_value(field, table_config);
                record.insert(field.field_name.clone(), value);
            }

            records.push(Value::Object(record));
        }

        records
    }

    /// Generate a value for a specific field
    fn generate_field_value(
        field: &crate::types::StructField,
        _table_config: &TableConfig,
    ) -> serde_json::Value {
        use crate::types::FieldType;
        use serde_json::json;

        // Use format if available
        if let Some(format) = &field.format {
            let value = format.generate_formatted_value();

            // Check if the format generates numeric values
            match format {
                crate::schemasync::mockmake::format::Format::Percentage
                | crate::schemasync::mockmake::format::Format::Latitude
                | crate::schemasync::mockmake::format::Format::Longitude
                | crate::schemasync::mockmake::format::Format::CurrencyAmount => {
                    // Try to parse as number
                    if let Ok(num) = value.parse::<f64>() {
                        return json!(num);
                    }
                }
                _ => {}
            }

            return json!(value);
        }

        // Generate based on field type
        match &field.field_type {
            FieldType::String => json!(crate::schemasync::Mockmaker::random_string(8)),
            FieldType::Bool => json!(rand::random::<bool>()),
            FieldType::U8
            | FieldType::U16
            | FieldType::U32
            | FieldType::U64
            | FieldType::U128
            | FieldType::Usize => json!(rand::random::<u32>() % 100),
            FieldType::I8
            | FieldType::I16
            | FieldType::I32
            | FieldType::I64
            | FieldType::I128
            | FieldType::Isize => json!(rand::random::<i32>() % 100),
            FieldType::F32 | FieldType::F64 => json!(rand::random::<f64>() * 100.0),
            FieldType::DateTime => json!(chrono::Utc::now().to_rfc3339()),
            FieldType::EvenframeDuration => {
                // Generate random duration in nanoseconds (0 to 1 day)
                json!(rand::random::<i64>() % 86_400_000_000_000i64)
            }
            FieldType::Timezone => {
                // Generate random IANA timezone string
                let timezones = [
                    "UTC",
                    "America/New_York",
                    "America/Los_Angeles",
                    "Europe/London",
                    "Europe/Paris",
                    "Asia/Tokyo",
                    "Asia/Shanghai",
                    "Australia/Sydney",
                ];
                let index = (rand::random::<f64>() * timezones.len() as f64) as usize;
                json!(timezones[index.min(timezones.len() - 1)])
            }
            FieldType::Option(inner) => {
                if rand::random::<bool>() {
                    let inner_field = crate::types::StructField {
                        field_name: field.field_name.clone(),
                        field_type: *inner.clone(),
                        format: field.format.clone(),
                        edge_config: None,
                        define_config: None,
                        validators: Vec::new(),
                        always_regenerate: false,
                    };
                    Self::generate_field_value(&inner_field, _table_config)
                } else {
                    json!(null)
                }
            }
            FieldType::Vec(_) => json!([]),
            FieldType::Other(type_name) => {
                // Handle common types
                if type_name.contains("DateTime") {
                    json!(chrono::Utc::now().to_rfc3339())
                } else {
                    json!(format!("{}:1", type_name.to_lowercase()))
                }
            }
            _ => json!(null),
        }
    }
}

/// Types of changes that can occur in an access definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessChangeType {
    JwtKeyChanged,
    IssuerKeyChanged,
    JwtUrlChanged,
    AuthenticateClauseChanged,
    DurationChanged,
    SigninChanged,
    SignupChanged,
    OtherChange(String),
}

impl AccessChangeType {
    /// Check if this change type is ignorable (e.g., rotating keys)
    pub fn is_ignorable(&self) -> bool {
        matches!(
            self,
            AccessChangeType::JwtKeyChanged | AccessChangeType::IssuerKeyChanged
        )
    }
}

impl std::fmt::Display for AccessChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessChangeType::JwtKeyChanged => write!(f, "JWT key changed"),
            AccessChangeType::IssuerKeyChanged => write!(f, "Issuer key changed"),
            AccessChangeType::JwtUrlChanged => write!(f, "JWT URL changed"),
            AccessChangeType::AuthenticateClauseChanged => write!(f, "Authenticate clause changed"),
            AccessChangeType::DurationChanged => write!(f, "EvenframeDuration changed"),
            AccessChangeType::SigninChanged => write!(f, "Signin changed"),
            AccessChangeType::SignupChanged => write!(f, "Signup changed"),
            AccessChangeType::OtherChange(msg) => write!(f, "{}", msg),
        }
    }
}

/// Represents changes between two schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChanges {
    pub new_tables: Vec<String>,
    pub removed_tables: Vec<String>,
    pub modified_tables: Vec<TableChanges>,
    pub new_accesses: Vec<String>,
    pub removed_accesses: Vec<String>,
    pub modified_accesses: Vec<AccessChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessChange {
    pub access_name: String,
    pub changes: Vec<AccessChangeType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableChanges {
    pub table_name: String,
    pub new_fields: Vec<String>,
    pub removed_fields: Vec<String>,
    pub modified_fields: Vec<FieldChange>,
    pub permission_changed: bool,
    pub schema_type_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field_name: String,
    pub old_type: String,
    pub new_type: String,
    pub change_type: ChangeType,
    pub required_changed: bool,
    pub default_changed: bool,
}

impl SchemaChanges {
    /// Check if a specific field is unchanged
    pub fn is_field_unchanged(&self, table: &str, field: &str) -> bool {
        // If table is new or removed, field is not unchanged
        if self.new_tables.contains(&table.to_string())
            || self.removed_tables.contains(&table.to_string())
        {
            return false;
        }

        // Check modified tables
        for table_change in &self.modified_tables {
            if table_change.table_name == table {
                // If field is new or removed, it's not unchanged
                if table_change.new_fields.contains(&field.to_string())
                    || table_change.removed_fields.contains(&field.to_string())
                {
                    return false;
                }

                // If field is modified, it's not unchanged
                for field_change in &table_change.modified_fields {
                    if field_change.field_name == field {
                        return false;
                    }
                }

                // Field exists in table and is not in any change list
                return true;
            }
        }

        // Table not in modified list, so if it exists in old schema, field is unchanged
        true
    }

    /// Get all fields that need new data generation
    pub fn get_fields_needing_generation(&self, table: &str) -> Vec<String> {
        let mut fields = Vec::new();

        // If table is new, all fields need generation
        if self.new_tables.contains(&table.to_string()) {
            return vec!["*".to_string()]; // Special marker for all fields
        }

        // Find table in modified tables
        for table_change in &self.modified_tables {
            if table_change.table_name == table {
                // Add all new fields
                fields.extend(table_change.new_fields.clone());

                // Optionally add modified fields based on configuration
                // For now, we'll regenerate modified fields
                for field_change in &table_change.modified_fields {
                    fields.push(field_change.field_name.clone());
                }
            }
        }

        fields
    }

    /// Create a summary of changes
    pub fn summary(&self) -> String {
        let mut summary = Vec::new();

        if !self.new_tables.is_empty() {
            summary.push(format!("New tables: {}", self.new_tables.join(", ")));
        }

        if !self.removed_tables.is_empty() {
            summary.push(format!(
                "Removed tables: {}",
                self.removed_tables.join(", ")
            ));
        }

        if !self.modified_tables.is_empty() {
            summary.push(format!(
                "Modified tables: {}",
                self.modified_tables
                    .iter()
                    .map(|t| t.table_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if !self.new_accesses.is_empty() {
            summary.push(format!("New accesses: {}", self.new_accesses.join(", ")));
        }

        if !self.removed_accesses.is_empty() {
            summary.push(format!(
                "Removed accesses: {}",
                self.removed_accesses.join(", ")
            ));
        }

        if !self.modified_accesses.is_empty() {
            summary.push(format!(
                "Modified accesses: {}",
                self.modified_accesses
                    .iter()
                    .map(|a| a.access_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if summary.is_empty() {
            "No changes detected".to_string()
        } else {
            summary.join("\n")
        }
    }
}

impl Comparator {
    /// Compare two schemas and return the differences
    pub fn compare(old: &SchemaDefinition, new: &SchemaDefinition) -> Result<SchemaChanges> {
        tracing::debug!("Starting detailed schema comparison");

        let mut changes = SchemaChanges {
            new_tables: Vec::new(),
            removed_tables: Vec::new(),
            modified_tables: Vec::new(),
            new_accesses: Vec::new(),
            removed_accesses: Vec::new(),
            modified_accesses: Vec::new(),
        };

        // Get all table names from both schemas
        let old_tables: HashSet<String> = old.tables.keys().cloned().collect();
        let new_tables: HashSet<String> = new.tables.keys().cloned().collect();

        tracing::trace!(
            old_table_count = old_tables.len(),
            new_table_count = new_tables.len(),
            "Comparing table sets"
        );

        // Find new tables
        for table in new_tables.difference(&old_tables) {
            tracing::trace!(table = %table, "Found new table");
            changes.new_tables.push(table.clone());
        }

        // Find removed tables
        for table in old_tables.difference(&new_tables) {
            tracing::trace!(table = %table, "Found removed table");
            changes.removed_tables.push(table.clone());
        }

        // Find modified tables
        for table in old_tables.intersection(&new_tables) {
            tracing::trace!(table = %table, "Comparing table");
            if let Some(table_changes) = Self::compare_tables(
                table,
                old.tables.get(table).unwrap(),
                new.tables.get(table).unwrap(),
            )? {
                tracing::trace!(
                    table = %table,
                    new_fields = table_changes.new_fields.len(),
                    removed_fields = table_changes.removed_fields.len(),
                    modified_fields = table_changes.modified_fields.len(),
                    "Table has changes"
                );
                changes.modified_tables.push(table_changes);
            }
        }

        // Also compare edges
        let old_edges: HashSet<String> = old.edges.keys().cloned().collect();
        let new_edges: HashSet<String> = new.edges.keys().cloned().collect();

        for edge in new_edges.difference(&old_edges) {
            changes.new_tables.push(edge.clone());
        }

        for edge in old_edges.difference(&new_edges) {
            changes.removed_tables.push(edge.clone());
        }

        for edge in old_edges.intersection(&new_edges) {
            if let Some(edge_changes) = Self::compare_tables(
                edge,
                old.edges.get(edge).unwrap(),
                new.edges.get(edge).unwrap(),
            )? {
                changes.modified_tables.push(edge_changes);
            }
        }

        // Compare accesses
        let old_access_names: HashSet<String> =
            old.accesses.iter().map(|a| a.name.clone()).collect();
        let new_access_names: HashSet<String> =
            new.accesses.iter().map(|a| a.name.clone()).collect();

        // Find new accesses
        for access_name in new_access_names.difference(&old_access_names) {
            changes.new_accesses.push(access_name.clone());
        }

        // Find removed accesses
        for access_name in old_access_names.difference(&new_access_names) {
            changes.removed_accesses.push(access_name.clone());
        }

        // Find modified accesses
        for access_name in old_access_names.intersection(&new_access_names) {
            let old_access = old
                .accesses
                .iter()
                .find(|a| &a.name == access_name)
                .unwrap();
            let new_access = new
                .accesses
                .iter()
                .find(|a| &a.name == access_name)
                .unwrap();

            if let Some(access_change) = Self::compare_accesses(old_access, new_access) {
                changes.modified_accesses.push(access_change);
            }
        }

        tracing::debug!(
            new_tables = changes.new_tables.len(),
            removed_tables = changes.removed_tables.len(),
            modified_tables = changes.modified_tables.len(),
            new_accesses = changes.new_accesses.len(),
            removed_accesses = changes.removed_accesses.len(),
            modified_accesses = changes.modified_accesses.len(),
            "Schema comparison complete"
        );

        Ok(changes)
    }

    /// Compare two table definitions
    fn compare_tables(
        table_name: &str,
        old_table: &TableDefinition,
        new_table: &TableDefinition,
    ) -> Result<Option<TableChanges>> {
        let mut table_changes = TableChanges {
            table_name: table_name.to_string(),
            new_fields: Vec::new(),
            removed_fields: Vec::new(),
            modified_fields: Vec::new(),
            permission_changed: false,
            schema_type_changed: false,
        };

        // Check schema type change
        if old_table.schema_type != new_table.schema_type {
            table_changes.schema_type_changed = true;
        }

        // Check permission changes
        if old_table.permissions != new_table.permissions {
            table_changes.permission_changed = true;
        }

        // Compare regular fields
        let old_fields: HashSet<String> = old_table.fields.keys().cloned().collect();
        let new_fields: HashSet<String> = new_table.fields.keys().cloned().collect();

        // Find new fields
        for field in new_fields.difference(&old_fields) {
            table_changes.new_fields.push(field.clone());
        }

        // Find removed fields
        for field in old_fields.difference(&new_fields) {
            table_changes.removed_fields.push(field.clone());
        }

        // Find modified fields
        for field in old_fields.intersection(&new_fields) {
            let old_field = old_table.fields.get(field).unwrap_or_else(|| {
                panic!(
                    "Something went wrong getting the old field from old_table.fields: {:#?}",
                    field
                )
            });
            let new_field = new_table.fields.get(field).unwrap_or_else(|| {
                panic!(
                    "Something went wrong getting the new field from new_table.fields: {:#?}",
                    field
                )
            });

            // First check if the types are different for deep comparison
            if old_field.field_type != new_field.field_type {
                // For complex types, do deep comparison
                let deep_changes =
                    Self::compare_object_types(field, &old_field.field_type, &new_field.field_type);
                if !deep_changes.is_empty() {
                    // If we have granular changes, use those instead
                    table_changes.modified_fields.extend(deep_changes);
                } else {
                    // Otherwise fall back to regular comparison
                    if let Some(field_change) = Self::compare_fields(field, old_field, new_field) {
                        table_changes.modified_fields.push(field_change);
                    }
                }
            } else {
                // Types are the same, check for other changes (required, default)
                if let Some(field_change) = Self::compare_fields(field, old_field, new_field) {
                    table_changes.modified_fields.push(field_change);
                }
            }
        }

        // Compare array wildcard fields
        let old_wildcard_fields: HashSet<String> =
            old_table.array_wildcard_fields.keys().cloned().collect();
        let new_wildcard_fields: HashSet<String> =
            new_table.array_wildcard_fields.keys().cloned().collect();

        // Find new wildcard fields (these represent new fields)
        for field in new_wildcard_fields.difference(&old_wildcard_fields) {
            table_changes.new_fields.push(format!("{}[*]", field));
        }

        // Find removed wildcard fields
        for field in old_wildcard_fields.difference(&new_wildcard_fields) {
            table_changes.removed_fields.push(format!("{}[*]", field));
        }

        // Find modified wildcard fields
        for field in old_wildcard_fields.intersection(&new_wildcard_fields) {
            if let Some(field_change) = Self::compare_fields(
                &format!("{}[*]", field),
                old_table.array_wildcard_fields.get(field).unwrap(),
                new_table.array_wildcard_fields.get(field).unwrap(),
            ) {
                table_changes.modified_fields.push(field_change);
            }
        }

        // Return None if no changes detected
        if table_changes.new_fields.is_empty()
            && table_changes.removed_fields.is_empty()
            && table_changes.modified_fields.is_empty()
            && !table_changes.permission_changed
            && !table_changes.schema_type_changed
        {
            Ok(None)
        } else {
            Ok(Some(table_changes))
        }
    }

    /// Compare two field definitions
    fn compare_fields(
        field_name: &str,
        old_field: &FieldDefinition,
        new_field: &FieldDefinition,
    ) -> Option<FieldChange> {
        let mut changed = false;

        // Check for basic changes first
        let mut basic_change = FieldChange {
            field_name: field_name.to_string(),
            old_type: old_field.field_type.to_string(),
            new_type: new_field.field_type.to_string(),
            change_type: ChangeType::Modified,
            required_changed: false,
            default_changed: false,
        };

        // Check required change
        if old_field.required != new_field.required {
            basic_change.required_changed = true;
            changed = true;
        }

        // Check default value change
        if old_field.default_value != new_field.default_value {
            basic_change.default_changed = true;
            changed = true;
        }

        // Check type change
        if old_field.field_type != new_field.field_type {
            changed = true;
        }

        if changed { Some(basic_change) } else { None }
    }

    /// Deep comparison of object types to find granular changes
    pub fn compare_object_types(
        prefix: &str,
        old_type: &ObjectType,
        new_type: &ObjectType,
    ) -> Vec<FieldChange> {
        let mut changes = Vec::new();

        match (old_type, new_type) {
            // Both are objects - compare fields
            (ObjectType::Object(old_fields), ObjectType::Object(new_fields)) => {
                let old_keys: HashSet<String> = old_fields.keys().cloned().collect();
                let new_keys: HashSet<String> = new_fields.keys().cloned().collect();

                // Find added fields
                for key in new_keys.difference(&old_keys) {
                    let field_path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };

                    changes.push(FieldChange {
                        field_name: field_path,
                        old_type: String::new(),
                        new_type: new_fields[key].to_string(),
                        change_type: ChangeType::Added,
                        required_changed: false,
                        default_changed: false,
                    });
                }

                // Find removed fields
                for key in old_keys.difference(&new_keys) {
                    let field_path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };

                    changes.push(FieldChange {
                        field_name: field_path,
                        old_type: old_fields[key].to_string(),
                        new_type: String::new(),
                        change_type: ChangeType::Removed,
                        required_changed: false,
                        default_changed: false,
                    });
                }

                // Compare common fields
                for key in old_keys.intersection(&new_keys) {
                    let field_path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };

                    let old_field_type = &old_fields[key];
                    let new_field_type = &new_fields[key];

                    if old_field_type != new_field_type {
                        // Recursively compare nested types
                        let nested_changes =
                            Self::compare_object_types(&field_path, old_field_type, new_field_type);
                        if nested_changes.is_empty() {
                            // If no nested changes, the types themselves are different
                            changes.push(FieldChange {
                                field_name: field_path,
                                old_type: old_field_type.to_string(),
                                new_type: new_field_type.to_string(),
                                change_type: ChangeType::Modified,
                                required_changed: false,
                                default_changed: false,
                            });
                        } else {
                            changes.extend(nested_changes);
                        }
                    }
                }
            }
            // Nullable types - unwrap and compare
            (ObjectType::Nullable(old_inner), ObjectType::Nullable(new_inner)) => {
                changes.extend(Self::compare_object_types(prefix, old_inner, new_inner));
            }
            // Different types entirely
            _ => {
                if old_type != new_type {
                    changes.push(FieldChange {
                        field_name: prefix.to_string(),
                        old_type: old_type.to_string(),
                        new_type: new_type.to_string(),
                        change_type: ChangeType::Modified,
                        required_changed: false,
                        default_changed: false,
                    });
                }
            }
        }

        changes
    }

    /// Compare two access definitions
    fn compare_accesses(
        old_access: &AccessDefinition,
        new_access: &AccessDefinition,
    ) -> Option<AccessChange> {
        let mut changes = Vec::new();

        // Compare access type
        if old_access.access_type != new_access.access_type {
            changes.push(AccessChangeType::OtherChange(format!(
                "Access type changed from {:?} to {:?}",
                old_access.access_type, new_access.access_type
            )));
        }

        // Compare database level
        if old_access.database_level != new_access.database_level {
            let old_level = if old_access.database_level {
                "DATABASE"
            } else {
                "NAMESPACE"
            };
            let new_level = if new_access.database_level {
                "DATABASE"
            } else {
                "NAMESPACE"
            };
            changes.push(AccessChangeType::OtherChange(format!(
                "Access level changed from {} to {}",
                old_level, new_level
            )));
        }

        // Compare signup query
        if old_access.signup_query != new_access.signup_query {
            changes.push(AccessChangeType::SignupChanged);
        }

        // Compare signin query
        if old_access.signin_query != new_access.signin_query {
            changes.push(AccessChangeType::SigninChanged);
        }

        // Compare JWT configuration
        if old_access.jwt_algorithm != new_access.jwt_algorithm {
            changes.push(AccessChangeType::OtherChange(format!(
                "JWT algorithm changed from {:?} to {:?}",
                old_access.jwt_algorithm, new_access.jwt_algorithm
            )));
        }

        if old_access.jwt_key != new_access.jwt_key {
            changes.push(AccessChangeType::JwtKeyChanged);
        }

        if old_access.jwt_url != new_access.jwt_url {
            changes.push(AccessChangeType::JwtUrlChanged);
        }

        if old_access.issuer_key != new_access.issuer_key {
            changes.push(AccessChangeType::IssuerKeyChanged);
        }

        // Compare authenticate clause
        if old_access.authenticate != new_access.authenticate {
            changes.push(AccessChangeType::AuthenticateClauseChanged);
        }

        // Compare durations
        if old_access.duration_for_token != new_access.duration_for_token {
            changes.push(AccessChangeType::DurationChanged);
        }

        if old_access.duration_for_session != new_access.duration_for_session {
            changes.push(AccessChangeType::DurationChanged);
        }

        // Compare bearer configuration
        if old_access.bearer_for != new_access.bearer_for {
            changes.push(AccessChangeType::OtherChange(format!(
                "Bearer FOR changed from {:?} to {:?}",
                old_access.bearer_for, new_access.bearer_for
            )));
        }

        if changes.is_empty() {
            None
        } else {
            Some(AccessChange {
                access_name: old_access.name.clone(),
                changes,
            })
        }
    }
}

/// Filter tables and objects to only include those with changes that need to be regenerated
///
/// This function takes a SchemaChanges instance and the full tables and objects hashmaps,
/// then returns new hashmaps containing only the tables and fields that have been changed
/// and need to be regenerated.
///
/// # Arguments
/// * `schema_changes` - The schema changes detected between old and new schemas
/// * `tables` - The full HashMap of table configurations
/// * `objects` - The full HashMap of object/struct configurations
/// * `enums` - The full HashMap of enum configurations
/// * `record_diffs` - Map of table names to record count differences
///
/// # Returns
/// A tuple of (filtered_tables, filtered_objects) containing only changed elements
pub fn filter_changed_tables_and_objects(
    schema_changes: &SchemaChanges,
    tables: &HashMap<String, TableConfig>,
    objects: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
    record_diffs: &HashMap<String, i32>,
) -> (HashMap<String, TableConfig>, HashMap<String, StructConfig>) {
    tracing::debug!("Filtering changed tables and objects");

    let mut filtered_tables = HashMap::new();
    let mut filtered_objects = HashMap::new();

    // Process new tables - these need full regeneration
    for table_name in &schema_changes.new_tables {
        if let Some(table_config) = tables.get(table_name) {
            tracing::trace!(table = %table_name, "Adding new table for full regeneration");
            filtered_tables.insert(table_name.clone(), table_config.clone());
        }
    }

    // Process tables that need more records (positive diffs)
    for (table_name, diff) in record_diffs {
        if *diff > 0 {
            tracing::trace!(
                table = %table_name,
                record_diff = diff,
                "Table needs additional records"
            );
            // This table needs additional records, so include it for regeneration
            if let Some(table_config) = tables.get(table_name) {
                // Only add if not already included from new_tables
                if !filtered_tables.contains_key(table_name) {
                    let mut modified_config = table_config.clone();
                    // Set n to the number of records we need to generate
                    if let Some(ref mut mock_config) = modified_config.mock_generation_config {
                        mock_config.n = *diff as usize;
                    }
                    filtered_tables.insert(table_name.clone(), modified_config);
                }
            }
        }
    }

    // Process all tables based on their PreservationMode
    for (table_name, table_config) in tables {
        // Skip if already processed as new table
        if schema_changes.new_tables.contains(table_name) {
            continue;
        }

        // Get preservation mode from table's mock generation config
        let preservation_mode = table_config
            .mock_generation_config
            .as_ref()
            .map(|config| config.preservation_mode.clone())
            .unwrap_or(PreservationMode::Smart);

        match preservation_mode {
            PreservationMode::None => {
                // Return the full unfiltered table
                filtered_tables.insert(table_name.clone(), table_config.clone());
            }
            PreservationMode::Full => {
                // Only generate for new items (new fields and fields with always_regenerate)
                if let Some(table_change) = schema_changes
                    .modified_tables
                    .iter()
                    .find(|tc| &tc.table_name == table_name)
                {
                    if !table_change.new_fields.is_empty()
                        || table_config
                            .struct_config
                            .fields
                            .iter()
                            .any(|f| f.always_regenerate)
                    {
                        let mut filtered_table = table_config.clone();
                        let mut filtered_fields = Vec::new();

                        // Only include new fields
                        for field_name in &table_change.new_fields {
                            if let Some(field) = table_config
                                .struct_config
                                .fields
                                .iter()
                                .find(|f| &f.field_name == field_name)
                            {
                                filtered_fields.push(field.clone());
                            }
                        }

                        // Include fields with always_regenerate
                        for field in &table_config.struct_config.fields {
                            if field.always_regenerate
                                && !filtered_fields
                                    .iter()
                                    .any(|f| f.field_name == field.field_name)
                            {
                                filtered_fields.push(field.clone());
                            }
                        }

                        if !filtered_fields.is_empty() {
                            filtered_table.struct_config.fields = filtered_fields;
                            filtered_tables.insert(table_name.clone(), filtered_table);
                        }
                    }
                } else {
                    // Check if any fields have always_regenerate
                    if table_config
                        .struct_config
                        .fields
                        .iter()
                        .any(|f| f.always_regenerate)
                    {
                        let mut filtered_table = table_config.clone();
                        let mut filtered_fields = Vec::new();

                        // Only include fields with always_regenerate
                        for field in &table_config.struct_config.fields {
                            if field.always_regenerate {
                                filtered_fields.push(field.clone());
                            }
                        }

                        if !filtered_fields.is_empty() {
                            filtered_table.struct_config.fields = filtered_fields;
                            filtered_tables.insert(table_name.clone(), filtered_table);
                        }
                    }
                }
            }
            PreservationMode::Smart => {
                // Process as currently implemented (default behavior)
                // Check if this table has modifications
                if let Some(table_change) = schema_changes
                    .modified_tables
                    .iter()
                    .find(|tc| &tc.table_name == table_name)
                {
                    let mut filtered_table = table_config.clone();
                    let mut filtered_fields = Vec::new();

                    // Collect all field names that need regeneration
                    let mut fields_to_include = HashSet::new();

                    // Add new fields
                    for field_name in &table_change.new_fields {
                        fields_to_include.insert(field_name.clone());
                    }

                    // Track nested field changes to build partial objects
                    let mut nested_changes: HashMap<
                        String,
                        Vec<(String, FieldType, crate::schemasync::compare::ChangeType)>,
                    > = HashMap::new();

                    // Add modified fields
                    for field_change in &table_change.modified_fields {
                        use crate::schemasync::compare::ChangeType;

                        if field_change.field_name.contains('.') {
                            // Parse nested field (e.g., "recurrence_rule.interval")
                            let parts: Vec<&str> = field_change.field_name.split('.').collect();
                            let parent_field = parts[0].to_string();
                            let nested_field = parts[1].to_string();

                            match field_change.change_type {
                                ChangeType::Removed => {
                                    // Track removed nested fields
                                    nested_changes.entry(parent_field).or_default().push((
                                        nested_field,
                                        FieldType::Unit,
                                        ChangeType::Removed,
                                    ));
                                }
                                ChangeType::Added => {
                                    // For added nested fields, create partial struct similar to removed fields
                                    // We need to find the actual field type from the table definition
                                    if let Some(parent_field_def) = table_config
                                        .struct_config
                                        .fields
                                        .iter()
                                        .find(|f| f.field_name == parent_field)
                                    {
                                        // Extract the nested field type from the parent field
                                        match &parent_field_def.field_type {
                                            FieldType::Option(inner) => {
                                                if let FieldType::Other(struct_name) = &**inner {
                                                    // Find the struct definition and get the nested field type
                                                    if let Some(struct_def) =
                                                        objects.get(struct_name)
                                                        && let Some(nested_field_def) = struct_def
                                                            .fields
                                                            .iter()
                                                            .find(|f| f.field_name == nested_field)
                                                    {
                                                        nested_changes
                                                            .entry(parent_field.clone())
                                                            .or_default()
                                                            .push((
                                                                nested_field.clone(),
                                                                nested_field_def.field_type.clone(),
                                                                ChangeType::Added,
                                                            ));
                                                    }
                                                }
                                            }
                                            FieldType::Other(struct_name) => {
                                                // Find the struct definition and get the nested field type
                                                if let Some(struct_def) = objects.get(struct_name)
                                                    && let Some(nested_field_def) = struct_def
                                                        .fields
                                                        .iter()
                                                        .find(|f| f.field_name == nested_field)
                                                {
                                                    nested_changes
                                                        .entry(parent_field.clone())
                                                        .or_default()
                                                        .push((
                                                            nested_field.clone(),
                                                            nested_field_def.field_type.clone(),
                                                            ChangeType::Added,
                                                        ));
                                                }
                                            }
                                            _ => {
                                                // Fallback to including the parent field
                                                fields_to_include.insert(parent_field);
                                            }
                                        }
                                    }
                                }
                                ChangeType::Modified => {
                                    // For modified nested fields, include the parent field
                                    fields_to_include.insert(parent_field);
                                }
                            }
                        } else {
                            // For non-nested fields, include them normally
                            fields_to_include.insert(field_change.field_name.clone());
                        }
                    }

                    // Build partial struct fields for nested changes
                    for (parent_field_name, changes) in nested_changes {
                        // Create a struct with only the changed nested fields
                        let mut nested_fields = Vec::new();
                        for (nested_field_name, field_type, _change_type) in changes {
                            nested_fields.push((nested_field_name, field_type));
                        }

                        // Create a field representing the partial object
                        let partial_struct_field = StructField {
                            field_name: parent_field_name,
                            field_type: FieldType::Struct(nested_fields),
                            edge_config: None,
                            define_config: None,
                            format: None,
                            validators: Vec::new(),
                            always_regenerate: false,
                        };
                        filtered_fields.push(partial_struct_field);
                    }

                    // Add removed fields as Unit type
                    for removed_field_name in &table_change.removed_fields {
                        let unit_field = StructField {
                            field_name: removed_field_name.clone(),
                            field_type: FieldType::Unit,
                            edge_config: None,
                            define_config: None,
                            format: None,
                            validators: Vec::new(),
                            always_regenerate: false,
                        };
                        filtered_fields.push(unit_field);
                    }

                    // Filter the fields in the struct config
                    for field in &table_config.struct_config.fields {
                        if fields_to_include.contains(&field.field_name) || field.always_regenerate
                        {
                            filtered_fields.push(field.clone());
                        }
                    }

                    // Only add the table if it has fields that need regeneration or removal
                    if !filtered_fields.is_empty()
                        || table_change.permission_changed
                        || table_change.schema_type_changed
                        || !table_change.removed_fields.is_empty()
                    {
                        filtered_table.struct_config.fields = filtered_fields;
                        filtered_tables.insert(table_name.clone(), filtered_table);
                    }
                } else {
                    // No changes in schema, but check if any fields have always_regenerate
                    if table_config
                        .struct_config
                        .fields
                        .iter()
                        .any(|f| f.always_regenerate)
                    {
                        let mut filtered_table = table_config.clone();
                        let mut filtered_fields = Vec::new();

                        // Only include fields with always_regenerate
                        for field in &table_config.struct_config.fields {
                            if field.always_regenerate {
                                filtered_fields.push(field.clone());
                            }
                        }

                        if !filtered_fields.is_empty() {
                            filtered_table.struct_config.fields = filtered_fields;
                            filtered_tables.insert(table_name.clone(), filtered_table);
                        }
                    }
                }
            }
        }
    }

    // Process objects - check if any referenced objects need to be included
    // This handles nested structs that might be used by changed tables
    let mut processed_objects = HashSet::new();
    let mut objects_to_process = Vec::new();

    // Collect all object types referenced by filtered tables
    for table in filtered_tables.values() {
        for field in &table.struct_config.fields {
            collect_referenced_objects(&field.field_type, &mut objects_to_process, enums);
        }
    }

    // Process all referenced objects
    while let Some(object_name) = objects_to_process.pop() {
        if processed_objects.contains(&object_name) {
            continue;
        }

        processed_objects.insert(object_name.clone());

        if let Some(object_config) = objects.get(&object_name) {
            filtered_objects.insert(object_name.clone(), object_config.clone());

            // Check for nested object references
            for field in &object_config.fields {
                collect_referenced_objects(&field.field_type, &mut objects_to_process, enums);
            }
        }
    }

    tracing::debug!(
        filtered_table_count = filtered_tables.len(),
        filtered_object_count = filtered_objects.len(),
        "Filtering complete"
    );

    (filtered_tables, filtered_objects)
}

/// Helper function to collect object type names referenced in a field type
fn collect_referenced_objects(
    field_type: &FieldType,
    objects_to_process: &mut Vec<String>,
    enums: &HashMap<String, TaggedUnion>,
) {
    match field_type {
        FieldType::Other(type_name) => {
            // Check if this is an enum
            if let Some(enum_def) = enums.get(type_name) {
                // For enums, we need to collect all variant data types
                for variant in &enum_def.variants {
                    if let Some(variant_data) = &variant.data {
                        match variant_data {
                            VariantData::InlineStruct(enum_struct) => {
                                objects_to_process.push(enum_struct.struct_name.clone())
                            }
                            VariantData::DataStructureRef(referenced_field_type) => {
                                if let FieldType::Other(data) = referenced_field_type {
                                    objects_to_process.push(data.clone());
                                }
                            }
                        }
                    }
                }
            } else {
                // Not an enum, just a regular object/struct
                objects_to_process.push(type_name.clone());
            }
        }
        FieldType::Option(inner) | FieldType::Vec(inner) | FieldType::RecordLink(inner) => {
            collect_referenced_objects(inner, objects_to_process, enums);
        }
        FieldType::Tuple(types) => {
            for t in types {
                collect_referenced_objects(t, objects_to_process, enums);
            }
        }
        FieldType::Struct(fields) => {
            for (_, field_type) in fields {
                collect_referenced_objects(field_type, objects_to_process, enums);
            }
        }
        FieldType::HashMap(key_type, value_type) | FieldType::BTreeMap(key_type, value_type) => {
            collect_referenced_objects(key_type, objects_to_process, enums);
            collect_referenced_objects(value_type, objects_to_process, enums);
        }
        _ => {}
    }
}

pub async fn compare_schemas(
    db: &Surreal<Client>,
    remote_schema_string: &str,
    new_schema_string: &str,
) -> Result<SchemaChanges> {
    tracing::debug!("Parsing and comparing schema exports");
    let importer = SchemaImporter::new(db);

    let schema_changes = Comparator::compare(
        &importer
            .parse_schema_from_export(remote_schema_string)
            .expect("something went wrong parsing remote schema export"),
        &importer
            .parse_schema_from_export(new_schema_string)
            .expect("something went wrong parsing new schema export"),
    )?;

    evenframe_log!(format!("{:#?}", schema_changes), "changes.log");
    Ok(schema_changes)
}

pub async fn export_schemas(
    remote_schema: &Surreal<Db>,
    new_schema: &Surreal<Db>,
) -> Result<(String, String)> {
    tracing::trace!("Exporting remote schema");
    remote_schema
        .export("remote_schema.surql")
        .with_config()
        .versions(false)
        .accesses(true)
        .analyzers(false)
        .functions(false)
        .records(false)
        .params(false)
        .users(false)
        .await
        .map_err(|e| {
            EvenframeError::database(format!(
                "There was a problem exporting the 'remote_schema' embedded database's schema: {e}"
            ))
        })?;
    let mut remote_schema_string = String::new();
    std::io::Read::read_to_string(
        &mut std::fs::File::open("remote_schema.surql")?,
        &mut remote_schema_string,
    )
    .expect("Something went wrong reading the file to string");

    tracing::trace!("Exporting new schema");
    new_schema
        .export("new_schema.surql")
        .with_config()
        .versions(false)
        .accesses(true)
        .analyzers(false)
        .functions(false)
        .records(false)
        .params(false)
        .users(false)
        .await
        .map_err(|e| {
            EvenframeError::database(format!(
                "There was a problem exporting the 'new_schema' embedded database's schema: {e}"
            ))
        })?;
    let mut new_schema_string = String::new();
    std::io::Read::read_to_string(
        &mut std::fs::File::open("new_schema.surql")?,
        &mut new_schema_string,
    )
    .expect("Something went wrong reading the file to string");

    tracing::trace!("Schema export complete");
    Ok((remote_schema_string, new_schema_string))
}

pub async fn setup_backup_and_schemas(db: &Surreal<Client>) -> Result<(Surreal<Db>, Surreal<Db>)> {
    tracing::trace!("Creating database backup");
    db.export("backup.surql").await.map_err(|e| {
        EvenframeError::database(format!(
            "There was a problem exporting the remote database to 'backup.surql': {e}"
        ))
    })?;
    let remote_schema = Surreal::new::<Mem>(())
        .await
        .expect("Something went wrong starting the remote_schema in-memory db");

    let mut backup = String::new();
    std::io::Read::read_to_string(&mut std::fs::File::open("backup.surql")?, &mut backup)
        .expect("Something went wrong reading the file to string");

    tracing::trace!("Importing backup to remote in-memory schema");
    remote_schema
        .use_ns("remote")
        .use_db("backup")
        .await
        .map_err(|e| {
            EvenframeError::database(format!(
                "There was a problem using the namespace or db for 'remote_schema': {e}"
            ))
        })?;

    remote_schema.query(&backup).await.map_err(|e| {
        EvenframeError::database(format!(
            "Something went wrong importing the remote schema to the in-memory db: {e}"
        ))
    })?;

    let new_schema = Surreal::new::<Mem>(()).await.map_err(|e| {
        EvenframeError::database(format!(
            "Something went wrong starting the new_schema in-memory db: {e}"
        ))
    })?;

    tracing::trace!("Setting up new in-memory schema");
    new_schema
        .use_ns("new")
        .use_db("memory")
        .await
        .map_err(|e| {
            EvenframeError::database(format!(
                "There was a problem exporting the 'remote_schema' embedded database's schema: {e}"
            ))
        })?;

    tracing::trace!("In-memory schemas ready");
    Ok((remote_schema, new_schema))
}
