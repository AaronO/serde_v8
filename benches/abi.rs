#![feature(test)]

use rusty_v8 as v8;
use serde_v8;

use serde_v8::utils::{v8_do, BasicRuntime};

extern crate test;
use test::Bencher;

fn bench_js(b: &mut Bencher, src: &str) {
    v8_do(|| {
        let mut runtime = BasicRuntime::new();
        let context = runtime.global_context();
        let scope = &mut v8::HandleScope::with_context(runtime.v8_isolate(), context);

        let code = v8::String::new(scope, src).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();

        b.iter(|| {
            script.run(scope).unwrap();
        })
    })
}

fn bench_js_loop(b: &mut Bencher, count: u32, src: &str) {
    let src = format!("for(let i = 0; i < {}; i++) ({})", count, src);
    bench_js(b, &src[..]);
}

#[bench]
fn abi_add_v8(b: &mut Bencher) {
    bench_js_loop(b, 100, "add(with_v8, 123, 321)");
}

#[bench]
fn abi_add_json(b: &mut Bencher) {
    bench_js_loop(b, 100, "add(with_json, 123, 321)");
}

#[bench]
fn abi_promote_v8(b: &mut Bencher) {
    bench_js_loop(b, 100, "promote(with_v8, 'Aaron', 'O Mullan', 27)");
}

#[bench]
fn abi_promote_json(b: &mut Bencher) {
    bench_js_loop(b, 100, "promote(with_json, 'Aaron', 'O Mullan', 27)");
}

#[bench]
fn abi_sum_v8(b: &mut Bencher) {
    bench_js_loop(b, 100, "sum(with_v8, [1,2,3,4,5])");
}

#[bench]
fn abi_sum_json(b: &mut Bencher) {
    bench_js_loop(b, 100, "sum(with_json, [1,2,3,4,5])");
}
