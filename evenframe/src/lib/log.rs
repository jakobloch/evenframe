/// Internal macro for common logging logic
#[macro_export]
#[doc(hidden)]
macro_rules! __internal_log_impl {
    // Helper variant - uses ABSOLUTE_TO_AVEL env var with fallback
    ($content:expr, $log_subdir:expr, helper) => {{
        let filename = format!("{}.log", chrono::Local::now().format("%Y_%m_%d_%H_%M_%S"));

        let logs_dir = if let Ok(project_root) = std::env::var("ABSOLUTE_TO_AVEL") {
            let root = if project_root.starts_with('/') {
                project_root
            } else {
                format!("/{}", project_root)
            };
            format!("{}/{}", root, $log_subdir)
        } else {
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                let path = std::path::Path::new(&manifest_dir);
                if let Some(parent) = path.parent().and_then(|p| p.parent()) {
                    format!("{}/{}", parent.display(), $log_subdir)
                } else {
                    panic!("Could not find directory")
                }
            } else {
                panic!("Could not find directory")
            }
        };

        $crate::__internal_log_impl!($content, logs_dir, filename, false, impl);
    }};

    ($content:expr, $log_subdir:expr, $filename:expr, helper) => {{
        let logs_dir = if let Ok(project_root) = std::env::var("ABSOLUTE_TO_AVEL") {
            let root = if project_root.starts_with('/') {
                project_root
            } else {
                format!("/{}", project_root)
            };
            format!("{}/{}", root, $log_subdir)
        } else {
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                let path = std::path::Path::new(&manifest_dir);
                if let Some(parent) = path.parent().and_then(|p| p.parent()) {
                    format!("{}/{}", parent.display(), $log_subdir)
                } else {
                    panic!("Could not find directory")
                }
            } else {
                panic!("Could not find directory")
            }
        };

        $crate::__internal_log_impl!($content, logs_dir, $filename, false, impl);
    }};

    ($content:expr, $log_subdir:expr, $filename:expr, $append:expr, helper) => {{
        let logs_dir = if let Ok(project_root) = std::env::var("ABSOLUTE_TO_AVEL") {
            let root = if project_root.starts_with('/') {
                project_root
            } else {
                format!("/{}", project_root)
            };
            format!("{}/{}", root, $log_subdir)
        } else {
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                let path = std::path::Path::new(&manifest_dir);
                if let Some(parent) = path.parent().and_then(|p| p.parent()) {
                    format!("{}/{}", parent.display(), $log_subdir)
                } else {
                    panic!("Could not find directory")
                }
            } else {
                panic!("Could not find directory")
            }
        };

        $crate::__internal_log_impl!($content, logs_dir, $filename, $append, impl);
    }};

    // Standard variant - uses ABSOLUTE_PATH env var
    ($content:expr, $log_subdir:expr, standard) => {{
        let filename = format!("{}.log", chrono::Local::now().format("%Y_%m_%d_%H_%M_%S"));
        let logs_dir = format!(
            "{}/{}",
            std::env::var("ABSOLUTE_PATH").expect("ABSOLUTE_PATH not set"),
            $log_subdir
        );

        $crate::__internal_log_impl!($content, logs_dir, filename, false, impl);
    }};

    ($content:expr, $log_subdir:expr, $filename:expr, standard) => {{
        let logs_dir = format!(
            "{}/{}",
            std::env::var("ABSOLUTE_PATH").expect("ABSOLUTE_PATH not set"),
            $log_subdir
        );

        $crate::__internal_log_impl!($content, logs_dir, $filename, false, impl);
    }};

    ($content:expr, $log_subdir:expr, $filename:expr, $append:expr, standard) => {{
        let logs_dir = format!(
            "{}/{}",
            std::env::var("ABSOLUTE_PATH").expect("ABSOLUTE_PATH not set"),
            $log_subdir
        );

        $crate::__internal_log_impl!($content, logs_dir, $filename, $append, impl);
    }};

    // Core implementation
    ($content:expr, $logs_dir:expr, $filename:expr, $append:expr, impl) => {{
        use std::io::Write;

        // Create logs directory if it doesn't exist
        let _ = std::fs::create_dir_all(&$logs_dir);

        let path_str = &format!("{}/{}", $logs_dir, $filename);
        let path = std::path::Path::new(path_str);

        let mut options = std::fs::OpenOptions::new();
        options.create(true);
        if $append {
            options.append(true);
        } else {
            options.write(true).truncate(true);
        }

        if let Ok(mut file_handle) = options.open(path) {
            // Check if the expression is a format! macro or string literal
            let expr_str = stringify!($content);
            let formatted = if expr_str.starts_with("format!")
                || expr_str.starts_with("&format!")
                || expr_str.starts_with("\"")
                || expr_str.starts_with("String::")
            {
                // For formatted strings, just output the content directly
                format!("{}\n", $content)
            } else if $filename.ends_with(".surql") {
                // For .surql files, output the content as a plain string without debug formatting
                format!("{}\n", $content)
            } else {
                // For other types, use debug output with location info
                let value_str = format!("{:#?}", &$content);

                // Check if it's a multi-line value
                if value_str.contains('\n') || value_str.len() > 80 {
                    format!(
                        "[{}:{}] {} = \n{}\n",
                        file!(),
                        line!(),
                        stringify!($content),
                        value_str
                    )
                } else {
                    format!(
                        "[{}:{}] {} = {}\n",
                        file!(),
                        line!(),
                        stringify!($content),
                        value_str
                    )
                }
            };
            let _ = file_handle.write_all(formatted.as_bytes());
        }
    }};
}

/// Logging macro for the evenframe_derive crate.
///
/// # Examples
///
/// Log to a timestamp-based file (e.g., "2024_01_12_14_30_52.log"):
/// ```no_run
/// # use helpers::evenframe_derive_log;
/// evenframe_derive_log!("Derive macro invoked");
/// ```
///
/// Log to a specific file (overwrites existing content):
/// ```no_run
/// # use helpers::evenframe_derive_log;
/// evenframe_derive_log!("Generated code", "codegen.log");
/// ```
///
/// Log to a specific file with append mode:
/// ```no_run
/// # use helpers::evenframe_derive_log;
/// evenframe_derive_log!("New derive", "codegen.log", true);
/// ```
#[macro_export]
macro_rules! evenframe_derive_log {
    ($content:expr) => {{
        $crate::__internal_log_impl!($content, "backend/evenframe_derive/logs", standard);
    }};
    ($content:expr, $filename:expr) => {{
        $crate::__internal_log_impl!(
            $content,
            "backend/evenframe_derive/logs",
            $filename,
            standard
        );
    }};
    ($content:expr, $filename:expr, $append:expr) => {{
        $crate::__internal_log_impl!(
            $content,
            "backend/evenframe_derive/logs",
            $filename,
            $append,
            standard
        );
    }};
}

/// Logging macro for the evenframe crate.
///
/// # Examples
///
/// Log to a timestamp-based file (e.g., "2024_01_12_14_30_52.log"):
/// ```no_run
/// # use helpers::evenframe_log;
/// evenframe_log!("Sync started");
/// ```
///
/// Log to a specific file (overwrites existing content):
/// ```no_run
/// # use helpers::evenframe_log;
/// evenframe_log!("Types generated", "output.log");
/// ```
///
/// Log to a specific file with append mode:
/// ```no_run
/// # use helpers::evenframe_log;
/// evenframe_log!("New type added", "output.log", true);
/// ```
#[macro_export]
macro_rules! evenframe_log {
    ($content:expr) => {{
        $crate::__internal_log_impl!($content, "backend/evenframe/logs", standard);
    }};
    ($content:expr, $filename:expr) => {{
        $crate::__internal_log_impl!($content, "backend/evenframe/logs", $filename, standard);
    }};
    ($content:expr, $filename:expr, $append:expr) => {{
        $crate::__internal_log_impl!(
            $content,
            "backend/evenframe/logs",
            $filename,
            $append,
            standard
        );
    }};
}
