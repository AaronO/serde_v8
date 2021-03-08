#![feature(test)]

use rusty_v8 as v8;
use serde_json;
use serde_v8;

use serde::Serialize;

use serde_v8::utils::{v8_do, BasicRuntime};

extern crate test;
use test::Bencher;

#[derive(Serialize)]
struct MathOp {
    arg1: u64,
    arg2: u64,
    operator: Option<String>,
}

fn serdo(f: impl FnOnce(&mut v8::HandleScope)) {
    v8_do(|| {
        let mut runtime = BasicRuntime::new();
        let context = runtime.global_context();
        let scope = &mut v8::HandleScope::with_context(runtime.v8_isolate(), context);

        f(scope);
    })
}

macro_rules! dualbench {
    ($v8_fn:ident, $json_fn:ident, $src:expr) => {
        #[bench]
        fn $v8_fn(b: &mut Bencher) {
            serdo(|scope| {
                let v = $src;
                b.iter(move || {
                    let _ = serde_v8::to_v8(scope, &v).unwrap();
                });
            });
        }

        #[bench]
        fn $json_fn(b: &mut Bencher) {
            let v = $src;
            b.iter(move || {
                let _ = serde_json::to_string(&v).unwrap();
            });
        }
    };
}

dualbench!(ser_struct_v8, ser_struct_json, MathOp{arg1: 10, arg2: 123, operator: None});
dualbench!(ser_bool_v8, ser_bool_json, true);
dualbench!(ser_int_v8, ser_int_json, 12345);
dualbench!(ser_array_v8, ser_array_json, vec![1,2,3,4,5,6,7,8,9,10]);
dualbench!(ser_str_v8, ser_str_json, "hello world");
dualbench!(ser_tuple_v8, ser_tuple_json, (1,false));
