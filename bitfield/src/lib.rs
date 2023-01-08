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
use seq::seq;

pub trait Specifier {
    const BITS: usize;
}

seq! {N in 1..=64 {
    pub enum B~N {}

    impl Specifier for B~N {
        const BITS: usize = N;
    }
}}

pub mod checks {
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

    pub struct CG<const N: usize>;

    pub trait DeductMod {
        type Enum;
    }

    impl DeductMod for CG<0> {
        type Enum = ZeroMod8;
    }

    impl DeductMod for CG<1> {
        type Enum = OneMod8;
    }

    impl DeductMod for CG<2> {
        type Enum = TwoMod8;
    }

    impl DeductMod for CG<3> {
        type Enum = ThreeMod8;
    }

    impl DeductMod for CG<4> {
        type Enum = FourMod8;
    }

    impl DeductMod for CG<5> {
        type Enum = FiveMod8;
    }

    impl DeductMod for CG<6> {
        type Enum = SixMod8;
    }

    impl DeductMod for CG<7> {
        type Enum = SevenMod8;
    }
}
