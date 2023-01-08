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

pub struct CG1<const N: usize>;
pub struct CG2<const N: usize, const M: usize>;

pub mod checks {
    use crate::CG1;

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

    impl DeductMod for CG1<0> {
        type Mod = ZeroMod8;
    }

    impl DeductMod for CG1<1> {
        type Mod = OneMod8;
    }

    impl DeductMod for CG1<2> {
        type Mod = TwoMod8;
    }

    impl DeductMod for CG1<3> {
        type Mod = ThreeMod8;
    }

    impl DeductMod for CG1<4> {
        type Mod = FourMod8;
    }

    impl DeductMod for CG1<5> {
        type Mod = FiveMod8;
    }

    impl DeductMod for CG1<6> {
        type Mod = SixMod8;
    }

    impl DeductMod for CG1<7> {
        type Mod = SevenMod8;
    }
}
