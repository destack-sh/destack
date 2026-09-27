import { type ClientContext, createORPCClient } from "@orpc/client";
import { injectContext } from "@destack/telemetry";
import { OpenAPILink, type OpenAPILinkOptions } from "@orpc/openapi-client/fetch";
import { ClientRetryPlugin } from "@orpc/client/plugins";
import type { Client, ServiceRouter } from "../service/index.ts";
import { ServiceTelemetry } from "../telemetry/index.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";

/** The service URL, request headers, fetch implementation, interceptors and bookmark of a client. */
export type ClientOptions<Context extends ClientContext = Record<never, never>> =
    OpenAPILinkOptions<Context> & {
        /** Require the watermarks earlier responses observed, to read the client's own writes. */
        readonly bookmark?: Bookmark;
    };

/** Create a typed HTTP client from a service definition. */
export function createClient<
    Definition extends ServiceRouter,
    Context extends ClientContext = Record<never, never>,
>(definition: Definition, options: ClientOptions<Context>): Client<Definition, Context> {
    // connect client tracing to the host's telemetry providers
    const telemetry = new ServiceTelemetry("client");

    // propagate the current trace through the HTTP transport, retrying calls whose context asks for it
    const link = new OpenAPILink<Context>(definition, {
        ...options,
        plugins: [new ClientRetryPlugin(), ...(options.plugins ?? [])],
        adapterInterceptors: [
            async ({ request, next }) => {
                // propagate the trace and the watermarks the client requires
                injectContext(request.headers);
                const bookmark = options.bookmark;
                if (bookmark && bookmark.watermarks.length > 0) {
                    request.headers.set(BOOKMARK_HEADER, bookmark.format());
                }

                // record the watermarks the response's writes reached
                const response = await next();
                const observed = response.headers.get(BOOKMARK_HEADER);
                if (bookmark && observed) {
                    for (const watermark of Bookmark.parse(observed).watermarks) {
                        bookmark.observe(watermark);
                    }
                }

                return response;
            },
            ...(options.adapterInterceptors ?? []),
        ],
    });

    return createORPCClient<Client<Definition, Context>>({
        call: (path, input, options) =>
            telemetry.invoke(path, () => link.call(path, input, options)),
    });
}
