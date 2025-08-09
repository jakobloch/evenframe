pub mod coordinate;
pub mod format;
pub mod maker;

use crate::compare::{filter_changed_tables_and_objects, Comparator};
use crate::coordinate::{CoordinateIncrement, DerivationType, ExtractType, TransformType};
use crate::dependency::sort_tables_by_dependencies;
use crate::evenframe_log;
use crate::mockmake::format::Format;
use crate::schemasync::compare::PreservationMode;
use crate::schemasync::surql::access::execute_access_query;
use crate::schemasync::{random_string, StructConfig, TableConfig, TaggedUnion};
use crate::types::{FieldType, StructField, VariantData};
use crate::wrappers::EvenframeRecordId;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::TZ_VARIANTS;
use convert_case::{Case, Casing};
use rand::seq::IndexedRandom;
use rand::Rng;
use std::collections::HashMap;
use surrealdb::engine::local::Db;
use surrealdb::engine::remote::http::Client;
use surrealdb::Surreal;
use tracing;

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
            let query = format!("SELECT id FROM {};", snake_case_table_name);
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
                    ids.push(format!("{}:{}", snake_case_table_name, next_id));
                    next_id += 1;
                }
            }

            // Store with both the original key and snake_case key for easier lookup
            map.insert(table_name.clone(), ids.clone());
            map.insert(snake_case_table_name, ids);
        }

        self.id_map = map;
        self.record_diffs = record_diffs;

        tracing::debug!(
            table_count = self.id_map.len(),
            "ID generation complete"
        );
        
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
        
        tracing::debug!(
            query_length = access_query.len(),
            "Executing access query"
        );
        
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
            &format!("Sorted table order: {:?}", sorted_table_names),
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
        tracing::trace!(field_count = fields.len(), total = total, "Generating sum values");
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
            for i in 0..generated_values.len() - 1 {
                let formatted = format!("{:.1}", generated_values[i]);
                let parsed = formatted
                    .parse::<f64>()
                    .unwrap_or_else(|_| generated_values[i]);
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

    fn generate_field_value_with_coordination(
        &self,
        gen_details: &TableConfig,
        field_name: &String,
        table_name: Option<&String>,
        field_ty: &FieldType,
        id_index: Option<usize>,
        coordinated_values: &HashMap<String, String>,
    ) -> String {
        tracing::trace!(
            field_name = %field_name,
            field_type = ?field_ty,
            "Generating field value with coordination"
        );
        let mut rng = rand::rng();

        match field_ty {
            FieldType::String => {
                format!("'{}'", random_string(8))
            }
            FieldType::Char => {
                let c = rng.random_range(32u8..=126u8) as char;
                format!("'{}'", c)
            }
            FieldType::Bool => format!("{}", rng.random_bool(0.5)),
            FieldType::Unit => "NONE".to_string(),
            FieldType::Decimal => format!("{:.3}dec", rng.random_range(0.0..100.0)),
            FieldType::F32 | FieldType::F64 | FieldType::OrderedFloat(_) => {
                format!("{:.2}f", rng.random_range(0.0..100.0))
            }
            // Combine signed integer types
            FieldType::I8
            | FieldType::I16
            | FieldType::I32
            | FieldType::I64
            | FieldType::I128
            | FieldType::Isize => {
                format!("{}", rng.random_range(0..100))
            }
            // Combine unsigned integer types
            FieldType::U8
            | FieldType::U16
            | FieldType::U32
            | FieldType::U64
            | FieldType::U128
            | FieldType::Usize => {
                format!("{}", rng.random_range(0..100))
            }
            FieldType::DateTime => {
                // Generate ISO 8601 datetime string
                format!("d'{}'", chrono::Utc::now().to_rfc3339())
            }
            FieldType::Duration => {
                // Generate random duration in nanoseconds (0 to 1 day in nanos)
                let nanos = rng.random_range(0..86_400_000_000_000i64); // 0 to 24 hours
                format!("duration::from::nanos({})", nanos)
            }
            FieldType::Timezone => {
                // Generate random IANA timezone string from chrono_tz
                let tz = &TZ_VARIANTS[rng.random_range(0..TZ_VARIANTS.len())];
                format!("'{}'", tz.name())
            }
            FieldType::EvenframeRecordId => {
                if gen_details.relation.is_some() && field_name == "in" {
                    let from_table = &gen_details.relation.as_ref().unwrap().from;
                    if let Some(ids) = self.id_map.get(from_table) {
                        if !ids.is_empty() {
                            format!("r'{}'", ids[rng.random_range(0..ids.len())].clone())
                        } else {
                            format!("r'{}:1'", from_table.to_lowercase())
                        }
                    } else {
                        panic!(
                            "{}",
                            format!(
                                "There were no id's for the table {}, field {}",
                                table_name.unwrap_or(&"(not found)".to_string()),
                                field_name
                            )
                        )
                    }
                } else if gen_details.relation.is_some() && field_name == "out" {
                    let to_table = &gen_details.relation.as_ref().unwrap().to;
                    if let Some(ids) = self.id_map.get(to_table) {
                        if !ids.is_empty() {
                            format!("r'{}'", ids[rng.random_range(0..ids.len())].clone())
                        } else {
                            panic!(
                                "{}",
                                format!(
                                    "There were no id's for the table {}, field {}",
                                    table_name.unwrap_or(&"(not found)".to_string()),
                                    field_name
                                )
                            )
                        }
                    } else {
                        panic!(
                            "{}",
                            format!(
                                "There were no id's for the table {}, field {}",
                                table_name.unwrap_or(&"(not found)".to_string()),
                                field_name
                            )
                        )
                    }
                } else if let Some(table) = table_name {
                    if let Some(ids) = self.id_map.get(table) {
                        if let Some(idx) = id_index {
                            if idx < ids.len() {
                                format!("r'{}'", ids[idx].clone())
                            } else {
                                format!("r'{}:{}'", table.to_lowercase(), idx + 1)
                            }
                        } else {
                            panic!(
                                "{}",
                                format!(
                                    "There were no id's for the table {}, field {}",
                                    table_name.unwrap_or(&"(not found)".to_string()),
                                    field_name
                                )
                            )
                        }
                    } else {
                        format!("r'{}:{}'", table.to_lowercase(), id_index.unwrap_or(0) + 1)
                    }
                } else {
                    panic!(
                        "{}",
                        format!(
                            "There were no id's for the table {}, field {}",
                            table_name.unwrap_or(&"(not found)".to_string()),
                            field_name
                        )
                    )
                }
            }
            // For an Option, randomly decide whether to generate a value or use NULL.
            FieldType::Option(inner) => {
                if rng.random_bool(0.5) {
                    "null".to_string()
                } else {
                    self.generate_field_value_with_coordination(
                        gen_details,
                        field_name,
                        table_name,
                        inner,
                        id_index,
                        coordinated_values,
                    )
                }
            }
            // For a vector, generate a dummy array with a couple of elements.
            FieldType::Vec(inner) => {
                let count = rand::rng().random_range(2..10);

                let items: Vec<String> = (0..count)
                    .map(|_| {
                        self.generate_field_value_with_coordination(
                            gen_details,
                            field_name,
                            None,
                            inner,
                            None,
                            coordinated_values,
                        )
                    })
                    .collect();
                format!("[{}]", items.join(", "))
            }
            // For a tuple, recursively generate values for each component.
            FieldType::Tuple(types) => {
                let values: Vec<String> = types
                    .iter()
                    .map(|ty| {
                        self.generate_field_value_with_coordination(
                            gen_details,
                            field_name,
                            table_name,
                            ty,
                            id_index,
                            coordinated_values,
                        )
                    })
                    .collect();
                format!("({})", values.join(", "))
            }
            // For a struct (named fields), create a JSON-like object.
            FieldType::Struct(fields) => {
                // Build nested coordination context
                let mut nested_coordinated_values = HashMap::new();
                let field_prefix = format!("{}.", field_name);

                // Extract coordinated values for nested fields
                for (coord_field, coord_value) in coordinated_values {
                    if coord_field.starts_with(&field_prefix) {
                        let nested_field = &coord_field[field_prefix.len()..];
                        nested_coordinated_values
                            .insert(nested_field.to_string(), coord_value.clone());
                    }
                }

                let field_values: Vec<String> = fields
                    .iter()
                    .map(|(fname, ftype)| {
                        // Check if we have a coordinated value for this nested field
                        let value = if let Some(coord_value) = nested_coordinated_values.get(fname)
                        {
                            // Use the coordinated value
                            match ftype {
                                FieldType::DateTime => format!("d'{}'", coord_value),
                                FieldType::String => format!("'{}'", coord_value),
                                _ => coord_value.clone(),
                            }
                        } else {
                            self.generate_field_value_with_coordination(
                                gen_details,
                                fname,
                                None,
                                ftype,
                                id_index,
                                &nested_coordinated_values,
                            )
                        };
                        format!("{}: {}", fname, value)
                    })
                    .collect();
                format!("{{ {} }}", field_values.join(", "))
            }

            FieldType::HashMap(key, value) => {
                let count = rand::rng().random_range(0..3);
                let entries: Vec<String> = (0..count)
                    .map(|_| {
                        let key = self.generate_field_value_with_coordination(
                            gen_details,
                            field_name,
                            None,
                            key,
                            None,
                            coordinated_values,
                        );
                        let value = self.generate_field_value_with_coordination(
                            gen_details,
                            field_name,
                            None,
                            value,
                            None,
                            coordinated_values,
                        );
                        format!("{}: {}", key, value)
                    })
                    .collect();
                format!("{{ {} }}", entries.join(", "))
            }
            FieldType::BTreeMap(key, value) => {
                let count = rand::rng().random_range(0..3);
                let entries: Vec<String> = (0..count)
                    .map(|_| {
                        let key = self.generate_field_value_with_coordination(
                            gen_details,
                            field_name,
                            None,
                            key,
                            None,
                            coordinated_values,
                        );
                        let value = self.generate_field_value_with_coordination(
                            gen_details,
                            field_name,
                            None,
                            value,
                            None,
                            coordinated_values,
                        );
                        format!("{}: {}", key, value)
                    })
                    .collect();
                format!("{{ {} }}", entries.join(", "))
            }
            FieldType::RecordLink(inner) => {
                return self.generate_field_value_with_coordination(
                    gen_details,
                    field_name,
                    None,
                    inner,
                    None,
                    coordinated_values,
                );
            }
            // For other types, try to see if the type is actually a reference to another table, a server-only schema, or an enum.
            FieldType::Other(ref type_name) => {
                let snake_case_name = type_name.to_case(Case::Snake);
                // First try to find by matching struct name
                if let Some((table_name, _)) = self
                    .tables
                    .iter()
                    .find(|(_, q)| &q.struct_config.name == type_name)
                {
                    if let Some(possible_ids) = self.id_map.get(table_name) {
                        let idx = rng.random_range(0..possible_ids.len());
                        format!("r'{}'", possible_ids[idx])
                    } else {
                        // Fallback if no ids were generated for this table
                        panic!(
                            "{}",
                            format!(
                                "There were no id's for the table {}, field {}",
                                table_name, field_name
                            )
                        )
                    }
                // Then try snake_case version
                } else if let Some((table_name, _)) =
                    self.tables.iter().find(|(key, _)| *key == &snake_case_name)
                {
                    if let Some(possible_ids) = self.id_map.get(table_name) {
                        let idx = rng.random_range(0..possible_ids.len());
                        format!("r'{}'", possible_ids[idx])
                    } else {
                        // Fallback if no ids were generated for this table
                        panic!(
                            "{}",
                            format!(
                                "There were no id's for the table {}, field {}",
                                table_name, field_name
                            )
                        )
                    }
                } else if let Some(struct_config) = self
                    .objects
                    .get(type_name)
                    .or_else(|| self.objects.get(&snake_case_name))
                {
                    self.generate_server_only_inline(gen_details, struct_config)
                } else if let Some(tagged_union) = self.enums.get(type_name) {
                    let variant = tagged_union.variants.choose(&mut rng).expect(
                        "Something went wrong selecting a random enum variant, returned None",
                    );
                    if let Some(ref variant_data) = variant.data {
                        // Generate dummy value for the enum variant's data, if available.
                        let variant_data_field_type = match variant_data {
                            VariantData::InlineStruct(inline_struct) => {
                                &FieldType::Other(inline_struct.name.clone())
                            }
                            VariantData::DataStructureRef(field_type) => field_type,
                        };
                        self.generate_field_value_with_coordination(
                            gen_details,
                            field_name,
                            None,
                            variant_data_field_type,
                            id_index,
                            coordinated_values,
                        )
                    } else {
                        format!("'{}'", variant.name)
                    }
                } else {
                    // If we can't find the type, assume it's a table reference and generate a record ID

                    panic!(
                        "{}",
                        format!(
                            "This type could not be parsed table {}, field {}",
                            table_name.unwrap_or(&"(not found)".to_string()),
                            field_name
                        )
                    )
                }
            }
        }
    }

    /// Generate field value with format support - checks format first, then falls back to type
    pub fn generate_field_value_with_format(
        &self,
        table_field: &StructField,
        gen_details: &TableConfig,
        table_name: Option<&String>,
        id_index: Option<usize>,
    ) -> String {
        self.generate_field_value_with_format_and_coordination(
            table_field,
            gen_details,
            table_name,
            &HashMap::new(),
            id_index,
        )
    }

    /// Generate field value with format support and coordination context
    pub fn generate_field_value_with_format_and_coordination(
        &self,
        table_field: &StructField,
        gen_details: &TableConfig,
        table_name: Option<&String>,
        coordinated_values: &HashMap<String, String>,
        id_index: Option<usize>,
    ) -> String {
        // Priority: explicit format only
        if let Some(format) = &table_field.format {
            let generated = format.generate_formatted_value();

            // Check if format generates numeric or boolean values that shouldn't be quoted
            match format {
                Format::Percentage
                | Format::Latitude
                | Format::Longitude
                | Format::CurrencyAmount
                | Format::AppointmentDurationNs => {
                    // These formats generate numeric values, don't quote
                    return generated;
                }
                Format::DateTime | Format::AppointmentDateTime | Format::DateWithinDays(_) => {
                    return format!("d'{}'", generated);
                }
                _ => {
                    // Most formats generate strings, quote them
                    return format!("'{}'", generated);
                }
            }
        }

        // Check table level override if present in MockGenerationConfig
        // (This would be passed down if we had it in the config)

        // Fallback to type-based generation
        self.generate_field_value_with_coordination(
            gen_details,
            &table_field.field_name,
            table_name,
            &table_field.field_type,
            id_index,
            coordinated_values,
        )
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
    pub preserve_unchanged: bool,
    pub preserve_modified: bool,
    pub batch_size: usize,
    pub regenerate_fields: Vec<String>,
    pub preservation_mode: PreservationMode,
}

impl Default for MockGenerationConfig {
    fn default() -> Self {
        Self {
            n: 10,
            table_level_override: None,
            coordination_rules: Vec::new(),
            preserve_unchanged: true,
            preserve_modified: false,
            batch_size: 1000,
            regenerate_fields: vec!["updated_at".to_string(), "created_at".to_string()],
            preservation_mode: PreservationMode::Smart,
        }
    }
}

impl MockGenerationConfig {
    /// Create a config for smart preservation mode
    pub fn with_smart_preservation(n: usize) -> Self {
        Self {
            n,
            preservation_mode: PreservationMode::Smart,
            preserve_unchanged: true,
            preserve_modified: false,
            ..Default::default()
        }
    }

    /// Create a config for full preservation mode
    pub fn with_full_preservation(n: usize) -> Self {
        Self {
            n,
            preservation_mode: PreservationMode::Full,
            preserve_unchanged: true,
            preserve_modified: true,
            ..Default::default()
        }
    }
}
