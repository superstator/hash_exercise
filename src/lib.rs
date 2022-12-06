use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

type MapEntry<T> = Vec<(String, T, Option<(Instant, Duration)>)>;

/// A simple hashmap with a flexible map size.
///
/// [N] controls the size of the internal map structure, and is a compile-time
/// constant. A larger [N] will allow faster lookups for large sets, at the expense
/// of higher initial memory usage.
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

    /// Insert or replace an item at the given key. An optional [ttl]
    /// may be set to control expiration of an item.
    pub fn insert(&mut self, key: &str, value: T, ttl: Option<Duration>) {
        // hash the key and find the corresponding slot in our map
        let idx = Self::hash(key);
        let slot: &mut MapEntry<T> = &mut self.map[idx];

        let expiration = ttl.map(|d| (Instant::now(), d));

        // find and update the value, or insert a new entry
        let item = slot.iter_mut().find(|i| i.0 == key);
        match item {
            Some(i) => i.1 = value,
            None => slot.push((key.to_string(), value, expiration))
        }
    }

    /// Get the item at the given key, if it exists and is not expired.
    /// If an item exists and IS expired, None will be returned, but the
    /// item will not be permanently lost until [expire()] is called.
    pub fn get(&self, key: &str) -> Option<(&T, Option<Duration>)> {
        // hash the key and find the corresponding slot in our map
        let idx = Self::hash(key);
        let slot: &MapEntry<T> = &self.map[idx];

        // find and return a ref to the item
        let item = slot.iter().find(|i| i.0 == key);
        match item {
            None => None,
            Some(i) => {
                if let Some((stamp, duration)) = i.2 {
                    if stamp.elapsed() > duration { return None; }
                }
                Some((&i.1, i.2.map(|d| d.1 - d.0.elapsed())))
            },
        }
    }

    /// Remove the item at the given key, and return it. Expiration status,
    /// if any, is ignored.
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

    /// Checks the expiration status of all keys, and permanently removes any expired items. A count
    /// of expired items is returned.
    pub fn expire(&mut self) -> usize {
        let expired: Vec<String> = self.map.iter_mut().flatten().filter(|i| i.2.is_some()).filter_map(|i| {
            let stamp = i.2.unwrap();
            if stamp.0.elapsed() > stamp.1 { Some(i.0.clone()) } else { None }
        }).collect();

        for key in &expired {
            self.remove(key);
        }
        expired.len()
    }

    /// Returns the total number of keys in the map, ignoring expiration status.
    pub fn len(&self) -> usize {
        self.map.iter().map(|i| i.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_insert_new() {
        let mut map: MiniMap<128, String> = MiniMap::new();
        map.insert("foo", "bar".to_string(), None);

        assert_eq!("bar", map.get("foo").unwrap().0);
    }

    #[test]
    fn can_insert_with_collision() {
        // very small hash size to maximize possibility of collision
        let mut map: MiniMap<2, String> = MiniMap::new();
        map.insert("foo", "1".to_string(), None);
        map.insert("bar", "2".to_string(), None);
        map.insert("baz", "3".to_string(), None);

        // first three keys above happen to have odd hashes, so they all end up in the second bucket
        assert_eq!(3, map.map[1].len());
    }

    #[test]
    fn can_get() {
        let mut map: MiniMap<128, String> = MiniMap::new();
        map.insert("foo", "bar".to_string(), None);
        map.insert("baz", "bat".to_string(), None);

        assert_eq!("bat", map.get("baz").unwrap().0);
    }

    #[test]
    fn can_remove() {
        let mut map: MiniMap<128, String> = MiniMap::new();
        map.insert("foo", "bar".to_string(), None);
        map.insert("baz", "bat".to_string(), None);

        assert_eq!(Some("bat".to_string()), map.remove("baz"));
        assert_eq!(None, map.remove("xyz"));
        assert_eq!(None, map.get("baz"));
    }

    #[test]
    fn can_expire() {
        let mut map: MiniMap<128, String> = MiniMap::new();
        map.insert("foo", "bar".to_string(), Some(Duration::from_millis(1)));

        assert_eq!("bar", map.get("foo").unwrap().0);
        assert_eq!(1, map.len());

        std::thread::sleep(Duration::from_millis(2));
        assert_eq!(None, map.get("foo"));
        assert_eq!(1, map.len());

        assert_eq!(1, map.expire());
        assert_eq!(0, map.len());
    }
}
