#![feature(test)]

use rusty_v8 as v8;
use serde_v8;

use serde_v8::utils::{v8_do, BasicRuntime, js_exec};

extern crate test;
use test::Bencher;

fn bench_js(b: &mut Bencher, setup_src: &str, loop_src: &str) {
    v8_do(|| {
        let mut runtime = BasicRuntime::new();
        let context = runtime.global_context();
        let scope = &mut v8::HandleScope::with_context(runtime.v8_isolate(), context);
        
        // Run setup code once
        js_exec(scope, setup_src);

        // Run loop code in the benchmark
        let code = v8::String::new(scope, loop_src).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        b.iter(|| {
            script.run(scope).unwrap();
        })
    })
}

fn bench_js_loop(b: &mut Bencher, count: u32, setup_src: &str, loop_src: &str) {
    let src = format!("for(let i = 0; i < {}; i++) ({})", count, loop_src);
    bench_js(b, setup_src, &src[..]);
}

// Utility macro to define simple ABI benches for multiple syscall types
macro_rules! multibench {
    ($jscode:expr, $iters:expr, $v8_fn:ident, $json_fn:ident, $void_fn:ident) => {
        #[bench]
        fn $v8_fn(b: &mut Bencher) {
            bench_js_loop(b, $iters, "var with_x = with_v8", $jscode);
        }

        #[bench]
        fn $json_fn(b: &mut Bencher) {
            bench_js_loop(b, $iters, "var with_x = with_json", $jscode);
        }
        
        #[bench]
        fn $void_fn(b: &mut Bencher) {
            bench_js_loop(b, $iters, "var with_x = with_void", $jscode);
        }
    };
}


multibench!("add(with_x, 123, 321)", 100, abi_add_v8, abi_add_json, abi_add_void);
multibench!("promote(with_x, 'Aaron', 'O Mullan', 27)", 100, abi_promote_v8, abi_promote_json, abi_promote_void);
multibench!("sum(with_x, [1,2,3,4,5])", 100, abi_sum_v8, abi_sum_json, abi_sum_void);
