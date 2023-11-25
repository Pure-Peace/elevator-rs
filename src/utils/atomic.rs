pub use arc_swap::*;
pub use atomic_float::{AtomicF32, AtomicF64};

use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::Deref,
    sync::{atomic::*, Arc},
};

pub trait AtomicBitOperation: AtomicValue {
    fn and(&self, val: Self::Value) -> Self::Value;
    fn nand(&self, val: Self::Value) -> Self::Value;
    fn or(&self, val: Self::Value) -> Self::Value;
    fn xor(&self, val: Self::Value) -> Self::Value;
}

pub trait AtomicOperation: AtomicValue {
    fn add(&self, val: Self::Value) -> Self::Value;
    fn max(&self, val: Self::Value) -> Self::Value;
    fn min(&self, val: Self::Value) -> Self::Value;
    fn sub(&self, val: Self::Value) -> Self::Value;
}

pub trait AtomicValue: Sized {
    type Value;
    fn val(&self) -> Self::Value;
    fn set(&self, val: Self::Value);
}

pub type Atomic<T> = AtomicAny<Arc<T>>;
pub type AtomicOption<T> = AtomicAny<Option<Arc<T>>>;

impl<T> Display for Atomic<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val())
    }
}

impl<T> Atomic<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self(Arc::new(val).into())
    }
}

impl<T> AtomicOption<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self(Some(Arc::new(val)).into())
    }

    #[inline]
    pub fn from_option(option: Option<T>) -> Self {
        Self(option.map(|inner| inner.into()).into())
    }
}

impl<T> Serialize for Atomic<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.load().as_ref().serialize(serializer)
    }
}

impl<T> Serialize for AtomicOption<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.load().as_deref().serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Atomic<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|t| Atomic::new(t))
    }
}

impl<'de, T> Deserialize<'de> for AtomicOption<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|t| t.map(AtomicOption::new).unwrap_or_default())
    }
}

impl<T> From<T> for Atomic<T> {
    #[inline]
    fn from(val: T) -> Self {
        Atomic::new(val)
    }
}

impl<T> From<Option<T>> for AtomicOption<T> {
    #[inline]
    fn from(val: Option<T>) -> Self {
        AtomicOption::from_option(val)
    }
}

#[derive(Debug, Default)]
pub struct AtomicAny<T: RefCnt>(ArcSwapAny<T>);

impl<T> AtomicValue for AtomicAny<T>
where
    T: RefCnt,
{
    type Value = T;

    #[inline]
    fn val(&self) -> Self::Value {
        self.0.load_full()
    }

    #[inline]
    fn set(&self, val: Self::Value) {
        self.0.store(val)
    }
}

impl<T> Clone for AtomicAny<T>
where
    T: RefCnt + Clone,
{
    fn clone(&self) -> Self {
        Self(ArcSwapAny::new(self.0.load().clone()))
    }
}

impl<T> Deref for AtomicAny<T>
where
    T: RefCnt,
{
    type Target = ArcSwapAny<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

macro_rules! implAtomicValue {
        ($($ty: ty$(,)*)*) => {
            paste::paste! {
                $(
                    #[derive(Debug, Default)]
                    pub struct [<$ty:camel>]([<Atomic $ty:camel>]);

                    impl [<$ty:camel>] {
                        pub const fn new(val: [<$ty:snake>]) -> Self {
                            Self([<Atomic $ty:camel>]::new(val))
                        }
                    }

                    impl AtomicValue for [<$ty:camel>] {
                        type Value = [<$ty:snake>];

                        #[inline]
                        fn val(&self) -> Self::Value {
                            self.0.load(Ordering::SeqCst)
                        }

                        #[inline]
                        fn set(&self, val: Self::Value) {
                            self.0.store(val, Ordering::SeqCst)
                        }
                    }

                    impl Deref for [<$ty:camel>] {
                        type Target = [<Atomic $ty:camel>];

                        fn deref(&self) -> &Self::Target {
                            &self.0
                        }
                    }

                    impl From<[<$ty:camel>]> for [<Atomic $ty:camel>] {
                        fn from(val: [<$ty:camel>]) -> Self {
                            val.0
                        }
                    }

                    impl From<[<$ty:snake>]> for [<$ty:camel>] {
                        #[inline]
                        fn from(val: [<$ty:snake>]) -> Self {
                            [<$ty:camel>]::new(val)
                        }
                    }

                    impl Clone for [<$ty:camel>] {
                        fn clone(&self) -> Self {
                            Self::from(self.val())
                        }
                    }
                )*
            }
        };
    }

impl Bool {
    #[inline]
    pub fn is_true(&self) -> bool {
        self.val()
    }

    #[inline]
    pub fn is_false(&self) -> bool {
        !self.is_true()
    }

    #[inline]
    pub fn set_false(&self) {
        self.set(false)
    }

    #[inline]
    pub fn set_true(&self) {
        self.set(true)
    }

    #[inline]
    pub fn toggle(&self) {
        self.set(!self.val())
    }
}

implAtomicValue!(bool, i8, u8, i16, u16, i32, u32, i64, u64, isize, usize, f32, f64);

macro_rules! implAtomicValueSerde {
        ($($ty: ty$(,)*)*) => {
            paste::paste! {
                $(
                    impl Serialize for [<$ty:camel>] {
                        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                        where
                            S: serde::Serializer,
                        {
                            [<$ty:snake>]::serialize(&self.val(), serializer)
                        }
                    }

                    impl<'de> Deserialize<'de> for [<$ty:camel>] {
                        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                        where
                            D: serde::Deserializer<'de>,
                        {
                            [<$ty:snake>]::deserialize(deserializer).map(Self::from)
                        }
                    }
                )*
            }
        };
    }

implAtomicValueSerde!(bool, i8, u8, i16, u16, i32, u32, i64, u64, isize, usize, f32, f64);

macro_rules! implAtomicOperation {
        ($($ty: ty$(,)*)*) => {
            paste::paste! {
                $(
                    impl AtomicOperation for [<$ty:camel>] {
                        #[inline]
                        fn add(&self, val: Self::Value) -> Self::Value {
                            self.0.fetch_add(val, Ordering::SeqCst)
                        }

                        #[inline]
                        fn max(&self, val: Self::Value) -> Self::Value {
                            self.0.fetch_max(val, Ordering::SeqCst)
                        }

                        #[inline]
                        fn min(&self, val: Self::Value) -> Self::Value {
                            self.0.fetch_min(val, Ordering::SeqCst)
                        }

                        #[inline]
                        fn sub(&self, val: Self::Value) -> Self::Value {
                            self.0.fetch_sub(val, Ordering::SeqCst)
                        }
                    }
                )*
            }
        };
    }

implAtomicOperation!(i8, u8, i16, u16, i32, u32, i64, u64, isize, usize, f32, f64);

macro_rules! implAtomicBitOperation {
        ($($ty: ty$(,)*)*) => {
            paste::paste! {
                $(
                    impl AtomicBitOperation for [<$ty:camel>] {
                        #[inline]
                        fn and(&self, val: Self::Value) -> Self::Value {
                            self.0.fetch_and(val, Ordering::SeqCst)
                        }

                        #[inline]
                        fn nand(&self, val: Self::Value) -> Self::Value {
                            self.0.fetch_nand(val, Ordering::SeqCst)
                        }

                        #[inline]
                        fn or(&self, val: Self::Value) -> Self::Value {
                            self.0.fetch_or(val, Ordering::SeqCst)
                        }

                        #[inline]
                        fn xor(&self, val: Self::Value) -> Self::Value {
                            self.0.fetch_xor(val, Ordering::SeqCst)
                        }
                    }
                )*
            }
        };
    }

implAtomicBitOperation!(i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);
