use crate::evenframe_log;
use crate::schemasync::TableConfig;
use crate::types::{FieldType, StructConfig, TaggedUnion, VariantData};
use convert_case::{Case, Casing};
use petgraph::algo::toposort;
use petgraph::{algo::kosaraju_scc, graphmap::DiGraphMap};
use std::collections::{HashMap, HashSet};

/// A helper struct to track recursion information for types
#[derive(Debug)]
pub struct RecursionInfo {
    /// `type_name -> scc_id`
    pub comp_of: HashMap<String, usize>,
    /// `scc_id -> { "is_recursive": bool, "members": Vec<String> }`
    pub meta: HashMap<usize, (bool, Vec<String>)>,
}

impl RecursionInfo {
    /// Returns true when current & target are in the **same** SCC and that SCC
    /// is either larger than 1 **or** has a self-loop
    pub fn is_recursive_pair(&self, current: &str, target: &str) -> bool {
        let c_id = self.comp_of.get(current);
        let t_id = self.comp_of.get(target);
        match (c_id, t_id) {
            (Some(c), Some(t)) if c == t => self.meta[c].0, // same comp & recursive
            _ => false,
        }
    }
}

/// Build the dependency graph from your `FieldType` tree and analyze recursion
pub fn analyse_recursion(
    structs: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
) -> RecursionInfo {
    let known: HashSet<_> = structs
        .values()
        .map(|sc| sc.name.to_case(Case::Pascal))
        .chain(enums.values().map(|e| e.enum_name.to_case(Case::Pascal)))
        .collect();

    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();

    for sc in structs.values() {
        let from = sc.name.to_case(Case::Pascal);
        let entry = deps.entry(from.clone()).or_default();
        for f in &sc.fields {
            collect_refs(&f.field_type, &known, entry);
        }
    }
    for e in enums.values() {
        let from = e.enum_name.to_case(Case::Pascal);
        let entry = deps.entry(from.clone()).or_default();
        for v in &e.variants {
            if let Some(variant_data) = &v.data {
                let variant_data_field_type = match variant_data {
                    VariantData::InlineStruct(inline_struct) => {
                        &FieldType::Other(inline_struct.name.clone())
                    }
                    VariantData::DataStructureRef(field_type) => field_type,
                };
                collect_refs(variant_data_field_type, &known, entry);
            }
        }
    }

    // Build graph
    let mut g: DiGraphMap<&str, ()> = DiGraphMap::new();
    for (from, tos) in &deps {
        // ensure node exists even if it has no outgoing edges
        g.add_node(from.as_str());
        for to in tos {
            g.add_edge(from.as_str(), to.as_str(), ());
        }
    }

    // Strongly connected components
    let sccs = kosaraju_scc(&g); // Vec<Vec<&str>>

    let mut comp_of = HashMap::<String, usize>::new();
    let mut meta = HashMap::<usize, (bool, Vec<String>)>::new();

    for (idx, comp) in sccs.iter().enumerate() {
        let self_loop = comp.len() == 1 && g.contains_edge(comp[0], comp[0]);
        let recursive = self_loop || comp.len() > 1;
        let members = comp.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        for m in &members {
            comp_of.insert(m.clone(), idx);
        }
        meta.insert(idx, (recursive, members));
    }

    RecursionInfo { comp_of, meta }
}

/// Returns the set of **direct** dependencies of a type name
/// (other structs / enums that it references in its fields or variants).
pub fn deps_of(
    name: &str,
    structs: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
) -> HashSet<String> {
    // Build a quick "known-types" set so we don't count primitives.
    let known: HashSet<_> = structs
        .values()
        .map(|sc| sc.name.to_case(Case::Pascal))
        .chain(enums.values().map(|e| e.enum_name.to_case(Case::Pascal)))
        .collect();

    let mut acc = HashSet::new();

    // If `name` is a struct, walk its fields
    if let Some(sc) = structs.values().find(|sc| sc.name.to_case(Case::Pascal) == name) {
        for f in &sc.fields {
            collect_refs(&f.field_type, &known, &mut acc);
        }
    }

    // If `name` is an enum, walk its variants
    if let Some(e) = enums
        .values()
        .find(|e| e.enum_name.to_case(Case::Pascal) == name)
    {
        for v in &e.variants {
            if let Some(variant_data) = &v.data {
                let variant_data_field_type = match variant_data {
                    VariantData::InlineStruct(inline_struct) => {
                        &FieldType::Other(inline_struct.name.clone())
                    }
                    VariantData::DataStructureRef(field_type) => field_type,
                };
                collect_refs(variant_data_field_type, &known, &mut acc);
            }
        }
    }

    acc
}

/// Collect references to other types from a FieldType
pub fn collect_refs(ft: &FieldType, known: &HashSet<String>, acc: &mut HashSet<String>) {
    use FieldType::*;
    match ft {
        Tuple(v) => v.iter().for_each(|f| collect_refs(f, known, acc)),
        Struct(v) => v.iter().for_each(|(_, f)| collect_refs(f, known, acc)),
        Option(i) | Vec(i) | RecordLink(i) => collect_refs(i, known, acc),
        HashMap(k, v) | BTreeMap(k, v) => {
            collect_refs(k, known, acc);
            collect_refs(v, known, acc);
        }
        Other(name) if known.contains(name) => {
            acc.insert(name.clone());
        }
        _ => {}
    }
}

/// Analyze recursion specifically for tables (TableConfig)
/// This is a specialized version that only considers table dependencies
pub fn analyse_recursion_tables(
    tables: &HashMap<String, crate::schemasync::TableConfig>,
) -> RecursionInfo {
    // Convert tables to structs for analysis
    let structs: HashMap<String, StructConfig> = tables
        .iter()
        .map(|(name, table)| (name.clone(), table.struct_config.clone()))
        .collect();

    // Tables don't have enums, so pass empty map
    let enums = HashMap::new();

    // Use the regular analyse_recursion with converted data
    analyse_recursion(&structs, &enums)
}

/// Get dependencies of a table by analyzing its struct config
pub fn deps_of_table(
    table_name: &str,
    tables: &HashMap<String, crate::schemasync::TableConfig>,
) -> HashSet<String> {
    // Build set of known table names in PascalCase
    let known: HashSet<_> = tables.keys().map(|s| s.to_case(Case::Pascal)).collect();

    // Build a map from PascalCase to original table names
    let pascal_to_original: HashMap<String, String> = tables
        .keys()
        .map(|k| (k.to_case(Case::Pascal), k.clone()))
        .collect();

    let mut acc = HashSet::new();

    // Find the table and analyze its fields
    if let Some(table) = tables.get(table_name) {
        for field in &table.struct_config.fields {
            collect_refs(&field.field_type, &known, &mut acc);
        }
    }

    // Convert PascalCase dependencies back to original table names
    acc.into_iter()
        .filter_map(|pascal_name| pascal_to_original.get(&pascal_name).cloned())
        .collect()
}

/// Collect all dependencies of a table including nested objects and enums
fn collect_table_dependencies(
    table_name: &str,
    tables: &HashMap<String, TableConfig>,
    objects: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
    visited_types: &mut HashSet<String>,
) -> HashSet<String> {
    let mut dependencies = HashSet::new();

    // Get the table configuration
    if let Some(table) = tables.get(table_name) {
        // If this is a relation table, it depends on both the 'from' and 'to' tables
        if let Some(relation) = &table.relation {
            // Add dependency on the 'from' table
            let from_snake = relation.from.to_case(Case::Snake);
            if tables.contains_key(&relation.from) {
                dependencies.insert(relation.from.clone());
            } else if tables.contains_key(&from_snake) {
                dependencies.insert(from_snake);
            }

            // Add dependency on the 'to' table
            let to_snake = relation.to.to_case(Case::Snake);
            if tables.contains_key(&relation.to) {
                dependencies.insert(relation.to.clone());
            } else if tables.contains_key(&to_snake) {
                dependencies.insert(to_snake);
            }
        }

        // Analyze each field in the table
        for field in &table.struct_config.fields {
            collect_field_type_dependencies(
                &field.field_type,
                tables,
                objects,
                enums,
                &mut dependencies,
                visited_types,
            );
        }
    }

    dependencies
}

/// Recursively collect dependencies from a field type
pub fn collect_field_type_dependencies(
    field_type: &FieldType,
    tables: &HashMap<String, TableConfig>,
    objects: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
    dependencies: &mut HashSet<String>,
    visited_types: &mut HashSet<String>,
) {
    match field_type {
        FieldType::Other(type_name) => {
            // Avoid infinite recursion
            if visited_types.contains(type_name) {
                return;
            }
            visited_types.insert(type_name.clone());

            let snake_case_name = type_name.to_case(Case::Snake);

            // Check if it's a table reference
            if tables.contains_key(type_name) {
                dependencies.insert(type_name.clone());
            } else if tables.contains_key(&snake_case_name) {
                dependencies.insert(snake_case_name.clone());
            }

            // Check if it's an object/struct and recursively analyze its fields
            if let Some(obj) = objects
                .get(type_name)
                .or_else(|| objects.get(&snake_case_name))
            {
                for field in &obj.fields {
                    collect_field_type_dependencies(
                        &field.field_type,
                        tables,
                        objects,
                        enums,
                        dependencies,
                        visited_types,
                    );
                }
            }

            // Check if it's an enum and analyze its variants
            if let Some(enum_def) = enums.get(type_name).or_else(|| enums.get(&snake_case_name)) {
                for variant in &enum_def.variants {
                    if let Some(variant_data) = &variant.data {
                        match variant_data {
                            VariantData::InlineStruct(inline_struct) => {
                                // Recursively analyze inline struct
                                if let Some(obj) = objects.get(&inline_struct.name) {
                                    for field in &obj.fields {
                                        collect_field_type_dependencies(
                                            &field.field_type,
                                            tables,
                                            objects,
                                            enums,
                                            dependencies,
                                            visited_types,
                                        );
                                    }
                                }
                            }
                            VariantData::DataStructureRef(ref_type) => {
                                collect_field_type_dependencies(
                                    ref_type,
                                    tables,
                                    objects,
                                    enums,
                                    dependencies,
                                    visited_types,
                                );
                            }
                        }
                    }
                }
            }
        }
        FieldType::Option(inner) | FieldType::Vec(inner) | FieldType::RecordLink(inner) => {
            collect_field_type_dependencies(
                inner,
                tables,
                objects,
                enums,
                dependencies,
                visited_types,
            );
        }
        FieldType::Tuple(types) => {
            for t in types {
                collect_field_type_dependencies(
                    t,
                    tables,
                    objects,
                    enums,
                    dependencies,
                    visited_types,
                );
            }
        }
        FieldType::Struct(fields) => {
            for (_, field_type) in fields {
                collect_field_type_dependencies(
                    field_type,
                    tables,
                    objects,
                    enums,
                    dependencies,
                    visited_types,
                );
            }
        }
        FieldType::HashMap(key_type, value_type) | FieldType::BTreeMap(key_type, value_type) => {
            collect_field_type_dependencies(
                key_type,
                tables,
                objects,
                enums,
                dependencies,
                visited_types,
            );
            collect_field_type_dependencies(
                value_type,
                tables,
                objects,
                enums,
                dependencies,
                visited_types,
            );
        }
        _ => {} // Primitive types
    }
}

/// Sort tables by dependencies using topological sort with SCC handling
pub fn sort_tables_by_dependencies(
    tables: &HashMap<String, TableConfig>,
    objects: &HashMap<String, StructConfig>,
    enums: &HashMap<String, TaggedUnion>,
) -> Vec<String> {
    // Build complete dependency graph including nested objects and enums
    let mut dependency_graph: HashMap<String, HashSet<String>> = HashMap::new();

    for table_name in tables.keys() {
        let mut visited_types = HashSet::new();
        let dependencies =
            collect_table_dependencies(table_name, tables, objects, enums, &mut visited_types);
        dependency_graph.insert(table_name.clone(), dependencies.clone());

        // Log dependencies for debugging
        if !dependencies.is_empty() {
            evenframe_log!(
                &format!("Table '{}' depends on: {:?}", table_name, &dependencies),
                "results.log",
                true
            );
        }
    }

    // Build petgraph for topological sorting
    let mut graph = DiGraphMap::<&str, ()>::new();

    // Add all nodes first
    for table_name in tables.keys() {
        graph.add_node(table_name.as_str());
    }

    // Add edges (A depends on B = edge from A to B)
    for (table_name, dependencies) in &dependency_graph {
        for dep in dependencies {
            if tables.contains_key(dep) {
                graph.add_edge(table_name.as_str(), dep.as_str(), ());
            }
        }
    }

    // Detect strongly connected components for circular dependencies
    let sccs = petgraph::algo::kosaraju_scc(&graph);

    // Build condensation graph (DAG of SCCs)
    let mut scc_map: HashMap<&str, usize> = HashMap::new();
    for (idx, scc) in sccs.iter().enumerate() {
        for node in scc {
            scc_map.insert(*node, idx);
        }
    }

    let mut condensation = DiGraphMap::<usize, ()>::new();
    for (from, tos) in &dependency_graph {
        if let Some(&from_scc) = scc_map.get(from.as_str()) {
            for to in tos {
                if let Some(&to_scc) = scc_map.get(to.as_str()) {
                    if from_scc != to_scc {
                        condensation.add_edge(from_scc, to_scc, ());
                    }
                }
            }
        }
    }

    // Topological sort of SCCs
    let sorted_sccs = match toposort(&condensation, None) {
        Ok(order) => order,
        Err(_) => {
            // If there's still a cycle (shouldn't happen with SCC), fall back to arbitrary order
            evenframe_log!(
                "Warning: Cycle detected in SCC condensation graph",
                "results.log",
                true
            );
            (0..sccs.len()).collect()
        }
    };

    // Build final sorted list
    let mut result = Vec::new();
    let mut processed_tables = HashSet::new();

    // Process SCCs in reverse topological order (dependencies first)
    for scc_idx in sorted_sccs.into_iter().rev() {
        // Find all tables in this SCC
        let mut scc_tables: Vec<String> = tables
            .keys()
            .filter(|name| scc_map.get(name.as_str()) == Some(&scc_idx))
            .cloned()
            .collect();

        // Sort within SCC for deterministic output
        scc_tables.sort();

        // Log SCC info if it contains multiple tables
        if scc_tables.len() > 1 {
            evenframe_log!(
                &format!(
                    "Circular dependency detected among tables: {:?}",
                    scc_tables
                ),
                "results.log",
                true
            );
        }

        for table in &scc_tables {
            processed_tables.insert(table.clone());
        }
        result.extend(scc_tables);
    }

    // Add any tables that weren't in the graph (isolated nodes with no dependencies)
    let mut missing_tables: Vec<String> = tables
        .keys()
        .filter(|name| !processed_tables.contains(*name))
        .cloned()
        .collect();
    missing_tables.sort();

    if !missing_tables.is_empty() {
        evenframe_log!(
            &format!(
                "Tables with no dependencies (adding at beginning): {:?}",
                missing_tables
            ),
            "results.log",
            true
        );
        // Add tables with no dependencies at the beginning
        result = missing_tables
            .into_iter()
            .chain(result.into_iter())
            .collect();
    }

    evenframe_log!(
        &format!("Final sorted table order: {:?}", result),
        "results.log",
        true
    );

    result
}
