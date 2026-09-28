import { PackageId } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { ResourceContext } from "@destack/resource/context";
import { defineSchema, schema } from "@destack/schema";
import type { ServiceConnection } from "../declare/index.ts";
import type { ServiceRouter } from "../service/service.ts";
import { createClient, type ClientOptions } from "./client.ts";

/** A service dependency bound to an endpoint. */
export const ServiceConnectionBinding = defineSchema(
    schema.object({
        /** The connection declaration. */
        declaration: DeclarationReference,
        /** The endpoint URL. */
        url: schema.string().min(1),
    }),
);
/** A service dependency bound to an endpoint. */
export type ServiceConnectionBinding = schema.Infer<typeof ServiceConnectionBinding>;

/** The service connections of an application. */
export const ClientConfiguration = defineSchema(
    schema.object({
        /** The receiving package. */
        packageId: PackageId,
        /** The bound connections. */
        services: schema.array(ServiceConnectionBinding),
    }),
);
/** The service connections of an application. */
export type ClientConfiguration = schema.Infer<typeof ClientConfiguration>;

/** The service clients of an application. */
export class ClientContext {
    /** The connection configuration. */
    readonly configuration: ClientConfiguration;
    /** The typed clients. */
    readonly resources: ResourceContext;
    /** The host's transport options. */
    readonly #options: Omit<ClientOptions, "url">;

    /** Create the context. */
    constructor(
        configuration: ClientConfiguration,
        options: Omit<ClientOptions, "url">,
        resources = new ResourceContext(),
    ) {
        this.configuration = ClientConfiguration.parse(configuration);
        this.#options = options;
        this.resources = resources;
    }

    /** Create the client of one connection. */
    bind<Router extends ServiceRouter>(connection: ServiceConnection<Router>): void {
        // require exactly one binding
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

        // register the client
        const client = createClient(connection.router, { ...this.#options, url: bindings[0].url });
        this.resources.bind(connection, client);
    }
}
