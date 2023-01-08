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

    pub trait ZeroMod8: TotalSizeIsMultipleOfEightBits {}

    pub trait OneMod8 {}

    pub trait TwoMod8 {}

    pub trait ThreeMod8 {}

    pub trait FourMod8 {}

    pub trait FiveMod8 {}

    pub trait SixMod8 {}

    pub trait SevenMod8 {}

    pub trait CG<const N: usize> {}

    impl TotalSizeIsMultipleOfEightBits for dyn CG<0> {}

    impl ZeroMod8 for dyn CG<0> {}

    impl OneMod8 for dyn CG<1> {}

    impl TwoMod8 for dyn CG<2> {}

    impl ThreeMod8 for dyn CG<3> {}

    impl FourMod8 for dyn CG<4> {}

    impl FiveMod8 for dyn CG<5> {}

    impl SixMod8 for dyn CG<6> {}

    impl SevenMod8 for dyn CG<7> {}

    // fn a() {
    //     type D = CG<{ 1 + 2 }>;
    //     type DDD = CG<{ (B1::BITS + 2) % 2 }>;
    // }

    // #[inline]
    // fn assert_type<C: TotalSizeIsMultipleOfEightBits + ?Sized>() {}
    //
    // fn a() {
    //     assert_type::<dyn BAdd<B1, B23>>();
    //
    //     <B1 as Specifier>::BITS
    // }
    //
    // #[macro_export]
    // macro_rules! hoge {
    //     // ($($t: ty),*) => {
    //     //
    //     // }
    //     ($t1: ty, $t2: ty) => {
    //         println!("{} + {}", stringify!($t1), stringify!($t2));
    //     };
    // }
}
