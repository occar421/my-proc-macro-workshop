// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
pub use bitfield_impl::bitfield;
pub use bitfield_impl::BitfieldSpecifier;
use seq::seq;

const fn div_ceil(n: usize, m: usize) -> usize {
    (n + m - 1) / m
}

pub trait Specifier {
    type Type;
    const BITS: usize;
    const BYTES: usize = div_ceil(Self::BITS, 8);

    fn from_be_bytes(bytes: Vec<u8>) -> Self::Type {
        if bytes.len() != Self::BYTES {
            unimplemented!();
        }
        Self::from_be_bytes_core(bytes)
    }

    fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type;

    fn to_be_bytes(value: Self::Type) -> Vec<u8> {
        let result = Self::to_be_bytes_core(value);
        if result.len() != Self::BYTES {
            unimplemented!();
        }
        result
    }

    fn to_be_bytes_core(value: Self::Type) -> Vec<u8>;
}

impl Specifier for bool {
    type Type = bool;
    const BITS: usize = 1;

    fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type {
        bytes.last().unwrap() == &1
    }

    fn to_be_bytes_core(value: Self::Type) -> Vec<u8> {
        vec![if value { 0x1 } else { 0x0 }]
    }
}

seq! {N in 1..=64 {
    pub enum B~N {}
}}

seq! { N in 1..=8 {
    impl Specifier for B~N {
        type Type = u8;
        const BITS: usize = N;
        const BYTES: usize = 1;

        fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type {
            Self::Type::from_be_bytes(bytes.try_into().unwrap())
        }

        fn to_be_bytes_core(value: Self::Type) -> Vec<u8> {
            value.to_be_bytes().to_vec()
        }
    }
}}

seq! { N in 9..=16 {
    impl Specifier for B~N {
        type Type = u16;
        const BITS: usize = N;
        const BYTES: usize = 2;

        fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type {
            Self::Type::from_be_bytes(bytes.try_into().unwrap())
        }

        fn to_be_bytes_core(value: Self::Type) -> Vec<u8> {
            value.to_be_bytes().to_vec()
        }
    }
}}

seq! { N in 17..=32 {
    impl Specifier for B~N {
        type Type = u32;
        const BITS: usize = N;
        const BYTES: usize = 4;

        fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type {
            Self::Type::from_be_bytes(bytes.try_into().unwrap())
        }

        fn to_be_bytes_core(value: Self::Type) -> Vec<u8> {
            value.to_be_bytes().to_vec()
        }
    }
}}

seq! { N in 33..=64 {
    impl Specifier for B~N {
        type Type = u64;
        const BITS: usize = N;
        const BYTES: usize = 8;

        fn from_be_bytes_core(bytes: Vec<u8>) -> Self::Type {
            Self::Type::from_be_bytes(bytes.try_into().unwrap())
        }

        fn to_be_bytes_core(value: Self::Type) -> Vec<u8> {
            value.to_be_bytes().to_vec()
        }
    }
}}

pub enum CGUsize<const N: usize> {}
pub enum CGBool<const B: bool> {}

pub mod checks {
    use crate::{CGBool, CGUsize};

    pub trait TotalSizeIsMultipleOfEightBits {}

    pub enum ZeroMod8 {}

    impl TotalSizeIsMultipleOfEightBits for ZeroMod8 {}

    pub enum OneMod8 {}

    pub enum TwoMod8 {}

    pub enum ThreeMod8 {}

    pub enum FourMod8 {}

    pub enum FiveMod8 {}

    pub enum SixMod8 {}

    pub enum SevenMod8 {}

    pub trait DeductMod {
        type Mod;
    }

    impl DeductMod for CGUsize<0> {
        type Mod = ZeroMod8;
    }

    impl DeductMod for CGUsize<1> {
        type Mod = OneMod8;
    }

    impl DeductMod for CGUsize<2> {
        type Mod = TwoMod8;
    }

    impl DeductMod for CGUsize<3> {
        type Mod = ThreeMod8;
    }

    impl DeductMod for CGUsize<4> {
        type Mod = FourMod8;
    }

    impl DeductMod for CGUsize<5> {
        type Mod = FiveMod8;
    }

    impl DeductMod for CGUsize<6> {
        type Mod = SixMod8;
    }

    impl DeductMod for CGUsize<7> {
        type Mod = SevenMod8;
    }

    pub trait DiscriminantInRange {}

    pub enum True {}

    pub enum False {}

    impl DiscriminantInRange for True {}

    pub trait DeductPowOf2 {
        type IsPowOf2;
    }

    impl DeductPowOf2 for CGBool<true> {
        type IsPowOf2 = True;
    }

    impl DeductPowOf2 for CGBool<false> {
        type IsPowOf2 = False;
    }
}
