import { DeclarationReference } from "@destack/package/declare";
import { ResourceContext } from "@destack/resource/context";
import { defineSchema, schema } from "@destack/schema";
import type { ServiceConnection } from "../declare/connection.ts";
import type { ServiceRouter } from "../service/service.ts";
import { ServiceError } from "../error/index.ts";
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

/** Bind a declared service client using host-selected routing and transport authentication. */
export function bindServiceConnection<Router extends ServiceRouter>(
    connection: ServiceConnection<Router>,
    binding: ServiceConnectionBinding,
    options: Omit<ClientOptions, "url">,
    context: ResourceContext,
): void {
    // reject routing intended for a different declaration
    if (
        binding.declaration.packageId !== connection.package.id ||
        binding.declaration.name !== connection.name
    ) {
        throw new ServiceError("BAD_REQUEST", {
            message: "service connection binding does not match its declaration",
        });
    }

    // use the same resource context for frontend and backend service clients
    context.bind(connection, createClient(connection.router, { ...options, url: binding.url }));
}
