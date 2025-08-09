use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{parse_file, Attribute, Item, Meta};
use tracing::{debug, trace, warn};

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct EvenframeType {
    pub name: String,
    pub module_path: String,
    pub file_path: String,
    pub kind: TypeKind,
    pub has_id_field: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Struct,
    Enum,
}

pub struct WorkspaceScanner {
    handlers_path: PathBuf,
}

impl WorkspaceScanner {
    pub fn new() -> Self {
        let handlers_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("handlers")
            .join("src")
            .join("lib");

        Self { handlers_path }
    }

    pub fn scan_for_evenframe_types(
        &self,
    ) -> Result<Vec<EvenframeType>, Box<dyn std::error::Error>> {
        let mut types = Vec::new();
        self.scan_directory(&self.handlers_path, &mut types, "handlers", 0)?;
        Ok(types)
    }

    fn scan_directory(
        &self,
        dir: &Path,
        types: &mut Vec<EvenframeType>,
        base_module: &str,
        depth: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        trace!("Scanning directory: {:?}, module: {}, depth: {}", dir, base_module, depth);
        
        // Prevent excessive recursion
        if depth > 10 {
            warn!("Maximum recursion depth reached at {:?}", dir);
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Skip symbolic links to avoid infinite recursion
            if path.symlink_metadata()?.file_type().is_symlink() {
                debug!("Skipping symlink: {:?}", path);
                continue;
            }

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if dir_name != "tests" && dir_name != "benches" {
                    let module_path = format!("{}::{}", base_module, dir_name);
                    self.scan_directory(&path, types, &module_path, depth + 1)?;
                }
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Some(file_name) = path.file_stem().and_then(|n| n.to_str()) {
                    if file_name != "mod" && file_name != "lib" {
                        let module_path = if file_name == "lib" {
                            base_module.to_string()
                        } else {
                            format!("{}::{}", base_module, file_name)
                        };
                        self.scan_rust_file(&path, types, &module_path)?;
                    } else if file_name == "lib" {
                        self.scan_rust_file(&path, types, base_module)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn scan_rust_file(
        &self,
        path: &Path,
        types: &mut Vec<EvenframeType>,
        module_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        trace!("Scanning file: {:?}", path);
        let content = fs::read_to_string(path)?;
        trace!("Parsing file: {:?}, size: {} bytes", path, content.len());
        let syntax_tree = parse_file(&content)?;
        trace!("Successfully parsed file: {:?}", path);

        for item in syntax_tree.items {
            match item {
                Item::Struct(item_struct) => {
                    if has_evenframe_derive(&item_struct.attrs) {
                        let has_id = has_id_field(&item_struct.fields);
                        types.push(EvenframeType {
                            name: item_struct.ident.to_string(),
                            module_path: module_path.to_string(),
                            file_path: path.to_string_lossy().to_string(),
                            kind: TypeKind::Struct,
                            has_id_field: has_id,
                        });
                    }
                }
                Item::Enum(item_enum) => {
                    if has_evenframe_derive(&item_enum.attrs) {
                        types.push(EvenframeType {
                            name: item_enum.ident.to_string(),
                            module_path: module_path.to_string(),
                            file_path: path.to_string_lossy().to_string(),
                            kind: TypeKind::Enum,
                            has_id_field: false,
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn has_evenframe_derive(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("derive") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if tokens.contains("Evenframe") {
                    return true;
                }
            }
        }
    }
    false
}

fn has_id_field(fields: &syn::Fields) -> bool {
    match fields {
        syn::Fields::Named(fields_named) => fields_named
            .named
            .iter()
            .any(|field| field.ident.as_ref().map(|id| id == "id").unwrap_or(false)),
        _ => false,
    }
}

pub fn get_unique_modules(types: &[EvenframeType]) -> Vec<String> {
    let mut modules = HashSet::new();
    for t in types {
        modules.insert(t.module_path.clone());
    }
    modules.into_iter().collect()
}
