// SchemaSync - Database schema synchronization
pub mod compare;
pub mod config;
pub mod edge;
pub mod mockmake;
pub mod permissions;
pub mod surql;
pub mod table;

use std::collections::HashMap;

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
        self.tables = Some(tables);
        self
    }

    pub fn with_objects(mut self, objects: &'a HashMap<String, StructConfig>) -> Self {
        self.objects = Some(objects);
        self
    }

    pub fn with_enums(mut self, enums: &'a HashMap<String, TaggedUnion>) -> Self {
        self.enums = Some(enums);
        self
    }

    /// Initialize database connection and config from environment
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();

        let config = crate::config::EvenframeConfig::new()?;

        let db = Surreal::new::<Http>(&config.schemasync.database.url).await?;

        let username = std::env::var("SURREALDB_USER").expect("SURREALDB_USER not set");
        let password = std::env::var("SURREALDB_PASSWORD").expect("SURREALDB_PASSWORD not set");

        db.signin(Root {
            username: &username,
            password: &password,
        })
        .await?;

        db.use_ns(&config.schemasync.database.namespace)
            .use_db(&config.schemasync.database.database)
            .await?;

        self.db = Some(db);
        self.schemasync_config = Some(config.schemasync);

        Ok(())
    }

    /// Run the complete schemasync pipeline
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize database and config first
        self.initialize().await?;

        // Validate that all required fields are set
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

        evenframe_log!("", "all_statements.surql");
        evenframe_log!("", "results.log");

        // Create Mockmaker instance (which contains Comparator)
        let mut mockmaker = Mockmaker::new(
            db.clone(),
            tables.clone(),
            objects.clone(),
            enums.clone(),
            config.clone(),
        );

        // Run initial ID generation and comparator setup
        mockmaker.generate_ids().await?;

        // Run the comparator pipeline
        let comparator = mockmaker.comparator.take().unwrap();
        mockmaker.comparator = Some(comparator.run().await?);

        // Define tables (this stays in Schemasync)
        self.define_tables(
            &db,
            mockmaker.get_new_schema().unwrap(),
            tables,
            objects,
            enums,
            &config,
        )
        .await?;

        // Continue with the rest of the mockmaker pipeline
        mockmaker.remove_old_data().await?;
        mockmaker.execute_access().await?;
        mockmaker.filter_changes().await?;
        mockmaker.generate_mock_data().await?;

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
        define_tables(
            db,
            new_schema,
            tables,
            objects,
            enums,
            config.mock_gen_config.full_refresh_mode,
        )
        .await
    }
}
