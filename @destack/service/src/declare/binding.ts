import {
    type ResourceBinding,
    type Connector,
    defineResourceKind,
    Resource,
} from "@destack/resource";
import { declaringModule, type ModuleMetadata, type Package } from "@destack/package";
import { DeclarationReference, reference } from "@destack/package/declare";
import { defineSchema, schema, Version } from "@destack/schema";
import { createClient } from "../client/client.ts";
import type { Client, ServiceRouter } from "../service/service.ts";
import type { Service } from "./service.ts";

/** A service binding's specification: the service it calls. */
export const ServiceBindingSpec = defineSchema(schema.object({ service: DeclarationReference }));
/** A service binding's specification. */
export type ServiceBindingSpec = schema.Infer<typeof ServiceBindingSpec>;

/** The state a service binding requires of its callee: the release its caller was built against. */
export const ServiceBindingState = defineSchema(schema.object({ release: Version }));
/** The state a service binding requires of its callee. */
export type ServiceBindingState = schema.Infer<typeof ServiceBindingState>;

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

    /** The HTTP connector, calling the service at the bound address through the host's egress with the bound credential. */
    override get connectors(): { readonly http: Connector<Client<Router>> } {
        return {
            http: {
                code: "http",
                connect: async (binding: ResourceBinding) => {
                    // require the credential the host lends the binding
                    const credential = binding.credential;
                    if (credential === undefined) {
                        throw new TypeError(`service binding ${this.name} holds no credential`);
                    }

                    return createClient(this.service, {
                        url: binding.reference,
                        headers: () => ({ authorization: `Bearer ${credential}` }),
                    });
                },
            },
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
