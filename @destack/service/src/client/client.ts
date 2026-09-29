import { type ClientContext, createORPCClient } from "@orpc/client";
import { injectContext } from "@destack/telemetry";
import { OpenAPILink, type OpenAPILinkOptions } from "@orpc/openapi-client/fetch";
import { ClientRetryPlugin } from "@orpc/client/plugins";
import type { Client, ServiceRouter } from "../service/index.ts";
import type { Service } from "../declare/service.ts";
import { VERSION_HEADER } from "../request/request.ts";
import { ServiceTelemetry } from "../telemetry/index.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";

/** The options of a client. */
export type ClientOptions<Context extends ClientContext = Record<never, never>> =
    OpenAPILinkOptions<Context> & {
        /** The watermarks each request requires. */
        readonly bookmark?: Bookmark;
    };

/** Create a typed HTTP client of a service, speaking the release of the service it was built against. */
export function createClient<
    Definition extends ServiceRouter,
    Context extends ClientContext = Record<never, never>,
>(
    service: Pick<Service<Definition>, "package" | "router">,
    options: ClientOptions<Context>,
): Client<Definition, Context> {
    // create the client telemetry
    const telemetry = new ServiceTelemetry("client");

    // create the HTTP transport
    const link = new OpenAPILink<Context>(service.router, {
        ...options,
        plugins: [new ClientRetryPlugin(), ...(options.plugins ?? [])],
        adapterInterceptors: [
            async ({ request, next }) => {
                // send the trace, the release and the required watermarks
                injectContext(request.headers);
                request.headers.set(VERSION_HEADER, service.package.version);
                const bookmark = options.bookmark;
                if (bookmark && bookmark.watermarks.length > 0) {
                    request.headers.set(BOOKMARK_HEADER, bookmark.format());
                }

                // observe the response's watermarks
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
