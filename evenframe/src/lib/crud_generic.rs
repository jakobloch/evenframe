/// Generic CRUD macros that can work with any state and error type
/// These macros require the state type to implement specific traits
/// to provide the necessary functionality

#[macro_export]
macro_rules! create_generic {
    ($type:ty) => {
        |state, payload| async move {
            // This is a simplified generic create handler
            // The actual implementation would depend on your specific requirements
            // Users can implement their own handlers based on their state and error types
            todo!("Implement create handler for your specific state and error types")
        }
    };
}

#[macro_export]
macro_rules! update_generic {
    ($type:ty) => {
        |state, payload| async move {
            todo!("Implement update handler for your specific state and error types")
        }
    };
}

#[macro_export]
macro_rules! delete_generic {
    ($type:ty) => {
        |state, payload| async move {
            todo!("Implement delete handler for your specific state and error types")
        }
    };
}

#[macro_export]
macro_rules! read_generic {
    ($type:ty) => {
        |state, payload| async move {
            todo!("Implement read handler for your specific state and error types")
        }
    };
}

#[macro_export]
macro_rules! read_all_generic {
    ($type:ty) => {
        |state| async move {
            todo!("Implement read_all handler for your specific state and error types")
        }
    };
}