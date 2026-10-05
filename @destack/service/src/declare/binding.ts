import {
    type ResourceBinding,
    type Connector,
    defineResourceKind,
    ResourceDeclaration,
} from "@destack/resource";
import { ModuleMetadata, type Package } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { defineSchema, schema, Version } from "@destack/schema";
import { createClient } from "../client/client.ts";
import type { Client, ServiceRouter } from "../service/service.ts";
import type { Service } from "./service.ts";

/** A service binding's specification: the service it calls, and the address it calls unless a stack binds another. */
export const ServiceBindingSpec = defineSchema(
    schema.object({
        /** The service. */
        service: DeclarationReference,
        /** The address an installation calls unless a stack binds another, such as `@destack/mail` for a service of the platform. */
        address: schema.string().min(1).exactOptional(),
        /** The procedures the installation calls, by their dotted paths in the service's router, which the space grants it. */
        calls: schema.array(schema.string().min(1)).exactOptional(),
    }),
);
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
export class ServiceBinding<
    Router extends ServiceRouter = ServiceRouter,
> extends ResourceDeclaration<Client<Router>, ServiceBindingDescription> {
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

                    // dispose nothing
                    const client = createClient(this.service, {
                        url: binding.reference,
                        headers: () => ({ authorization: `Bearer ${credential}` }),
                    });

                    return Object.assign(client, {
                        [Symbol.asyncDispose]: () => Promise.resolve(),
                    });
                },
            },
        };
    }
}

/** Declare a dependency on a service, called at an address a stack binds or at the one it names. */
export function defineServiceBinding<Router extends ServiceRouter>(
    name: string,
    service: Service<Router>,
    options: {
        /** The address an installation calls unless a stack binds another. */
        readonly address?: string;
        /** The procedures of the service's router the installation calls, granted to it at install. */
        readonly calls?: readonly unknown[];
    } = {},
    module?: ModuleMetadata,
): ServiceBinding<Router> {
    // stamp the declaring package and name each called procedure by its path
    const owner = ModuleMetadata.require(module, "defineServiceBinding").package;
    const calls = options.calls?.map((procedure) => procedurePath(service.router, procedure, name));
    const description = ServiceKind.description.parse({
        name,
        kind: ServiceKind.name,
        spec: {
            service: DeclarationReference.of(service),
            ...(options.address === undefined ? {} : { address: options.address }),
            ...(calls === undefined ? {} : { calls }),
        },
    });

    return new ServiceBinding(owner, description, service);
}

/** Find a procedure's dotted path in a service's router, refusing one the router lacks. */
function procedurePath(router: ServiceRouter, procedure: unknown, binding: string): string {
    // walk the router's nested procedures for the one given
    const visit = (node: unknown, path: readonly string[]): string | undefined => {
        if (node === procedure) {
            return path.join(".");
        }
        if (typeof node !== "object" || node === null || path.length > 4) {
            return undefined;
        }

        return Object.entries(node)
            .map(([key, child]) => visit(child, [...path, key]))
            .find((found) => found !== undefined);
    };
    const found = visit(router, []);
    if (found === undefined || found === "") {
        throw new TypeError(
            `service binding ${binding} calls a procedure its service does not route`,
        );
    }

    return found;
}
