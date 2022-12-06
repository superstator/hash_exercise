use std::time::{Duration, Instant};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

type MapEntry<T> = Vec<(String, T, Option<(Instant, Duration)>)>;

/// A simple hashmap with a flexible map size.
///
/// [N] controls the size of the internal map structure, and is a compile-time
/// constant. A larger [N] will allow faster lookups for large sets, at the expense
/// of higher initial memory usage.
#[derive(Clone)]
pub struct MiniMap<const N: usize, T: Clone> {
    pub(crate) map: Arc<Mutex<Vec<MapEntry<T>>>>
}
impl<const N: usize, T: Clone> MiniMap<N, T> {
    pub fn new() -> MiniMap<N, T> {
        let mut map: Vec<MapEntry<T>> = Vec::with_capacity(N);
        for _ in 0..N { map.push(vec![]); }
        MiniMap { map: Arc::new(Mutex::new(map)) }
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
        let mut map = (*self.map).lock().unwrap();
        let slot: &mut MapEntry<T> = &mut (*map)[idx];

        let expiration = ttl.map(|d| (Instant::now(), d));

        // find and update the value, or insert a new entry
        let item = slot.iter_mut().find(|i| i.0 == key);
        match item {
            Some(i) => i.1 = value,
            None => slot.push((key.to_string(), value, expiration))
        }
    }

    /// Insert or replace an item at the given key. An optional [ttl]
    /// may be set to control expiration of an item.
    pub fn insert_many(&mut self, keys: &[&str], values: &[T], ttl: Option<Duration>) -> Result<(), String> {
        if keys.len() != values.len() { return Err("Must supply same number of keys and values".to_string()); }

        // get a lock up front and reuse it for all inserts
        let mut map = (*self.map).lock().unwrap();

        for (i, key) in keys.iter().enumerate() {
            // hash the key and find the corresponding slot in our map
            let idx = Self::hash(key);
            let slot: &mut MapEntry<T> = &mut (*map)[idx];

            let expiration = ttl.map(|d| (Instant::now(), d));

            // find and update the value, or insert a new entry
            let item = slot.iter_mut().find(|i| i.0 == *key);
            match item {
                Some(item) => item.1 = values[i].clone(),
                None => slot.push((key.to_string(), values[i].clone(), expiration))
            }
        }

        Ok(())
    }

    /// Get the item at the given key, if it exists and is not expired.
    /// If an item exists and IS expired, None will be returned, but the
    /// item will not be permanently lost until [expire()] is called.
    pub fn get(&self, key: &str) -> Option<(T, Option<Duration>)> {
        // hash the key and find the corresponding slot in our map
        let idx = Self::hash(key);
        let map = (*self.map).lock().unwrap();
        let slot: &MapEntry<T> = &(*map)[idx];

        // find and return a ref to the item
        let item = slot.iter().find(|i| i.0 == key);
        match item {
            None => None,
            Some(i) => {
                if let Some((stamp, duration)) = i.2 {
                    if stamp.elapsed() > duration { return None; }
                }
                Some((i.1.clone(), i.2.map(|d| d.1 - d.0.elapsed())))
            },
        }
    }

    /// Remove the item at the given key, and return it. Expiration status,
    /// if any, is ignored.
    pub fn remove(&mut self, key: &str) -> Option<T> {
        // hash the key and find the corresponding slot in our map
        let idx = Self::hash(key);
        let mut map = (*self.map).lock().unwrap();
        let slot: &mut MapEntry<T> = &mut (*map)[idx];

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
        let mut map = (*self.map).lock().unwrap();

        let expired: Vec<String> = map.iter_mut().flatten().filter(|i| i.2.is_some()).filter_map(|i| {
            let stamp = i.2.unwrap();
            if stamp.0.elapsed() > stamp.1 { Some(i.0.clone()) } else { None }
        }).collect();
        drop(map);

        for key in &expired {
            self.remove(key);
        }
        expired.len()
    }

    /// Returns the total number of keys in the map, ignoring expiration status.
    pub fn len(&self) -> usize {
        let mut map = (*self.map).lock().unwrap();
        (*map).iter().map(|i| i.len()).sum()
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
        assert_eq!(3, map.map.deref().lock().unwrap().deref()[1].len());
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

    #[test]
    fn can_insert_threaded() {
        let map: MiniMap<128, String> = MiniMap::new();

        let mut map1 = map.clone();
        let mut map2 = map.clone();
        let t2 = std::thread::spawn(move || { map2.insert("b","t2".to_string(), None);});
        let t1 = std::thread::spawn(move || { map1.insert("a","t1".to_string(), None);});
        t1.join().unwrap();
        t2.join().unwrap();

        assert_eq!(2, map.len());
    }

    #[cfg(feature = "perf_test")]
    mod perf_tests {
        use std::time::Instant;
        use crate::MiniMap;
        use rand::RngCore;

        #[test]
        fn can_meet_perf_goals() {
            let mut map: MiniMap<100000, String> = MiniMap::new();
            let mut keys = vec![];
            let mut values = vec![];


            for i in 0..10_000_000 {
                keys.push(get_int_id::<3>(i).unwrap());
                values.push(get_random_id::<32>());
            }
            let key_slices: Vec<&str> = keys.iter().map(|k| k.as_ref()).collect();
            map.insert_many(key_slices.as_slice(), &values, None);

            let mut times: Vec<u128> = vec![];
            // probe for random keys and record performance
            for i in 0..10 {
                let v = get_random_id::<3>();
                let s = Instant::now();
                let r = map.get(&v);
                let e = s.elapsed().as_nanos();
                times.push(e);
            }

            assert!(times.iter().max().unwrap() < &1_000_000u128)
        }

        /// Returns a random hex string of N bytes
        fn get_random_id<const N: usize>() -> String {
            let mut rng = rand::thread_rng();
            let mut id = [0u8; N];
            rng.fill_bytes(id.as_mut_slice());
            hex::encode(id)
        }

        /// Returns the given integer [id] as a hex value with [N] bytes, or an error if
        /// [id] would overflow [N]
        fn get_int_id<const N: usize>(id: usize) -> Result<String, usize> {
            match id > 2usize.pow(N as u32 * 8) {
                true => Err(id),
                false => Ok(hex::encode(id.to_le_bytes())[0..N*2].to_string())
            }
        }
    }
}
