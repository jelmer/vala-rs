//! Curated accessors used when indexing the code tree: source positions,
//! fully-qualified names, and the symbol a use site resolves to.

use vala_sys as ffi;

use crate::object::{opt_string, take_string, RawWrapper};
use crate::{
    CodeNode, DataType, Expression, MemberAccess, Method, Property, SourceReference, Symbol,
    Variable,
};

impl CodeNode {
    /// The source location this node was parsed from, if any. Synthetic nodes
    /// (inserted by the compiler) may have none.
    pub fn source_reference(&self) -> Option<SourceReference> {
        unsafe {
            SourceReference::from_raw_none(ffi::vala_code_node_get_source_reference(self.as_raw()))
        }
    }
}

impl Symbol {
    /// The fully-qualified name, e.g. `Foo.Bar.method`. Suitable as the basis
    /// for a stable SCIP symbol identifier. `None` for the anonymous root.
    pub fn full_name(&self) -> Option<String> {
        unsafe { take_string(ffi::vala_symbol_get_full_name(self.as_raw())) }
    }
}

impl Symbol {
    /// The symbol that lexically encloses this one (the containing class,
    /// namespace, ...), or `None` for the root namespace.
    pub fn parent_symbol(&self) -> Option<Symbol> {
        unsafe { Symbol::from_raw_none(ffi::vala_symbol_get_parent_symbol(self.as_raw())) }
    }
}

impl Expression {
    /// The symbol this expression resolves to after semantic analysis, if any.
    /// This is the link from a *use* site back to its *definition*.
    pub fn symbol_reference(&self) -> Option<Symbol> {
        unsafe { Symbol::from_raw_none(ffi::vala_expression_get_symbol_reference(self.as_raw())) }
    }
}

impl DataType {
    /// A qualified, human-readable rendering of the type, e.g. `Gtk.Widget?`.
    /// Suitable for hover text.
    pub fn to_qualified_string(&self) -> Option<String> {
        unsafe {
            take_string(ffi::vala_data_type_to_qualified_string(
                self.as_raw(),
                std::ptr::null_mut(),
            ))
        }
    }
}

impl Method {
    /// The declared return type. `Method` implements libvala's `Callable`
    /// interface, where the accessor lives; the pointer cast is sound because
    /// libvala fundamental types share one address across the hierarchy.
    pub fn return_type(&self) -> Option<DataType> {
        unsafe {
            DataType::from_raw_none(ffi::vala_callable_get_return_type(
                self.as_raw() as *mut ffi::ValaCallable
            ))
        }
    }
}

impl Variable {
    /// The declared type of the variable (covers fields, locals and parameters,
    /// which all derive from `Variable` in libvala).
    pub fn variable_type(&self) -> Option<DataType> {
        unsafe { DataType::from_raw_none(ffi::vala_variable_get_variable_type(self.as_raw())) }
    }
}

impl Property {
    /// The declared type of the property.
    pub fn property_type(&self) -> Option<DataType> {
        unsafe { DataType::from_raw_none(ffi::vala_property_get_property_type(self.as_raw())) }
    }
}

impl MemberAccess {
    /// The accessed member's name as written at the use site.
    pub fn member_name(&self) -> Option<String> {
        unsafe { opt_string(ffi::vala_member_access_get_member_name(self.as_raw())) }
    }

    /// The inner expression (the receiver), e.g. `foo` in `foo.bar`.
    pub fn inner(&self) -> Option<Expression> {
        unsafe { Expression::from_raw_none(ffi::vala_member_access_get_inner(self.as_raw())) }
    }
}
