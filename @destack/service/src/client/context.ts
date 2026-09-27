import { PackageId } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { ResourceContext } from "@destack/resource/context";
import { defineSchema, schema } from "@destack/schema";
import type { ServiceConnection } from "../declare/index.ts";
import type { ServiceRouter } from "../service/service.ts";
import { createClient, type ClientOptions } from "./client.ts";

/** A service dependency resolved to a reachable endpoint by the host. */
export const ServiceConnectionBinding = defineSchema(
    schema.object({
        /** The consumer's package-qualified connection declaration. */
        declaration: DeclarationReference,
        /** The endpoint used by this client, relative to its trusted origin when applicable. */
        url: schema.string().min(1),
    }),
);
/** A service dependency resolved to a reachable endpoint by the host. */
export type ServiceConnectionBinding = schema.Infer<typeof ServiceConnectionBinding>;

/** Host-selected service connections for an application. */
export const ClientConfiguration = defineSchema(
    schema.object({
        /** Package receiving the configured connections. */
        packageId: PackageId,
        /** Endpoints selected for package-qualified connection declarations. */
        services: schema.array(ServiceConnectionBinding),
    }),
);
/** Host-selected service connections for an application. */
export type ClientConfiguration = schema.Infer<typeof ClientConfiguration>;

/** Application-scoped service clients resolved through the shared resource context. */
export class ClientContext {
    /** Host-selected connections. */
    readonly configuration: ClientConfiguration;
    /** Typed clients available to application code. */
    readonly resources: ResourceContext;
    /** Transport authentication supplied by the hosting runtime. */
    readonly #options: Omit<ClientOptions, "url">;

    /** Retain validated connection configuration and host transport options. */
    constructor(
        configuration: ClientConfiguration,
        options: Omit<ClientOptions, "url">,
        resources = new ResourceContext(),
    ) {
        this.configuration = ClientConfiguration.parse(configuration);
        this.#options = options;
        this.resources = resources;
    }

    /** Construct the client selected for one declared dependency. */
    bind<Router extends ServiceRouter>(connection: ServiceConnection<Router>): void {
        // require one unambiguous endpoint for the consumer declaration
        const bindings = this.configuration.services.filter(
            (binding) =>
                binding.declaration.packageId === connection.package.id &&
                binding.declaration.name === connection.name,
        );
        if (bindings.length !== 1) {
            throw new TypeError(
                `client requires one binding for ${connection.package.id}/${connection.name}`,
            );
        }

        // share the resource context between frontend and backend service clients
        const client = createClient(connection.router, { ...this.#options, url: bindings[0].url });
        this.resources.bind(connection, client);
    }
}
