#![feature(test)]

use rusty_v8 as v8;
use serde_json;
use serde_v8;
use std::convert::TryFrom;

use serde::Deserialize;

use serde_v8::utils::{js_exec, v8_do, BasicRuntime};

extern crate test;
use test::Bencher;

#[derive(Debug, Deserialize, PartialEq)]
struct MathOp {
    arg1: u64,
    arg2: u64,
    operator: Option<String>,
}

fn dedo(code: &str, f: impl FnOnce(&mut v8::HandleScope, v8::Local<v8::Value>)) {
    v8_do(|| {
        let mut runtime = BasicRuntime::new();
        let context = runtime.global_context();
        let scope = &mut v8::HandleScope::with_context(runtime.v8_isolate(), context);
        let v = js_exec(scope, code);

        f(scope, v);
    })
}

fn dedo_json(code: &str, f: impl FnOnce(String)) {
    let code = format!("JSON.stringify({})", code);
    dedo(&code[..], |scope, v| {
        let s: String = serde_v8::from_v8(scope, v).unwrap();
        f(s);
    })
}

#[bench]
fn de_struct_v8(b: &mut Bencher) {
    dedo("({arg1: 10, arg2: 123 })", |scope, obj| {
        let mut cache = serde_v8::KeyCache::new();
        let mut total = 0;
        b.iter(move || {
            let op: MathOp = serde_v8::from_v8_cached(scope, obj, &mut cache).unwrap();
            total = total + op.arg1 + op.arg2;
        });
    });
}

#[bench]
fn de_struct_v8_opt(b: &mut Bencher) {
    dedo("({arg1: 10, arg2: 123 })", |scope, v| {
        let k_arg1 = v8::String::new(scope, "arg1").unwrap().into();
        let k_arg2 = v8::String::new(scope, "arg2").unwrap().into();
        let obj = v8::Local::<v8::Object>::try_from(v).unwrap();
        let mut total = 0;
        b.iter(move || {
            let v_arg1 = obj.get(scope, k_arg1).unwrap();
            let v_arg2 = obj.get(scope, k_arg2).unwrap();
            let op = MathOp {
                arg1: serde_v8::from_v8(scope, v_arg1).unwrap(),
                arg2: serde_v8::from_v8(scope, v_arg2).unwrap(),
                operator: None,
            };
            total = total + op.arg1 + op.arg2;
        });
    });
}

#[bench]
fn de_struct_json(b: &mut Bencher) {
    dedo_json("({arg1: 10, arg2: 123 })", |s| {
        let mut total = 0;
        b.iter(move || {
            let op: MathOp = serde_json::from_str(&s).unwrap();
            total = total + op.arg1 + op.arg2;
        });
    });
}

#[bench]
fn de_struct_json_deopt(b: &mut Bencher) {
    // JSON.stringify() in loop (semi-simulating ABI loop)
    dedo("({arg1: 10, arg2: 123 })", |scope, obj| {
        let mut total = 0;
        b.iter(move || {
            let mut scope = v8::HandleScope::new(scope);
            let s = v8::json::stringify(&mut scope, obj).unwrap();
            let rs = s.to_rust_string_lossy(&mut scope);
            let op: MathOp = serde_json::from_str(&rs).unwrap();
            total = total + op.arg1 + op.arg2;
        });
    });
}

macro_rules! dualbench {
    ($v8_fn:ident, $json_fn:ident, $src:expr, $t:ty) => {
        #[bench]
        fn $v8_fn(b: &mut Bencher) {
            dedo($src, |scope, v| {
                b.iter(move || {
                    let _: $t = serde_v8::from_v8(scope, v).unwrap();
                });
            });
        }

        #[bench]
        fn $json_fn(b: &mut Bencher) {
            dedo_json($src, |s| {
                b.iter(move || {
                    let _: $t = serde_json::from_str(&s).unwrap();
                });
            });
        }
    };
}

dualbench!(de_bool_v8, de_bool_json, "true", bool);
dualbench!(de_int_v8, de_int_json, "12345", u32);
dualbench!(
    de_array_v8,
    de_array_json,
    "[1,2,3,4,5,6,7,8,9,10]",
    Vec<u32>
);
dualbench!(de_str_v8, de_str_json, "'hello world'", String);
dualbench!(de_tuple_v8, de_tuple_json, "[1,false]", (u8, bool));

#[bench]
fn de_tuple_v8_opt(b: &mut Bencher) {
    dedo("[1,false]", |scope, obj| {
        let arr = v8::Local::<v8::Array>::try_from(obj).unwrap();
        let obj = v8::Local::<v8::Object>::from(arr);

        b.iter(move || {
            let v1 = obj.get_index(scope, 0).unwrap();
            let v2 = obj.get_index(scope, 1).unwrap();
            let _: (u8, bool) = (
                serde_v8::from_v8(scope, v1).unwrap(),
                serde_v8::from_v8(scope, v2).unwrap(),
            );
        });
    });
}
