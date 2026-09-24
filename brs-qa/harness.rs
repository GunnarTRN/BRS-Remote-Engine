#![allow(non_snake_case, non_camel_case_types, dead_code)]
use std::{cell::RefCell, io, sync::Mutex};
type BOOL = i32;
const FALSE: BOOL = 0;
const HKEY_LOCAL_MACHINE: u32 = 0;
const KEY_READ: u32 = 1;
const KEY_WRITE: u32 = 2;
const ERROR_FILE_NOT_FOUND: u32 = 2;
#[derive(Default)]
struct State {
    value: Option<u32>, open_error: bool, read_error: Option<i32>,
    verify_error: bool, write_error: bool, restore_error: bool,
    delete_error: bool, external: Option<Option<u32>>,
    calls: usize, writes: Vec<u32>, deletes: usize, logs: Vec<String>,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }
#[macro_export]
macro_rules! record_log { ($($arg:tt)*) => { STATE.with(|s| s.borrow_mut().logs.push(format!($($arg)*))) }; }
mod log { pub use crate::record_log as error; pub use crate::record_log as info; }
struct RegKey;
trait FromU32 { fn from_u32(value: u32) -> Self; }
impl FromU32 for u32 { fn from_u32(value: u32) -> Self { value } }
impl RegKey {
    fn predef(_: u32) -> Self { Self }
    fn open_subkey_with_flags(&self, _: &str, _: u32) -> io::Result<Self> {
        STATE.with(|s| if s.borrow().open_error { Err(io::Error::from_raw_os_error(5)) } else { Ok(Self) })
    }
    fn get_value<T: FromU32, P: AsRef<str>>(&self, _: P) -> io::Result<T> {
        STATE.with(|s| {
            let s = s.borrow();
            if s.calls > 0 && s.verify_error { return Err(io::Error::from_raw_os_error(5)); }
            if let Some(code) = s.read_error { return Err(io::Error::from_raw_os_error(code)); }
            s.value.map(T::from_u32).ok_or_else(|| io::Error::from_raw_os_error(2))
        })
    }
    fn set_value(&self, _: &str, value: &u32) -> io::Result<()> {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            if s.write_error || (s.calls > 0 && s.restore_error) { return Err(io::Error::from_raw_os_error(5)); }
            s.value = Some(*value); s.writes.push(*value); Ok(())
        })
    }
    fn delete_value(&self, _: &str) -> io::Result<()> {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            if s.delete_error { return Err(io::Error::from_raw_os_error(5)); }
            s.value = None; s.deletes += 1; Ok(())
        })
    }
}
unsafe fn SendSAS(_: BOOL) {
    STATE.with(|s| {
        let mut s = s.borrow_mut(); s.calls += 1;
        if let Some(value) = s.external { s.value = value; }
    });
}
// FUNCTION_UNDER_TEST
fn run(state: State, check: impl FnOnce(&State)) {
    STATE.with(|s| *s.borrow_mut() = state);
    send_sas();
    STATE.with(|s| check(&s.borrow()));
}
#[test] fn explicit_zero_is_restored() {
    run(State { value: Some(0), ..State::default() }, |s| {
        assert_eq!(s.value, Some(0)); assert_eq!(s.writes, vec![1,0]); assert_eq!(s.deletes,0); assert_eq!(s.calls,1);
    });
}
#[test] fn missing_is_removed_afterward() {
    run(State::default(), |s| { assert_eq!(s.value,None); assert_eq!(s.writes,vec![1]); assert_eq!(s.deletes,1); assert_eq!(s.calls,1); });
}
#[test] fn value_two_is_restored() {
    run(State { value: Some(2), ..State::default() }, |s| { assert_eq!(s.value,Some(2)); assert_eq!(s.writes,vec![1,2]); assert_eq!(s.calls,1); });
}
#[test] fn allowed_values_are_not_written() {
    for value in [1,3] { run(State { value: Some(value), ..State::default() }, |s| { assert_eq!(s.value,Some(value)); assert!(s.writes.is_empty()); assert_eq!(s.deletes,0); assert_eq!(s.calls,1); }); }
}
#[test] fn read_errors_do_not_send_or_write() {
    // Access denied, invalid type/data, and a path error are not a missing value.
    for error in [5,13,3] { run(State { value: Some(0), read_error: Some(error), ..State::default() }, |s| { assert_eq!(s.calls,0); assert!(s.writes.is_empty()); assert_eq!(s.deletes,0); assert_eq!(s.value,Some(0)); assert!(!s.logs.is_empty()); }); }
}
#[test] fn open_error_aborts() {
    run(State { open_error: true, ..State::default() }, |s| { assert_eq!(s.calls,0); assert!(s.writes.is_empty()); assert!(!s.logs.is_empty()); });
}
#[test] fn write_error_aborts() {
    run(State { value: Some(0), write_error: true, ..State::default() }, |s| { assert_eq!(s.calls,0); assert_eq!(s.value,Some(0)); assert_eq!(s.deletes,0); assert!(!s.logs.is_empty()); });
}
#[test] fn restore_error_is_reported() {
    run(State { value: Some(0), restore_error: true, ..State::default() }, |s| { assert_eq!(s.calls,1); assert_eq!(s.value,Some(1)); assert!(s.logs.iter().any(|x| x.contains("Failed to restore"))); });
}
#[test] fn delete_error_is_reported() {
    run(State { delete_error: true, ..State::default() }, |s| { assert_eq!(s.calls,1); assert_eq!(s.value,Some(1)); assert!(s.logs.iter().any(|x| x.contains("Failed to restore"))); });
}
#[test] fn external_value_is_preserved() {
    run(State { value: Some(0), external: Some(Some(3)), ..State::default() }, |s| { assert_eq!(s.calls,1); assert_eq!(s.value,Some(3)); assert_eq!(s.writes,vec![1]); assert!(s.logs.iter().any(|x| x.contains("changed externally"))); });
}
#[test] fn external_deletion_is_preserved() {
    run(State { value: Some(0), external: Some(None), ..State::default() }, |s| { assert_eq!(s.value,None); assert_eq!(s.writes,vec![1]); assert!(s.logs.iter().any(|x| x.contains("Cannot verify"))); });
}
#[test] fn verification_error_does_not_overwrite() {
    run(State { value: Some(0), verify_error: true, ..State::default() }, |s| { assert_eq!(s.value,Some(1)); assert_eq!(s.writes,vec![1]); assert!(s.logs.iter().any(|x| x.contains("Cannot verify"))); });
}
