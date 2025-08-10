use crate::mockmake::Mockmaker;
use crate::schemasync::config::SchemasyncMockGenConfig;
use crate::schemasync::table::TableConfig;
use crate::types::StructField;
use bon::Builder;
use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing;
use try_from_expr::TryFromExpr;
use uuid::Uuid;

/// Parse a field path like "recurrence_rule.recurrence_begins" into (parent, child)
fn parse_field_path(field_path: &str) -> (Option<String>, String) {
    tracing::trace!(field_path = %field_path, "Parsing field path");
    if let Some(dot_pos) = field_path.find('.') {
        let parent = field_path[..dot_pos].to_string();
        let child = field_path[dot_pos + 1..].to_string();
        tracing::trace!(parent = %parent, child = %child, "Field path has parent");
        (Some(parent), child)
    } else {
        tracing::trace!("Field path has no parent");
        (None, field_path.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Builder)]
pub struct CoordinationGroup {
    #[builder(default)]
    pub id: Uuid,
    #[builder(default)]
    pub tables: HashSet<TableName>,
    #[builder(default)]
    pub field_coordination_pairs: Vec<(HashSet<(TableName, String)>, Coordination)>,
}
impl Mockmaker {
    pub fn generate_coordinated_values(
        &self,
        table_name: &str,
        table_config: &TableConfig,
        coordination_group: CoordinationGroup,
    ) -> Vec<HashMap<String, String>> {
        let mut all_values = Vec::new();

        tracing::debug!(
            table = %table_name,
            coordination_pairs = coordination_group.field_coordination_pairs.len(),
            "Generating coordinated values for table"
        );

        let n = self
            .tables
            .get(table_name)
            .expect("TableConfig was not found")
            .mock_generation_config
            .as_ref()
            .map(|c| c.n)
            .unwrap_or(self.schemasync_config.mock_gen_config.default_record_count);

        // Build field map for quick lookup
        let field_map: HashMap<String, &StructField> = table_config
            .struct_config
            .fields
            .iter()
            .map(|f| (f.field_name.clone(), f))
            .collect();

        let table_name_snake = table_config.struct_config.name.to_case(Case::Snake);

        for index in 0..n as usize {
            let mut record_values = HashMap::new();

            for (field_set, coordination) in &coordination_group.field_coordination_pairs {
                match coordination {
                    Coordination::InitializeEqual(_) => {
                        // Separate fields into direct and nested
                        let mut direct_fields = Vec::new();
                        let mut nested_field_values = HashMap::new();

                        for (table, field_name) in field_set {
                            if table == &table_name_snake {
                                let (parent, child) = parse_field_path(field_name);
                                if let Some(parent_field) = parent {
                                    // This is a nested field like "recurrence_rule.recurrence_begins"
                                    nested_field_values
                                        .insert(field_name.clone(), (parent_field, child));
                                } else {
                                    // This is a direct field
                                    if let Some(field) = field_map.get(field_name) {
                                        direct_fields.push(*field);
                                    }
                                }
                            }
                        }

                        // Generate value for all fields in the coordination group
                        if !direct_fields.is_empty() || !nested_field_values.is_empty() {
                            // Use the first direct field to determine the value type, or generate a default
                            let base_value = if let Some(first_field) = direct_fields.first() {
                                if let Some(format) = &first_field.format {
                                    format.generate_formatted_value()
                                } else {
                                    // Generate based on field type
                                    match &first_field.field_type {
                                        crate::types::FieldType::DateTime => {
                                            chrono::Utc::now().to_rfc3339()
                                        }
                                        _ => crate::schemasync::random_string(8),
                                    }
                                }
                            } else {
                                // If no direct fields, generate a datetime value (common for coordination)
                                chrono::Utc::now().to_rfc3339()
                            };

                            // Apply to direct fields
                            for field in &direct_fields {
                                record_values.insert(field.field_name.clone(), base_value.clone());
                            }

                            // Store nested field values for later use
                            for (full_path, _) in &nested_field_values {
                                record_values.insert(full_path.clone(), base_value.clone());
                            }
                        }
                    }
                    Coordination::InitializeSequential {
                        field_names: _,
                        increment,
                    } => {
                        // Get fields that belong to the current table
                        let fields: Vec<_> = field_set
                            .iter()
                            .filter_map(|(table, field_name)| {
                                if table == &table_name_snake {
                                    field_map.get(field_name).copied()
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if !fields.is_empty() {
                            let values =
                                Mockmaker::generate_sequential_values(&fields, index, increment);
                            record_values.extend(values);
                        }
                    }
                    Coordination::InitializeSum {
                        field_names: _,
                        total,
                    } => {
                        // Get fields that belong to the current table
                        let fields: Vec<_> = field_set
                            .iter()
                            .filter_map(|(table, field_name)| {
                                if table == &table_name_snake {
                                    field_map.get(field_name).copied()
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if !fields.is_empty() {
                            let values = Mockmaker::generate_sum_values(&fields, index, *total);
                            record_values.extend(values);
                        }
                    }
                    Coordination::InitializeDerive {
                        source_field_names: _,
                        target_field_name,
                        derivation,
                    } => {
                        // For derive, we need source values from the current record
                        let source_fields: Vec<_> = field_set
                            .iter()
                            .filter_map(|(table, field_name)| {
                                if table == &table_name_snake && field_name != target_field_name {
                                    field_map.get(field_name).copied()
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if !source_fields.is_empty() {
                            let values = Mockmaker::generate_derive_values(
                                &source_fields,
                                target_field_name,
                                derivation,
                                &record_values,
                            );
                            record_values.extend(values);
                        }
                    }
                    Coordination::InitializeCoherent(coherent_dataset) => {
                        // Get fields that belong to the current table
                        let fields: Vec<_> = field_set
                            .iter()
                            .filter_map(|(table, field_name)| {
                                if table == &table_name_snake {
                                    field_map.get(field_name).copied()
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if !fields.is_empty() {
                            let values = Mockmaker::generate_coherent_values(
                                &fields,
                                coherent_dataset,
                                index,
                            );
                            record_values.extend(values);
                        }
                    }
                }
            }

            all_values.push(record_values);
        }

        all_values
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TryFromExpr)]
pub enum CoordinatedValue {
    String(String),
    F64(f64),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TryFromExpr)]
pub enum Coordination {
    /// Initialize multiple fields with the same value
    InitializeEqual(Vec<String>),

    /// Initialize fields in sequence (e.g., start < end dates)
    InitializeSequential {
        field_names: Vec<String>,
        increment: CoordinateIncrement,
    },

    /// Fields must sum to a total (e.g., percentage fields = 100)
    InitializeSum {
        field_names: Vec<String>,
        total: f64,
    },

    /// One field derives from another (e.g., full_name from first + last)
    InitializeDerive {
        source_field_names: Vec<String>,
        target_field_name: String,
        derivation: DerivationType,
    },

    /// Ensure fields are from same dataset (e.g., matching city/state/zip)
    InitializeCoherent(CoherentDataset),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TryFromExpr)]
pub enum CoordinateIncrement {
    Days(i32),
    Hours(i32),
    Minutes(i32),
    Numeric(f64),
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TryFromExpr)]
pub enum DerivationType {
    Concatenate(String), // separator
    Extract(ExtractType),
    Transform(TransformType),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TryFromExpr)]
pub enum ExtractType {
    FirstWord,
    LastWord,
    Domain,   // from email
    Username, // from email
    Initials,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TryFromExpr)]
pub enum TransformType {
    Uppercase,
    Lowercase,
    Capitalize,
    Truncate(usize),
    Hash,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TryFromExpr)]
pub enum CoherentDataset {
    Address {
        city: String,
        state: String,
        zip: String,
        country: String,
    },
    PersonName {
        first_name: String,
        last_name: String,
        full_name: String,
    },
    GeoLocation {
        latitude: String,
        longitude: String,
        city: String,
        country: String,
    },
    DateRange {
        start_date: String,
        end_date: String,
    },
}

/// Context for managing field coordination during mock data generation
#[derive(Debug)]
pub struct TableInsertsState {
    pub table_name: String,
    /// Coordination rules from mock_data attribute
    pub coordination_group: CoordinationGroup,
}
type TableName = String;
impl TableInsertsState {
    pub fn new(table_config: &TableConfig, global_config: &SchemasyncMockGenConfig) -> Self {
        let table_name_snake = table_config.struct_config.name.to_case(Case::Snake);
        let mut table_names = HashSet::new();
        let mut field_coordination_pairs = Vec::new();

        for coordination_group in &global_config.global_coordination_groups {
            for (table_field_pair_set, coordination) in &coordination_group.field_coordination_pairs
            {
                if table_field_pair_set
                    .iter()
                    .any(|(table, _)| table == &table_name_snake)
                {
                    // Add all matching table names from this set
                    for (table, _) in table_field_pair_set {
                        if table == &table_name_snake {
                            table_names.insert(table.clone());
                        }
                    }
                    // Add the coordination pair
                    field_coordination_pairs
                        .push((table_field_pair_set.clone(), coordination.clone()));
                }
            }
        }

        if table_names.is_empty() {
            table_names.insert(table_name_snake.clone());
        }

        let mut coordination_group = CoordinationGroup::builder().build();
        coordination_group.tables = table_names;
        coordination_group.field_coordination_pairs = field_coordination_pairs;

        Self {
            table_name: table_name_snake,
            coordination_group,
        }
    }
}

/// Trait for custom coordinators
pub trait CustomCoordinator: Send + Sync {
    fn generate(&self, fields: &[&str], index: usize) -> HashMap<String, String>;
}

// Extended address dataset with more US cities
pub const EXTENDED_ADDRESSES: &[(&str, &str, &str, &str)] = &[
    // Original addresses
    ("New York", "NY", "10001", "USA"),
    ("Los Angeles", "CA", "90001", "USA"),
    ("Chicago", "IL", "60601", "USA"),
    ("Houston", "TX", "77001", "USA"),
    ("Phoenix", "AZ", "85001", "USA"),
    ("Philadelphia", "PA", "19101", "USA"),
    ("San Antonio", "TX", "78201", "USA"),
    ("San Diego", "CA", "92101", "USA"),
    ("Dallas", "TX", "75201", "USA"),
    ("San Jose", "CA", "95101", "USA"),
    // Additional cities
    ("Austin", "TX", "78701", "USA"),
    ("Jacksonville", "FL", "32099", "USA"),
    ("Fort Worth", "TX", "76101", "USA"),
    ("Columbus", "OH", "43085", "USA"),
    ("Charlotte", "NC", "28201", "USA"),
    ("San Francisco", "CA", "94101", "USA"),
    ("Indianapolis", "IN", "46201", "USA"),
    ("Seattle", "WA", "98101", "USA"),
    ("Denver", "CO", "80201", "USA"),
    ("Boston", "MA", "02101", "USA"),
    ("El Paso", "TX", "79901", "USA"),
    ("Nashville", "TN", "37201", "USA"),
    ("Detroit", "MI", "48201", "USA"),
    ("Oklahoma City", "OK", "73101", "USA"),
    ("Portland", "OR", "97201", "USA"),
    ("Las Vegas", "NV", "89101", "USA"),
    ("Memphis", "TN", "38101", "USA"),
    ("Louisville", "KY", "40201", "USA"),
    ("Baltimore", "MD", "21201", "USA"),
    ("Milwaukee", "WI", "53201", "USA"),
    ("Albuquerque", "NM", "87101", "USA"),
    ("Tucson", "AZ", "85701", "USA"),
    ("Fresno", "CA", "93701", "USA"),
    ("Mesa", "AZ", "85201", "USA"),
    ("Sacramento", "CA", "94203", "USA"),
    ("Atlanta", "GA", "30301", "USA"),
    ("Kansas City", "MO", "64101", "USA"),
    ("Colorado Springs", "CO", "80901", "USA"),
    ("Omaha", "NE", "68101", "USA"),
    ("Raleigh", "NC", "27601", "USA"),
    ("Miami", "FL", "33101", "USA"),
    ("Long Beach", "CA", "90801", "USA"),
    ("Virginia Beach", "VA", "23450", "USA"),
    ("Oakland", "CA", "94601", "USA"),
    ("Minneapolis", "MN", "55401", "USA"),
    ("Tulsa", "OK", "74101", "USA"),
    ("Tampa", "FL", "33601", "USA"),
    ("Arlington", "TX", "76001", "USA"),
    ("New Orleans", "LA", "70112", "USA"),
];

// Extended geo location dataset with international cities
pub const EXTENDED_GEO_LOCATIONS: &[(&str, f64, f64, &str)] = &[
    // US Cities
    ("New York", 40.7128, -74.0060, "USA"),
    ("Los Angeles", 34.0522, -118.2437, "USA"),
    ("Chicago", 41.8781, -87.6298, "USA"),
    ("Houston", 29.7604, -95.3698, "USA"),
    ("Phoenix", 33.4484, -112.0740, "USA"),
    ("Philadelphia", 39.9526, -75.1652, "USA"),
    ("San Antonio", 29.4241, -98.4936, "USA"),
    ("San Diego", 32.7157, -117.1611, "USA"),
    ("Dallas", 32.7767, -96.7970, "USA"),
    ("San Jose", 37.3382, -121.8863, "USA"),
    ("Austin", 30.2672, -97.7431, "USA"),
    ("San Francisco", 37.7749, -122.4194, "USA"),
    ("Seattle", 47.6062, -122.3321, "USA"),
    ("Denver", 39.7392, -104.9903, "USA"),
    ("Boston", 42.3601, -71.0589, "USA"),
    ("Miami", 25.7617, -80.1918, "USA"),
    // International Cities
    ("London", 51.5074, -0.1278, "UK"),
    ("Paris", 48.8566, 2.3522, "France"),
    ("Tokyo", 35.6762, 139.6503, "Japan"),
    ("Sydney", -33.8688, 151.2093, "Australia"),
    ("Berlin", 52.5200, 13.4050, "Germany"),
    ("Madrid", 40.4168, -3.7038, "Spain"),
    ("Rome", 41.9028, 12.4964, "Italy"),
    ("Toronto", 43.6532, -79.3832, "Canada"),
    ("Amsterdam", 52.3676, 4.9041, "Netherlands"),
    ("Stockholm", 59.3293, 18.0686, "Sweden"),
    ("Oslo", 59.9139, 10.7522, "Norway"),
    ("Copenhagen", 55.6761, 12.5683, "Denmark"),
    ("Helsinki", 60.1699, 24.9384, "Finland"),
    ("Vienna", 48.2082, 16.3738, "Austria"),
    ("Prague", 50.0755, 14.4378, "Czech Republic"),
    ("Warsaw", 52.2297, 21.0122, "Poland"),
    ("Budapest", 47.4979, 19.0402, "Hungary"),
    ("Athens", 37.9838, 23.7275, "Greece"),
    ("Lisbon", 38.7223, -9.1393, "Portugal"),
    ("Dublin", 53.3498, -6.2603, "Ireland"),
    ("Brussels", 50.8503, 4.3517, "Belgium"),
    ("Zurich", 47.3769, 8.5417, "Switzerland"),
    ("Mumbai", 19.0760, 72.8777, "India"),
    ("Singapore", 1.3521, 103.8198, "Singapore"),
    ("Hong Kong", 22.3193, 114.1694, "Hong Kong"),
    ("Shanghai", 31.2304, 121.4737, "China"),
    ("Beijing", 39.9042, 116.4074, "China"),
    ("Seoul", 37.5665, 126.9780, "South Korea"),
    ("Bangkok", 13.7563, 100.5018, "Thailand"),
    ("Dubai", 25.2048, 55.2708, "UAE"),
    ("Cairo", 30.0444, 31.2357, "Egypt"),
    ("Istanbul", 41.0082, 28.9784, "Turkey"),
    ("Moscow", 55.7558, 37.6173, "Russia"),
    ("São Paulo", -23.5505, -46.6333, "Brazil"),
    ("Buenos Aires", -34.6037, -58.3816, "Argentina"),
    ("Mexico City", 19.4326, -99.1332, "Mexico"),
    ("Lima", -12.0464, -77.0428, "Peru"),
    ("Bogotá", 4.7110, -74.0721, "Colombia"),
    ("Santiago", -33.4489, -70.6693, "Chile"),
    ("Cape Town", -33.9249, 18.4241, "South Africa"),
    ("Johannesburg", -26.2041, 28.0473, "South Africa"),
    ("Lagos", 6.5244, 3.3792, "Nigeria"),
    ("Nairobi", -1.2921, 36.8219, "Kenya"),
];

// Extended person name combinations
pub const EXTENDED_PERSON_NAMES: &[(&str, &str, &str)] = &[
    // Common American names
    ("John", "Smith", "Male"),
    ("Jane", "Johnson", "Female"),
    ("Michael", "Williams", "Male"),
    ("Sarah", "Brown", "Female"),
    ("David", "Jones", "Male"),
    ("Emma", "Garcia", "Female"),
    ("James", "Miller", "Male"),
    ("Lisa", "Davis", "Female"),
    ("Robert", "Rodriguez", "Male"),
    ("Mary", "Martinez", "Female"),
    ("William", "Hernandez", "Male"),
    ("Patricia", "Lopez", "Female"),
    ("Richard", "Gonzalez", "Male"),
    ("Jennifer", "Wilson", "Female"),
    ("Thomas", "Anderson", "Male"),
    ("Linda", "Thomas", "Female"),
    ("Charles", "Taylor", "Male"),
    ("Elizabeth", "Moore", "Female"),
    ("Joseph", "Jackson", "Male"),
    ("Barbara", "Martin", "Female"),
    ("Christopher", "Lee", "Male"),
    ("Susan", "Perez", "Female"),
    ("Daniel", "Thompson", "Male"),
    ("Jessica", "White", "Female"),
    ("Matthew", "Harris", "Male"),
    ("Karen", "Sanchez", "Female"),
    ("Anthony", "Clark", "Male"),
    ("Nancy", "Ramirez", "Female"),
    ("Mark", "Lewis", "Male"),
    ("Betty", "Robinson", "Female"),
    ("Donald", "Walker", "Male"),
    ("Helen", "Young", "Female"),
    ("Kenneth", "Allen", "Male"),
    ("Sandra", "King", "Female"),
    ("Steven", "Wright", "Male"),
    ("Donna", "Scott", "Female"),
    ("Paul", "Torres", "Male"),
    ("Carol", "Nguyen", "Female"),
    ("Joshua", "Hill", "Male"),
    ("Michelle", "Flores", "Female"),
    ("Andrew", "Green", "Male"),
    ("Laura", "Adams", "Female"),
    ("George", "Nelson", "Male"),
    ("Dorothy", "Baker", "Female"),
    ("Kevin", "Hall", "Male"),
    ("Maria", "Rivera", "Female"),
    ("Brian", "Campbell", "Male"),
    ("Amy", "Mitchell", "Female"),
    ("Edward", "Carter", "Male"),
    ("Shirley", "Roberts", "Female"),
];

// Financial test scenarios
pub const FINANCIAL_SCENARIOS: &[(&str, f64, f64, f64)] = &[
    ("Retail Purchase", 99.99, 8.00, 107.99),
    ("Restaurant Bill", 85.00, 6.80, 91.80),
    ("Online Order", 249.99, 20.00, 269.99),
    ("Grocery Shopping", 156.43, 12.51, 168.94),
    ("Electronics", 599.00, 47.92, 646.92),
    ("Clothing", 125.50, 10.04, 135.54),
    ("Books", 45.99, 3.68, 49.67),
    ("Subscription", 19.99, 1.60, 21.59),
    ("Software License", 299.00, 23.92, 322.92),
    ("Hardware", 1299.00, 103.92, 1402.92),
    ("Office Supplies", 67.89, 5.43, 73.32),
    ("Fuel Purchase", 45.00, 3.60, 48.60),
    ("Pharmacy", 34.99, 2.80, 37.79),
    ("Entertainment", 150.00, 12.00, 162.00),
    ("Home Improvement", 425.00, 34.00, 459.00),
    ("Automotive Parts", 189.99, 15.20, 205.19),
    ("Pet Supplies", 78.50, 6.28, 84.78),
    ("Sports Equipment", 299.99, 24.00, 323.99),
    ("Garden Supplies", 112.50, 9.00, 121.50),
    ("Art Supplies", 89.99, 7.20, 97.19),
    ("Musical Instruments", 699.00, 55.92, 754.92),
    ("Furniture", 899.00, 71.92, 970.92),
    ("Appliances", 1599.00, 127.92, 1726.92),
    ("Jewelry", 450.00, 36.00, 486.00),
    ("Cosmetics", 65.50, 5.24, 70.74),
    ("Toys", 39.99, 3.20, 43.19),
    ("Video Games", 59.99, 4.80, 64.79),
    ("Streaming Service", 14.99, 1.20, 16.19),
    ("Cloud Storage", 9.99, 0.80, 10.79),
    ("Phone Bill", 85.00, 6.80, 91.80),
];

// Date range test scenarios
pub const DATE_RANGE_SCENARIOS: &[(&str, i64, &str)] = &[
    ("Sprint", 14, "2 week sprint"),
    ("Quarter", 90, "Financial quarter"),
    ("Semester", 120, "Academic semester"),
    ("Project Phase", 30, "Monthly phase"),
    ("Trial Period", 7, "Weekly trial"),
    ("Contract", 365, "Annual contract"),
    ("Warranty", 730, "2-year warranty"),
    ("Subscription", 30, "Monthly subscription"),
    ("Campaign", 45, "Marketing campaign"),
    ("Event", 3, "3-day event"),
    ("Weekend", 2, "Weekend getaway"),
    ("Workweek", 5, "Business week"),
    ("Fortnight", 14, "Two weeks"),
    ("Billing Cycle", 30, "Monthly billing"),
    ("Academic Year", 280, "School year"),
    ("Summer Break", 90, "Summer vacation"),
    ("Holiday Season", 45, "Holiday period"),
    ("Training Program", 60, "2-month training"),
    ("Probation Period", 90, "3-month probation"),
    ("Notice Period", 30, "1-month notice"),
    ("Lease Term", 365, "1-year lease"),
    ("Conference", 4, "4-day conference"),
    ("Workshop", 1, "1-day workshop"),
    ("Internship", 90, "3-month internship"),
    ("Product Launch", 21, "3-week launch"),
    ("Beta Test", 30, "1-month beta"),
    ("Evaluation Period", 15, "2-week evaluation"),
    ("Certification", 180, "6-month certification"),
    ("Membership", 365, "Annual membership"),
    ("Insurance Term", 180, "6-month term"),
];

// Company and job title combinations
pub const COMPANY_JOB_COMBINATIONS: &[(&str, &str, &str)] = &[
    ("TechCorp Inc", "Software Engineer", "Technology"),
    ("DataSoft LLC", "Data Scientist", "Analytics"),
    ("CloudVision Corp", "DevOps Engineer", "Infrastructure"),
    ("InnovateTech Ltd", "Product Manager", "Product"),
    ("NextGen Co", "UX Designer", "Design"),
    ("ProSystems Inc", "Backend Developer", "Engineering"),
    ("Digital Solutions LLC", "Frontend Developer", "Engineering"),
    ("Global Analytics Corp", "Business Analyst", "Business"),
    ("Enterprise Systems Ltd", "System Administrator", "IT"),
    ("Creative Studios Co", "Graphic Designer", "Design"),
    ("Marketing Plus Inc", "Marketing Manager", "Marketing"),
    ("Sales Force LLC", "Sales Representative", "Sales"),
    ("Finance Pro Corp", "Financial Analyst", "Finance"),
    ("Legal Associates Ltd", "Legal Counsel", "Legal"),
    ("HR Solutions Co", "HR Manager", "Human Resources"),
    ("Operations Hub Inc", "Operations Manager", "Operations"),
    ("Quality First LLC", "QA Engineer", "Quality"),
    ("Security Shield Corp", "Security Analyst", "Security"),
    ("Mobile Apps Ltd", "Mobile Developer", "Engineering"),
    ("AI Innovations Co", "ML Engineer", "AI/ML"),
    ("Web Dynamics Inc", "Full Stack Developer", "Engineering"),
    ("Data Insights LLC", "Data Engineer", "Analytics"),
    ("Cloud Native Corp", "Cloud Architect", "Infrastructure"),
    ("Product Vision Ltd", "Product Designer", "Design"),
    ("Tech Startup Co", "CTO", "Executive"),
    (
        "Enterprise Cloud Inc",
        "Solutions Architect",
        "Architecture",
    ),
    ("Digital Marketing LLC", "SEO Specialist", "Marketing"),
    ("Sales Tech Corp", "Account Executive", "Sales"),
    ("FinTech Solutions Ltd", "Blockchain Developer", "Finance"),
    ("Legal Tech Co", "Compliance Officer", "Legal"),
    ("People First Inc", "Talent Acquisition", "Human Resources"),
    ("Supply Chain LLC", "Logistics Manager", "Operations"),
    ("Test Automation Corp", "SDET", "Quality"),
    ("CyberSec Ltd", "Penetration Tester", "Security"),
    ("App Innovations Co", "iOS Developer", "Engineering"),
    ("Research Labs Inc", "Research Scientist", "R&D"),
    (
        "Platform Solutions LLC",
        "Platform Engineer",
        "Infrastructure",
    ),
    ("Growth Hacking Corp", "Growth Manager", "Marketing"),
    (
        "Customer Success Ltd",
        "Customer Success Manager",
        "Support",
    ),
    ("Tech Consulting Co", "Technical Consultant", "Consulting"),
];

// Product catalog test data
pub const PRODUCT_CATALOG: &[(&str, &str, f64, &str)] = &[
    ("Premium Widget", "WDG-001", 29.99, "Electronics"),
    ("Deluxe Gadget", "GDG-002", 49.99, "Electronics"),
    ("Pro Device", "DEV-003", 99.99, "Hardware"),
    ("Ultra Tool", "TUL-004", 19.99, "Tools"),
    ("Super System", "SYS-005", 299.99, "Software"),
    ("Advanced Platform", "PLT-006", 499.99, "Software"),
    ("Professional Solution", "SOL-007", 999.99, "Enterprise"),
    ("Basic Widget", "WDG-008", 9.99, "Electronics"),
    ("Standard Gadget", "GDG-009", 24.99, "Electronics"),
    ("Essential Device", "DEV-010", 39.99, "Hardware"),
    ("Smart Sensor", "SNS-011", 79.99, "IoT"),
    ("Power Bank", "PWR-012", 34.99, "Accessories"),
    ("Wireless Charger", "CHG-013", 44.99, "Accessories"),
    ("USB Hub", "HUB-014", 24.99, "Accessories"),
    ("Memory Card", "MEM-015", 19.99, "Storage"),
    ("External Drive", "DRV-016", 89.99, "Storage"),
    ("Network Switch", "NET-017", 149.99, "Networking"),
    ("Router Pro", "RTR-018", 199.99, "Networking"),
    ("Security Camera", "CAM-019", 129.99, "Security"),
    ("Smart Lock", "LCK-020", 249.99, "Security"),
    ("Development Kit", "DEV-021", 399.99, "Development"),
    ("API Gateway", "API-022", 599.99, "Software"),
    ("Database Tool", "DBT-023", 799.99, "Software"),
    ("Analytics Suite", "ANL-024", 1299.99, "Enterprise"),
    ("Monitoring System", "MON-025", 899.99, "Enterprise"),
    ("Backup Solution", "BKP-026", 499.99, "Software"),
    ("Cloud Service", "CLD-027", 299.99, "Services"),
    ("Support Package", "SUP-028", 199.99, "Services"),
    ("Training Course", "TRN-029", 399.99, "Education"),
    ("Certification Exam", "CRT-030", 299.99, "Education"),
];
