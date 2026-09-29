import {
    ServiceKind,
    type ServiceBinding,
    type ServiceBindingDescription,
} from "../declare/binding.ts";

/** Describe a service binding for the package manifest. */
export function describeServiceBinding(binding: ServiceBinding): ServiceBindingDescription {
    return ServiceKind.description.parse({
        name: binding.name,
        kind: binding.kind,
        spec: binding.spec,
    });
}
