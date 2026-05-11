#![allow(clippy::arc_with_non_send_sync)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::policy::RuntimeSelector;
use destack_engine as engine;
use destack_workspace::RuntimeOptions;

use crate::diagnostic::RuntimeError;
use crate::host::Session;
use crate::runtime::binding::{
    BindingAffinity, BindingDescriptor, BindingEffect, BindingProvider, BindingReplayKind,
    BindingReplayPayload,
};
use crate::runtime::policy::{CustomAction, Rule, Trigger};
use crate::runtime::{Hook, HookDecision, HookSelector, Worker, WorkerOptions, World};

/// Ensures before-binding callbacks can deny one matching binding call.
#[test]
fn test_on_before_binding_allows_hook_callback_deny() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world = World::from_options(&options).expect("hook test world should build");
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let mut worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &mut world.state,
        &shared,
        &engine::StaticSpace::empty(),
        WorkerOptions::default(),
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

    let descriptor = BindingDescriptor::new(
        "destack.test.hook.block",
        "()",
        BindingEffect::Pure,
        BindingReplayKind::BindingCall,
        BindingReplayPayload::Results,
        &[],
        BindingProvider::Runtime,
        BindingAffinity::None,
    );

    // matching binding calls should fail with the callback message
    let error = {
        let world_state = &mut world.state;
        let call_context = super::tests::binding_call_context(&mut worker, &host, world_state);

        call_context
            .on_before_binding(descriptor)
            .expect_err("matching call should be denied")
    };
    assert!(matches!(
        error.as_ref(),
        RuntimeError::Internal { message } if message == "blocked by callback"
    ));

    // unregistering the callback should restore allow behavior
    assert!(worker.hooks.off(callback_id));
    let is_allowed = {
        let world_state = &mut world.state;
        let call_context = super::tests::binding_call_context(&mut worker, &host, world_state);

        call_context.on_before_binding(descriptor).is_ok()
    };
    assert!(is_allowed);
}

/// Ensures callback selectors only apply to matching binding names.
#[test]
fn test_on_before_binding_respects_hook_selector_binding_glob() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world = World::from_options(&options).expect("hook test world should build");
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let mut worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &mut world.state,
        &shared,
        &engine::StaticSpace::empty(),
        WorkerOptions::default(),
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

    let descriptor = BindingDescriptor::new(
        "destack.test.hook.allowed",
        "()",
        BindingEffect::Pure,
        BindingReplayKind::BindingCall,
        BindingReplayPayload::Results,
        &[],
        BindingProvider::Runtime,
        BindingAffinity::None,
    );

    // non-matching binding calls should continue normally
    let is_allowed = {
        let world_state = &mut world.state;
        let call_context = super::tests::binding_call_context(&mut worker, &host, world_state);

        call_context.on_before_binding(descriptor).is_ok()
    };
    assert!(is_allowed);

    // unregister should remove exactly one callback
    assert!(worker.hooks.off(callback_id));
}

/// Ensures custom-action handlers run for matching runtime rules.
#[test]
fn test_on_before_binding_runs_custom_action_handler() {
    // create one worker in one shared world
    let options = RuntimeOptions::default();
    let mut world = World::from_options(&options).expect("hook test world should build");
    let shared = super::tests::runtime_shared_heap(&world, &options);
    let mut worker = Worker::new_in_world(
        Vec::new(),
        &options,
        &mut world.state,
        &shared,
        &engine::StaticSpace::empty(),
        WorkerOptions::default(),
        super::tests::TestEngine::default(),
    )
    .expect("worker should construct in world");
    let host = Session::from_runtime_options(&options, worker.runtime_id);

    // install one custom-action rule for one binding pattern
    world
        .install_rule(
            Rule::custom(
                "test.hook.custom.action",
                CustomAction::new("test.custom.handler").payload("{\"mock\":true}"),
                Trigger::once(Hook::BindingBefore),
            )
            .when(RuntimeSelector::binding("destack.test.hook.custom.*")),
        )
        .expect("custom rule should install");

    // register one custom action handler callback
    let invocations = Arc::new(AtomicU64::new(0));
    let invocations_for_handler = invocations.clone();
    worker
        .hooks
        .on_custom_action("test.custom.handler", move |invocation| {
            assert_eq!(invocation.rule_id, "test.hook.custom.action");
            assert_eq!(invocation.handler, "test.custom.handler");
            assert_eq!(invocation.payload.as_deref(), Some("{\"mock\":true}"));
            invocations_for_handler.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });

    // matching binding call should run one custom action invocation
    let descriptor = BindingDescriptor::new(
        "destack.test.hook.custom.call",
        "()",
        BindingEffect::Pure,
        BindingReplayKind::BindingCall,
        BindingReplayPayload::Results,
        &[],
        BindingProvider::Runtime,
        BindingAffinity::None,
    );
    let is_allowed = {
        let world_state = &mut world.state;
        let call_context = super::tests::binding_call_context(&mut worker, &host, world_state);

        call_context.on_before_binding(descriptor).is_ok()
    };
    assert!(is_allowed);
    assert_eq!(invocations.load(Ordering::Relaxed), 1);
}
