use web_sys::window;

/// Set a key-value pair in local storage
pub fn set(key: &str, value: &str) {
    if let Some(storage) = window().and_then(|win| win.local_storage().ok().flatten()) {
        let _ = storage.set_item(key, value);
    }
}

/// Get a value from local storage
pub fn get(key: &str) -> Option<String> {
    window()
        .and_then(|win| win.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

/// Delete a key from local storage
pub fn delete(key: &str) {
    if let Some(storage) = window().and_then(|win| win.local_storage().ok().flatten()) {
        let _ = storage.remove_item(key);
    }
}

/// Check if a key exists in local storage
pub fn exists(key: &str) -> bool {
    window()
        .and_then(|win| win.local_storage().ok().flatten())
        .map(|storage| storage.get_item(key).is_ok())
        .unwrap_or(false)
}

/// Clear all keys from local storage
pub fn clear() {
    if let Some(storage) = window().and_then(|win| win.local_storage().ok().flatten()) {
        let _ = storage.clear();
    }
}
