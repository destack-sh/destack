import {
    type ClientContext,
    createORPCClient,
    createORPCErrorFromJson,
    isORPCErrorJson,
} from "@orpc/client";
import { injectContext, telemetry } from "@destack/telemetry";
import { OpenAPILink, type OpenAPILinkOptions } from "@orpc/openapi-client/fetch";
import { ClientRetryPlugin } from "@orpc/client/plugins";
import type { Client, ServiceRouter } from "../service/index.ts";
import type { Service } from "../declare/service.ts";
import { VERSION_HEADER } from "../request/request.ts";
import { ServiceTelemetry } from "../telemetry/index.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";
import { wait } from "../timer/index.ts";

/** The longest a call waits for a starting service, about ten space starts' first snapshots at a second each. */
const STARTING_WAIT_MILLISECONDS = 10_000;

/** The options of a client. */
export type ClientOptions<Context extends ClientContext = Record<never, never>> =
    OpenAPILinkOptions<Context> & {
        /** The watermarks each request requires. */
        readonly bookmark?: Bookmark;
    };

/** A typed HTTP client of a declared service, as createClient creates it. */
export type ServiceClient<
    Declared extends Pick<Service, "router">,
    Context extends ClientContext = Record<never, never>,
> = Client<Declared["router"], Context>;

/** Create a typed HTTP client of a service, speaking the release of the service it was built against. */
export function createClient<
    Definition extends ServiceRouter,
    Context extends ClientContext = Record<never, never>,
>(
    service: Pick<Service<Definition>, "package" | "router">,
    options: ClientOptions<Context>,
): Client<Definition, Context> {
    // create the client telemetry
    const instruments = new ServiceTelemetry("client");

    // create the HTTP transport
    const link = new OpenAPILink<Context>(service.router, {
        ...options,
        plugins: [new ClientRetryPlugin(), ...(options.plugins ?? [])],
        customErrorResponseBodyDecoder: answeredError,
        adapterInterceptors: [
            async (call) => {
                // send the trace, the release and the required watermarks
                const headers = call.request.headers;
                injectContext(headers);
                headers.set(VERSION_HEADER, service.package.version);
                const bookmark = options.bookmark;
                if (bookmark !== undefined && bookmark.watermarks.length > 0) {
                    headers.set(BOOKMARK_HEADER, bookmark.format());
                }

                // send a copy of the request each time, retrying a starting service after the pause it asks for within a bounded wait
                const deadline = Date.now() + STARTING_WAIT_MILLISECONDS;
                const { url, method } = call.request;
                const body = call.request.body === null ? null : await call.request.arrayBuffer();
                const send = () =>
                    call.next({
                        ...call,
                        request: new Request(url, {
                            method,
                            headers: call.request.headers,
                            body,
                            signal: call.request.signal,
                        }),
                    });
                let response = await send();
                for (
                    let pause = startingPause(response);
                    pause !== undefined && Date.now() + pause <= deadline;
                    pause = startingPause(response)
                ) {
                    await wait(pause, call.signal === undefined ? {} : { signal: call.signal });
                    response = await send();
                }

                // observe the response's watermarks
                const observed = response.headers.get(BOOKMARK_HEADER);
                if (bookmark !== undefined && observed !== null && observed !== "") {
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
        call: (path, input, callOptions) =>
            instruments.invoke(path, () => link.call(path, input, callOptions)),
    });
}

/** Read the pause a starting service asks for before a retry, absent for any other answer (RFC 9110 10.2.3). */
function startingPause(response: {
    readonly status: number;
    readonly headers: Headers;
}): number | undefined {
    const seconds = Number(response.headers.get("retry-after"));

    return response.status === 503 && Number.isInteger(seconds) && seconds > 0
        ? seconds * 1000
        : undefined;
}

/** Read a service's answered failure, marking a server failure captured, as the answering service captured it. */
function answeredError(body: unknown) {
    // leave a malformed answer to the link's own failure
    if (!isORPCErrorJson(body)) {
        return undefined;
    }

    // build the failure
    const error = createORPCErrorFromJson(body);
    if (error.status >= 500) {
        telemetry.markCaptured(error);
    }

    return error;
}
