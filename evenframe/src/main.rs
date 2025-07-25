mod config_builders;

use helpers::evenframe::schemasync::Schemasync; // Import your new struct
use helpers::evenframe::{
    config::EvenframeConfig,
    typesync::{arktype::generate_arktype_type_string, effect::generate_effect_schema_string},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = EvenframeConfig::new()?;

    let generate_dummy_values = config.schemasync.should_generate_mocks;
    let generate_arktype_types = config.typesync.should_generate_arktype_types;
    let generate_effect_schemas = config.typesync.should_generate_effect_types;

    // Get the config builder closure
    let build_configs = config_builders::build_all_configs();
    let (enums, tables, objects) = build_configs();

    if generate_arktype_types {
        let structs = config_builders::merge_tables_and_objects(&tables, &objects);
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
        // Much simpler now!
        Schemasync::new()
            .with_tables(&tables)
            .with_objects(&objects)
            .with_enums(&enums)
            .run()
            .await?;
    }

    Ok(())
}
