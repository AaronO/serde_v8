use rusty_v8 as v8;
use std::sync::Once;

pub fn v8_init() {
    let platform = v8::new_default_platform().unwrap();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
}

pub fn v8_shutdown() {
    unsafe {
        v8::V8::dispose();
    }
    v8::V8::shutdown_platform();
}

pub fn v8_do(f: impl FnOnce()) {
    static V8_INIT: Once = Once::new();
    V8_INIT.call_once(|| {
        v8_init();
    });
    f();
    // v8_shutdown();
}
