//! Walk a libvala code tree from Rust by implementing [`Visitor`].
//!
//! libvala traverses trees with the `CodeVisitor` C class: you subclass it,
//! override the `visit_*` methods, and call `accept`/`accept_children` to drive
//! recursion. There is no way to subclass a GLib type purely in Rust, so this
//! module registers a static `CodeVisitor` subtype once and routes its vtable
//! slots through trampolines that dispatch to a boxed [`Visitor`] trait object
//! stored alongside the instance.
//!
//! The default trait methods recurse into children, so an implementor only
//! overrides the node kinds it cares about. To recurse explicitly (e.g. to
//! visit a node's body after recording it), call [`Walker::walk_children`].

use std::ffi::CString;
use std::os::raw::c_void;
use std::sync::Once;

use vala_sys as ffi;

use crate::object::RawWrapper;
use crate::{
    Class, CodeNode, Constant, Delegate, Enum, Field, Interface, MemberAccess, Method, MethodCall,
    Namespace, Property, Signal, Struct,
};

/// Handle to the running traversal, used to recurse into a node's children.
pub struct Walker {
    visitor: *mut ffi::ValaCodeVisitor,
}

impl Walker {
    /// Recurse into the children of `node`, dispatching them back through this
    /// visitor.
    pub fn walk_children<N>(&self, node: &N)
    where
        N: AsRef<CodeNode>,
    {
        unsafe {
            ffi::vala_code_node_accept_children(node.as_ref().as_raw(), self.visitor);
        }
    }
}

/// Callbacks invoked while walking a code tree. Every method has a default that
/// simply recurses into the node's children, so implementors override only the
/// kinds they need. When overriding, call [`Walker::walk_children`] to continue
/// descending; omit it to prune the subtree.
#[allow(unused_variables)]
pub trait Visitor {
    /// A namespace declaration.
    fn visit_namespace(&mut self, w: &Walker, node: &Namespace) {
        w.walk_children(node);
    }
    /// A class declaration.
    fn visit_class(&mut self, w: &Walker, node: &Class) {
        w.walk_children(node);
    }
    /// A struct declaration.
    fn visit_struct(&mut self, w: &Walker, node: &Struct) {
        w.walk_children(node);
    }
    /// An interface declaration.
    fn visit_interface(&mut self, w: &Walker, node: &Interface) {
        w.walk_children(node);
    }
    /// An enum declaration.
    fn visit_enum(&mut self, w: &Walker, node: &Enum) {
        w.walk_children(node);
    }
    /// A method declaration.
    fn visit_method(&mut self, w: &Walker, node: &Method) {
        w.walk_children(node);
    }
    /// A field declaration.
    fn visit_field(&mut self, w: &Walker, node: &Field) {
        w.walk_children(node);
    }
    /// A property declaration.
    fn visit_property(&mut self, w: &Walker, node: &Property) {
        w.walk_children(node);
    }
    /// A signal declaration.
    fn visit_signal(&mut self, w: &Walker, node: &Signal) {
        w.walk_children(node);
    }
    /// A constant declaration.
    fn visit_constant(&mut self, w: &Walker, node: &Constant) {
        w.walk_children(node);
    }
    /// A delegate declaration.
    fn visit_delegate(&mut self, w: &Walker, node: &Delegate) {
        w.walk_children(node);
    }
    /// A member access expression (`foo.bar`) -- a use site.
    fn visit_member_access(&mut self, w: &Walker, node: &MemberAccess) {
        w.walk_children(node);
    }
    /// A method call expression -- a use site.
    fn visit_method_call(&mut self, w: &Walker, node: &MethodCall) {
        w.walk_children(node);
    }
}

/// Walk `node` and everything beneath it with `visitor`.
pub fn walk<V, N>(visitor: &mut V, node: &N)
where
    V: Visitor,
    N: AsRef<CodeNode>,
{
    unsafe {
        let inst = new_instance(visitor as &mut dyn Visitor);
        ffi::vala_code_node_accept(node.as_ref().as_raw(), inst);
        free_instance(inst);
    }
}

/// Walk a single source file with `visitor`. Prefer this over [`walk`] from the
/// root namespace when indexing: it visits only the declarations in `file`,
/// skipping the standard-library packages loaded into the context.
pub fn walk_source_file<V: Visitor>(visitor: &mut V, file: &crate::SourceFile) {
    unsafe {
        let inst = new_instance(visitor as &mut dyn Visitor);
        ffi::vala_source_file_accept(file.as_raw(), inst);
        free_instance(inst);
    }
}

// Layout: the registered instance is `[ ValaCodeVisitor | HandlerPtr ]`. We
// store a fat trait-object pointer at the tail and read it back in each
// trampoline. The pointer is only valid for the duration of a single `walk`
// call, which holds the exclusive borrow; the lifetime is erased here and
// re-established by `dispatch`.
type HandlerPtr = *mut dyn Visitor;

static REGISTER: Once = Once::new();
static mut VISITOR_TYPE: ffi::GType = 0;
static mut PARENT_INSTANCE_SIZE: u32 = 0;

unsafe fn registered_type() -> ffi::GType {
    REGISTER.call_once(|| {
        let parent = ffi::vala_code_visitor_get_type();
        let mut query = std::mem::zeroed::<gobject_sys::GTypeQuery>();
        gobject_sys::g_type_query(parent, &mut query);
        PARENT_INSTANCE_SIZE = query.instance_size;

        let name = CString::new("ValaRsVisitor").unwrap();
        VISITOR_TYPE = gobject_sys::g_type_register_static_simple(
            parent,
            name.as_ptr(),
            query.class_size,
            Some(class_init),
            query.instance_size + std::mem::size_of::<HandlerPtr>() as u32,
            None,
            0,
        );
    });
    VISITOR_TYPE
}

/// Pointer to the handler slot stored at the tail of the instance.
unsafe fn handler_slot(inst: *mut ffi::ValaCodeVisitor) -> *mut HandlerPtr {
    let base = inst as *mut u8;
    base.add(PARENT_INSTANCE_SIZE as usize) as *mut HandlerPtr
}

unsafe fn new_instance(handler: &mut (dyn Visitor + '_)) -> *mut ffi::ValaCodeVisitor {
    let ty = registered_type();
    // Chain up through libvala's constructor so the fundamental type's ref_count
    // and private data are initialised (g_type_create_instance alone is not
    // enough for a Vala fundamental class).
    let obj = ffi::vala_code_visitor_construct(ty);
    // Erase the handler borrow's lifetime into the stored pointer. This is sound
    // because the instance is created and destroyed entirely within `walk`,
    // which holds the borrow for the whole traversal.
    let erased: HandlerPtr = std::mem::transmute::<*mut dyn Visitor, HandlerPtr>(handler);
    *handler_slot(obj) = erased;
    obj
}

unsafe fn free_instance(inst: *mut ffi::ValaCodeVisitor) {
    ffi::vala_code_visitor_unref(inst as ffi::gpointer);
}

unsafe fn dispatch<'a>(self_: *mut ffi::ValaCodeVisitor) -> (&'a mut dyn Visitor, Walker) {
    let handler = &mut **handler_slot(self_);
    (handler, Walker { visitor: self_ })
}

/// Define a trampoline that wraps the C node pointer and calls a trait method.
macro_rules! trampoline {
    ($fn_name:ident, $ffi_node:ty, $wrapper:ty, $method:ident) => {
        unsafe extern "C" fn $fn_name(self_: *mut ffi::ValaCodeVisitor, node: *mut $ffi_node) {
            let (handler, walker) = dispatch(self_);
            if let Some(n) = <$wrapper>::from_raw_none(node) {
                handler.$method(&walker, &n);
            }
        }
    };
}

trampoline!(tr_namespace, ffi::ValaNamespace, Namespace, visit_namespace);
trampoline!(tr_class, ffi::ValaClass, Class, visit_class);
trampoline!(tr_struct, ffi::ValaStruct, Struct, visit_struct);
trampoline!(tr_interface, ffi::ValaInterface, Interface, visit_interface);
trampoline!(tr_enum, ffi::ValaEnum, Enum, visit_enum);
trampoline!(tr_method, ffi::ValaMethod, Method, visit_method);
trampoline!(tr_field, ffi::ValaField, Field, visit_field);
trampoline!(tr_property, ffi::ValaProperty, Property, visit_property);
trampoline!(tr_signal, ffi::ValaSignal, Signal, visit_signal);
trampoline!(tr_constant, ffi::ValaConstant, Constant, visit_constant);
trampoline!(tr_delegate, ffi::ValaDelegate, Delegate, visit_delegate);
trampoline!(
    tr_member_access,
    ffi::ValaMemberAccess,
    MemberAccess,
    visit_member_access
);
trampoline!(
    tr_method_call,
    ffi::ValaMethodCall,
    MethodCall,
    visit_method_call
);

/// Generic recurse trampoline: descend into a node's children. libvala's base
/// `CodeVisitor.visit_*` methods are empty and do not recurse, so to reach
/// nested nodes (statements, expressions) we install this on every slot and let
/// it carry traversal through node kinds the [`Visitor`] trait does not expose.
unsafe extern "C" fn recurse(self_: *mut ffi::ValaCodeVisitor, node: *mut ffi::ValaCodeNode) {
    ffi::vala_code_node_accept_children(node, self_);
}

/// `SourceFile` is not a `CodeNode`, so its recursion uses a dedicated entry.
unsafe extern "C" fn recurse_source_file(
    self_: *mut ffi::ValaCodeVisitor,
    file: *mut ffi::ValaSourceFile,
) {
    ffi::vala_source_file_accept_children(file, self_);
}

/// The concrete signature of a vtable slot (they differ only in the node
/// pointer type, which is ABI-identical to `*mut ValaCodeNode`).
type SlotFn = unsafe extern "C" fn(*mut ffi::ValaCodeVisitor, *mut ffi::ValaCodeNode);

unsafe extern "C" fn class_init(klass: *mut c_void, _data: *mut c_void) {
    let vtable = klass as *mut ffi::ValaCodeVisitorClass;

    // First, make every slot recurse so traversal reaches deeply nested nodes.
    // Each slot is a `fn(self, *mut ValaXxx)`; all node pointers are ABI-equal
    // to `*mut ValaCodeNode`, so the single `recurse` fn fits every slot.
    let r: SlotFn = recurse;
    fill_all_slots(vtable, r);
    // SourceFile is not a CodeNode and needs its own recursion entry.
    (*vtable).visit_source_file = Some(recurse_source_file);

    // Then override the slots the Visitor trait exposes.
    (*vtable).visit_namespace = Some(tr_namespace);
    (*vtable).visit_class = Some(tr_class);
    (*vtable).visit_struct = Some(tr_struct);
    (*vtable).visit_interface = Some(tr_interface);
    (*vtable).visit_enum = Some(tr_enum);
    (*vtable).visit_method = Some(tr_method);
    (*vtable).visit_field = Some(tr_field);
    (*vtable).visit_property = Some(tr_property);
    (*vtable).visit_signal = Some(tr_signal);
    (*vtable).visit_constant = Some(tr_constant);
    (*vtable).visit_delegate = Some(tr_delegate);
    (*vtable).visit_member_access = Some(tr_member_access);
    (*vtable).visit_method_call = Some(tr_method_call);
}

/// Assign the recurse trampoline to every `visit_*` slot, transmuting it to each
/// slot's concrete function-pointer type (ABI-identical: pointer args only).
// Each slot's fn type differs only in the node pointer argument, so a single
// named target type cannot cover them; the transmute target is intentionally
// inferred per slot.
#[allow(clippy::missing_transmute_annotations)]
unsafe fn fill_all_slots(vtable: *mut ffi::ValaCodeVisitorClass, r: SlotFn) {
    macro_rules! set {
        ($($slot:ident),* $(,)?) => {
            $( (*vtable).$slot = Some(std::mem::transmute::<SlotFn, _>(r)); )*
        };
    }
    set!(
        visit_namespace,
        visit_class,
        visit_struct,
        visit_interface,
        visit_enum,
        visit_enum_value,
        visit_error_domain,
        visit_error_code,
        visit_delegate,
        visit_constant,
        visit_field,
        visit_method,
        visit_creation_method,
        visit_formal_parameter,
        visit_property,
        visit_property_accessor,
        visit_signal,
        visit_constructor,
        visit_destructor,
        visit_type_parameter,
        visit_using_directive,
        visit_data_type,
        visit_block,
        visit_empty_statement,
        visit_declaration_statement,
        visit_local_variable,
        visit_initializer_list,
        visit_expression_statement,
        visit_if_statement,
        visit_switch_statement,
        visit_switch_section,
        visit_switch_label,
        visit_loop_statement,
        visit_while_statement,
        visit_do_statement,
        visit_for_statement,
        visit_foreach_statement,
        visit_break_statement,
        visit_continue_statement,
        visit_return_statement,
        visit_yield_statement,
        visit_throw_statement,
        visit_try_statement,
        visit_catch_clause,
        visit_lock_statement,
        visit_unlock_statement,
        visit_delete_statement,
        visit_array_creation_expression,
        visit_boolean_literal,
        visit_character_literal,
        visit_integer_literal,
        visit_real_literal,
        visit_regex_literal,
        visit_string_literal,
        visit_template,
        visit_tuple,
        visit_null_literal,
        visit_member_access,
        visit_method_call,
        visit_element_access,
        visit_base_access,
        visit_postfix_expression,
        visit_object_creation_expression,
        visit_sizeof_expression,
        visit_typeof_expression,
        visit_unary_expression,
        visit_cast_expression,
        visit_named_argument,
        visit_pointer_indirection,
        visit_addressof_expression,
        visit_reference_transfer_expression,
        visit_binary_expression,
        visit_type_check,
        visit_conditional_expression,
        visit_lambda_expression,
        visit_assignment,
        visit_with_statement,
        visit_slice_expression,
    );
}
