use crate::evenframe_log;
use crate::schemasync::config::{AccessConfig, AccessType};
use std::env;
use surrealdb::{
    engine::{local::Db, remote::http::Client},
    Surreal,
};

/// Generate a DEFINE ACCESS statement for SurrealDB
/// This creates access methods with OVERWRITE always enabled as requested
pub fn generate_access_definition(access_config: &AccessConfig) -> String {
    let access_name = &access_config.name;

    // Start with DEFINE ACCESS OVERWRITE statement
    let mut query = format!("DEFINE ACCESS OVERWRITE {} ON DATABASE", access_name);

    // Add TYPE and configuration based on access type
    match &access_config.access_type {
        AccessType::Record => {
            query.push_str(&format!(
                " TYPE RECORD
    SIGNUP ( CREATE {} SET email = $email, password = crypto::argon2::generate($password) )
    SIGNIN ( SELECT * FROM {} WHERE email = $email AND crypto::argon2::compare(password, $password) )
    DURATION FOR TOKEN 15m, FOR SESSION 6h",
                access_config.table_name, access_config.table_name
            ));
        }
        AccessType::Jwt => {
            // Basic JWT configuration - can be expanded based on needs
            query.push_str(
                " TYPE JWT
    ALGORITHM HS256
    KEY 'your-secret-key-here'",
            );
        }
        AccessType::Bearer => {
            // Bearer for record users by default
            query.push_str(" TYPE BEARER FOR RECORD");
        }
        AccessType::System => {
            // System access is typically not defined via DEFINE ACCESS
            // Return empty string or handle differently based on requirements
            return String::new();
        }
    }

    // Add semicolon to complete the statement
    query.push(';');

    query
}
pub async fn execute_access_query(
    db: &Surreal<Client>,
    access_query: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let access_result = db.query(access_query).await;
    match access_result {
        Ok(_) => evenframe_log!(
            &format!(
                "Successfully executed define access statements for db {}",
                env::var("SURREALDB_DB").expect("SURREALDB_DB not set")
            ),
            "results.log",
            true
        ),
        Err(e) => {
            let error_msg = format!(
                "Failed to execute define access statements for db {}: {}",
                env::var("SURREALDB_DB").expect("SURREALDB_DB not set"),
                e
            );
            evenframe_log!(&error_msg, "results.log", true);
            return Err(e.into());
        }
    }
    Ok(())
}
pub async fn setup_access_definitions(
    new_schema: &Surreal<Db>,
    schemasync_config: &crate::schemasync::config::SchemasyncConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut access_query = String::new();
    evenframe_log!(
        &format!("{:#?}", &schemasync_config.database.accesses),
        "access_config.surql"
    );

    for access in &schemasync_config.database.accesses {
        access_query = generate_access_definition(access);
        if let Err(e) = new_schema.query(&access_query).await {
            eprintln!("Failed to create access '{}': {}", access.name, e);
        }
    }

    evenframe_log!(&access_query, "access_query.surql");
    Ok(access_query)
}
