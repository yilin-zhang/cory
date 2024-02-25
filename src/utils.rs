use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

#[derive(Debug)]
pub struct AtomicF64 {
    storage: AtomicU64,
}

impl AtomicF64 {
    pub fn new(value: f64) -> Self {
        let as_u64 = value.to_bits();
        Self {
            storage: AtomicU64::new(as_u64),
        }
    }
    pub fn store(&self, value: f64, ordering: Ordering) {
        let as_u64 = value.to_bits();
        self.storage.store(as_u64, ordering)
    }
    pub fn load(&self, ordering: Ordering) -> f64 {
        let as_u64 = self.storage.load(ordering);
        f64::from_bits(as_u64)
    }
}

#[derive(Debug)]
pub struct AtomicF32 {
    storage: AtomicU32,
}

#[allow(dead_code)]
impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        let as_u32 = value.to_bits();
        Self {
            storage: AtomicU32::new(as_u32),
        }
    }
    pub fn store(&self, value: f32, ordering: Ordering) {
        let as_u32 = value.to_bits();
        self.storage.store(as_u32, ordering)
    }
    pub fn load(&self, ordering: Ordering) -> f32 {
        let as_u32 = self.storage.load(ordering);
        f32::from_bits(as_u32)
    }
}

#[allow(dead_code)]
pub fn inc_by_precision<T: Into<f64> + From<f64>>(value: T, inc: T, precision: u32) -> T {
    // Convert to f64, calculate, convert back to T
    let base: f64 = 10.0;
    let value_f64 = value.into();
    let inc_f64 = inc.into();
    let result = (value_f64 + inc_f64) * base.powf((precision - 1) as f64).round() / 10.0;
    T::from(result)
}

#[allow(dead_code)]
pub fn round_by_precision<T: Into<f64> + From<f64>>(value: T, precision: u32) -> T {
    inc_by_precision(value, 0.0.into(), precision)
}
