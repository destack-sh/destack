import { type ClientContext, createORPCClient, ErrorEvent, mapEventIterator } from "@orpc/client";
import { isAsyncIteratorObject } from "@orpc/shared";
import { CallError } from "../error/call.ts";
import { injectContext, markCaptured } from "@destack/telemetry";
import { OpenAPILink, type OpenAPILinkOptions } from "@orpc/openapi-client/fetch";
import { ClientRetryPlugin } from "@orpc/client/plugins";
import type { Client, ProcedureRouter } from "../procedure/index.ts";
import { ProcedureMetadata } from "../procedure/procedure.ts";
import { VERSION_HEADER } from "../procedure/procedure.ts";
import { CallTelemetry } from "../procedure/telemetry.ts";
import { BOOKMARK_HEADER, Bookmark } from "../procedure/bookmark.ts";
import type { Environment } from "@destack/environment";

/** The longest a call waits for a starting server. */
const STARTING_WAIT_MILLISECONDS = 10_000;

/** The options of a client. */
export type ClientOptions<Context extends ClientContext = Record<never, never>> =
    OpenAPILinkOptions<Context> & {
        /** The watermarks each request requires. */
        readonly bookmark?: Bookmark;
    };

/** What a client calls: a router of procedures, such as a package handle's. */
export interface ClientTarget<Definition extends ProcedureRouter> {
    /** The procedures, each with the package whose release its calls speak. */
    readonly router: Definition;
}

/** A typed HTTP client of a package, as createClient creates it. */
export type ClientOf<
    Declared extends { readonly router: ProcedureRouter },
    Context extends ClientContext = Record<never, never>,
> = Client<Declared["router"], Context>;

/** Create a typed HTTP client of a package at an endpoint, sending and retrying in a world. */
export function createClient<
    Definition extends ProcedureRouter,
    Context extends ClientContext = Record<never, never>,
>(
    target: ClientTarget<Definition>,
    options: ClientOptions<Context>,
    environment: Environment,
): Client<Definition, Context> {
    // create the client telemetry, and read each called procedure's release once
    const instruments = new CallTelemetry("client", environment);
    const releases = new Map<string, string>();
    const releaseOf = (path: readonly string[]) => {
        // read a path's release the first time it is called
        const key = path.join(".");
        const release =
            releases.get(key) ?? ProcedureMetadata.at(target.router, path).package.version;
        releases.set(key, release);

        return release;
    };

    // create the HTTP transport, sending through the environment's network unless the endpoint fetches
    const link = new OpenAPILink<Context>(target.router, {
        fetch: (request) => environment.network.fetch(request),
        ...options,
        plugins: [new ClientRetryPlugin(), ...(options.plugins ?? [])],
        customErrorResponseBodyDecoder: answeredError,
        interceptors: [
            async (call) => {
                // read a streamed failure with its reason and domain, as an answered one reads
                const output: unknown = await call.next();

                return isAsyncIteratorObject(output)
                    ? mapEventIterator(output, {
                          value: async (value: unknown) => value,
                          error: async (error: unknown) => streamedError(error),
                      })
                    : output;
            },
            ...(options.interceptors ?? []),
        ],
        adapterInterceptors: [
            async (call) => {
                // send the trace, the release and the required watermarks
                const headers = call.request.headers;
                injectContext(headers, environment.telemetry);
                headers.set(VERSION_HEADER, releaseOf(call.path));
                const bookmark = options.bookmark;
                if (bookmark !== undefined && bookmark.watermarks.length > 0) {
                    headers.set(BOOKMARK_HEADER, bookmark.format());
                }

                // send a clone of the request each time, retrying a starting server after its pause
                const { clock } = environment;
                const deadline = clock.now() + STARTING_WAIT_MILLISECONDS;
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
                    pause !== undefined && clock.now() + pause <= deadline;
                    pause = startingPause(response)
                ) {
                    await clock.sleep(pause, call.signal);
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

/** Read the pause a starting server asks for before a retry (RFC 9110 10.2.3). */
function startingPause(response: {
    readonly status: number;
    readonly headers: Headers;
}): number | undefined {
    const seconds = Number(response.headers.get("retry-after"));

    return response.status === 503 && Number.isInteger(seconds) && seconds > 0
        ? seconds * 1000
        : undefined;
}

/** Read a stream's failure event as its call error. */
function streamedError(error: unknown): unknown {
    return error instanceof ErrorEvent ? (CallError.fromJson(error.data) ?? error) : error;
}

/** Read a server's answered failure, leaving a malformed answer to the link's failure. */
function answeredError(body: unknown) {
    // build the failure with its reason and domain
    const error = CallError.fromJson(body);
    if (error !== undefined && error.status >= 500) {
        markCaptured(error);
    }

    return error;
}
