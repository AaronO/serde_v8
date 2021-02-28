use rusty_v8 as v8;

use crate::utils::bindings;

pub fn js_exec<'s>(scope: &mut v8::HandleScope<'s>, src: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, src).unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    script.run(scope).unwrap()
}

pub struct BasicRuntime {
    isolate: Option<v8::OwnedIsolate>,
    context: Option<v8::Global<v8::Context>>,
}

impl BasicRuntime {
    pub fn new() -> Self {
        let mut isolate = v8::Isolate::new(v8::CreateParams::default());
        let global_context = {
            let scope = &mut v8::HandleScope::new(&mut isolate);
            let context = bindings::bindings_init(scope);
            bindings::bindings_init(scope);
            v8::Global::new(scope, context)
        };
        let mut runtime = BasicRuntime {
            isolate: Some(isolate),
            context: Some(global_context),
        };

        runtime.js_init();

        runtime
    }
    pub fn scope(&mut self) -> v8::HandleScope {
        let context = self.global_context();
        let scope = v8::HandleScope::with_context(self.v8_isolate(), context);
        scope
    }
    pub fn exec(&mut self, src: &str) -> v8::Local<v8::Value> {
        let context = self.global_context();
        let scope = &mut v8::HandleScope::with_context(self.v8_isolate(), context);
        js_exec(scope, src)
    }
    pub fn global_context(&mut self) -> v8::Global<v8::Context> {
        self.context.clone().unwrap()
    }
    pub fn v8_isolate(&mut self) -> &mut v8::OwnedIsolate {
        self.isolate.as_mut().unwrap()
    }
    pub fn js_init(&mut self) {
        self.exec(include_str!("core.js"));
    }
}
