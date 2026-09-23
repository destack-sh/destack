import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { defineSchema, schema } from "@destack/schema";
import type { ServiceConnection } from "../declare/index.ts";
import type { Service } from "../service/service.ts";
import { ServiceError } from "../error/index.ts";
import { bindServiceConnection, ServiceConnectionBinding } from "./binding.ts";
import type { ClientOptions } from "./client.ts";

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
    bind<Router extends Service>(connection: ServiceConnection<Router>): void {
        // require one unambiguous endpoint for the consumer declaration
        const bindings = this.configuration.services.filter(
            (binding) =>
                binding.declaration.packageId === connection.packageId &&
                binding.declaration.name === connection.name,
        );
        if (bindings.length !== 1) {
            throw new ServiceError("BAD_REQUEST", {
                message: `client requires one binding for ${connection.packageId}/${connection.name}`,
            });
        }

        bindServiceConnection(connection, bindings[0], this.#options, this.resources);
    }
}
