use super::*;

/// Set one window modal state.
pub(crate) unsafe fn window_set_modal(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setModal",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setModal")?;

    // reject modal transitions without an owner relationship
    if modal && owner_relationship(&resolved_binding).is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // snapshot state before mutation, for event and rollback lanes
    let previous = resolved_binding.clone();
    let previous_owner = owner_relationship(&resolved_binding);
    let previous_modal = resolved_binding.modal;
    resolved_binding.modal = modal;
    let next_owner = owner_relationship(&resolved_binding);

    // apply host owner transition and rollback resolved_binding state on failure
    if let Err(error) = apply_modal_owner_transition(
        binding,
        previous_owner,
        previous_modal,
        next_owner,
        resolved_binding.modal,
        "destack.display.window.setModal",
    ) {
        resolved_binding.modal = previous_modal;
        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one parent-window relationship.
pub(crate) unsafe fn window_set_parent(
    binding: &BindingCallContext,
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

    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setParent",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setParent")?;
    // snapshot state before mutation, for event and rollback lanes
    let previous = resolved_binding.clone();
    let previous_owner = owner_relationship(&resolved_binding);
    let previous_modal = resolved_binding.modal;
    let next_owner = resolved_binding.transient_for.or(parent);

    // reject modal state without an owner relationship
    if resolved_binding.modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "parent",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply relationship update and rollback host owner lane on failure
    resolved_binding.parent = parent;
    // evaluate this condition
    if let Err(error) = apply_owner_relationship(
        binding,
        &resolved_binding,
        "destack.display.window.setParent",
    ) {
        resolved_binding.parent = previous.parent;
        return Err(error);
    }

    // evaluate this condition
    if let Err(error) = apply_modal_owner_transition(
        binding,
        previous_owner,
        previous_modal,
        next_owner,
        resolved_binding.modal,
        "destack.display.window.setParent",
    ) {
        let transition_reversal = apply_modal_owner_transition(
            binding,
            next_owner,
            resolved_binding.modal,
            previous_owner,
            previous_modal,
            "destack.display.window.setParent.rollback",
        );

        resolved_binding.parent = previous.parent;
        let relationship_reversal = apply_owner_relationship(
            binding,
            &resolved_binding,
            "destack.display.window.setParent.rollback",
        );

        // keep primary error: rollback is best effort during failure unwind
        if transition_reversal.is_err() || relationship_reversal.is_err() {
            binding.warn(
                "display",
                "destack.display.window.setParent.rollback",
                "rollback failed while unwinding modal transition",
                None,
            );
        }

        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one transient-owner relationship.
pub(crate) unsafe fn window_set_transient_for(
    binding: &BindingCallContext,
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

    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setTransientFor",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setTransientFor")?;
    // snapshot state before mutation, for event and rollback lanes
    let previous = resolved_binding.clone();
    let previous_owner = owner_relationship(&resolved_binding);
    let previous_modal = resolved_binding.modal;
    let next_owner = transientfor.or(resolved_binding.parent);

    // reject modal state without an owner relationship
    if resolved_binding.modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "transientFor",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // apply relationship update and rollback host owner lane on failure
    resolved_binding.transient_for = transientfor;
    // evaluate this condition
    if let Err(error) = apply_owner_relationship(
        binding,
        &resolved_binding,
        "destack.display.window.setTransientFor",
    ) {
        resolved_binding.transient_for = previous.transient_for;
        return Err(error);
    }

    // evaluate this condition
    if let Err(error) = apply_modal_owner_transition(
        binding,
        previous_owner,
        previous_modal,
        next_owner,
        resolved_binding.modal,
        "destack.display.window.setTransientFor",
    ) {
        let transition_reversal = apply_modal_owner_transition(
            binding,
            next_owner,
            resolved_binding.modal,
            previous_owner,
            previous_modal,
            "destack.display.window.setTransientFor.rollback",
        );

        resolved_binding.transient_for = previous.transient_for;
        let relationship_reversal = apply_owner_relationship(
            binding,
            &resolved_binding,
            "destack.display.window.setTransientFor.rollback",
        );

        // keep primary error: rollback is best effort during failure unwind
        if transition_reversal.is_err() || relationship_reversal.is_err() {
            binding.warn(
                "display",
                "destack.display.window.setTransientFor.rollback",
                "rollback failed while unwinding modal transition",
                None,
            );
        }

        return Err(error);
    }

    // publish state deltas after host mutation succeeds
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}
