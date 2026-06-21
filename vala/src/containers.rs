//! Container getters that return [`List`] views over child declarations, for
//! walking the code tree without subclassing a visitor.

use vala_sys as ffi;

use crate::collections::List;
use crate::object::RawWrapper;
use crate::{
    Class, Constant, Delegate, Enum, Field, Interface, Method, Namespace, ObjectTypeSymbol,
    Property, Signal, Struct,
};

/// Generate a `List<T>`-returning getter from a `vala_*` function that yields a
/// `*mut ValaList`.
macro_rules! list_getter {
    ($(#[$m:meta])* $owner:ty, $name:ident, $elem:ty, $ffi:path) => {
        impl $owner {
            $(#[$m])*
            pub fn $name(&self) -> List<$elem> {
                // libvala container getters return `unowned` (borrowed) lists.
                unsafe {
                    List::from_raw_none($ffi(self.as_raw() as *mut _))
                        .expect(concat!(stringify!($ffi), " returned null"))
                }
            }
        }
    };
}

list_getter!(
    /// Child namespaces.
    Namespace, namespaces, Namespace, ffi::vala_namespace_get_namespaces
);
list_getter!(
    /// Classes declared directly in this namespace.
    Namespace, classes, Class, ffi::vala_namespace_get_classes
);
list_getter!(
    /// Structs declared directly in this namespace.
    Namespace, structs, Struct, ffi::vala_namespace_get_structs
);
list_getter!(
    /// Interfaces declared directly in this namespace.
    Namespace, interfaces, Interface, ffi::vala_namespace_get_interfaces
);
list_getter!(
    /// Enums declared directly in this namespace.
    Namespace, enums, Enum, ffi::vala_namespace_get_enums
);
list_getter!(
    /// Methods declared directly in this namespace.
    Namespace, methods, Method, ffi::vala_namespace_get_methods
);
list_getter!(
    /// Fields declared directly in this namespace.
    Namespace, fields, Field, ffi::vala_namespace_get_fields
);
list_getter!(
    /// Constants declared directly in this namespace.
    Namespace, constants, Constant, ffi::vala_namespace_get_constants
);
list_getter!(
    /// Delegates declared directly in this namespace.
    Namespace, delegates, Delegate, ffi::vala_namespace_get_delegates
);

list_getter!(
    /// Methods declared on this class or interface.
    ObjectTypeSymbol, methods, Method, ffi::vala_object_type_symbol_get_methods
);
list_getter!(
    /// Fields declared on this class or interface.
    ObjectTypeSymbol, fields, Field, ffi::vala_object_type_symbol_get_fields
);
list_getter!(
    /// Properties declared on this class or interface.
    ObjectTypeSymbol, properties, Property, ffi::vala_object_type_symbol_get_properties
);
list_getter!(
    /// Signals declared on this class or interface.
    ObjectTypeSymbol, signals, Signal, ffi::vala_object_type_symbol_get_signals
);
list_getter!(
    /// Constants declared on this class or interface.
    ObjectTypeSymbol, constants, Constant, ffi::vala_object_type_symbol_get_constants
);
list_getter!(
    /// Nested classes declared on this class or interface.
    ObjectTypeSymbol, classes, Class, ffi::vala_object_type_symbol_get_classes
);
list_getter!(
    /// Nested enums declared on this class or interface.
    ObjectTypeSymbol, enums, Enum, ffi::vala_object_type_symbol_get_enums
);
