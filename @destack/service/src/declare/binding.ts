import {
    defineResourceKind,
    Resource,
    type Provider,
    type ProviderContext,
} from "@destack/resource";
import { declaringModule, type ModuleMetadata, type Package } from "@destack/package";
import { DeclarationReference, reference } from "@destack/package/declare";
import { defineSchema, schema } from "@destack/schema";
import { createClient } from "../client/client.ts";
import type { Client, ServiceRouter } from "../service/service.ts";
import type { Service } from "./service.ts";

/** A service binding's specification: the service it calls. */
export const ServiceBindingSpec = defineSchema(schema.object({ service: DeclarationReference }));
/** A service binding's specification. */
export type ServiceBindingSpec = schema.Infer<typeof ServiceBindingSpec>;

/** The service resource kind: typed clients of a package's service, bound to its endpoint. */
export const ServiceKind = defineResourceKind("service", { spec: ServiceBindingSpec });
/** A named dependency on a service. */
export type ServiceBindingDescription = schema.Infer<typeof ServiceKind.description>;

/** A dependency on a package's service, bound to a typed client of its endpoint. */
export class ServiceBinding<Router extends ServiceRouter = ServiceRouter> extends Resource<
    Client<Router>,
    ServiceBindingDescription
> {
    /** The service the client calls. */
    readonly service: Service<Router>;

    /** Create the binding. */
    constructor(owner: Package, description: ServiceBindingDescription, service: Service<Router>) {
        super(owner, description);
        this.service = service;
    }

    /** Open the HTTP provider, calling the service as the workload. */
    override get providers() {
        return {
            http: async (
                _reference: URL,
                context: ProviderContext,
            ): Promise<Provider<Client<Router>, typeof ServiceKind>> => ({
                kind: ServiceKind.name,
                code: "http",
                connect: async (record) => {
                    // require the endpoint the binding holds
                    if (record.reference === null) {
                        throw new TypeError(`service binding ${this.name} holds no endpoint`);
                    }

                    return createClient(this.service, {
                        url: record.reference,
                        headers: () => ({ authorization: `Bearer ${context.credential()}` }),
                    });
                },
            }),
        };
    }
}

/** Declare a dependency on a service. */
export function defineServiceBinding<Router extends ServiceRouter>(
    name: string,
    service: Service<Router>,
    module?: ModuleMetadata,
): ServiceBinding<Router> {
    const owner = declaringModule(module, "defineServiceBinding").package;
    const description = ServiceKind.description.parse({
        name,
        kind: ServiceKind.name,
        spec: { service: reference(service) },
    });

    return new ServiceBinding(owner, description, service);
}
