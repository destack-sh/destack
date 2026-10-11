import { graph } from "@destack/package";
import { schema } from "@destack/schema";
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

/** Bind a service binding to the service it calls. */
export function serviceBindingSymbols(binding: ServiceBinding): graph.MemberSymbol[] {
    const service = describeServiceBinding(binding).spec.service;

    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: [
                {
                    kind: "binds",
                    symbol: { packageId: service.packageId, kind: "service", name: service.name },
                },
            ],
        },
    ]);
}
