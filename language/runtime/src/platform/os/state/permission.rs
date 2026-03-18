use super::access::os_state;
use super::*;

/// Open host settings for runtime permissions.
pub(crate) fn permission_open_settings(binding: &BindingCallContext) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_permission::open_settings())
}

/// Request host authorization for one permission selector.
pub(crate) fn permission_request(
    binding: &BindingCallContext,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    let permission_state = binding
        .host()
        .submit_operation(host_permission::request(permission))?;
    let runtime_state = os_state(binding)?;
    runtime_state.set_permission_state(permission, permission_state);

    Ok(permission_state)
}

/// Request host authorization for one permission selector list.
pub(crate) fn permission_request_many(
    binding: &BindingCallContext,
    permissions: Vec<Permission>,
) -> RuntimeResult<Vec<PermissionEntry>> {
    let permission_entries = binding
        .host()
        .submit_operation(host_permission::request_many(permissions))?;
    let runtime_state = os_state(binding)?;

    for entry in &permission_entries {
        runtime_state.set_permission_state(entry.permission, entry.state);
    }

    Ok(permission_entries)
}

/// Read one runtime-owned permission state.
pub(crate) fn permission_state(
    binding: &BindingCallContext,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    let runtime_state = os_state(binding)?;

    runtime_state.permission_state(permission)
}

/// Read multiple runtime-owned permission states.
pub(crate) fn permission_state_many(
    binding: &BindingCallContext,
    permissions: &[Permission],
) -> RuntimeResult<Vec<PermissionEntry>> {
    let runtime_state = os_state(binding)?;

    runtime_state.permission_state_many(permissions)
}

impl PlatformOsState {
    /// Apply one host permission result.
    pub(crate) fn observe_permission(&self, permission: &str, granted: bool) {
        let Some(permission) = parse_host_permission_name(permission) else {
            return;
        };

        let state = if granted {
            PermissionState::Granted
        } else {
            PermissionState::Denied
        };

        let mut permission_states = self.permission_states.write();
        permission_states.insert(permission, state);
    }

    /// Read one cached permission state.
    pub(crate) fn permission_state(
        &self,
        permission: Permission,
    ) -> RuntimeResult<PermissionState> {
        let permission_states = self.permission_states.read();
        let Some(state) = permission_states.get(&permission).copied() else {
            return Err(super::core::invalid_data(
                "destack.os.permission.state",
                "permission state is not yet known for this runtime",
            ));
        };

        Ok(state)
    }

    /// Read multiple cached permission states.
    pub(crate) fn permission_state_many(
        &self,
        permissions: &[Permission],
    ) -> RuntimeResult<Vec<PermissionEntry>> {
        let mut entries = Vec::with_capacity(permissions.len());

        for permission in permissions {
            let state = self.permission_state(*permission)?;
            entries.push(PermissionEntry {
                permission: *permission,
                state,
            });
        }

        Ok(entries)
    }

    /// Record one permission state update for this runtime.
    pub(crate) fn set_permission_state(&self, permission: Permission, state: PermissionState) {
        let mut permission_states = self.permission_states.write();
        permission_states.insert(permission, state);
    }
}
