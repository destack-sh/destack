use std::sync::Arc;

use destack_workspace::RuntimeOptions;

use crate::diagnostic::RuntimeError;
use crate::platform::PlatformContext;
use crate::runtime::bindings::BindingDescriptor;
use crate::runtime::{Agent, BindingCallContext, Hook, HookDecision, HookSelector, World};

/// Ensures before-binding callbacks can deny one matching binding call.
#[test]
fn test_on_before_binding_allows_hook_callback_deny() {
    // create one agent in one shared world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");

    // register one deny callback for matching binding names
    let callback_id = agent.hooks.on(
        Hook::BindingBefore,
        HookSelector {
            binding: Some("destack.test.hook.*".to_string()),
        },
        Arc::new(|_event| HookDecision::Deny {
            message: "blocked by callback".to_string(),
        }),
    );

    // matching binding calls should fail with the callback message
    let call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        agent.bindings.policy_snapshot(),
    );
    let descriptor = BindingDescriptor::pure("destack.test.hook.block", "()");
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_err());
    let error = result.expect_err("matching call should be denied");
    assert!(matches!(
        error.as_ref(),
        RuntimeError::Internal { message } if message == "blocked by callback"
    ));

    // unregistering the callback should restore allow behavior
    assert!(agent.hooks.off(callback_id));
    let call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        agent.bindings.policy_snapshot(),
    );
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_ok());
}

/// Ensures callback selectors only apply to matching binding names.
#[test]
fn test_on_before_binding_respects_hook_selector_binding_glob() {
    // create one agent in one shared world
    let options = RuntimeOptions::default();
    let world = Arc::new(World::default());
    let agent =
        Agent::from_options_in_world(PlatformContext::new(Vec::new()), &options, world.clone())
            .expect("agent should construct in world");

    // register one deny callback with one non-matching binding pattern
    let callback_id = agent.hooks.on(
        Hook::BindingBefore,
        HookSelector {
            binding: Some("destack.test.hook.nonmatching.*".to_string()),
        },
        Arc::new(|_event| HookDecision::Deny {
            message: "blocked by callback".to_string(),
        }),
    );

    // non-matching binding calls should continue normally
    let call_context = BindingCallContext::new(
        &agent,
        agent.event_loop.as_ref(),
        agent.bindings.policy_snapshot(),
    );
    let descriptor = BindingDescriptor::pure("destack.test.hook.allowed", "()");
    let result = call_context.on_before_binding(descriptor);
    assert!(result.is_ok());

    // unregister should remove exactly one callback
    assert!(agent.hooks.off(callback_id));
}
