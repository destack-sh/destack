#[cfg(test)]
mod tests {
    use destack_javascript_machine::v8;

    #[test]
    fn test_v8() {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let isolate = &mut v8::Isolate::new(Default::default());

        let scope = std::pin::pin!(v8::HandleScope::new(isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let code = v8::String::new(scope, "'Hello' + ' World!'").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();
        let result = result.to_string(scope).unwrap();
        assert_eq!(result.to_rust_string_lossy(scope), "Hello World!");
    }
}
