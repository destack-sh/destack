use super::with_harness_context;
use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;

/// Enumerate interfaces through the host backend and return at least one row.
#[test]
fn test_net_list_interfaces_returns_rows() {
    with_harness_context(|mut context| {
        // query host interfaces and decode name and index rows
        let interfaces = context.destack_net_list_interfaces()?;
        let interfaces = context.interface_name_index_list_from_value(interfaces)?;

        // ensure at least one interface row is present
        if interfaces.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "interfaces",
                "expected at least one host interface row",
            ))
            .boxed());
        }

        Ok(())
    });
}

/// Resolve interface names and indices through reciprocal lookup bindings.
#[test]
fn test_net_interface_name_index_roundtrip() {
    with_harness_context(|mut context| {
        // enumerate host interfaces and decode name and index rows
        let interfaces = context.destack_net_list_interfaces()?;
        let interfaces = context.interface_name_index_list_from_value(interfaces)?;

        // find one interface with a stable name and index for lookup checks
        let mut matched = false;
        for (name, index) in interfaces {
            if name.is_empty() || index == 0 {
                continue;
            }

            // resolve name to index and ensure the mapping is stable
            let resolved_index =
                context.destack_net_interface_index(context.string_value(&name))?;
            if resolved_index != index {
                continue;
            }

            // resolve index to name and ensure the mapping is non-empty
            let resolved_name = context.destack_net_interface_name(index)?;
            let resolved_name = context.string_from_value(resolved_name)?;
            if resolved_name.is_empty() {
                continue;
            }

            matched = true;
            break;
        }

        // require one successful reciprocal mapping row
        if !matched {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "interfaces",
                "expected at least one reciprocal interface lookup mapping",
            ))
            .boxed());
        }

        Ok(())
    });
}
