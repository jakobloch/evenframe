mod config_builders;
mod workspace_scanner;

use evenframe::schemasync::Schemasync; // Import your new struct
use evenframe::{
    config::EvenframeConfig,
    typesync::{arktype::generate_arktype_type_string, effect::generate_effect_schema_string},
};
use tracing::{debug, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with environment variable control
    // Set RUST_LOG=debug for debug output, RUST_LOG=info for info only
    tracing_subscriber::fmt::init();
    // Load configuration
    let config = EvenframeConfig::new()?;

    let generate_dummy_values = config.schemasync.should_generate_mocks;
    let generate_arktype_types = config.typesync.should_generate_arktype_types;
    let generate_effect_schemas = config.typesync.should_generate_effect_types;

    // Get the config builder closure
    info!("Building all configs...");
    let (enums, tables, objects) = config_builders::build_all_configs();
    info!(
        "Config building complete. Found {} enums, {} tables, {} objects",
        enums.len(),
        tables.len(),
        objects.len()
    );

    if generate_arktype_types {
        debug!("Generating arktype types...");
        let structs = config_builders::merge_tables_and_objects(&tables, &objects);
        debug!("Merged {} structs", structs.len());
        std::fs::write(
            "../../frontend/src/lib/core/types/arktype.ts",
            format!(
                "import {{ scope }} from 'arktype';\n\n{}\n\n export const validator = scope({{
  ...bindings.export(),
            }}).export();",
                generate_arktype_type_string(&structs, &enums, false),
            ),
        )?;
    }

    if generate_effect_schemas {
        let structs = config_builders::merge_tables_and_objects(&tables, &objects);
        std::fs::write(
            "../../frontend/src/lib/core/types/bindings.ts",
            format!(
                "import {{ Schema }} from \"effect\";\n\n{}",
                generate_effect_schema_string(&structs, &enums, false),
            ),
        )?;
    }

    if generate_dummy_values {
        info!("Starting Schemasync for mock data generation");
        // Much simpler now!
        let schemasync = Schemasync::new()
            .with_tables(&tables)
            .with_objects(&objects)
            .with_enums(&enums);

        debug!("Running Schemasync...");
        schemasync.run().await?;
        info!("Schemasync completed successfully");
    }

    Ok(())
}
