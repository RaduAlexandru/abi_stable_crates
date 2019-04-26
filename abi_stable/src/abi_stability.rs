/*!
types and traits related to abi stability.
*/

#[macro_use]
pub mod type_layout;
pub(crate) mod abi_checking;
pub mod stable_abi_trait;
pub mod tagging;

#[cfg(test)]
mod layout_tests;

pub use self::{
    stable_abi_trait::{
        AbiInfo, AbiInfoWrapper, StableAbi,
        SharedStableAbi,
    },
    tagging::{
        Tag,
    }

};

use self::{
    stable_abi_trait::{
        GetAbiInfo,
    },
    type_layout::{
        LifetimeIndex, TLData, TLField,
        TypeLayout, TypeLayoutParams,
    },
};

use self::type_layout::RustPrimitive;
