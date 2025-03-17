use std::sync::{Arc, Mutex, OnceLock};

use super::values::RuntimeValue;

pub static OBJECT_STORE: OnceLock<Mutex<Vec<Option<Arc<RuntimeValue>>>>> = OnceLock::new();
pub static FREE_LIST: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();

pub fn initialise_store() {
    OBJECT_STORE.get_or_init(|| Mutex::new(vec![]));
    FREE_LIST.get_or_init(|| Mutex::new(vec![]));
}

pub fn get_lock<'a>() -> std::sync::MutexGuard<'a, Vec<Option<Arc<RuntimeValue>>>> {
    OBJECT_STORE.get().unwrap().lock().unwrap()
}

pub fn allocate(value: RuntimeValue) -> usize {
    let mut store = get_lock();
    let mut free_list = FREE_LIST.get().unwrap().lock().unwrap();

    if let Some(idx) = free_list.pop() {
        store[idx] = Some(Arc::from(value));
        idx
    } else {
        store.push(Some(Arc::from(value)));
        store.len() - 1
    }
}

pub fn deallocate(index: usize) -> () {
    let mut store = get_lock();
    let mut free_list = FREE_LIST.get().unwrap().lock().unwrap();

    if let Some(slot) = store.get_mut(index) {
        *slot = None;
        free_list.push(index);
    }
}

pub fn store_get(index: usize) -> RuntimeValue {
    get_lock()
        .get(index)
        .unwrap_or_else(|| panic!("Object {} does not exist", index))
        .as_ref()
        .unwrap()
        .as_ref()
        .clone()
}

pub fn store_set(index: usize, value: RuntimeValue) -> () {
    let mut lock = get_lock();
    lock[index] = Some(Arc::from(value));
}

#[cfg(test)]
mod tests {
    use crate::runtime::values::{self, RuntimeValueUtils};

    #[test]
    fn main() {
        // Allocate
        super::initialise_store();
        let index = super::allocate(values::Null::new().wrap());
        assert_eq!(index, 0);

        // Deallocate
        super::deallocate(index);

        // Reallocate
        let new = super::allocate(values::Null::new().wrap());
        assert_eq!(new, 0);
    }
}
