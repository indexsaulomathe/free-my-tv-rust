//! The calls `web/logic.js` makes. Strings go in as (pointer, length) of UTF-8 the caller wrote with `alloc`, and
//! come out in one buffer (`out_ptr`, `out_len`). A call answers 1 when the buffer holds its answer, 0 when the
//! JavaScript should do it instead.

use std::cell::RefCell;

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn answer(text: &str) -> i32 {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        o.clear();
        o.extend_from_slice(text.as_bytes());
    });
    1
}

/// The caller's string; None when it isn't UTF-8.
///
/// # Safety
/// `ptr` and `len` must be a block `alloc` gave, that the caller wrote.
unsafe fn arg<'a>(ptr: *const u8, len: usize) -> Option<&'a str> {
    if len == 0 {
        return Some("");
    }
    std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).ok()
}

/// A block for the caller to write a string into.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `ptr` and `len` must be a block `alloc(len)` gave, freed once.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

#[no_mangle]
pub extern "C" fn out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn weather(ptr: *const u8, len: usize, now: f64) -> i32 {
    let Some(o) = arg(ptr, len).and_then(|t| serde_json::from_str(t).ok()) else {
        return 0;
    };
    match crate::parse_weather(&o, now) {
        Some(w) => answer(&w.to_string()),
        None => 0,
    }
}

/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn backdrop(ptr: *const u8, len: usize) -> i32 {
    let Some(o) = arg(ptr, len).and_then(|t| serde_json::from_str(t).ok()) else {
        return 0;
    };
    match crate::best_backdrop(&o) {
        Some(b) => answer(&b.to_string()),
        None => 0,
    }
}

/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn local_time(ptr: *const u8, len: usize, offset: f64) -> f64 {
    arg(ptr, len).map_or(0.0, |s| crate::local_time(s, offset))
}

#[no_mangle]
pub extern "C" fn condition(code: f64, day: i32) -> i32 {
    answer(crate::condition(code, day != 0))
}

#[no_mangle]
pub extern "C" fn symbol(code: f64, day: i32) -> i32 {
    answer(crate::symbol(code, day != 0))
}

/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn franchise(ptr: *const u8, len: usize) -> i32 {
    arg(ptr, len).map_or(0, |s| answer(&crate::franchise(s)))
}

/// The index in SERVICES, -1 for none, -2 to leave it to the JavaScript.
///
/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn service(ptr: *const u8, len: usize, id: f64) -> i32 {
    match arg(ptr, len) {
        Some(s) => crate::service(s, id).map_or(-1, |i| i as i32),
        None => -2,
    }
}

/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn plural_category(ptr: *const u8, len: usize, n: f64) -> i32 {
    arg(ptr, len).map_or(0, |lang| answer(crate::plural_category(lang, n)))
}

/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn plural_form(
    forms: *const u8,
    forms_len: usize,
    category: *const u8,
    category_len: usize,
    count: *const u8,
    count_len: usize,
) -> i32 {
    match (
        arg(forms, forms_len),
        arg(category, category_len),
        arg(count, count_len),
    ) {
        (Some(f), Some(c), Some(n)) => answer(&crate::plural_form(f, c, n)),
        _ => 0,
    }
}

/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn base(ptr: *const u8, len: usize) -> i32 {
    arg(ptr, len).map_or(0, |s| answer(&crate::base(s)))
}

/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn locale_of(
    code: *const u8,
    code_len: usize,
    tv: *const u8,
    tv_len: usize,
) -> i32 {
    match (arg(code, code_len), arg(tv, tv_len)) {
        (Some(c), Some(t)) => answer(&crate::locale_of(c, t)),
        _ => 0,
    }
}

/// `languages`: the codes, one per line.
///
/// # Safety
/// See [`arg`].
#[no_mangle]
pub unsafe extern "C" fn resolve(
    setting: *const u8,
    setting_len: usize,
    tv: *const u8,
    tv_len: usize,
    languages: *const u8,
    languages_len: usize,
) -> i32 {
    match (
        arg(setting, setting_len),
        arg(tv, tv_len),
        arg(languages, languages_len),
    ) {
        (Some(s), Some(t), Some(l)) => {
            let list: Vec<&str> = l.split('\n').filter(|x| !x.is_empty()).collect();
            answer(&crate::resolve(s, t, &list))
        }
        _ => 0,
    }
}
