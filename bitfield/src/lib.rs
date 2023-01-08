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
    use crate::*;
    use paste::paste;

    pub trait TotalSizeIsMultipleOfEightBits {}

    pub trait ZeroMod8: TotalSizeIsMultipleOfEightBits {}

    pub trait OneMod8 {}

    pub trait TwoMod8 {}

    pub trait ThreeMod8 {}

    pub trait FourMod8 {}

    pub trait FiveMod8 {}

    pub trait SixMod8 {}

    pub trait SevenMod8 {}

    macro_rules! iter_b {
        ($($($num:literal),* => $word:ident),*) => {
            $(
                iter_b!($($num)* => $word);
            )*
        };
        ($($num:literal)* => $word:ident) => {
            paste! {
                $(
                    impl [< $word Mod8 >] for [< B $num >] {}
                )*
            }
        }
    }

    iter_b![
        1, 9,17,25,33,41,49,57 => One,
        2,10,18,26,34,42,50,58 => Two,
        3,11,19,27,35,43,51,59 => Three,
        4,12,20,28,36,44,52,60 => Four,
        5,13,21,29,37,45,53,61 => Five,
        6,14,22,30,38,46,54,62 => Six,
        7,15,23,31,39,47,55,63 => Seven,
        8,16,24,32,40,48,56,64 => Zero
    ];

    macro_rules! iter_mul_8 {
        ($($num:literal),*) => {
            paste! {
                $(
                    impl TotalSizeIsMultipleOfEightBits for [< B $num >] {}
                )*
            }
        }
    }

    iter_mul_8![8, 16, 24, 32, 40, 48, 56, 64];

    pub trait BAdd<T1, T2> {}

    impl TotalSizeIsMultipleOfEightBits for dyn BAdd<dyn ZeroMod8, dyn ZeroMod8> {}

    impl ZeroMod8 for dyn BAdd<dyn ZeroMod8, dyn ZeroMod8> {}

    impl TotalSizeIsMultipleOfEightBits for dyn BAdd<dyn OneMod8, dyn SevenMod8> {}

    impl ZeroMod8 for dyn BAdd<dyn OneMod8, dyn SevenMod8> {}
}
