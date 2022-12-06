use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

type MapEntry<T> = Vec<(String, T, Option<(Instant, Duration)>)>;

pub struct MiniMap<const N: usize, T> {
    pub(crate) map: [MapEntry<T>; N]
}
impl<const N: usize, T> MiniMap<N, T> {
    pub fn new() -> MiniMap<N, T> {
        MiniMap { map: [(); N].map(|_| Vec::new()) }
    }

    fn hash(key: &str) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() % N as u64) as usize
    }

    pub fn insert(&mut self, key: &str, value: T) {
        // hash the key and find the corresponding slot in our map
        let idx = Self::hash(key);
        let slot: &mut MapEntry<T> = &mut self.map[idx];

        // find and update the value, or insert a new entry
        let item = slot.iter_mut().find(|i| i.0 == key);
        match item {
            Some(i) => i.1 = value,
            None => slot.push((key.to_string(), value, None))
        }
    }

    pub fn get(&self, key: &str) -> Option<(&T, Option<Duration>)> {
        // hash the key and find the corresponding slot in our map
        let idx = Self::hash(key);
        let slot: &MapEntry<T> = &self.map[idx];

        // find and return a ref to the item
        let item = slot.iter().find(|i| i.0 == key);
        match item {
            Some(i) => Some((&i.1, None)),
            None => None
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<T> {
        // hash the key and find the corresponding slot in our map
        let idx = Self::hash(key);
        let slot: &mut MapEntry<T> = &mut self.map[idx];

        // find and remove the item
        let item = slot.iter().position(|i| i.0 == key);
        match item {
            Some(i) => Some(slot.remove(i).1),
            None => None
        }
    }

    pub fn len(&self) -> usize {
        self.map.iter().map(|i| i.len()).sum()
    }
}
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_insert_new() {
        let mut map: MiniMap<128, String> = MiniMap::new();
        map.insert("foo", "bar".to_string());

        assert_eq!("bar", map.get("foo").unwrap().0);
    }

    #[test]
    fn can_insert_with_collision() {
        // very small hash size to maximize possibility of collision
        let mut map: MiniMap<2, String> = MiniMap::new();
        map.insert("foo", "1".to_string());
        map.insert("bar", "2".to_string());
        map.insert("baz", "3".to_string());

        // first three keys above happen to have odd hashes, so they all end up in the second bucket
        assert_eq!(3, map.map[1].len());
    }

    #[test]
    fn can_get() {
        let mut map: MiniMap<128, String> = MiniMap::new();
        map.insert("foo", "bar".to_string());
        map.insert("baz", "bat".to_string());

        assert_eq!("bat", map.get("baz").unwrap().0);
    }

    #[test]
    fn can_remove() {
        let mut map: MiniMap<128, String> = MiniMap::new();
        map.insert("foo", "bar".to_string());
        map.insert("baz", "bat".to_string());

        assert_eq!(Some("bat".to_string()), map.remove("baz"));
        assert_eq!(None, map.remove("xyz"));
        assert_eq!(None, map.get("baz"));
    }
}
