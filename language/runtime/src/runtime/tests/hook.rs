#![allow(clippy::arc_with_non_send_sync)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_engine as engine;
use destack_workspace::{RuntimeOptions, RuntimeSelector};

use crate::diagnostic::RuntimeError;
use crate::host::Session;
use crate::runtime::bindings::BindingDescriptor;
use crate::runtime::policy::{CustomEffect, Rule, Trigger};
use crate::runtime::{Hook, HookDecision, HookSelector, Worker, World};

/// Ensures before-binding callbacks can deny one matching binding call.
#[test]
fn test_on_before_binding_allows_hook_callback_deny() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world = World::from_options(&options).expect("hook test world should build");
    let world_scope = world.world_scope();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_scope,
        &shared,
        &engine::StaticSpace::empty(),
        super::tests::TestEngine::default(),
    )
    .expect("worker should construct in world");
    let host = Session::from_runtime_options(&options, worker.runtime_id);

    // register one deny callback for matching binding names
    let callback_id =
        worker
            .hooks
            .on_before(HookSelector::binding("destack.test.hook.*"), |_event| {
                HookDecision::Deny {
                    message: "blocked by callback".to_string(),
                }
            });

    // matching binding calls should fail with the callback message
    let call_context = super::tests::binding_call_context(&worker, &host, &world_scope);
    let descriptor = BindingDescriptor::pure("destack.test.hook.block", "()");
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_err());
    let error = result.expect_err("matching call should be denied");
    assert!(matches!(
        error.as_ref(),
        RuntimeError::Internal { message } if message == "blocked by callback"
    ));

    // unregistering the callback should restore allow behavior
    assert!(worker.hooks.off(callback_id));
    let call_context = super::tests::binding_call_context(&worker, &host, &world_scope);
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_ok());
}

/// Ensures callback selectors only apply to matching binding names.
#[test]
fn test_on_before_binding_respects_hook_selector_binding_glob() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world = World::from_options(&options).expect("hook test world should build");
    let world_scope = world.world_scope();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_scope,
        &shared,
        &engine::StaticSpace::empty(),
        super::tests::TestEngine::default(),
    )
    .expect("worker should construct in world");
    let host = Session::from_runtime_options(&options, worker.runtime_id);

    // register one deny callback with one non-matching binding pattern
    let callback_id = worker.hooks.on_before(
        HookSelector::binding("destack.test.hook.nonmatching.*"),
        |_event| HookDecision::Deny {
            message: "blocked by callback".to_string(),
        },
    );

    // non-matching binding calls should continue normally
    let call_context = super::tests::binding_call_context(&worker, &host, &world_scope);
    let descriptor = BindingDescriptor::pure("destack.test.hook.allowed", "()");
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_ok());

    // unregister should remove exactly one callback
    assert!(worker.hooks.off(callback_id));
}

/// Ensures custom-effect handlers execute for matching runtime rules.
#[test]
fn test_on_before_binding_dispatches_custom_effect_handler() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world = World::from_options(&options).expect("hook test world should build");
    let world_scope = world.world_scope();
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &world_scope,
        &shared,
        &engine::StaticSpace::empty(),
        super::tests::TestEngine::default(),
    )
    .expect("worker should construct in world");
    let host = Session::from_runtime_options(&options, worker.runtime_id);

    // install one custom-effect rule for one binding pattern
    world
        .install_rule(
            Rule::custom(
                "test.hook.custom.effect",
                CustomEffect::new("test.custom.handler").payload("{\"mock\":true}"),
                Trigger::once(Hook::BindingBefore),
            )
            .when(RuntimeSelector::binding("destack.test.hook.custom.*")),
        )
        .expect("custom rule should install");

    // register one custom effect handler callback
    let invocations = Arc::new(AtomicU64::new(0));
    let invocations_for_handler = invocations.clone();
    worker
        .hooks
        .on_custom_effect("test.custom.handler", move |invocation| {
            assert_eq!(invocation.rule_id, "test.hook.custom.effect");
            assert_eq!(invocation.handler, "test.custom.handler");
            assert_eq!(invocation.payload.as_deref(), Some("{\"mock\":true}"));
            invocations_for_handler.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });

    // matching binding call should dispatch one custom effect invocation
    let call_context = super::tests::binding_call_context(&worker, &host, &world_scope);
    let descriptor = BindingDescriptor::pure("destack.test.hook.custom.call", "()");
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_ok());
    assert_eq!(invocations.load(Ordering::Relaxed), 1);
}
