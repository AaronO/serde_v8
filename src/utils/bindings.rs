use crate::de::from_v8;
use rusty_v8 as v8;

use std::cell::Cell;
use std::convert::TryFrom;

use serde_json;

use crate::utils::ops;
use crate::utils::zero_copy_buf::ZeroCopyBuf;

pub fn bindings_init<'s>(scope: &mut v8::HandleScope<'s, ()>) -> v8::Local<'s, v8::Context> {
    fn set_func(
        scope: &mut v8::HandleScope<'_>,
        obj: v8::Local<v8::Object>,
        name: &'static str,
        callback: impl v8::MapFnTo<v8::FunctionCallback>,
    ) {
        let key = v8::String::new(scope, name).unwrap();
        let tmpl = v8::FunctionTemplate::new(scope, callback);
        let val = tmpl.get_function(scope).unwrap();
        obj.set(scope, key.into(), val.into());
    }
    let scope = &mut v8::EscapableHandleScope::new(scope);
    let context = v8::Context::new(scope);
    let global = context.global(scope);

    let scope = &mut v8::ContextScope::new(scope, context);
    set_func(scope, global, "syscall_v8", syscall_v8);
    set_func(scope, global, "syscall_json", syscall_json);
    set_func(scope, global, "syscall_native", syscall_native);
    set_func(scope, global, "core_encode", encode);
    set_func(scope, global, "core_decode", decode);
    scope.escape(context)
}

fn syscall_v8<'s>(
    scope: &mut v8::HandleScope<'s>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let id: u32 = from_v8(scope, args.get(0)).unwrap();
    let arg = args.get(1);
    // Call ops
    // TODO: serialize return values
    if id == 1 {
        ops::sum(from_v8(scope, arg).unwrap());
    } else if id == 2 {
        ops::add(from_v8(scope, arg).unwrap());
    } else if id == 3 {
        ops::promote(from_v8(scope, arg).unwrap());
    }
    rv.set(v8::null(scope).into());
}

// A syscall system with no args or encoding
// to measure the baseline and upper-limit
fn syscall_native<'s>(
    scope: &mut v8::HandleScope<'s>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let id: u32 = from_v8(scope, args.get(0)).unwrap();
    let _arg = args.get(1);
    // Call ops
    // TODO: serialize return values
    if id == 1 {
        ops::sum(vec![1,2,3,4,5]);
    } else if id == 2 {
        ops::add(ops::AddArgs{a: 123, b: 321});
    } else if id == 3 {
        ops::promote(ops::Person{first_name: "Bill".to_owned(), last_name: "Gates".to_owned(), age: 60 });
    }
    rv.set(v8::null(scope).into());
}

fn syscall_json<'s>(
    scope: &mut v8::HandleScope<'s>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let id: u32 = from_v8(scope, args.get(0)).unwrap();
    let argbuf = v8::Local::<v8::ArrayBufferView>::try_from(args.get(1))
        .map(|view| ZeroCopyBuf::new(scope, view))
        .map_err(|err| {
            let msg = format!("Err with buf arg: {}", err);
            let msg = v8::String::new(scope, &msg).unwrap();
            v8::Exception::type_error(scope, msg)
        })
        .ok()
        .unwrap();

    // TODO: serialzie return values once serde_v8 can do it
    if id == 1 {
        ops::sum(serde_json::from_slice(&argbuf).unwrap());
    } else if id == 2 {
        ops::add(serde_json::from_slice(&argbuf).unwrap());
    } else if id == 3 {
        ops::promote(serde_json::from_slice(&argbuf).unwrap());
    }
    rv.set(v8::null(scope).into());

    // let retbuf = {
    //     if id == 1 {
    //         serde_json::to_vec(
    //             &ops::sum(serde_json::from_slice(&argbuf).unwrap())
    //         )
    //     } else if id == 2 {
    //         serde_json::to_vec(
    //             &ops::add(serde_json::from_slice(&argbuf).unwrap())
    //         )
    //     } else if id == 3 {
    //         serde_json::to_vec(
    //             &ops::promote(serde_json::from_slice(&argbuf).unwrap())
    //         )
    //     } else {
    //         serde_json::to_vec(&())
    //     }
    // }.map(Into::into).unwrap();
    // rv.set(boxed_slice_to_uint8array(scope, retbuf).into());
}

pub fn boxed_slice_to_uint8array<'sc>(
    scope: &mut v8::HandleScope<'sc>,
    buf: Box<[u8]>,
) -> v8::Local<'sc, v8::Uint8Array> {
    assert!(!buf.is_empty());
    let buf_len = buf.len();
    let backing_store = v8::ArrayBuffer::new_backing_store_from_boxed_slice(buf);
    let backing_store_shared = backing_store.make_shared();
    let ab = v8::ArrayBuffer::with_backing_store(scope, &backing_store_shared);
    v8::Uint8Array::new(scope, ab, 0, buf_len).expect("Failed to create UintArray8")
}

fn encode(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let text = match v8::Local::<v8::String>::try_from(args.get(0)) {
        Ok(s) => s,
        Err(_) => {
            throw_type_error(scope, "Invalid argument");
            return;
        }
    };
    let text_str = text.to_rust_string_lossy(scope);
    let text_bytes = text_str.as_bytes().to_vec().into_boxed_slice();

    let buf = if text_bytes.is_empty() {
        let ab = v8::ArrayBuffer::new(scope, 0);
        v8::Uint8Array::new(scope, ab, 0, 0).expect("Failed to create UintArray8")
    } else {
        let buf_len = text_bytes.len();
        let backing_store = v8::ArrayBuffer::new_backing_store_from_boxed_slice(text_bytes);
        let backing_store_shared = backing_store.make_shared();
        let ab = v8::ArrayBuffer::with_backing_store(scope, &backing_store_shared);
        v8::Uint8Array::new(scope, ab, 0, buf_len).expect("Failed to create UintArray8")
    };

    rv.set(buf.into())
}

fn decode(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let view = match v8::Local::<v8::ArrayBufferView>::try_from(args.get(0)) {
        Ok(view) => view,
        Err(_) => {
            throw_type_error(scope, "Invalid argument");
            return;
        }
    };

    let backing_store = view.buffer(scope).unwrap().get_backing_store();
    let buf =
        unsafe { get_backing_store_slice(&backing_store, view.byte_offset(), view.byte_length()) };

    // Strip BOM
    let buf = if buf.len() >= 3 && buf[0] == 0xef && buf[1] == 0xbb && buf[2] == 0xbf {
        &buf[3..]
    } else {
        buf
    };

    // If `String::new_from_utf8()` returns `None`, this means that the
    // length of the decoded string would be longer than what V8 can
    // handle. In this case we return `RangeError`.
    //
    // For more details see:
    // - https://encoding.spec.whatwg.org/#dom-textdecoder-decode
    // - https://github.com/denoland/deno/issues/6649
    // - https://github.com/v8/v8/blob/d68fb4733e39525f9ff0a9222107c02c28096e2a/include/v8.h#L3277-L3278
    match v8::String::new_from_utf8(scope, &buf, v8::NewStringType::Normal) {
        Some(text) => rv.set(text.into()),
        None => {
            let msg = v8::String::new(scope, "string too long").unwrap();
            let exception = v8::Exception::range_error(scope, msg);
            scope.throw_exception(exception);
        }
    };
}

pub(crate) unsafe fn get_backing_store_slice(
    backing_store: &v8::SharedRef<v8::BackingStore>,
    byte_offset: usize,
    byte_length: usize,
) -> &[u8] {
    let cells: *const [Cell<u8>] = &backing_store[byte_offset..byte_offset + byte_length];
    let bytes = cells as *const [u8];
    &*bytes
}

#[allow(clippy::mut_from_ref)]
pub(crate) unsafe fn get_backing_store_slice_mut(
    backing_store: &v8::SharedRef<v8::BackingStore>,
    byte_offset: usize,
    byte_length: usize,
) -> &mut [u8] {
    let cells: *const [Cell<u8>] = &backing_store[byte_offset..byte_offset + byte_length];
    let bytes = cells as *const _ as *mut [u8];
    &mut *bytes
}

fn throw_type_error(scope: &mut v8::HandleScope, message: impl AsRef<str>) {
    let message = v8::String::new(scope, message.as_ref()).unwrap();
    let exception = v8::Exception::type_error(scope, message);
    scope.throw_exception(exception);
}
