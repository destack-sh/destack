import { type ClientContext, createORPCClient } from "@orpc/client";
import { injectContext } from "@destack/telemetry";
import { OpenAPILink, type OpenAPILinkOptions } from "@orpc/openapi-client/fetch";
import type { Client, Service } from "../service/index.ts";
import { ServiceTelemetry } from "../telemetry/index.ts";

/** Configure the service URL, request headers, fetch implementation, and interceptors. */
export type ClientOptions<Context extends ClientContext = Record<never, never>> =
    OpenAPILinkOptions<Context>;

/** Create a typed HTTP client from a service definition. */
export function createClient<
    Definition extends Service,
    Context extends ClientContext = Record<never, never>,
>(definition: Definition, options: ClientOptions<Context>): Client<Definition, Context> {
    // connect client tracing to the host's telemetry providers
    const telemetry = new ServiceTelemetry("client");

    // propagate the current trace through the HTTP transport
    const link = new OpenAPILink<Context>(definition, {
        ...options,
        adapterInterceptors: [
            ({ request, next }) => {
                injectContext(request.headers);

                return next();
            },
            ...(options.adapterInterceptors ?? []),
        ],
    });

    return createORPCClient<Client<Definition, Context>>({
        call: (path, input, options) =>
            telemetry.invoke(path, () => link.call(path, input, options)),
    });
}
