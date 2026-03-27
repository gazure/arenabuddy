use std::collections::{BTreeMap, btree_map::Entry};

pub struct MultiMap<K, V> {
    inner: BTreeMap<K, Vec<V>>,
}

impl<K: Ord, V> Default for MultiMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord, V> MultiMap<K, V> {
    pub fn new() -> Self {
        Self { inner: BTreeMap::new() }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.inner.entry(key).or_default().push(value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.get_all(key)?.first()
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.get_all_mut(key)?.first_mut()
    }

    pub fn get_all(&self, key: &K) -> Option<&[V]> {
        self.inner.get(key).map(std::convert::AsRef::as_ref)
    }

    pub fn get_all_mut(&mut self, key: &K) -> Option<&mut Vec<V>> {
        self.inner.get_mut(key)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn remove_all(&mut self, key: &K) -> Option<Vec<V>> {
        self.inner.remove(key)
    }

    pub fn pop(&mut self, key: &K) -> Option<V> {
        let values = self.inner.get_mut(key)?;
        let popped = values.pop();
        if values.is_empty() {
            self.inner.remove(key);
        }
        popped
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.inner.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.values().flat_map(|v| v.iter())
    }

    pub fn vec_values(&self) -> impl Iterator<Item = &Vec<V>> {
        self.inner.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter().flat_map(|(k, v)| v.iter().map(move |v| (k, v)))
    }

    pub fn vec_iter(&self) -> impl Iterator<Item = (&K, &Vec<V>)> {
        self.inner.iter()
    }

    pub fn vec_iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut Vec<V>)> {
        self.inner.iter_mut()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.inner
            .iter_mut()
            .flat_map(|(k, v)| v.iter_mut().map(move |v| (k, v)))
    }

    pub fn entry(&mut self, key: K) -> Entry<'_, K, Vec<V>> {
        self.inner.entry(key)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let multimap: MultiMap<String, i32> = MultiMap::new();
        assert_eq!(multimap.len(), 0);
        assert_eq!(multimap.get(&"key".to_string()), None);
    }

    #[test]
    fn test_insert_single_value() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 10);

        assert_eq!(multimap.get(&"key1"), Some(&10));
        assert_eq!(multimap.len(), 1);
    }

    #[test]
    fn test_insert_multiple_values_same_key() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 10);
        multimap.insert("key1", 20);
        multimap.insert("key1", 30);

        assert_eq!(multimap.get(&"key1"), Some(&10));
        assert_eq!(multimap.get_all(&"key1"), Some(&[10, 20, 30][..]));
        assert_eq!(multimap.len(), 1);
    }

    #[test]
    fn test_get_all() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key1", 3);
        multimap.insert("key2", 4);

        assert_eq!(multimap.get_all(&"key1"), Some(&[1, 2, 3][..]));
        assert_eq!(multimap.get_all(&"key2"), Some(&[4][..]));
        assert_eq!(multimap.get_all(&"key3"), None);
    }

    #[test]
    fn test_clear() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key2", 3);

        multimap.clear();
        assert_eq!(multimap.len(), 0);
        assert_eq!(multimap.get(&"key1"), None);
    }

    #[test]
    fn test_entry() {
        let mut multimap = MultiMap::new();
        multimap.entry("key1").or_default().push(10);
        assert_eq!(multimap.get_all(&"key1"), Some(&[10][..]));

        multimap.entry("key1").or_default().push(20);
        assert_eq!(multimap.get_all(&"key1"), Some(&[10, 20][..]));
    }

    #[test]
    fn test_vec_values() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key2", 3);

        let vec_values: Vec<_> = multimap.vec_values().collect();
        assert_eq!(vec_values[0], &vec![1, 2]);
        assert_eq!(vec_values[1], &vec![3]);
    }
}
