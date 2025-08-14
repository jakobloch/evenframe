pub mod coordinate;
pub mod field_value;
pub mod format;
pub mod regex_val_gen;

use crate::{
    compare::{filter_changed_tables_and_objects, Comparator},
    coordinate::{CoordinateIncrement, DerivationType, ExtractType, TransformType},
    dependency::sort_tables_by_dependencies,
    evenframe_log,
    mockmake::format::Format,
    schemasync::{
        compare::PreservationMode, surql::access::execute_access_query, StructConfig, TableConfig,
        TaggedUnion,
    },
    types::StructField,
    wrappers::EvenframeRecordId,
};
use bon::Builder;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use convert_case::{Case, Casing};
use rand::Rng;
use std::collections::HashMap;
use surrealdb::engine::local::Db;
use surrealdb::engine::remote::http::Client;
use surrealdb::Surreal;
use tracing;

#[derive(Debug, Builder)]
pub struct Mockmaker {
    db: Surreal<Client>,
    pub(super) tables: HashMap<String, TableConfig>,
    objects: HashMap<String, StructConfig>,
    enums: HashMap<String, TaggedUnion>,
    pub(super) schemasync_config: crate::schemasync::config::SchemasyncConfig,
    pub comparator: Option<Comparator>,

    // Runtime state
    pub(super) id_map: HashMap<String, Vec<String>>,
    pub(super) record_diffs: HashMap<String, i32>,
    filtered_tables: HashMap<String, TableConfig>,
    filtered_objects: HashMap<String, StructConfig>,
}

impl Mockmaker {
    pub fn new(
        db: Surreal<Client>,
        tables: HashMap<String, TableConfig>,
        objects: HashMap<String, StructConfig>,
        enums: HashMap<String, TaggedUnion>,
        schemasync_config: crate::schemasync::config::SchemasyncConfig,
    ) -> Self {
        Self {
            db: db.clone(),
            tables,
            objects,
            enums,
            schemasync_config: schemasync_config.clone(),
            comparator: Some(Comparator::new(db, schemasync_config)),
            id_map: HashMap::new(),
            record_diffs: HashMap::new(),
            filtered_tables: HashMap::new(),
            filtered_objects: HashMap::new(),
        }
    }

    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Starting Mockmaker pipeline");

        // Step 1: Generate IDs
        tracing::debug!("Step 1: Generating IDs for mock data");
        self.generate_ids().await?;

        // Step 2: Run comparator pipeline
        tracing::debug!("Step 2: Running comparator pipeline");
        let comparator = self.comparator.take().unwrap();
        self.comparator = Some(comparator.run().await?);

        // Step 3: Run remaining mockmaker steps
        tracing::debug!("Step 3: Removing old data based on schema changes");
        self.remove_old_data().await?;

        tracing::debug!("Step 4: Executing access queries");
        self.execute_access().await?;

        tracing::debug!("Step 5: Filtering changed tables and objects");
        self.filter_changes().await?;

        tracing::debug!("Step 6: Generating mock data");
        self.generate_mock_data().await?;

        tracing::info!("Mockmaker pipeline completed successfully");
        Ok(())
    }

    /// Generate IDs for tables
    pub async fn generate_ids(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::trace!("Starting ID generation for all tables");
        let mut map = HashMap::new();
        let mut record_diffs = HashMap::new();

        // Process tables sequentially to avoid reference issues
        // Since these are just SELECT queries, they should be fast enough
        for (table_name, table_config) in &self.tables {
            tracing::trace!(table = %table_name, "Generating IDs for table");
            let snake_case_table_name = table_name.to_case(Case::Snake);

            // Determine desired count from config or default
            let desired_count =
                if let Some(mock_generation_config) = &table_config.mock_generation_config {
                    mock_generation_config.n
                } else {
                    self.schemasync_config.mock_gen_config.default_batch_size
                };

            // Query existing IDs
            let query = format!("SELECT id FROM {snake_case_table_name};",);
            tracing::trace!(query = %query, "Querying existing IDs");
            let mut response = self.db.query(query).await.expect(
                "Something went wrong getting the ids from the db for mock data generation",
            );

            let existing_ids: Vec<EvenframeRecordId> =
                response.take(0).unwrap_or_else(|_| Vec::new());

            let mut ids = Vec::new();
            let existing_count = existing_ids.len();

            // Calculate the difference between existing and desired counts
            let record_diff = desired_count as i32 - existing_count as i32;

            tracing::trace!(
                table = %table_name,
                existing_count = existing_count,
                desired_count = desired_count,
                record_diff = record_diff,
                "Calculated record difference"
            );

            // Store the difference in the record_diffs map
            record_diffs.insert(table_name.clone(), record_diff);
            record_diffs.insert(snake_case_table_name.clone(), record_diff);

            if existing_count >= desired_count {
                // We have enough or more IDs than needed
                // Just use the first desired_count IDs
                for (i, record_id) in existing_ids.into_iter().enumerate() {
                    if i < desired_count {
                        let id_string = record_id.to_string();
                        ids.push(id_string);
                    } else {
                        // Stop after we have enough
                        break;
                    }
                }
            } else {
                // We need to use existing IDs and generate more
                // First, use all existing IDs
                for record_id in existing_ids {
                    ids.push(record_id.to_string());
                }

                // Generate additional IDs
                let mut next_id = existing_count + 1;
                while ids.len() < desired_count {
                    ids.push(format!("{snake_case_table_name}:{next_id}"));
                    next_id += 1;
                }
            }

            // Store with both the original key and snake_case key for easier lookup
            map.insert(table_name.clone(), ids.clone());
            map.insert(snake_case_table_name, ids);
        }

        self.id_map = map;
        self.record_diffs = record_diffs;

        tracing::debug!(table_count = self.id_map.len(), "ID generation complete");

        evenframe_log!(
            format!("Record count differences: {:#?}", self.record_diffs),
            "record_diffs.log"
        );

        Ok(())
    }

    /// Remove old data based on schema changes
    pub async fn remove_old_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::trace!("Removing old data based on schema changes");
        let comparator = self.comparator.as_ref().unwrap();
        let schema_changes = comparator.get_schema_changes().unwrap();

        let remove_statements = self.generate_remove_statements(schema_changes);

        tracing::debug!(
            statement_length = remove_statements.len(),
            "Generated remove statements"
        );

        evenframe_log!(&remove_statements, "remove_statements.surql");

        if !remove_statements.is_empty() {
            tracing::trace!("Executing remove statements");
            self.db.query(remove_statements).await?;
        }

        tracing::trace!("Old data removal complete");
        Ok(())
    }

    /// Execute access query on main database
    pub async fn execute_access(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::trace!("Executing access definitions");
        let comparator = self.comparator.as_ref().unwrap();
        let access_query = comparator.get_access_query();

        tracing::debug!(query_length = access_query.len(), "Executing access query");

        execute_access_query(&self.db, access_query).await
    }

    /// Filter changed tables and objects
    pub async fn filter_changes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::trace!("Filtering changes based on schema comparison");
        let comparator = self.comparator.as_ref().unwrap();
        let schema_changes = comparator.get_schema_changes().unwrap();

        let (filtered_tables, filtered_objects) =
            if self.schemasync_config.mock_gen_config.full_refresh_mode {
                tracing::debug!("Full refresh mode enabled - using all tables and objects");
                (self.tables.clone(), self.objects.clone())
            } else {
                tracing::debug!("Incremental mode - filtering changed items only");
                filter_changed_tables_and_objects(
                    schema_changes,
                    &self.tables,
                    &self.objects,
                    &self.enums,
                    &self.record_diffs,
                )
            };

        self.filtered_tables = filtered_tables;
        self.filtered_objects = filtered_objects;

        tracing::info!(
            filtered_tables = self.filtered_tables.len(),
            filtered_objects = self.filtered_objects.len(),
            "Filtering complete"
        );

        evenframe_log!(
            format!("{:#?}{:#?}", self.filtered_objects, self.filtered_tables),
            "filtered.log"
        );

        Ok(())
    }

    pub(super) async fn generate_mock_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::trace!("Starting mock data generation");

        // Sort tables by dependencies to ensure proper insertion order
        let sorted_table_names =
            sort_tables_by_dependencies(&self.filtered_tables, &self.filtered_objects, &self.enums);

        tracing::debug!(
            table_count = sorted_table_names.len(),
            "Tables sorted by dependencies"
        );

        evenframe_log!(
            &format!("Sorted table order: {sorted_table_names:?}"),
            "results.log",
            true
        );

        for table_name in sorted_table_names {
            if let Some(table) = &self.filtered_tables.get(&table_name) {
                let snake_table_name = &table_name.to_case(Case::Snake);

                tracing::trace!(
                    table = %table_name,
                    is_relation = table.relation.is_some(),
                    "Processing table for mock data"
                );

                if self.schemasync_config.should_generate_mocks {
                    let stmts = if table.relation.is_some() {
                        tracing::trace!(table = %table_name, "Generating INSERT statements for relation");
                        self.generate_insert_statements(snake_table_name, table)
                    } else {
                        tracing::trace!(table = %table_name, "Generating UPSERT statements for table");
                        self.generate_upsert_statements(snake_table_name, table)
                    };

                    tracing::debug!(
                        table = %table_name,
                        statement_count = stmts.lines().count(),
                        "Generated mock data statements"
                    );

                    evenframe_log!(&stmts, "all_statements.surql", true);

                    // Execute and validate upsert statements
                    use crate::schemasync::surql::execute::execute_and_validate;

                    match execute_and_validate(&self.db, &stmts, "UPSERT", &table_name).await {
                        Ok(_results) => {
                            tracing::debug!(table = %table_name, "Mock data inserted successfully");
                        }
                        Err(e) => {
                            tracing::error!(
                                table = %table_name,
                                error = %e,
                                "Failed to execute statements"
                            );
                            let error_msg = format!(
                                "Failed to execute upsert statements for table {}: {}",
                                table_name, e
                            );
                            evenframe_log!(&error_msg, "results.log", true);
                            return Err(e);
                        }
                    }
                }
            }
        }
        tracing::info!("Mock data generation complete");
        Ok(())
    }

    // Getter for new_schema so Schemasync can access it
    pub fn get_new_schema(&self) -> Option<&Surreal<Db>> {
        self.comparator.as_ref()?.get_new_schema()
    }

    /// Generate sequential values for fields
    pub fn generate_sequential_values(
        fields: &[&StructField],
        _index: usize,
        increment: &CoordinateIncrement,
    ) -> HashMap<String, String> {
        tracing::trace!(field_count = fields.len(), "Generating sequential values");
        let mut values = HashMap::new();

        // Generate base value
        let first_field = &fields[0];

        match &first_field.format {
            Some(Format::DateTime) => {
                // Generate base datetime
                let base: DateTime<Utc> = Utc::now();
                values.insert(first_field.field_name.clone(), base.to_rfc3339());

                // Generate subsequent values
                for (i, field) in fields.iter().skip(1).enumerate() {
                    let incremented = match increment {
                        CoordinateIncrement::Days(d) => {
                            base + Duration::days(*d as i64 * (i + 1) as i64)
                        }
                        CoordinateIncrement::Hours(h) => {
                            base + Duration::hours(*h as i64 * (i + 1) as i64)
                        }
                        CoordinateIncrement::Minutes(m) => {
                            base + Duration::minutes(*m as i64 * (i + 1) as i64)
                        }
                        _ => base, // Fallback to base if increment type doesn't match
                    };
                    values.insert(field.field_name.clone(), incremented.to_rfc3339());
                }
            }
            Some(Format::Date) => {
                // Generate base date
                let base = NaiveDate::from_ymd_opt(2024, 1, 1)
                    .or_else(|| NaiveDate::from_ymd_opt(2024, 1, 2))
                    .or_else(|| NaiveDate::from_ymd_opt(2023, 12, 31))
                    .expect("At least one of these dates should be valid");
                values.insert(first_field.field_name.clone(), base.to_string());

                // Generate subsequent values
                for (i, field) in fields.iter().skip(1).enumerate() {
                    let incremented = match increment {
                        CoordinateIncrement::Days(d) => {
                            base + Duration::days(*d as i64 * (i + 1) as i64)
                        }
                        _ => base, // For dates, only day increment makes sense
                    };
                    values.insert(field.field_name.clone(), incremented.to_string());
                }
            }
            Some(Format::DateWithinDays(_)) => {
                // Generate base datetime for DateWithinDays format
                let base: DateTime<Utc> = Utc::now();
                values.insert(first_field.field_name.clone(), base.to_rfc3339());

                // Generate subsequent values
                for (i, field) in fields.iter().skip(1).enumerate() {
                    let incremented = match increment {
                        CoordinateIncrement::Days(d) => {
                            base + Duration::days(*d as i64 * (i + 1) as i64)
                        }
                        CoordinateIncrement::Hours(h) => {
                            base + Duration::hours(*h as i64 * (i + 1) as i64)
                        }
                        CoordinateIncrement::Minutes(m) => {
                            base + Duration::minutes(*m as i64 * (i + 1) as i64)
                        }
                        _ => base, // Fallback to base if increment type doesn't match
                    };
                    values.insert(field.field_name.clone(), incremented.to_rfc3339());
                }
            }
            _ => {
                // Numeric sequential
                let mut rng = rand::rng();
                let base: f64 = rng.random_range(0.0..100.0);

                for (i, field) in fields.iter().enumerate() {
                    let value = match increment {
                        CoordinateIncrement::Numeric(n) => base + (n * i as f64),
                        _ => base + i as f64,
                    };
                    values.insert(field.field_name.clone(), value.to_string());
                }
            }
        }

        values
    }

    /// Generate values that sum to a total
    fn generate_sum_values(
        fields: &[&StructField],
        _index: usize,
        total: f64,
    ) -> HashMap<String, String> {
        tracing::trace!(
            field_count = fields.len(),
            total = total,
            "Generating sum values"
        );
        let mut values = HashMap::new();
        let mut rng = rand::rng();

        if fields.is_empty() {
            return values;
        }

        // Generate random percentages that sum to total
        let mut remaining = total;
        let mut generated_values = Vec::new();

        for i in 0..fields.len() - 1 {
            let max_value = remaining / (fields.len() - i) as f64 * 1.5; // Allow some variance
            let value = rng.random_range(0.0..max_value.min(remaining));
            generated_values.push(value);
            remaining -= value;
        }

        // Last value gets the remainder to ensure exact sum
        generated_values.push(remaining);

        // Assign values to fields, but handle rounding carefully for percentages
        let is_percentage = fields
            .iter()
            .any(|f| matches!(f.format, Some(Format::Percentage)));

        if is_percentage {
            // For percentages, we need to ensure the formatted values still sum to exactly 100
            let mut formatted_values = Vec::new();
            let mut formatted_sum = 0.0;

            // Format all but the last value
            for value in generated_values {
                let formatted = format!("{:.1}", value);
                let parsed = formatted.parse::<f64>().unwrap_or(value);
                formatted_sum += parsed;
                formatted_values.push(formatted);
            }

            // Calculate what the last value should be to maintain exact sum
            let last_value = total - formatted_sum;
            formatted_values.push(format!("{:.1}", last_value));

            // Assign the formatted values
            for (field, formatted_value) in fields.iter().zip(formatted_values.iter()) {
                values.insert(field.field_name.clone(), formatted_value.clone());
            }
        } else {
            // For non-percentage fields, use the original logic
            for (field, value) in fields.iter().zip(generated_values.iter()) {
                let formatted_value = match &field.format {
                    Some(Format::CurrencyAmount) => format!("${:.2}", value),
                    _ => format!("{:.2}", value),
                };
                values.insert(field.field_name.clone(), formatted_value);
            }
        }

        values
    }

    /// Generate derived values from source fields
    fn generate_derive_values(
        source_fields: &[&StructField],
        target_field: &str,
        derivation: &DerivationType,
        source_values: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        tracing::trace!(target_field = %target_field, "Generating derived values");
        let mut values = HashMap::new();

        match derivation {
            DerivationType::Concatenate(separator) => {
                let concatenated = source_fields
                    .iter()
                    .filter_map(|field| source_values.get(&field.field_name))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(separator);
                values.insert(target_field.to_string(), concatenated);
            }
            DerivationType::Extract(extract_type) => {
                if let Some(first_field) = source_fields.first() {
                    if let Some(source_value) = source_values.get(&first_field.field_name) {
                        let extracted = match extract_type {
                            ExtractType::FirstWord => source_value
                                .split_whitespace()
                                .next()
                                .unwrap_or("")
                                .to_string(),
                            ExtractType::LastWord => source_value
                                .split_whitespace()
                                .last()
                                .unwrap_or("")
                                .to_string(),
                            ExtractType::Domain => {
                                // Extract domain from email
                                source_value.split('@').nth(1).unwrap_or("").to_string()
                            }
                            ExtractType::Username => {
                                // Extract username from email
                                source_value.split('@').next().unwrap_or("").to_string()
                            }
                            ExtractType::Initials => {
                                // Extract initials from words
                                source_value
                                    .split_whitespace()
                                    .filter_map(|word| word.chars().next())
                                    .collect::<String>()
                                    .to_uppercase()
                            }
                        };
                        values.insert(target_field.to_string(), extracted);
                    }
                }
            }
            DerivationType::Transform(transform_type) => {
                if let Some(first_field) = source_fields.first() {
                    if let Some(source_value) = source_values.get(&first_field.field_name) {
                        let transformed = match transform_type {
                            TransformType::Uppercase => source_value.to_uppercase(),
                            TransformType::Lowercase => source_value.to_lowercase(),
                            TransformType::Capitalize => {
                                let mut chars = source_value.chars();
                                match chars.next() {
                                    None => String::new(),
                                    Some(first) => {
                                        first.to_uppercase().collect::<String>() + chars.as_str()
                                    }
                                }
                            }
                            TransformType::Truncate(len) => {
                                source_value.chars().take(*len).collect()
                            }
                            TransformType::Hash => {
                                // Simple hash representation
                                format!(
                                    "{:x}",
                                    source_value.len() * 31
                                        + source_value.chars().map(|c| c as usize).sum::<usize>()
                                )
                            }
                        };
                        values.insert(target_field.to_string(), transformed);
                    }
                }
            }
        }

        values
    }

    /// Generate coherent values from predefined datasets
    fn generate_coherent_values(
        _fields: &[&StructField],
        dataset: &crate::coordinate::CoherentDataset,
        index: usize,
    ) -> HashMap<String, String> {
        tracing::trace!(index = index, "Generating coherent values");
        use crate::coordinate::*;

        /// Coherent address data
        const COHERENT_ADDRESSES: &[(&str, &str, &str, &str)] = &[
            ("New York", "NY", "10001", "USA"),
            ("Los Angeles", "CA", "90001", "USA"),
            ("Chicago", "IL", "60601", "USA"),
            ("Houston", "TX", "77001", "USA"),
            ("Phoenix", "AZ", "85001", "USA"),
            ("Philadelphia", "PA", "19101", "USA"),
            ("San Antonio", "TX", "78201", "USA"),
            ("San Diego", "CA", "92101", "USA"),
            ("Dallas", "TX", "75201", "USA"),
            ("San Jose", "CA", "95101", "USA"),
        ];

        /// Coherent geo location data
        const CITY_COORDINATES: &[(&str, f64, f64, &str)] = &[
            ("New York", 40.7128, -74.0060, "USA"),
            ("Los Angeles", 34.0522, -118.2437, "USA"),
            ("Chicago", 41.8781, -87.6298, "USA"),
            ("Houston", 29.7604, -95.3698, "USA"),
            ("Phoenix", 33.4484, -112.0740, "USA"),
            ("Philadelphia", 39.9526, -75.1652, "USA"),
            ("San Antonio", 29.4241, -98.4936, "USA"),
            ("San Diego", 32.7157, -117.1611, "USA"),
            ("Dallas", 32.7767, -96.7970, "USA"),
            ("San Jose", 37.3382, -121.8863, "USA"),
        ];

        match dataset {
            CoherentDataset::Address {
                city,
                state,
                zip,
                country,
            } => {
                let (city_val, state_val, zip_val, country_val) =
                    COHERENT_ADDRESSES[index % COHERENT_ADDRESSES.len()];
                let mut values = HashMap::new();
                values.insert(city.clone(), city_val.to_string());
                values.insert(state.clone(), state_val.to_string());
                values.insert(zip.clone(), zip_val.to_string());
                values.insert(country.clone(), country_val.to_string());
                values
            }
            CoherentDataset::PersonName {
                first_name,
                last_name,
                full_name,
            } => {
                // Use the extended person names from coordinate.rs
                let names = crate::coordinate::EXTENDED_PERSON_NAMES;
                let (first, last, _gender) = names[index % names.len()];
                let mut values = HashMap::new();
                values.insert(first_name.clone(), first.to_string());
                values.insert(last_name.clone(), last.to_string());
                values.insert(full_name.clone(), format!("{} {}", first, last));
                values
            }
            CoherentDataset::GeoLocation {
                latitude,
                longitude,
                city,
                country,
            } => {
                let (city_val, lat, lng, country_val) =
                    CITY_COORDINATES[index % CITY_COORDINATES.len()];
                let mut values = HashMap::new();
                values.insert(latitude.clone(), lat.to_string());
                values.insert(longitude.clone(), lng.to_string());
                values.insert(city.clone(), city_val.to_string());
                values.insert(country.clone(), country_val.to_string());
                values
            }
            CoherentDataset::DateRange {
                start_date,
                end_date,
            } => {
                // Generate coherent start/end dates
                let base = NaiveDate::from_ymd_opt(2024, 1, 1)
                    .or_else(|| NaiveDate::from_ymd_opt(2024, 1, 2))
                    .or_else(|| NaiveDate::from_ymd_opt(2023, 12, 31))
                    .expect("At least one of these dates should be valid");
                let start_offset = (index * 7) as i64; // Weekly intervals
                let duration_days = 14; // 2 week duration

                let start = base + Duration::days(start_offset);
                let end = start + Duration::days(duration_days);

                let mut values = HashMap::new();
                values.insert(start_date.clone(), start.to_string());
                values.insert(end_date.clone(), end.to_string());
                values
            }
        }
    }

    pub fn random_string(len: usize) -> String {
        use rand::distr::Alphanumeric;
        let mut rng = rand::rng();
        (0..len).map(|_| rng.sample(Alphanumeric) as char).collect()
    }
}

/// Unified configuration for mock data generation
/// Combines features from both MockGenerationConfig and merge::MockConfig
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MockGenerationConfig {
    // From original MockGenerationConfig
    pub n: usize,
    pub table_level_override: Option<HashMap<StructField, Format>>,
    pub coordination_rules: Vec<crate::schemasync::mockmake::coordinate::Coordination>,
    pub batch_size: usize,
    pub regenerate_fields: Vec<String>,
    pub preservation_mode: PreservationMode,
}

impl Default for MockGenerationConfig {
    fn default() -> Self {
        // Try to load config, fall back to hardcoded defaults if unavailable
        let (n, batch_size, preservation_mode) = match crate::config::EvenframeConfig::new() {
            Ok(config) => (
                config.schemasync.mock_gen_config.default_record_count,
                config.schemasync.mock_gen_config.default_batch_size,
                config.schemasync.mock_gen_config.default_preservation_mode,
            ),
            Err(_) => {
                // Fall back to reasonable defaults if config can't be loaded
                (10, 1000, PreservationMode::Smart)
            }
        };

        Self {
            n,
            table_level_override: None,
            coordination_rules: Vec::new(),
            batch_size,
            regenerate_fields: vec![],
            preservation_mode,
        }
    }
}

impl quote::ToTokens for MockGenerationConfig {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let n = self.n;
        let batch_size = self.batch_size;

        // Convert coordination rules to tokens
        let coordination_rules_tokens = if self.coordination_rules.is_empty() {
            quote::quote! { vec![] }
        } else {
            // We need to serialize coordination rules properly
            // For now, just create an empty vec as coordination rules need their own ToTokens impl
            quote::quote! { vec![] }
        };

        // Convert regenerate fields to tokens
        let regenerate_fields = &self.regenerate_fields;

        // Convert preservation mode to tokens
        let preservation_mode_tokens = match &self.preservation_mode {
            PreservationMode::Smart => {
                quote::quote! { ::evenframe::schemasync::compare::PreservationMode::Smart }
            }
            PreservationMode::Full => {
                quote::quote! { ::evenframe::schemasync::compare::PreservationMode::Full }
            }
            PreservationMode::None => {
                quote::quote! { ::evenframe::schemasync::compare::PreservationMode::None }
            }
        };

        // Generate the full config token stream
        let config_tokens = quote::quote! {
            MockGenerationConfig {
                n: #n,
                table_level_override: None,
                coordination_rules: #coordination_rules_tokens,
                batch_size: #batch_size,
                regenerate_fields: vec![#(#regenerate_fields.to_string()),*],
                preservation_mode: #preservation_mode_tokens,
            }
        };

        tokens.extend(config_tokens);
    }
}
