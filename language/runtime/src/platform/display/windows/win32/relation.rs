use super::*;

/// Set one window modal state.
pub(crate) unsafe fn window_set_modal(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setModal",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setModal")?;

    // reject modal transitions without an owner relationship
    if modal && owner_relationship(&binding).is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // snapshot state before mutation, for event and rollback lanes
    let previous = binding.clone();
    let previous_owner = owner_relationship(&binding);
    let previous_modal = binding.modal;
    binding.modal = modal;
    let next_owner = owner_relationship(&binding);

    // apply host owner transition and rollback binding state on failure
    if let Err(error) = apply_modal_owner_transition(
        context,
        previous_owner,
        previous_modal,
        next_owner,
        binding.modal,
        "destack.display.window.setModal",
    ) {
        binding.modal = previous_modal;
        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one parent-window relationship.
pub(crate) unsafe fn window_set_parent(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    // reject self-parent relationship
    if parent == Some(window) {
        return Err(core_platform::invalid_argument(
            "parent",
            "window cannot be parent of itself",
        ));
    }

    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setParent",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setParent")?;
    // snapshot state before mutation, for event and rollback lanes
    let previous = binding.clone();
    let previous_owner = owner_relationship(&binding);
    let previous_modal = binding.modal;
    let next_owner = binding.transient_for.or(parent);

    // reject modal state without an owner relationship
    if binding.modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "parent",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply relationship update and rollback host owner lane on failure
    binding.parent = parent;
    if let Err(error) =
        apply_owner_relationship(context, &binding, "destack.display.window.setParent")
    {
        binding.parent = previous.parent;
        return Err(error);
    }

    if let Err(error) = apply_modal_owner_transition(
        context,
        previous_owner,
        previous_modal,
        next_owner,
        binding.modal,
        "destack.display.window.setParent",
    ) {
        let transition_reversal = apply_modal_owner_transition(
            context,
            next_owner,
            binding.modal,
            previous_owner,
            previous_modal,
            "destack.display.window.setParent.rollback",
        );

        binding.parent = previous.parent;
        let relationship_reversal = apply_owner_relationship(
            context,
            &binding,
            "destack.display.window.setParent.rollback",
        );

        // keep primary error: rollback is best effort during failure unwind
        if transition_reversal.is_err() || relationship_reversal.is_err() {
            context.warn(
                "display",
                "destack.display.window.setParent.rollback",
                "rollback failed while unwinding modal transition",
                None,
            );
        }

        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one transient-owner relationship.
pub(crate) unsafe fn window_set_transient_for(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    // reject self-transient relationship
    if transientfor == Some(window) {
        return Err(core_platform::invalid_argument(
            "transientFor",
            "window cannot be transient owner of itself",
        ));
    }

    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setTransientFor",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setTransientFor")?;
    // snapshot state before mutation, for event and rollback lanes
    let previous = binding.clone();
    let previous_owner = owner_relationship(&binding);
    let previous_modal = binding.modal;
    let next_owner = transientfor.or(binding.parent);

    // reject modal state without an owner relationship
    if binding.modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "transientFor",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply relationship update and rollback host owner lane on failure
    binding.transient_for = transientfor;
    if let Err(error) =
        apply_owner_relationship(context, &binding, "destack.display.window.setTransientFor")
    {
        binding.transient_for = previous.transient_for;
        return Err(error);
    }

    if let Err(error) = apply_modal_owner_transition(
        context,
        previous_owner,
        previous_modal,
        next_owner,
        binding.modal,
        "destack.display.window.setTransientFor",
    ) {
        let transition_reversal = apply_modal_owner_transition(
            context,
            next_owner,
            binding.modal,
            previous_owner,
            previous_modal,
            "destack.display.window.setTransientFor.rollback",
        );

        binding.transient_for = previous.transient_for;
        let relationship_reversal = apply_owner_relationship(
            context,
            &binding,
            "destack.display.window.setTransientFor.rollback",
        );

        // keep primary error: rollback is best effort during failure unwind
        if transition_reversal.is_err() || relationship_reversal.is_err() {
            context.warn(
                "display",
                "destack.display.window.setTransientFor.rollback",
                "rollback failed while unwinding modal transition",
                None,
            );
        }

        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}
