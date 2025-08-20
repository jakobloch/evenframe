use evenframe_core::error::{EvenframeError, Result};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Attribute, Item, Meta, parse_file};
use tracing::{debug, info, trace, warn};
use walkdir::WalkDir;

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
    start_path: PathBuf,
    apply_aliases: Vec<String>,
}

impl WorkspaceScanner {
    /// Creates a new WorkspaceScanner that starts scanning from the current directory.
    ///
    /// # Arguments
    ///
    /// * `apply_aliases` - A list of attribute aliases to look for.
    pub fn new(apply_aliases: Vec<String>) -> Result<Self> {
        let start_path = env::current_dir()?;
        Ok(Self::with_path(start_path, apply_aliases))
    }

    /// Creates a new WorkspaceScanner with a specific start path.
    ///
    /// # Arguments
    ///
    /// * `start_path` - The directory to start scanning from. It will search this
    ///   directory and its children for Rust workspaces or standalone crates.
    /// * `apply_aliases` - A list of attribute aliases to look for.
    pub fn with_path(start_path: PathBuf, apply_aliases: Vec<String>) -> Self {
        Self {
            start_path,
            apply_aliases,
        }
    }

    /// Scans for Rust workspaces and collects all Evenframe types within them.
    pub fn scan_for_evenframe_types(&self) -> Result<Vec<EvenframeType>> {
        info!(
            "Starting workspace scan for Evenframe types from path: {:?}",
            self.start_path
        );

        let mut types = Vec::new();
        let mut processed_manifests = HashSet::new();

        // Use WalkDir to efficiently find all Cargo.toml files.
        for entry in WalkDir::new(&self.start_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "Cargo.toml")
        {
            let manifest_path = entry.path();
            // Avoid processing the same manifest multiple times.
            if processed_manifests.contains(manifest_path) {
                continue;
            }

            trace!("Found potential manifest: {:?}", manifest_path);
            if let Err(e) =
                self.process_manifest(manifest_path, &mut types, &mut processed_manifests)
            {
                warn!("Failed to process manifest at {:?}: {}", manifest_path, e);
            }
        }

        info!(
            "Workspace scan complete. Found {} Evenframe types",
            types.len()
        );
        debug!(
            "Type breakdown: {} structs, {} enums",
            types.iter().filter(|t| t.kind == TypeKind::Struct).count(),
            types.iter().filter(|t| t.kind == TypeKind::Enum).count()
        );

        Ok(types)
    }

    /// Processes a Cargo.toml file, determines if it's a workspace or a single
    /// crate, and scans the corresponding source files.
    fn process_manifest(
        &self,
        manifest_path: &Path,
        types: &mut Vec<EvenframeType>,
        processed_manifests: &mut HashSet<PathBuf>,
    ) -> Result<()> {
        let manifest_dir = manifest_path
            .parent()
            .ok_or_else(|| EvenframeError::InvalidPath {
                path: manifest_path.to_path_buf(),
            })?;

        let content = fs::read_to_string(manifest_path)?;
        let manifest: toml::Value = toml::from_str(&content)
            .map_err(|e| EvenframeError::parse_error(manifest_path, e.to_string()))?;

        // Check if this is a workspace manifest and scan its members.
        if let Some(workspace) = manifest.get("workspace").and_then(|w| w.as_table()) {
            if let Some(members) = workspace.get("members").and_then(|m| m.as_array()) {
                debug!("Processing workspace at: {:?}", manifest_dir);
                processed_manifests.insert(manifest_path.to_path_buf());

                for member in members.iter().filter_map(|v| v.as_str()) {
                    // Note: For a full implementation, you might use the `glob` crate
                    // to handle patterns like "crates/*". This example handles direct paths.
                    let member_path = manifest_dir.join(member);
                    if member_path.is_dir() {
                        let crate_name = member_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown_crate");
                        let src_path = member_path.join("src");
                        if src_path.exists() {
                            info!(
                                "Scanning workspace member: {} at {:?}",
                                crate_name, src_path
                            );
                            self.scan_directory(&src_path, types, crate_name, 0)?;
                        } else {
                            warn!(
                                "Workspace member '{}' does not have a 'src' directory.",
                                member
                            );
                        }
                    } else {
                        warn!(
                            "Workspace member path '{}' is not a directory or does not exist.",
                            member
                        );
                    }
                }
            }
        }
        // Check if this is a standalone crate and scan it.
        else if manifest.get("package").is_some() {
            debug!("Processing single crate at: {:?}", manifest_dir);
            processed_manifests.insert(manifest_path.to_path_buf());
            let crate_name = manifest
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or_else(|| {
                    manifest_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown_crate")
                });

            let src_path = manifest_dir.join("src");
            if src_path.exists() {
                info!("Scanning crate: {} at {:?}", crate_name, src_path);
                self.scan_directory(&src_path, types, crate_name, 0)?;
            }
        }

        Ok(())
    }

    /// Recursively scans a directory for Rust source files.
    fn scan_directory(
        &self,
        dir: &Path,
        types: &mut Vec<EvenframeType>,
        base_module: &str,
        depth: usize,
    ) -> Result<()> {
        trace!(
            "Scanning directory: {:?}, module: {}, depth: {}",
            dir, base_module, depth
        );

        if depth > 10 {
            return Err(EvenframeError::MaxRecursionDepth {
                depth: 10,
                path: dir.to_path_buf(),
            });
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

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
                let file_stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");

                // FIX: Correctly handle `mod.rs` files.
                if file_stem == "lib" || file_stem == "main" {
                    // Crate root, use the base module path directly.
                    self.scan_rust_file(&path, types, base_module)?;
                } else if path.file_name().and_then(|n| n.to_str()) == Some("mod.rs") {
                    // A `mod.rs` file defines the module for its parent directory.
                    // The `base_module` path is already correct for this case.
                    self.scan_rust_file(&path, types, base_module)?;
                } else {
                    // A regular submodule file (e.g., `user.rs`).
                    let module_path = format!("{}::{}", base_module, file_stem);
                    self.scan_rust_file(&path, types, &module_path)?;
                }
            }
        }
        Ok(())
    }

    /// Parses a single Rust file to find relevant structs and enums.
    fn scan_rust_file(
        &self,
        path: &Path,
        types: &mut Vec<EvenframeType>,
        module_path: &str,
    ) -> Result<()> {
        trace!("Scanning file: {:?}, module: {}", path, module_path);
        let content = fs::read_to_string(path)?;
        let syntax_tree =
            parse_file(&content).map_err(|e| EvenframeError::parse_error(path, e.to_string()))?;

        for item in syntax_tree.items {
            let (attrs, ident, kind, fields) = match item {
                Item::Struct(s) => (s.attrs, s.ident, TypeKind::Struct, Some(s.fields)),
                Item::Enum(e) => (e.attrs, e.ident, TypeKind::Enum, None),
                _ => continue,
            };

            if has_evenframe_derive(&attrs) || self.has_apply_alias(&attrs) {
                let name = ident.to_string();
                let has_id_field = fields.is_some_and(|f| has_id_field(&f));

                debug!(
                    "Found Evenframe {:?} '{}' in module '{}'",
                    kind, name, module_path
                );

                types.push(EvenframeType {
                    name,
                    module_path: module_path.to_string(),
                    file_path: path.to_string_lossy().to_string(),
                    kind,
                    has_id_field,
                });
            }
        }
        Ok(())
    }

    /// Checks for `#[apply(Alias)]` attributes.
    fn has_apply_alias(&self, attrs: &[Attribute]) -> bool {
        self.apply_aliases.iter().any(|alias| {
            attrs.iter().any(|attr| {
                if attr.path().is_ident("apply")
                    && let Meta::List(meta_list) = &attr.meta
                {
                    return meta_list.tokens.to_string() == *alias;
                }

                false
            })
        })
    }
}

/// Checks for `#[derive(..., Evenframe, ...)]`.
fn has_evenframe_derive(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("derive")
            && let Meta::List(meta_list) = &attr.meta
        {
            return meta_list.tokens.to_string().contains("Evenframe");
        }

        false
    })
}

/// Checks if a struct has a field named `id`.
fn has_id_field(fields: &syn::Fields) -> bool {
    if let syn::Fields::Named(fields_named) = fields {
        fields_named
            .named
            .iter()
            .any(|field| field.ident.as_ref().is_some_and(|id| id == "id"))
    } else {
        false
    }
}

/// Helper function to extract unique module paths from the found types.
pub fn _get_unique_modules(types: &[EvenframeType]) -> Vec<String> {
    let mut modules: HashSet<_> = types.iter().map(|t| t.module_path.clone()).collect();
    let unique_modules: Vec<String> = modules.drain().collect();
    debug!(
        "Found {} unique modules from {} types",
        unique_modules.len(),
        types.len()
    );
    trace!("Unique modules: {:?}", unique_modules);
    unique_modules
}
