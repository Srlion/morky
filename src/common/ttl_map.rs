#![allow(dead_code)]

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use serde::{Serialize, Serializer, ser::SerializeMap};

#[derive(Debug, Clone)]
pub struct TtlMap<K, V> {
    inner: Arc<Mutex<HashMap<K, (V, Instant)>>>,
    default_ttl: Duration,
}

impl<K, V> Serialize for TtlMap<K, V>
where
    K: Eq + Hash + Send + Serialize + 'static,
    V: Send + Serialize + 'static,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let map = self.inner.lock().unwrap();
        let now = Instant::now();

        let mut s = serializer.serialize_map(None)?;
        for (k, (v, _)) in map.iter().filter(|(_, (_, exp))| *exp > now) {
            s.serialize_entry(k, v)?;
        }
        s.end()
    }
}

impl<K: Eq + Hash + Send + 'static, V: Send + 'static> TtlMap<K, V> {
    pub fn new(sweep: Duration, default_ttl: Duration) -> Self {
        let inner = Arc::new(Mutex::new(HashMap::new()));
        let weak = Arc::downgrade(&inner);

        thread::spawn(move || {
            loop {
                thread::sleep(sweep);
                let Some(map) = weak.upgrade() else { return };
                let now = Instant::now();
                map.lock().unwrap().retain(|_, (_, exp)| *exp > now);
            }
        });

        Self { inner, default_ttl }
    }

    pub fn insert(&self, k: K, v: V) {
        self.insert_with_ttl(k, v, self.default_ttl);
    }

    pub fn insert_with_ttl(&self, k: K, v: V, ttl: Duration) {
        self.inner
            .lock()
            .unwrap()
            .insert(k, (v, Instant::now() + ttl));
    }

    pub fn get(&self, k: &K) -> Option<V>
    where
        V: Clone,
    {
        let map = self.inner.lock().unwrap();
        map.get(k)
            .filter(|(_, exp)| *exp > Instant::now())
            .map(|(v, _)| v.clone())
    }

    pub fn remove(&self, k: &K) {
        self.inner.lock().unwrap().remove(k);
    }
}
