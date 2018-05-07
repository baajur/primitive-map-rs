use bucket::Bucket;
use kv::{Key, Value};
use smallvec::SmallVec;
use std::marker::PhantomData;
use std::usize;

use bucket::helpers::IndexOf;

pub type VecBucket<K, V> = Vec<(K, V)>;

impl<K: Key, V: Value> Bucket<K, V> for VecBucket<K, V> {
    fn new() -> Self {
        Vec::with_capacity(1)
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        let old_value = self.index_of(key).map(|idx| self.swap_remove(idx).1);
        self.push((key, value));
        old_value
    }

    fn get(&self, key: K) -> Option<&V> {
        self.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.iter_mut().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    fn reached_max_capacity(&self) -> bool {
        false
    }
}

impl<K: Key, V: Value> IndexOf<K> for VecBucket<K, V> {
    fn index_of(&self, key: K) -> Option<usize> {
        self.iter()
            .enumerate()
            .find(|(idx, (k, _))| *k == key)
            .map(|(idx, _)| idx)
    }
}

