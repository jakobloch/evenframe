use crate::{
    compare::PreservationMode,
    evenframe_log,
    schemasync::table::TableConfig,
    types::{StructConfig, TaggedUnion},
};
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use surrealdb::{
    engine::{local::Db, remote::http::Client},
    Surreal,
};
use syn::{parenthesized, LitStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DefineConfig {
    pub select_permissions: Option<String>,
    pub update_permissions: Option<String>,
    pub create_permissions: Option<String>,
    pub data_type: Option<String>,
    pub should_skip: bool,
    pub default: Option<String>,
    pub default_always: Option<String>,
    pub value: Option<String>,
    pub assert: Option<String>,
    pub readonly: Option<bool>,
    pub flexible: Option<bool>,
}

impl ToTokens for DefineConfig {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // Helper closure to convert Option<String> to tokens.
        let opt_lit = |s: &Option<String>| -> TokenStream {
            if let Some(ref text) = s {
                let lit = LitStr::new(text, Span::call_site());
                // Wrap the literal in String::from to produce a String.
                quote! { Some(String::from(#lit)) }
            } else {
                quote! { None }
            }
        };

        let select_permissions = opt_lit(&self.select_permissions);
        let update_permissions = opt_lit(&self.update_permissions);
        let create_permissions = opt_lit(&self.create_permissions);
        let data_type = opt_lit(&self.data_type);
        let default = opt_lit(&self.default);
        let default_always = opt_lit(&self.default_always);
        let value = opt_lit(&self.value);
        let assert_field = opt_lit(&self.assert);
        let readonly = if let Some(b) = self.readonly {
            quote! { Some(#b) }
        } else {
            quote! { None }
        };
        let flexible = if let Some(f) = self.flexible {
            quote! { Some(#f) }
        } else {
            quote! { None }
        };

        let should_skip = self.should_skip;

        tokens.extend(quote! {
            ::evenframe::schemasync::DefineConfig {
                select_permissions: #select_permissions,
                update_permissions: #update_permissions,
                create_permissions: #create_permissions,
                data_type: #data_type,
                should_skip: #should_skip,
                default: #default,
                default_always: #default_always,
                value: #value,
                assert: #assert_field,
                readonly: #readonly,
                flexible: #flexible
            }
        });
    }
}

impl DefineConfig {
    pub fn parse(field: &syn::Field) -> syn::Result<Option<DefineConfig>> {
        let mut select_permissions: Option<String> = None;
        let mut update_permissions: Option<String> = None;
        let mut create_permissions: Option<String> = None;
        let mut data_type: Option<String> = None;
        let mut should_skip: Option<bool> = None;
        let mut default: Option<String> = None;
        let mut default_always: Option<String> = None;
        let mut value: Option<String> = None;
        let mut assert: Option<String> = None;
        let mut readonly: Option<bool> = None;
        let mut flexible: Option<bool> = None;

        for attr in &field.attrs {
            if attr.path().is_ident("define_field_statement") {
                attr.parse_nested_meta(|meta| {
                    // Helper closure for optional string fields that works directly on the ParseBuffer.
                    let parse_opt_string =
                        |content: &mut syn::parse::ParseBuffer| -> syn::Result<Option<String>> {
                            if content.peek(syn::Ident) {
                                let ident: syn::Ident = content.parse()?;
                                if ident == "None" {
                                    Ok(None)
                                } else {
                                    Err(syn::Error::new(
                                        ident.span(),
                                        "expected `None` or a string literal",
                                    ))
                                }
                            } else {
                                let lit: syn::LitStr = content.parse()?;
                                if lit.value() == "None" {
                                    Ok(None)
                                } else {
                                    Ok(Some(lit.value()))
                                }
                            }
                        };
                    if meta.path.is_ident("flexible") {
                        let content;
                        parenthesized!(content in meta.input);
                        if flexible.is_some() {
                            return Err(meta.error("duplicate flexible attribute"));
                        }
                        flexible = Some(content.parse::<syn::LitBool>()?.value);
                        return Ok(());
                    }
                    if meta.path.is_ident("select_permissions") {
                        let mut content;
                        parenthesized!(content in meta.input);
                        if select_permissions.is_some() {
                            return Err(meta.error("duplicate select_permissions attribute"));
                        }
                        select_permissions = parse_opt_string(&mut content)?;
                        return Ok(());
                    }
                    if meta.path.is_ident("update_permissions") {
                        let mut content;
                        parenthesized!(content in meta.input);
                        if update_permissions.is_some() {
                            return Err(meta.error("duplicate update_permissions attribute"));
                        }
                        update_permissions = parse_opt_string(&mut content)?;
                        return Ok(());
                    }
                    if meta.path.is_ident("create_permissions") {
                        let mut content;
                        parenthesized!(content in meta.input);
                        if create_permissions.is_some() {
                            return Err(meta.error("duplicate create_permissions attribute"));
                        }
                        create_permissions = parse_opt_string(&mut content)?;
                        return Ok(());
                    }
                    if meta.path.is_ident("data_type") {
                        let mut content;
                        parenthesized!(content in meta.input);
                        if data_type.is_some() {
                            return Err(meta.error("duplicate data_type attribute"));
                        }
                        data_type = parse_opt_string(&mut content)?;
                        return Ok(());
                    }
                    if meta.path.is_ident("should_skip") {
                        let content;
                        parenthesized!(content in meta.input);
                        if should_skip.is_some() {
                            return Err(meta.error("duplicate should_skip attribute"));
                        }
                        should_skip = Some(content.parse::<syn::LitBool>()?.value);
                        return Ok(());
                    }
                    if meta.path.is_ident("default") {
                        let mut content;
                        parenthesized!(content in meta.input);
                        if default.is_some() {
                            return Err(meta.error("duplicate default attribute"));
                        }
                        default = parse_opt_string(&mut content)?;
                        return Ok(());
                    }
                    if meta.path.is_ident("default_always") {
                        let mut content;
                        parenthesized!(content in meta.input);
                        if default_always.is_some() {
                            return Err(meta.error("duplicate default_always attribute"));
                        }
                        default_always = parse_opt_string(&mut content)?;
                        return Ok(());
                    }
                    if meta.path.is_ident("value") {
                        let mut content;
                        parenthesized!(content in meta.input);
                        if value.is_some() {
                            return Err(meta.error("duplicate value attribute"));
                        }
                        value = parse_opt_string(&mut content)?;
                        return Ok(());
                    }
                    if meta.path.is_ident("assert") {
                        let mut content;
                        parenthesized!(content in meta.input);
                        if assert.is_some() {
                            return Err(meta.error("duplicate assert attribute"));
                        }
                        assert = parse_opt_string(&mut content)?;
                        return Ok(());
                    }
                    if meta.path.is_ident("readonly") {
                        let content;
                        parenthesized!(content in meta.input);
                        if readonly.is_some() {
                            return Err(meta.error("duplicate readonly attribute"));
                        }
                        readonly = Some(content.parse::<syn::LitBool>()?.value);
                        return Ok(());
                    }

                    Err(meta.error("unrecognized define detail"))
                })?;

                let should_skip = should_skip.unwrap_or(false);
                return Ok(Some(DefineConfig {
                    select_permissions,
                    update_permissions,
                    create_permissions,
                    data_type,
                    should_skip,
                    default,
                    default_always,
                    value,
                    assert,
                    readonly,
                    flexible,
                }));
            }
        }

        Ok(Some(DefineConfig {
            select_permissions: Some("FULL".to_string()),
            update_permissions: Some("FULL".to_string()),
            create_permissions: Some("FULL".to_string()),
            data_type: None,
            should_skip: false,
            default: None,
            default_always: None,
            value: None,
            assert: None,
            readonly: None,
            flexible: Some(false),
        }))
    }
}

pub async fn define_tables(
    db: &Surreal<Client>,
    new_schema: &Surreal<Db>,
    tables: &HashMap<String, TableConfig>,
    objects: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
    full_refresh_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    for (table_name, table) in tables {
        let snake_table_name = &table_name.to_case(Case::Snake);
        let define_stmts = generate_define_statements(
            snake_table_name,
            table,
            tables,
            objects,
            enums,
            full_refresh_mode,
        );
        evenframe_log!(&define_stmts, "define_statements.surql", true);

        // Execute and check define statements
        let _ = new_schema.query(&define_stmts).await?;
        let define_result = db.query(&define_stmts).await;
        match define_result {
            Ok(_) => evenframe_log!(
                &format!(
                    "Successfully executed define statements for table {}",
                    table_name
                ),
                "results.log",
                true
            ),
            Err(e) => {
                let error_msg = format!(
                    "Failed to execute define statements for table {}: {}",
                    table_name, e
                );
                evenframe_log!(&error_msg, "results.log", true);
                return Err(e.into());
            }
        }
    }
    Ok(())
}

pub fn generate_define_statements(
    table_name: &str,
    ds: &TableConfig,
    query_details: &HashMap<String, TableConfig>,
    server_only: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
    full_refresh_mode: bool,
) -> String {
    let table_type = if ds.relation.is_some() {
        &format!(
            "RELATION FROM {} TO {}",
            ds.relation.as_ref().unwrap().from,
            ds.relation.as_ref().unwrap().to
        )
    } else {
        "NORMAL"
    };
    let select_permissions = ds
        .permissions
        .as_ref()
        .and_then(|p| p.select_permissions.as_deref())
        .unwrap_or("FULL");
    let create_permissions = ds
        .permissions
        .as_ref()
        .and_then(|p| p.create_permissions.as_deref())
        .unwrap_or("FULL");
    let update_permissions = ds
        .permissions
        .as_ref()
        .and_then(|p| p.update_permissions.as_deref())
        .unwrap_or("FULL");
    let delete_permissions = ds
        .permissions
        .as_ref()
        .and_then(|p| p.delete_permissions.as_deref())
        .unwrap_or("FULL");

    let mut output = "".to_owned();

    if let Some(mock_config) = &ds.mock_generation_config {
        if mock_config.preservation_mode == PreservationMode::None || full_refresh_mode {
            // We do one DELETE to clear old data
            output.push_str(&format!("REMOVE TABLE {table_name};\n"));
        }
    } else if full_refresh_mode {
        output.push_str(&format!("REMOVE TABLE {table_name};\n"));
    }

    output.push_str(&format!(
        "DEFINE TABLE OVERWRITE {table_name} SCHEMAFULL TYPE {table_type} CHANGEFEED 3d PERMISSIONS FOR select {select_permissions} FOR update {update_permissions} FOR create {create_permissions} FOR delete {delete_permissions};\n"
    ));

    for table_field in &ds.struct_config.fields {
        // if struct field is an edge it should not be defined in the table itself
        if table_field.edge_config.is_none()
            && (table_field.field_name != "in"
                && table_field.field_name != "out"
                && table_field.field_name != "id")
        {
            if table_field.define_config.is_some() {
                output.push_str(&table_field.generate_define_statement(
                    enums.clone(),
                    server_only.clone(),
                    query_details.clone(),
                    &table_name.to_string(),
                ));
            } else {
                output.push_str(&format!(
                    "DEFINE FIELD OVERWRITE {} ON TABLE {} TYPE any PERMISSIONS FULL;\n",
                    table_field.field_name, table_name
                ))
            }
        }
    }

    output
}
