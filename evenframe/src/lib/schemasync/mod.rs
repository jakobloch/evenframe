// SchemaSync - Database schema synchronization
pub mod compare;
pub mod config;
pub mod edge;
pub mod mockmake;
pub mod permissions;
pub mod surql;
pub mod table;

use std::collections::HashMap;
use tracing::{debug, error, info, trace};

// Re-export commonly used types
pub use edge::{Direction, EdgeConfig, Subquery};
pub use mockmake::{coordinate, format};
pub use permissions::PermissionsConfig;
pub use surql::{define::DefineConfig, generate_query, random_string, QueryType};
use surrealdb::{
    engine::{
        local::Db,
        remote::http::{Client, Http},
    },
    opt::auth::Root,
    Surreal,
};
pub use table::TableConfig;

use crate::{
    evenframe_log,
    mockmake::Mockmaker,
    schemasync::surql::define::define_tables,
    types::{StructConfig, TaggedUnion},
};

pub struct Schemasync<'a> {
    // Input parameters - set via builder methods
    tables: Option<&'a HashMap<String, TableConfig>>,
    objects: Option<&'a HashMap<String, StructConfig>>,
    enums: Option<&'a HashMap<String, TaggedUnion>>,

    // Internal state - initialized automatically
    db: Option<Surreal<Client>>,
    schemasync_config: Option<crate::schemasync::config::SchemasyncConfig>,
}

impl<'a> Schemasync<'a> {
    /// Create a new empty Schemasync instance
    pub fn new() -> Self {
        trace!("Creating new Schemasync instance");
        Self {
            tables: None,
            objects: None,
            enums: None,
            db: None,
            schemasync_config: None,
        }
    }

    /// Builder methods for setting up the parameters
    pub fn with_tables(mut self, tables: &'a HashMap<String, TableConfig>) -> Self {
        debug!("Configuring Schemasync with {} tables", tables.len());
        trace!("Table names: {:?}", tables.keys().collect::<Vec<_>>());
        self.tables = Some(tables);
        self
    }

    pub fn with_objects(mut self, objects: &'a HashMap<String, StructConfig>) -> Self {
        debug!("Configuring Schemasync with {} objects", objects.len());
        trace!("Object names: {:?}", objects.keys().collect::<Vec<_>>());
        self.objects = Some(objects);
        self
    }

    pub fn with_enums(mut self, enums: &'a HashMap<String, TaggedUnion>) -> Self {
        debug!("Configuring Schemasync with {} enums", enums.len());
        trace!("Enum names: {:?}", enums.keys().collect::<Vec<_>>());
        self.enums = Some(enums);
        self
    }

    /// Initialize database connection and config from environment
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing Schemasync database connection and configuration");
        dotenv::dotenv().ok();
        debug!("Loaded environment variables from .env file");

        let config = crate::config::EvenframeConfig::new()?;
        debug!("Loaded Evenframe configuration successfully");
        trace!("Database URL: {}", config.schemasync.database.url);
        trace!("Database namespace: {}", config.schemasync.database.namespace);
        trace!("Database name: {}", config.schemasync.database.database);

        let db = Surreal::new::<Http>(&config.schemasync.database.url).await?;
        debug!("Created SurrealDB connection");

        let username = std::env::var("SURREALDB_USER").expect("SURREALDB_USER not set");
        let password = std::env::var("SURREALDB_PASSWORD").expect("SURREALDB_PASSWORD not set");
        debug!("Retrieved database credentials from environment");

        db.signin(Root {
            username: &username,
            password: &password,
        })
        .await?;
        debug!("Successfully signed in to SurrealDB");

        db.use_ns(&config.schemasync.database.namespace)
            .use_db(&config.schemasync.database.database)
            .await?;
        info!("Connected to database namespace '{}' and database '{}'", 
              config.schemasync.database.namespace, config.schemasync.database.database);

        self.db = Some(db);
        self.schemasync_config = Some(config.schemasync);
        debug!("Schemasync initialization completed successfully");

        Ok(())
    }

    /// Run the complete schemasync pipeline
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Schemasync pipeline execution");
        
        // Initialize database and config first
        self.initialize().await?;

        // Validate that all required fields are set
        debug!("Validating required fields for Schemasync pipeline");
        let db = self
            .db
            .take()
            .ok_or("Database connection failed to initialize")?;
        let tables = self.tables.ok_or("Tables not provided")?;
        let objects = self.objects.ok_or("Objects not provided")?;
        let enums = self.enums.ok_or("Enums not provided")?;
        let config = self
            .schemasync_config
            .take()
            .ok_or("Config failed to initialize")?;
        
        info!("Pipeline validation completed - {} tables, {} objects, {} enums", 
              tables.len(), objects.len(), enums.len());

        evenframe_log!("", "all_statements.surql");
        evenframe_log!("", "results.log");
        debug!("Initialized logging files");

        // Create Mockmaker instance (which contains Comparator)
        info!("Creating Mockmaker instance for data generation and comparison");
        let mut mockmaker = Mockmaker::new(
            db.clone(),
            tables.clone(),
            objects.clone(),
            enums.clone(),
            config.clone(),
        );
        debug!("Mockmaker instance created successfully");

        // Run initial ID generation and comparator setup
        info!("Generating IDs for mock data");
        mockmaker.generate_ids().await?;
        debug!("ID generation completed");

        // Run the comparator pipeline
        info!("Running schema comparison pipeline");
        let comparator = mockmaker.comparator.take().unwrap();
        mockmaker.comparator = Some(comparator.run().await?);
        debug!("Schema comparison completed");

        // Define tables (this stays in Schemasync)
        info!("Defining database tables and schema");
        self.define_tables(
            &db,
            mockmaker.get_new_schema().unwrap(),
            tables,
            objects,
            enums,
            &config,
        )
        .await
        .map_err(|e| {
            error!("Failed to define tables: {}", e);
            e
        })?;
        debug!("Table definitions completed successfully");

        // Continue with the rest of the mockmaker pipeline
        info!("Removing old data from database");
        mockmaker.remove_old_data().await
            .map_err(|e| {
                error!("Failed to remove old data: {}", e);
                e
            })?;
        debug!("Old data removal completed");

        info!("Executing access control setup");
        mockmaker.execute_access().await
            .map_err(|e| {
                error!("Failed to execute access setup: {}", e);
                e
            })?;
        debug!("Access control setup completed");

        info!("Filtering schema changes");
        mockmaker.filter_changes().await
            .map_err(|e| {
                error!("Failed to filter changes: {}", e);
                e
            })?;
        debug!("Schema changes filtering completed");

        info!("Generating mock data");
        mockmaker.generate_mock_data().await
            .map_err(|e| {
                error!("Failed to generate mock data: {}", e);
                e
            })?;
        debug!("Mock data generation completed");

        info!("Schemasync pipeline execution completed successfully");
        Ok(())
    }

    /// Define tables in both schemas (this stays in Schemasync)
    async fn define_tables(
        &self,
        db: &Surreal<Client>,
        new_schema: &Surreal<Db>,
        tables: &HashMap<String, TableConfig>,
        objects: &HashMap<String, StructConfig>,
        enums: &HashMap<String, TaggedUnion>,
        config: &crate::schemasync::config::SchemasyncConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Defining tables with full_refresh_mode: {}", config.mock_gen_config.full_refresh_mode);
        trace!("Table definitions for: {:?}", tables.keys().collect::<Vec<_>>());
        
        define_tables(
            db,
            new_schema,
            tables,
            objects,
            enums,
            config.mock_gen_config.full_refresh_mode,
        )
        .await
        .map_err(|e| {
            error!("Failed to execute table definitions: {}", e);
            e
        })
    }
}
