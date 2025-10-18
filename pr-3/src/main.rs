use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

// ──────────────────────────────────────────────────────────────────────────────
// Given abstractions (DO NOT MODIFY)
// ──────────────────────────────────────────────────────────────────────────────

trait Storage<K, V> {
    fn set(&mut self, key: K, val: V);
    fn get(&self, key: &K) -> Option<&V>;
    fn remove(&mut self, key: &K) -> Option<V>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct User {
    id: u64,
    email: Cow<'static, str>,
    activated: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// HashMap-based Storage
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct HashMapStorage<K, V>(HashMap<K, V>);

impl<K, V> HashMapStorage<K, V> {
    fn new() -> Self {
        Self(HashMap::new())
    }
}

impl<K, V> Storage<K, V> for HashMapStorage<K, V>
where
    K: Eq + Hash,
{
    fn set(&mut self, key: K, val: V) {
        self.0.insert(key, val);
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.0.remove(key)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// UserRepository (STATIC dispatch via generics)
// ──────────────────────────────────────────────────────────────────────────────

struct UserRepositoryStatic<S>
where
    S: Storage<u64, User>,
{
    storage: S,
}

impl<S> UserRepositoryStatic<S>
where
    S: Storage<u64, User>,
{
    fn new(storage: S) -> Self {
        Self { storage }
    }

    fn get(&self, id: u64) -> Option<&User> {
        self.storage.get(&id)
    }

    fn add(&mut self, user: User) -> bool {
        if self.storage.get(&user.id).is_some() {
            return false;
        }
        self.storage.set(user.id, user);
        true
    }

    fn update(&mut self, user: User) -> bool {
        if self.storage.get(&user.id).is_none() {
            return false;
        }
        self.storage.set(user.id, user);
        true
    }

    fn remove(&mut self, id: u64) -> Option<User> {
        self.storage.remove(&id)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// UserRepository (DYNAMIC dispatch via trait object)
// ──────────────────────────────────────────────────────────────────────────────

struct UserRepositoryDynamic {
    storage: Box<dyn Storage<u64, User>>,
}

impl UserRepositoryDynamic {
    fn new(storage: Box<dyn Storage<u64, User>>) -> Self {
        Self { storage }
    }

    fn get(&self, id: u64) -> Option<&User> {
        self.storage.get(&id)
    }

    fn add(&mut self, user: User) -> bool {
        if self.storage.get(&user.id).is_some() {
            return false;
        }
        self.storage.set(user.id, user);
        true
    }

    fn update(&mut self, user: User) -> bool {
        if self.storage.get(&user.id).is_none() {
            return false;
        }
        self.storage.set(user.id, user);
        true
    }

    fn remove(&mut self, id: u64) -> Option<User> {
        self.storage.remove(&id)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Demo main
// ──────────────────────────────────────────────────────────────────────────────

fn main() {
    let mut repo = UserRepositoryStatic::new(HashMapStorage::<u64, User>::new());
    let _ = repo.add(User {
        id: 1,
        email: Cow::Borrowed("user@example.com"),
        activated: true,
    });
    if let Some(u) = repo.get(1) {
        println!("Found user: {u:?}");
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_user(id: u64, email: &'static str, activated: bool) -> User {
        User {
            id,
            email: Cow::Borrowed(email),
            activated,
        }
    }

    #[test]
    fn static_repo_add_get_update_remove() {
        let mut repo = UserRepositoryStatic::new(HashMapStorage::<u64, User>::new());

        assert!(repo.add(mk_user(1, "a@ex", false)));
        assert!(!repo.add(mk_user(1, "dup@ex", true)));

        let u = repo.get(1).expect("user should exist");
        assert_eq!(u.email, "a@ex");
        assert!(!u.activated);

        assert!(repo.update(mk_user(1, "new@ex", true)));
        let u = repo.get(1).unwrap();
        assert_eq!(u.email, "new@ex");
        assert!(u.activated);

        assert!(!repo.update(mk_user(2, "nope@ex", false)));

        let removed = repo.remove(1).expect("remove should return user");
        assert_eq!(removed.email, "new@ex");
        assert!(repo.get(1).is_none());
    }

    #[test]
    fn dynamic_repo_add_get_update_remove() {
        let storage: Box<dyn Storage<u64, User>> = Box::new(HashMapStorage::<u64, User>::new());
        let mut repo = UserRepositoryDynamic::new(storage);

        assert!(repo.add(mk_user(42, "x@ex", true)));
        assert!(repo.get(42).is_some());

        assert!(repo.update(mk_user(42, "y@ex", false)));
        assert_eq!(repo.get(42).unwrap().email, "y@ex");
        assert!(!repo.get(42).unwrap().activated);

        let removed = repo.remove(42).unwrap();
        assert_eq!(removed.id, 42);
        assert!(repo.get(42).is_none());
    }

    // ── Injectable proof: custom storage with counters ──
    struct TracingStorage {
        inner: HashMapStorage<u64, User>,
        sets: usize,
        removes: usize,
    }

    impl Default for TracingStorage {
        fn default() -> Self {
            Self {
                inner: HashMapStorage::new(),
                sets: 0,
                removes: 0,
            }
        }
    }

    impl Storage<u64, User> for TracingStorage {
        fn set(&mut self, key: u64, val: User) {
            self.sets += 1;
            self.inner.set(key, val);
        }
        fn get(&self, key: &u64) -> Option<&User> {
            self.inner.get(key)
        }
        fn remove(&mut self, key: &u64) -> Option<User> {
            self.removes += 1;
            self.inner.remove(key)
        }
    }

    // Рахуємо get виклики через thread_local, не змінюючи сигнатуру трейт-методу (&self)
    struct TracingStorageWithGetCount(TracingStorage);

    impl Storage<u64, User> for TracingStorageWithGetCount {
        fn set(&mut self, key: u64, val: User) {
            self.0.set(key, val);
        }
        fn get(&self, key: &u64) -> Option<&User> {
            GETS_COUNT.with(|c| c.set(c.get() + 1));
            self.0.get(key)
        }
        fn remove(&mut self, key: &u64) -> Option<User> {
            self.0.remove(key)
        }
    }

    thread_local! {
        static GETS_COUNT: std::cell::Cell<usize> = std::cell::Cell::new(0);
    }

    #[test]
    fn static_repo_is_injectable_with_custom_storage() {
        let tracing = TracingStorage::default();
        let mut repo = UserRepositoryStatic::new(tracing);

        assert!(repo.add(mk_user(7, "t@ex", true)));
        assert!(repo.update(mk_user(7, "t2@ex", true)));
        let _ = repo.remove(7);

        // зчитуємо лічильники з полів сховища
        let TracingStorage { sets, removes, .. } = repo.storage;
        assert_eq!(sets, 2, "one add + one update");
        assert_eq!(removes, 1, "one remove");
    }

    #[test]
    fn dynamic_repo_is_injectable_with_custom_storage() {
        GETS_COUNT.with(|c| c.set(0));
        let storage: Box<dyn Storage<u64, User>> =
            Box::new(TracingStorageWithGetCount(TracingStorage::default()));
        let mut repo = UserRepositoryDynamic::new(storage);

        assert!(repo.add(mk_user(9, "g@ex", false)));
        let _ = repo.get(9);                          
        let _ = repo.remove(9);

        let gets = GETS_COUNT.with(|c| c.get());
        assert_eq!(gets, 2, "one pre-insert existence check + one explicit get");
    }
}
