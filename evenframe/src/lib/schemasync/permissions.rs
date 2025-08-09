use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parenthesized;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PermissionsConfig {
    pub all_permissions: Option<String>,
    pub select_permissions: Option<String>,
    pub update_permissions: Option<String>,
    pub delete_permissions: Option<String>,
    pub create_permissions: Option<String>,
}

impl ToTokens for PermissionsConfig {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let all = if let Some(ref s) = self.all_permissions {
            quote! { Some(#s.to_string()) }
        } else {
            quote! { None }
        };
        let select = if let Some(ref s) = self.select_permissions {
            quote! { Some(#s.to_string()) }
        } else {
            quote! { None }
        };
        let update = if let Some(ref s) = self.update_permissions {
            quote! { Some(#s.to_string()) }
        } else {
            quote! { None }
        };
        let delete = if let Some(ref s) = self.delete_permissions {
            quote! { Some(#s.to_string()) }
        } else {
            quote! { None }
        };
        let create = if let Some(ref s) = self.create_permissions {
            quote! { Some(#s.to_string()) }
        } else {
            quote! { None }
        };

        tokens.extend(quote! {
            ::evenframe::schemasync::PermissionsConfig {
                all_permissions: #all,
                select_permissions: #select,
                update_permissions: #update,
                delete_permissions: #delete,
                create_permissions: #create,
            }
        });
    }
}

impl PermissionsConfig {
    pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Option<PermissionsConfig>> {
        let mut all_permissions: Option<String> = None;
        let mut select_permissions: Option<String> = None;
        let mut update_permissions: Option<String> = None;
        let mut delete_permissions: Option<String> = None;
        let mut create_permissions: Option<String> = None;

        for attr in attrs {
            if attr.path().is_ident("permissions") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("all") {
                        let content;
                        parenthesized!(content in meta.input);
                        if all_permissions.is_some() {
                            return Err(meta.error("duplicate all permissions attribute"));
                        }
                        all_permissions = Some(content.parse::<syn::LitStr>()?.value());
                        return Ok(());
                    }
                    if meta.path.is_ident("select") {
                        let content;
                        parenthesized!(content in meta.input);
                        if select_permissions.is_some() {
                            return Err(meta.error("duplicate select permissions attribute"));
                        }
                        select_permissions = Some(content.parse::<syn::LitStr>()?.value());
                        return Ok(());
                    }
                    if meta.path.is_ident("update") {
                        let content;
                        parenthesized!(content in meta.input);
                        if update_permissions.is_some() {
                            return Err(meta.error("duplicate update permissions attribute"));
                        }
                        update_permissions = Some(content.parse::<syn::LitStr>()?.value());
                        return Ok(());
                    }
                    if meta.path.is_ident("delete") {
                        let content;
                        parenthesized!(content in meta.input);
                        if delete_permissions.is_some() {
                            return Err(meta.error("duplicate delete permissions attribute"));
                        }
                        delete_permissions = Some(content.parse::<syn::LitStr>()?.value());
                        return Ok(());
                    }
                    if meta.path.is_ident("create") {
                        let content;
                        parenthesized!(content in meta.input);
                        if create_permissions.is_some() {
                            return Err(meta.error("duplicate create permissions attribute"));
                        }
                        create_permissions = Some(content.parse::<syn::LitStr>()?.value());
                        return Ok(());
                    }

                    Err(meta.error("unrecognized permission type"))
                })?;

                return Ok(Some(PermissionsConfig {
                    all_permissions,
                    select_permissions,
                    update_permissions,
                    delete_permissions,
                    create_permissions,
                }));
            }
        }

        Ok(None)
    }
}