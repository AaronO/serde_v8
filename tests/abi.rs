use rusty_v8 as v8;
use serde_v8;

use serde::Deserialize;

use serde_v8::utils::{js_exec, v8_do, BasicRuntime};

fn test_js<'s, T: Deserialize<'s> + Default>(src: &str) -> T {
    let mut output: T = T::default();
    v8_do(|| {
        let mut runtime = BasicRuntime::new();
        let context = runtime.global_context();
        let scope = &mut v8::HandleScope::with_context(runtime.v8_isolate(), context);

        let v = js_exec(scope, src);
        output = serde_v8::from_v8(scope, v).unwrap();
    });
    output
}

#[test]
fn abi_sum_v8() {
    assert_eq!(test_js::<u32>("sum(with_v8, [1,2,3,4,5])"), 15);
}

#[test]
fn abi_sum_json() {
    assert_eq!(test_js::<u32>("sum(with_json, [1,2,3,4,5])"), 15);
}
