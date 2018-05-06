#![feature(nll)]

extern crate smallvec;

pub mod bucket;
pub mod hash;
pub mod kv;

use std::marker::PhantomData;

pub use bucket::{Bucket, BucketList, BucketListNew, OptionBucket, SmallVecBucket, VecBucket};
pub use hash::{DefaultHasher, Hash, Hasher};
pub use kv::{Key, Value};

pub struct PrimitiveMap<
    K: Key,
    V: Value,
    B: Bucket<K, V> = SmallVecBucket<K, V>,
    BL: BucketList<K, V, B> = Vec<B>,
    H: Hasher<K> = DefaultHasher<K>,
> {
    buckets: BL,
    _marker: PhantomData<(K, V, H, B)>,
}

impl<K, V, B, BL, H> Clone for PrimitiveMap<K, V, B, BL, H>
where
    K: Key,
    V: Value,
    B: Bucket<K, V> + Clone,
    BL: BucketList<K, V, B> + Clone,
    H: Hasher<K> + Default,
{
    fn clone(&self) -> Self {
        PrimitiveMap::custom(
            self.buckets.clone(),
            H::default()
        )
    }
}

/// `Vec`-based `map` with `SmallVec`(4) buckets.
/// The balanced default
pub type VecPrimitiveMap<K, V> =
    PrimitiveMap<K, V, SmallVecBucket<K, V>, Vec<SmallVecBucket<K, V>>, DefaultHasher<K>>;

/// `Array`-based `map` with `SmallVec`(1) buckets.
/// The main array is stored on the stack,
/// the buckets may extend onto heap.
pub type ArrayPrimitiveMap<K, V, A> = PrimitiveMap<K, V, SmallVecBucket<K, V>, A, DefaultHasher<K>>;

/// Linear-probing PrimitiveMap alias.
/// Useful in embedded environments and where full-stack `map` alignment is necessary
pub type LinearPrimitiveMap<K, V, A> = PrimitiveMap<K, V, OptionBucket<K, V>, A, DefaultHasher<K>>;

impl<K, V, B, BL, H> PrimitiveMap<K, V, B, BL, H>
where
    K: Key,
    V: Value,
    B: Bucket<K, V>,
    BL: BucketList<K, V, B> + BucketListNew<K, V, B>,
    H: Hasher<K>,
{
    fn default() -> Self {
        PrimitiveMap::custom(BL::initialized(), H::default())
    }

    fn with_capacity(cap: usize) -> Self {
        PrimitiveMap::custom(BL::initialized_with_capacity(cap), H::default())
    }
}

impl<K, V, B, BL> PrimitiveMap<K, V, B, BL>
where
    K: Key,
    V: Value,
    B: Bucket<K, V>,
    BL: BucketList<K, V, B>,
    DefaultHasher<K>: Hasher<K>,
{
    fn with_buckets(buckets: BL) -> Self {
        PrimitiveMap::custom(buckets, DefaultHasher::new())
    }
}

impl<K, V, BL> PrimitiveMap<K, V, OptionBucket<K, V>, BL>
where
    K: Key,
    V: Value,
    BL: BucketList<K, V, OptionBucket<K, V>>,
    DefaultHasher<K>: Hasher<K>,
{
    fn with_linear_probing(buckets: BL) -> Self {
        PrimitiveMap::custom(buckets, DefaultHasher::new())
    }
}

impl<K, V, B, BL, H> PrimitiveMap<K, V, B, BL, H>
where
    K: Key,
    V: Value,
    B: Bucket<K, V>,
    BL: BucketList<K, V, B>,
    H: Hasher<K>,
{
    fn custom(buckets: BL, _: H) -> Self {
        PrimitiveMap {
            buckets,
            _marker: PhantomData,
        }
    }
}

impl<K, V, B, BL, H> PrimitiveMap<K, V, B, BL, H>
where
    K: Key,
    V: Value,
    B: Bucket<K, V>,
    BL: BucketList<K, V, B>,
    H: Hasher<K>,
{
    pub fn insert(&mut self, key: K, value: V) {
        let addr = self.get_addr(key);
        let bucket = self.buckets
            .search_mut(addr, |bucket| !bucket.reached_max_capacity())
            .expect("PrimitiveMap capacity is exhausted");
        bucket.push(key, value)
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let addr = self.get_addr(key);

        // TODO: optimize double-work here
        let bucket = self.buckets
            .search(addr, |bucket| bucket.get(key).is_some());

        bucket.and_then(|b| b.get(key))
    }

    fn get_addr(&self, key: K) -> usize {
        let hash = H::hash(key);
        H::compress(hash, self.buckets.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bucket::{Array1024, Array64};

    #[test]
    fn create_vec() {
        // Vec map with StackVec(1) bucket
        let mut map = VecPrimitiveMap::default();
        map.insert(1, 1);
        map.get(1);
    }

    #[test]
    fn create_lp() {
        // Array64 map with Option<(K, V)> buckets (linear probing)
        let mut map = LinearPrimitiveMap::with_buckets(Array64::initialized());
        map.insert(1, 1);
        map.get(1);
    }

    #[test]
    fn create_custom() {
        let buckets = Vec::<OptionBucket<_, _>>::initialized_with_capacity(1000);
        let hasher = DefaultHasher::new();
        let mut map = PrimitiveMap::custom(buckets, hasher);
        map.insert(1, 1);
        map.get(1);
    }

    #[test]
    fn insert_dynamic() {
        let mut map = VecPrimitiveMap::default();
        map.insert(0u8, 10u32);
    }

    #[test]
    fn insert_fixed() {
        let mut map = LinearPrimitiveMap::with_buckets(Array64::initialized());
        map.insert(0u16, 10u32);
    }

    #[test]
    fn get_empty_dynamic() {
        let map = VecPrimitiveMap::default();
        assert_eq!(map.get(0u32), None::<&u32>);
    }

    #[test]
    fn get_empty_fixed() {
        let map = LinearPrimitiveMap::with_buckets(Array64::initialized());
        assert_eq!(map.get(0u32), None::<&u32>);
    }

    #[test]
    fn insert_and_get_dynamic() {
        let mut map = VecPrimitiveMap::default();
        map.insert(0i8, 10u32);
        assert_eq!(map.get(0i8), Some(&10u32));
    }

    #[test]
    fn insert_and_get_fixed() {
        let mut map = LinearPrimitiveMap::with_buckets(Array64::initialized());
        map.insert(0i16, 10u32);
        assert_eq!(map.get(0i16), Some(&10u32));
    }

    #[test]
    fn insert_saturate_buckets_dynamic() {
        let mut map = VecPrimitiveMap::with_capacity(100);
        for i in 0..10000 {
            map.insert(i, i);
        }
        for i in 0..10000 {
            assert_eq!(map.get(i), Some(&i))
        }
    }

    #[test]
    fn insert_full_load_linear_probing() {
        let mut map = LinearPrimitiveMap::with_buckets(Array1024::initialized());
        for i in 0..1024 {
            map.insert(i, i);
        }
        for i in 0..1024 {
            assert_eq!(map.get(i), Some(&i))
        }
    }
}
