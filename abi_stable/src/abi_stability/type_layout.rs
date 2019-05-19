/*!
Types for modeling the layout of a datatype
*/

use std::{
    cell::RefCell,
    collections::HashSet,
    fmt::{self, Debug, Display, Formatter},
    mem,
};


use crate::{
    const_utils::empty_slice, version::VersionStrings, 
    std_types::{RNone, ROption, RSome, RStr, StaticSlice,StaticStr},
    ignored_wrapper::CmpIgnored,
    prefix_type::{FieldAccessibility,IsConditional},
    reflection::ModReflMode,
};

use super::{
    AbiInfo, 
    GetAbiInfo,
    tagging::Tag,
};


/// The parameters for `TypeLayout::from_params`.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TypeLayoutParams {
    pub name: &'static str,
    pub package: &'static str,
    pub package_version: VersionStrings,
    pub file:&'static str,
    pub line:u32,
    pub data: TLData,
    pub generics: GenericParams,
}


/// The layout of a type,
/// also includes metadata about where the type was defined.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, StableAbi)]
// #[sabi(debug_print)]
pub struct TypeLayout {
    pub name: StaticStr,
    pub package: StaticStr,
    pub package_version: VersionStrings,
    pub file:CmpIgnored<StaticStr>, // This is for the Debug string
    pub line:CmpIgnored<u32>, // This is for the Debug string
    pub size: usize,
    pub alignment: usize,
    pub data: TLData,
    pub full_type: FullType,
    pub phantom_fields: StaticSlice<TLField>,
    /// Extra data stored for reflection,
    /// so as to not break the abi every time that more stuff is added for reflection.
    pub reflection_tag:Tag,
    pub tag:Tag,
    pub mod_refl_mode:ModReflMode,
    pub repr_attr:ReprAttr,
}


/// Which lifetime is being referenced by a field.
/// Allows lifetimes to be renamed,so long as the "same" lifetime is being referenced.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
pub enum LifetimeIndex {
    Static,
    Param(usize),
}



/// Represents all the generic parameters of a type.
/// 
/// This is different for every different generic parameter,
/// if any one of them changes it won't compare equal,
/// `<Vec<u32>>::ABI_INFO.get().layout.full_type.generics`
/// ẁon't compare equal to
/// `<Vec<()>>::ABI_INFO.get().layout.full_type.generics`
/// 
///
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, StableAbi)]
pub struct GenericParams {
    pub lifetime: StaticSlice<StaticStr>,
    pub type_: StaticSlice<&'static TypeLayout>,
    pub const_: StaticSlice<StaticStr>,
}

/// The typename and generics of the type this layout is associated to,
/// used for printing types.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, StableAbi)]
pub struct FullType {
    pub name: StaticStr,
    pub primitive: ROption<RustPrimitive>,
    pub generics: GenericParams,
}

/// What kind of type this is.struct/enum/etc.
///
/// Unions are currently treated as structs.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, StableAbi)]
pub enum TLData {
    /// All the bytes for the type are valid (not necessarily all bit patterns).
    ///
    /// If you use this variant,
    /// you must ensure the continuing validity of the same bit-patterns.
    Primitive,
    /// For structs and unions.
    Struct { fields: StaticSlice<TLField> },
    /// For enums.
    Enum {
        variants: StaticSlice<TLEnumVariant>,
    },
    /// vtables and modules that can be extended in minor versions.
    PrefixType(TLPrefixType),
}


/// vtables and modules that can be extended in minor versions.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, StableAbi)]
pub struct TLPrefixType {
    /// The first field in the suffix
    pub first_suffix_field:usize,
    pub accessible_fields:FieldAccessibility,
    pub conditional_prefix_fields:StaticSlice<IsConditional>,
    pub fields: StaticSlice<TLField>,
}



/// A discriminant-only version of TLData.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
pub enum TLDataDiscriminant {
    Primitive,
    Struct,
    Enum,
    PrefixType,
}

/// The layout of an enum variant.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, StableAbi)]
pub struct TLEnumVariant {
    pub name: StaticStr,
    pub discriminant:TLDiscriminant,
    pub fields: StaticSlice<TLField>,
}


/// The discriminant of an enum variant.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, StableAbi)]
pub enum TLDiscriminant{
    No,
    Isize(isize),
    Usize(usize),
    Signed(i64),
    Unsigned(u64),
}

impl TLDiscriminant{
    pub const fn from_u8(n:u8)->Self{
        TLDiscriminant::Unsigned(n as u64)
    }
    pub const fn from_u16(n:u16)->Self{
        TLDiscriminant::Unsigned(n as u64)
    }
    pub const fn from_u32(n:u32)->Self{
        TLDiscriminant::Unsigned(n as u64)
    }
    pub const fn from_u64(n:u64)->Self{
        TLDiscriminant::Unsigned(n)
    }

    pub const fn from_i8(n:i8)->Self{
        TLDiscriminant::Signed(n as i64)
    }
    pub const fn from_i16(n:i16)->Self{
        TLDiscriminant::Signed(n as i64)
    }
    pub const fn from_i32(n:i32)->Self{
        TLDiscriminant::Signed(n as i64)
    }
    pub const fn from_i64(n:i64)->Self{
        TLDiscriminant::Signed(n)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
pub enum ReprAttr{
    C(ROption<DiscriminantRepr>),
    Transparent,
    /// Means that only `repr(IntegerType)` was used.
    Int(DiscriminantRepr),
}

/// How the discriminant of an enum is represented.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
pub enum DiscriminantRepr {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    /// Reserved,just in case that u128 gets a c-compatible layout
    U128,
    /// Reserved,just in case that i128 gets a c-compatible layout
    I128,
    Usize,
    /// This is the default discriminant type for `repr(C)`.
    Isize,
}


/// The layout of a field.
#[repr(C)]
#[derive(Copy, Clone, StableAbi)]
pub struct TLField {
    /// The field's name.
    pub name: StaticStr,
    /// Which lifetimes in the struct are referenced in the field type.
    pub lifetime_indices: StaticSlice<LifetimeIndex>,
    /// The layout of the field's type.
    ///
    /// This is a function pointer to avoid infinite recursion,
    /// if you have a `&'static AbiInfo`s with the same address as one of its parent type,
    /// you've encountered a cycle.
    pub abi_info: GetAbiInfo,
    /// All the function pointer types in the field.
    pub functions:StaticSlice<TLFunction>,

    /// Whether this field is only a function pointer.
    pub is_function:bool,

    pub field_accessor:FieldAccessor,
}

/// Used to print a field as its field and its type.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, StableAbi)]
pub struct TLFieldAndType {
    inner: TLField,
}

/// What primitive type this is.Used mostly for printing the type.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
pub enum RustPrimitive {
    Reference,
    MutReference,
    ConstPtr,
    MutPtr,
    Array,
}


#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, StableAbi)]
pub enum FieldAccessor {
    /// Accessible with `self.field_name`
    Direct,
    /// Accessible with `fn field_name(&self)->FieldType`
    Method{
        name:ROption<StaticStr>,
    },
    /// Accessible with `fn field_name(&self)->Option<FieldType>`
    MethodOption,
    /// This field is completely inaccessible.
    Opaque,
}


impl FieldAccessor{
    pub const fn method_named(name:&'static str)->Self{
        FieldAccessor::Method{
            name:RSome(StaticStr::new(name))
        }
    }
}


///////////////////////////

impl TLField {
    pub const fn new(
        name: &'static str,
        lifetime_indices: &'static [LifetimeIndex],
        abi_info: GetAbiInfo,
    ) -> Self {
        Self {
            name: StaticStr::new(name),
            lifetime_indices: StaticSlice::new(lifetime_indices),
            abi_info,
            functions:StaticSlice::new(empty_slice()),
            is_function:false,
            field_accessor:FieldAccessor::Direct,
        }
    }

    pub const fn with_functions(
        name: &'static str,
        lifetime_indices: &'static [LifetimeIndex],
        abi_info: GetAbiInfo,
        functions:&'static [TLFunction],
        is_function:bool,
    ) -> Self {
        Self {
            name: StaticStr::new(name),
            lifetime_indices: StaticSlice::new(lifetime_indices),
            abi_info,
            functions: StaticSlice::new(functions),
            is_function,
            field_accessor:FieldAccessor::Direct,
        }
    }

    pub const fn set_field_accessor(mut self,field_accessor:FieldAccessor)->Self{
        self.field_accessor=field_accessor;
        self
    }



    /// Used for calling recursive methods,
    /// so as to avoid infinite recursion in types that reference themselves(even indirectly).
    fn recursive<F, U>(self, f: F) -> U
    where
        F: FnOnce(usize,TLFieldShallow) -> U,
    {
        let mut already_recursed = false;
        let mut recursion_depth=!0;
        let mut visited_nodes=!0;

        ALREADY_RECURSED.with(|state| {
            let mut state = state.borrow_mut();
            recursion_depth=state.recursion_depth;
            visited_nodes=state.visited_nodes;
            state.recursion_depth+=1;
            state.visited_nodes+=1;
            already_recursed = state.visited.replace(self.abi_info.get()).is_some();
        });

        let _guard=if visited_nodes==0 { Some(ResetRecursion) }else{ None };

        let field=TLFieldShallow::new(self, !already_recursed );
        let res = f( recursion_depth, field);

        ALREADY_RECURSED.with(|state| {
            let mut state = state.borrow_mut();
            state.recursion_depth-=1;
        });

        res
    }
}

impl PartialEq for TLField {
    fn eq(&self, other: &Self) -> bool {
        self.recursive(|_,this| {
            let r = TLFieldShallow::new(*other, this.abi_info.is_some());
            this == r
        })
    }
}

/// Need to avoid recursion somewhere,so I decided to stop at the field level.
impl Debug for TLField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.recursive(|recursion_depth,x|{
            if recursion_depth>=2 {
                writeln!(f,"<printing recursion limit>")
            }else{
                fmt::Debug::fmt(&x, f)
            }
        })
    }
}

///////////////////////////


struct ResetRecursion;

impl Drop for ResetRecursion{
    fn drop(&mut self){
        ALREADY_RECURSED.with(|state|{
            let mut state = state.borrow_mut();
            state.recursion_depth=0;
            state.visited_nodes=0;
            state.visited.clear();
        });
    }
}


struct RecursionState{
    recursion_depth:usize,
    visited_nodes:u64,
    visited:HashSet<*const AbiInfo>,
}


thread_local! {
    static ALREADY_RECURSED: RefCell<RecursionState> = RefCell::new(RecursionState{
        recursion_depth:0,
        visited_nodes:0,
        visited: HashSet::default(),
    });
}

///////////////////////////

impl TLFieldAndType {
    pub fn new(inner: TLField) -> Self {
        Self { inner }
    }

    pub fn name(&self) -> RStr<'static> {
        self.inner.name.as_rstr()
    }

    pub fn full_type(&self) -> FullType {
        self.inner.abi_info.get().layout.full_type
    }
}

impl Debug for TLFieldAndType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TLFieldAndType")
            .field("field_name:", &self.inner.name)
            .field("type:", &self.inner.abi_info.get().layout.full_type())
            .finish()
    }
}

impl Display for TLFieldAndType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.inner.name,
            self.inner.abi_info.get().layout.full_type()
        )
    }
}

///////////////////////////

impl TypeLayout {
    pub(crate) const fn from_std_lib<T>(
        type_name: &'static str,
        data: TLData,
        generics: GenericParams,
    ) -> Self {
        Self::from_std_lib_phantom::<T>(type_name, RNone, data, generics, empty_slice())
    }

    pub(crate) const fn from_std_lib_phantom<T>(
        type_name: &'static str,
        prim: ROption<RustPrimitive>,
        data: TLData,
        genparams: GenericParams,
        phantom: &'static [TLField],
    ) -> Self {
        Self {
            name: StaticStr::new(type_name),
            package: StaticStr::new("std"),
            package_version: VersionStrings {
                major: StaticStr::new("1"),
                minor: StaticStr::new("0"),
                patch: StaticStr::new("0"),
            },
            file:CmpIgnored::new(StaticStr::new("<standard_library>")),
            line:CmpIgnored::new(0),
            size: mem::size_of::<T>(),
            alignment: mem::align_of::<T>(),
            data,
            full_type: FullType::new(type_name, prim, genparams),
            phantom_fields: StaticSlice::new(phantom),
            reflection_tag:Tag::null(),
            tag:Tag::null(),
            mod_refl_mode:ModReflMode::Module,
            repr_attr:ReprAttr::C(RNone),
        }
    }

    pub(crate) const fn full_type(&self) -> FullType {
        self.full_type
    }

    pub const fn from_params<T>(p: TypeLayoutParams) -> Self {
        let name = StaticStr::new(p.name);
        Self {
            name,
            package: StaticStr::new(p.package),
            package_version: p.package_version,
            file:CmpIgnored::new(StaticStr::new(p.file)),
            line:CmpIgnored::new(p.line),
            size: mem::size_of::<T>(),
            alignment: mem::align_of::<T>(),
            data: p.data,
            full_type: FullType {
                name,
                primitive: RNone,
                generics: p.generics,
            },
            phantom_fields: StaticSlice::new(empty_slice()),
            reflection_tag:Tag::null(),
            tag:Tag::null(),
            mod_refl_mode:ModReflMode::Module,
            repr_attr:ReprAttr::C(RNone),
        }
    }

    pub const fn set_phantom_fields(mut self,phantom_fields: &'static [TLField])->Self{
        self.phantom_fields=StaticSlice::new(phantom_fields);
        self
    }

    pub const fn set_tag(mut self,tag:Tag)->Self{
        self.tag=tag;
        self
    }

    pub const fn set_reflection_tag(mut self,reflection_tag:Tag)->Self{
        self.reflection_tag=reflection_tag;
        self
    }

    pub const fn set_mod_refl_mode(mut self,mod_refl_mode:ModReflMode)->Self{
        self.mod_refl_mode=mod_refl_mode;
        self
    }
    pub const fn set_repr_attr(mut self,repr_attr:ReprAttr)->Self{
        self.repr_attr=repr_attr;
        self
    }
}

///////////////////////////

impl GenericParams {
    pub const fn new(
        lifetime: &'static [StaticStr],
        type_: &'static [&'static TypeLayout],
        const_: &'static [StaticStr],
    ) -> Self {
        Self {
            lifetime: StaticSlice::new(lifetime),
            type_: StaticSlice::new(type_),
            const_: StaticSlice::new(const_),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lifetime.is_empty() && self.type_.is_empty() && self.const_.is_empty()
    }
}

impl Display for GenericParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt("<", f)?;

        let post_iter = |i: usize, len: usize, f: &mut Formatter<'_>| -> fmt::Result {
            if i + 1 < len {
                fmt::Display::fmt(", ", f)?;
            }
            Ok(())
        };

        for (i, param) in self.lifetime.iter().cloned().enumerate() {
            fmt::Display::fmt(param.as_str(), &mut *f)?;
            post_iter(i, self.lifetime.len(), &mut *f)?;
        }
        for (i, param) in self.type_.iter().cloned().enumerate() {
            fmt::Debug::fmt(&param.full_type(), &mut *f)?;
            post_iter(i, self.type_.len(), &mut *f)?;
        }
        for (i, param) in self.const_.iter().cloned().enumerate() {
            fmt::Display::fmt(param.as_str(), &mut *f)?;
            post_iter(i, self.const_.len(), &mut *f)?;
        }
        fmt::Display::fmt(">", f)?;
        Ok(())
    }
}

///////////////////////////

impl TLData {
    pub const fn struct_(fields: &'static [TLField]) -> Self {
        TLData::Struct {
            fields: StaticSlice::new(fields),
        }
    }
    pub const fn enum_(variants: &'static [TLEnumVariant]) -> Self {
        TLData::Enum {
            variants: StaticSlice::new(variants),
        }
    }

    pub const fn prefix_type(
        first_suffix_field:usize,
        accessible_fields:FieldAccessibility,
        conditional_prefix_fields:&'static [IsConditional],
        fields: &'static [TLField],
    )->Self{
        TLData::PrefixType(TLPrefixType{
            first_suffix_field,
            accessible_fields,
            conditional_prefix_fields:StaticSlice::new(conditional_prefix_fields),
            fields:StaticSlice::new(fields),
        })
    }

    pub fn as_discriminant(&self) -> TLDataDiscriminant {
        match self {
            TLData::Primitive { .. } => TLDataDiscriminant::Primitive,
            TLData::Struct { .. } => TLDataDiscriminant::Struct,
            TLData::Enum { .. } => TLDataDiscriminant::Enum,
            TLData::PrefixType { .. } => TLDataDiscriminant::PrefixType,
        }
    }
}

///////////////////////////

impl TLEnumVariant {
    pub const fn new(name: &'static str, fields: &'static [TLField]) -> Self {
        Self {
            name: StaticStr::new(name),
            discriminant:TLDiscriminant::No,
            fields: StaticSlice::new(fields),
        }
    }

    pub const fn set_discriminant(mut self,discriminant:TLDiscriminant)->Self{
        self.discriminant=discriminant;
        self
    }
}

///////////////////////////

impl FullType {
    pub const fn new(
        name: &'static str,
        primitive: ROption<RustPrimitive>,
        generics: GenericParams,
    ) -> Self {
        Self {
            name: StaticStr::new(name),
            primitive,
            generics,
        }
    }
}

impl Display for FullType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}
impl Debug for FullType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (typename, start_gen, before_ty, ty_sep, end_gen) = match self.primitive {
            RSome(RustPrimitive::Reference) => ("&", "", " ", " ", " "),
            RSome(RustPrimitive::MutReference) => ("&", "", " mut ", " ", " "),
            RSome(RustPrimitive::ConstPtr) => ("*const", " ", "", " ", " "),
            RSome(RustPrimitive::MutPtr) => ("*mut", " ", "", " ", " "),
            RSome(RustPrimitive::Array) => ("", "[", "", ";", "]"),
            RNone => (self.name.as_str(), "<", "", ", ", ">"),
        };

        fmt::Display::fmt(typename, f)?;
        let mut is_before_ty = true;
        let generics = self.generics;
        if !generics.is_empty() {
            fmt::Display::fmt(start_gen, f)?;

            let post_iter = |i: usize, len: usize, f: &mut Formatter<'_>| -> fmt::Result {
                if i+1 < len {
                    fmt::Display::fmt(ty_sep, f)?;
                }
                Ok(())
            };

            let mut i=0;

            let total_generics_len=
                generics.lifetime.len()+generics.type_.len()+generics.const_.len();

            for param in generics.lifetime.iter().cloned() {
                fmt::Display::fmt(param.as_str(), &mut *f)?;
                post_iter(i,total_generics_len, &mut *f)?;
                i+=1;
            }
            for param in generics.type_.iter().cloned() {
                if is_before_ty {
                    fmt::Display::fmt(before_ty, &mut *f)?;
                    is_before_ty = false;
                }
                fmt::Debug::fmt(&param.full_type(), &mut *f)?;
                post_iter(i,total_generics_len, &mut *f)?;
                i+=1;
            }
            for param in generics.const_.iter().cloned() {
                fmt::Display::fmt(param.as_str(), &mut *f)?;
                post_iter(i,total_generics_len, &mut *f)?;
                i+=1;
            }
            fmt::Display::fmt(end_gen, f)?;
        }
        Ok(())
    }
}

////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq)]
struct TLFieldShallow {
    pub(crate) name: StaticStr,
    pub(crate) full_type: FullType,
    pub(crate) lifetime_indices: StaticSlice<LifetimeIndex>,
    /// This is None if it already printed that AbiInfo
    pub(crate) abi_info: Option<&'static AbiInfo>,

    pub(crate)functions:StaticSlice<TLFunction>,

    pub(crate)is_function:bool,

    pub(crate)field_accessor:FieldAccessor,
}

impl TLFieldShallow {
    fn new(field: TLField, include_abi_info: bool) -> Self {
        let abi_info = field.abi_info.get();
        TLFieldShallow {
            name: field.name,
            lifetime_indices: field.lifetime_indices,
            abi_info: if include_abi_info {
                Some(abi_info)
            } else {
                None
            },
            full_type: abi_info.layout.full_type,

            functions:field.functions,
            is_function:field.is_function,
            field_accessor:field.field_accessor,
        }
    }
}


////////////////////////////////////




#[repr(C)]
#[derive(Debug,Copy, Clone, PartialEq, StableAbi)]
pub struct TLFunction{
    /// The name of the field this is used inside of.
    pub name: StaticStr,
    
    /// The named lifetime parameters of function itself.
    pub bound_lifetimes: StaticSlice<StaticStr>,

    /// The parameters of the function,with names.
    /// 
    /// Lifetime indices at and after `bound_lifetimes.len()`
    /// are lifetimes declared in the function pointer.
    pub params:StaticSlice<TLField>,

    /// The return value of the function.
    /// 
    /// Lifetime indices at and after `bound_lifetimes.len()`
    /// are lifetimes declared in the function pointer.
    pub returns:ROption<TLField>,
}


impl TLFunction{
    pub const fn new(
        name: &'static str,
        bound_lifetimes: &'static [StaticStr],
        params:&'static [TLField],
        returns:ROption<TLField>,
    )->Self{
        Self{
            name:StaticStr::new(name),
            bound_lifetimes:StaticSlice::new(bound_lifetimes),
            params:StaticSlice::new(params),
            returns,
        }
    }
}

impl Display for TLFunction{
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        write!(f,"fn(")?;
        let param_count=self.params.len();
        for (param_i,param) in self.params.iter().enumerate() {
            Display::fmt(&TLFieldAndType::new(*param),f)?;
            if param_i+1!=param_count {
                Display::fmt(&", ",f)?;
            }
        }
        write!(f,")")?;
        if let RSome(returns)=self.returns {
            Display::fmt(&"->",f)?;
            Display::fmt(&TLFieldAndType::new(returns),f)?;
        }
        Ok(())
    }
}