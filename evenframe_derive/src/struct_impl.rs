use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, LitStr};

use crate::attributes::{
    parse_format_attribute, parse_mock_data_attribute, parse_relation_attribute,
    parse_table_validators,
};
use crate::deserialization_impl::generate_custom_deserialize;
use crate::type_parser::parse_data_type;
use crate::validator_parser::parse_field_validators;

pub fn generate_struct_impl(input: DeriveInput) -> TokenStream {
    let ident = input.ident.clone();
    let evenframe_config = match ::helpers::evenframe::config::EvenframeConfig::new() {
        Ok(evenframe_config) => evenframe_config,
        Err(e) => {
            return syn::Error::new(
                ident.span(),
                format!("Failed to load Evenframe configuration: {}\n\nMake sure evenframe.toml exists in your project root and is properly formatted.", e)
            )
            .to_compile_error()
            .into();
        }
    };

    if let Data::Struct(ref data_struct) = input.data {
        // Ensure the struct has named fields.
        let fields_named = if let Fields::Named(ref fields_named) = data_struct.fields {
            fields_named
        } else {
            return syn::Error::new(
                ident.span(),
                format!("Evenframe derive macro only supports structs with named fields.\n\nExample of a valid struct:\n\nstruct {} {{\n    id: String,\n    name: String,\n}}", ident),
            )
            .to_compile_error()
            .into();
        };

        // Parse struct-level attributes
        let permissions_config =
            match ::helpers::evenframe::schemasync::PermissionsConfig::parse(&input.attrs) {
                Ok(config) => config,
                Err(err) => {
                    return syn::Error::new(
                        input.span(),
                        format!("Failed to parse permissions configuration: {}\n\nExample usage:\n#[permissions(\n    select = \"true\",\n    create = \"auth.role == 'admin'\",\n    update = \"$auth.id == id\",\n    delete = \"false\"\n)]\nstruct MyStruct {{ ... }}", err)
                    )
                    .to_compile_error()
                    .into();
                }
            };

        // Parse mock_data attribute
        let mock_data_config = match parse_mock_data_attribute(&input.attrs) {
            Ok(config) => config,
            Err(err) => return err.to_compile_error().into(),
        };

        // Parse table-level validators
        let table_validators = match parse_table_validators(&input.attrs) {
            Ok(validators) => validators,
            Err(err) => return err.to_compile_error().into(),
        };

        // Parse relation attribute
        let relation_config = match parse_relation_attribute(&input.attrs) {
            Ok(config) => config,
            Err(err) => return err.to_compile_error().into(),
        };

        // Check if an "id" field exists.
        // Structs with an "id" field are treated as persistable entities (database tables).
        // Structs without an "id" field are treated as application-level data structures.
        let has_id = fields_named
            .named
            .iter()
            .any(|field| {
                // Check if field name is "id" - unwrap_or(false) handles unnamed fields gracefully
                field.ident.as_ref().map(|id| id == "id").unwrap_or(false)
            });

        // Single pass over all fields.
        let mut table_field_tokens = Vec::new();
        let mut json_assignments = Vec::new();
        let mut fetch_fields = Vec::new(); // For fields marked with #[fetch]
        let mut subqueries: Vec<String> = Vec::new();
        for field in fields_named.named.iter() {
            let field_ident = match field.ident.as_ref() {
                Some(ident) => ident,
                None => {
                    return syn::Error::new(
                        field.span(),
                        "Internal error: Field identifier is missing. This should not happen with named fields."
                    )
                    .to_compile_error()
                    .into();
                }
            };
            let field_name = field_ident.to_string();
            // Remove the r# prefix from raw identifiers (e.g., r#type -> type)
            let field_name_trim = field_name.trim_start_matches("r#");

            // Build the field type token.
            let ty = &field.ty;
            let field_type = parse_data_type(ty);

            // Parse any edge attribute.
            let edge_config = match ::helpers::evenframe::schemasync::EdgeConfig::parse(field) {
                Ok(details) => details,
                Err(err) => {
                    return syn::Error::new(
                        field.span(),
                        format!("Failed to parse edge configuration for field '{}': {}\n\nExample usage:\n#[edge(name = \"has_user\", direction = \"from\", to = \"User\")]\npub user: RecordLink<User>", field_name, err)
                    )
                    .to_compile_error()
                    .into();
                }
            };

            // Parse any define details.
            let define_config = match ::helpers::evenframe::schemasync::DefineConfig::parse(field) {
                Ok(details) => details,
                Err(err) => {
                    return syn::Error::new(
                        field.span(),
                        format!("Failed to parse define configuration for field '{}': {}\n\nExample usage:\n#[define(default = \"0\", readonly = true)]\npub count: u32", field_name, err)
                    )
                    .to_compile_error()
                    .into();
                }
            };

            // Parse any format attribute.
            let format = match parse_format_attribute(&field.attrs) {
                Ok(fmt) => fmt,
                Err(err) => {
                    return syn::Error::new(
                        field.span(),
                        format!("Failed to parse format attribute for field '{}': {}", field_name, err)
                    )
                    .to_compile_error()
                    .into();
                }
            };

            // Parse field-level validators
            let field_validators = match parse_field_validators(&field.attrs) {
                Ok(v) => v,
                Err(err) => {
                    return syn::Error::new(
                        field.span(),
                        format!("Failed to parse validators for field '{}': {}\n\nExample usage:\n#[validate(min_length = 3, max_length = 50)]\npub name: String\n\n#[validate(email)]\npub email: String", field_name, err)
                    )
                    .to_compile_error()
                    .into();
                }
            };

            // Parse any subquery attribute, overrides default edge subquery if found
            let has_explicit_subquery = field
                .attrs
                .iter()
                .find(|a| a.path().is_ident("subquery"))
                .map(|attr| {
                    match attr.parse_args::<LitStr>() {
                        Ok(lit) => {
                            subqueries.push(lit.value());
                            Ok(())
                        }
                        Err(e) => {
                            Err(syn::Error::new(
                                attr.span(),
                                format!("Invalid subquery attribute format: {}\n\nThe subquery attribute requires a string literal:\n#[subquery(\"SELECT * FROM users WHERE active = true\")]\n\nMake sure to use double quotes around the SQL query.", e),
                            ))
                        }
                    }
                })
                .transpose();
            
            match has_explicit_subquery {
                Err(err) => return err.to_compile_error().into(),
                Ok(Some(())) => {}, // Explicit subquery was added
                Ok(None) => {
                    // No explicit subquery attribute, generate default from edge config
                    if let Some(ref details) = edge_config {
                    let subquery =
                        if details.direction == ::helpers::evenframe::schemasync::Direction::From {
                            format!(
                                "(SELECT ->{}.* AS data FROM $parent.id FETCH data.out)[0].data as {}",
                                details.edge_name, field_name
                            )
                        } else if details.direction == ::helpers::evenframe::schemasync::Direction::To {
                            format!(
                                "(SELECT <-{}.* AS data FROM $parent.id FETCH data.in)[0].data as {}",
                                details.edge_name, field_name
                            )
                        } else {
                            "".to_string()
                        };

                        subqueries.push(subquery);
                    }
                }
            }

            // Check for a fetch attribute.
            let has_fetch = field.attrs.iter().any(|a| a.path().is_ident("fetch"));

            // Build the schema token for this field.
            let edge_config_tokens = if let Some(ref details) = edge_config {
                quote! {
                    Some(#details)
                }
            } else {
                quote! { None }
            };

            // Build the schema token for this field.
            let define_config_tokens = if let Some(ref define) = define_config {
                quote! {
                    Some(#define)
                }
            } else {
                quote! { None }
            };

            // Build the schema token for this field.
            let format_tokens = if let Some(ref fmt) = format {
                quote! { Some(#fmt) }
            } else {
                quote! { None }
            };

            // Build validators token for this field
            let validators_tokens = if field_validators.is_empty() {
                quote! { vec![] }
            } else {
                quote! { vec![#(#field_validators),*] }
            };

            table_field_tokens.push(quote! {
                ::helpers::evenframe::schemasync::StructField {
                    field_name: #field_name_trim.to_string(),
                    field_type: #field_type,
                    edge_config: #edge_config_tokens,
                    define_config: #define_config_tokens,
                    format: #format_tokens,
                    validators: #validators_tokens,
                    always_regenerate: false
                }
            });

            // For the JSON payload, skip the "id" field and any field with an edge attribute.
            if field_name != "id" && edge_config.is_none() {
                json_assignments.push(quote! {
                    #field_name: payload.#field_ident,
                });
            }

            // If the field has a fetch attribute, add its name for the FETCH clause.
            if has_fetch {
                fetch_fields.push(field_name);
            }
        }

        // Build the JSON payload block.
        // let json_payload = quote! { { #(#json_assignments)* } };

        // Generate tokens for parsed attributes (shared between implementations)
        let struct_name = ident.to_string();

        let permissions_config_tokens = if let Some(ref config) = permissions_config {
            quote! { Some(#config) }
        } else {
            quote! { None }
        };

        let table_validators_tokens = if !table_validators.is_empty() {
            let validator_strings = table_validators.iter().map(|v| quote! { #v.to_string() });
            quote! {
                vec![
                    #(::helpers::evenframe::validator::Validator::StringValidator(
                        ::helpers::evenframe::validator::StringValidator::StringEmbedded(#validator_strings)
                    )),*
                ]
            }
        } else {
            quote! { vec![] }
        };

        let mock_data_tokens = if let Some((n, _overrides, coordinates)) = mock_data_config {
            let coord_rules = if let Some(coords) = coordinates {
                quote! { vec![#(#coords),*] }
            } else {
                quote! { vec![] }
            };

            let default_preservation_mode = evenframe_config
                .schemasync
                .mock_gen_config
                .default_preservation_mode;

            quote! {
                Some(::helpers::evenframe::schemasync::mock::MockGenerationConfig {
                    n: #n,
                    table_level_override: None, // Overrides parsing is handled separately
                    coordination_rules: #coord_rules,
                    preserve_unchanged: false,
                    preserve_modified: false,
                    batch_size: 1000,
                    regenerate_fields: vec!["updated_at".to_string(), "created_at".to_string()],
                    preservation_mode: #default_preservation_mode,
                })
            }
        } else {
            quote! { None }
        };

        let relation_tokens = if let Some(ref rel) = relation_config {
            quote! { Some(#rel) }
        } else {
            quote! { None }
        };

        let evenframe_persistable_struct_impl = {
            quote! {
                impl ::helpers::evenframe::traits::EvenframePersistableStruct for #ident {
                    fn name() -> String {
                        #struct_name.to_string()
                    }

                    fn validators() -> Vec<::helpers::evenframe::validator::Validator> {
                        #table_validators_tokens
                    }

                    fn permissions_config() -> Option<::helpers::evenframe::schemasync::PermissionsConfig> {
                        #permissions_config_tokens
                    }

                    fn struct_config() -> ::helpers::evenframe::schemasync::StructConfig {
                        ::helpers::evenframe::schemasync::StructConfig {
                            name: helpers::case::to_snake_case(#struct_name),
                            fields: vec![ #(#table_field_tokens),* ],
                            validators: vec![],
                        }
                    }

                    fn table_fields() -> Vec<::helpers::evenframe::schemasync::StructField> {
                        vec![ #(#table_field_tokens),* ]
                    }

                    fn table_config() -> Option<::helpers::evenframe::schemasync::TableConfig> {
                        Some(::helpers::evenframe::schemasync::TableConfig {
                            struct_config: ::helpers::evenframe::schemasync::StructConfig {
                                name: helpers::case::to_snake_case(#struct_name),
                                fields: vec![ #(#table_field_tokens),* ],
                                validators: vec![],
                            },
                            relation: #relation_tokens,
                            permissions: #permissions_config_tokens,
                            mock_generation_config: Self::mock_generation_config(),
                        })
                    }

                    fn get_table_config(&self) -> Option<::helpers::evenframe::schemasync::TableConfig> {
                        Self::table_config()
                    }


                    fn mock_generation_config() -> Option<::helpers::evenframe::schemasync::mock::MockGenerationConfig> {
                        #mock_data_tokens
                    }

                    fn router() -> axum::Router<helpers::app_state::AppState> {
                        let collection_name = helpers::case::to_snake_case(&#struct_name);
                        let collection_name_plural = format!("{}s", collection_name);

                        axum::Router::new()
                            .route(&format!("/{}", collection_name), axum::routing::post(helpers::create!(#ident)))
                            .route(&format!("/{}", collection_name), axum::routing::put(helpers::update!(#ident)))
                            .route(&format!("/{}", collection_name), axum::routing::delete(helpers::delete!(#ident)))
                            .route(&format!("/{}", collection_name), axum::routing::get(helpers::read!(#ident)))
                            .route(&format!("/{}", collection_name_plural), axum::routing::get(helpers::read_all!(#ident)))
                    }
                }
            }
        };

        // Generate EvenframeAppStruct trait implementation
        let evenframe_app_struct_impl = {
            quote! {
                impl ::helpers::evenframe::traits::EvenframeAppStruct for #ident {
                    fn name() -> String {
                        #struct_name.to_string()
                    }

                    fn struct_config() -> ::helpers::evenframe::schemasync::StructConfig {
                        ::helpers::evenframe::schemasync::StructConfig {
                            name: helpers::case::to_snake_case(#struct_name),
                            fields: vec![ #(#table_field_tokens),* ],
                            validators: vec![],
                        }
                    }

                    fn table_fields() -> Vec<::helpers::evenframe::schemasync::StructField> {
                        vec![ #(#table_field_tokens),* ]
                    }
                }
            }
        };

        // Check if any field has validators
        // We check this to determine if we need to generate custom deserialization
        let has_field_validators = fields_named.named.iter().any(|field| {
            match parse_field_validators(&field.attrs) {
                Ok(validators) => !validators.is_empty(),
                Err(_) => true, // If parsing fails, assume validators exist to be safe
            }
        });

        // Generate custom deserialization if there are field validators
        let deserialize_impl = if has_field_validators || !table_validators.is_empty() {
            generate_custom_deserialize(&input)
        } else {
            quote! {}
        };

        let output = if has_id {
            quote! {
                // Import the trait so it's available for method calls
                use ::helpers::evenframe::traits::EvenframePersistableStruct as _;

                #evenframe_persistable_struct_impl
                #deserialize_impl

            }
        } else {
            quote! {
                // Import the trait so it's available for method calls
                use ::helpers::evenframe::traits::EvenframeAppStruct as _;

                #evenframe_app_struct_impl
                #deserialize_impl
            }
        };

        output.into()
    } else {
        syn::Error::new(
            ident.span(),
            format!("The Evenframe derive macro can only be applied to structs.\n\nYou tried to apply it to: {}\n\nExample of correct usage:\n#[derive(Evenframe)]\nstruct MyStruct {{\n    id: String,\n    // ... other fields\n}}", ident)
        )
        .to_compile_error()
        .into()
    }
}
