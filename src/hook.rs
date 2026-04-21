use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

type HookFn<E> = Box<dyn Fn(E) + Send + Sync>;
type BoxedHook = Arc<dyn Any + Send + Sync>;

static HOOKS: LazyLock<Mutex<HashMap<TypeId, Vec<BoxedHook>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub trait Event: Any + Clone + Send + Sync + 'static {
    fn fire(self)
    where
        Self: Sized,
    {
        call(self);
    }
}

pub fn add<E: Event>(f: impl Fn(E) + Send + Sync + 'static) {
    let mut hooks = HOOKS.lock().unwrap();
    let entry = hooks.entry(TypeId::of::<E>()).or_default();
    let boxed: HookFn<E> = Box::new(f);
    entry.push(Arc::new(boxed));
}

pub fn add_async<E, F, Fut>(f: F)
where
    E: Event,
    F: Fn(E) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    add(move |e: E| {
        let fut = f(e);
        crate::tokio_handle().spawn(fut);
    });
}

fn call<E: Event>(event: E) {
    // Snapshot under the lock, then release before dispatching.
    let snapshot: Vec<Arc<HookFn<E>>> = {
        let hooks = HOOKS.lock().unwrap();
        hooks
            .get(&TypeId::of::<E>())
            .map(|list| {
                list.iter()
                    .filter_map(|h| Arc::clone(h).downcast::<HookFn<E>>().ok())
                    .collect()
            })
            .unwrap_or_default()
    }; // lock released here

    let mut iter = snapshot.into_iter().peekable();
    while let Some(f) = iter.next() {
        if iter.peek().is_some() {
            f(event.clone());
        } else {
            f(event);
            return;
        }
    }
}
