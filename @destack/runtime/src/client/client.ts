import {
    type ClientContext,
    createORPCClient,
    createORPCErrorFromJson,
    isORPCErrorJson,
} from "@orpc/client";
import { injectContext, telemetry } from "@destack/telemetry";
import { OpenAPILink, type OpenAPILinkOptions } from "@orpc/openapi-client/fetch";
import { ClientRetryPlugin } from "@orpc/client/plugins";
import type { Client, ProcedureRouter } from "../runtime/index.ts";
import { ProcedureMeta } from "../procedure/procedure.ts";
import { VERSION_HEADER } from "../request/request.ts";
import { CallTelemetry } from "../procedure/call-telemetry.ts";
import { BOOKMARK_HEADER, Bookmark } from "../bookmark/index.ts";
import { Clock } from "@destack/environment";

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
    /** The procedures, each naming the package whose release its calls speak. */
    readonly router: Definition;
}

/** A typed HTTP client of a package, as createClient creates it. */
export type ClientOf<
    Declared extends { readonly router: ProcedureRouter },
    Context extends ClientContext = Record<never, never>,
> = Client<Declared["router"], Context>;

/** Create a typed HTTP client of a package. */
export function createClient<
    Definition extends ProcedureRouter,
    Context extends ClientContext = Record<never, never>,
>(target: ClientTarget<Definition>, options: ClientOptions<Context>): Client<Definition, Context> {
    // create the client telemetry, and read each called procedure's release once
    const instruments = new CallTelemetry("client");
    const releases = new Map<string, string>();
    const releaseOf = (path: readonly string[]) => {
        // read a path's release the first time it is called
        const key = path.join(".");
        const release = releases.get(key) ?? ProcedureMeta.at(target.router, path).package.version;
        releases.set(key, release);

        return release;
    };

    // create the HTTP transport
    const link = new OpenAPILink<Context>(target.router, {
        ...options,
        plugins: [new ClientRetryPlugin(), ...(options.plugins ?? [])],
        customErrorResponseBodyDecoder: answeredError,
        adapterInterceptors: [
            async (call) => {
                // send the trace, the release and the required watermarks
                const headers = call.request.headers;
                injectContext(headers);
                headers.set(VERSION_HEADER, releaseOf(call.path));
                const bookmark = options.bookmark;
                if (bookmark !== undefined && bookmark.watermarks.length > 0) {
                    headers.set(BOOKMARK_HEADER, bookmark.format());
                }

                // send a clone of the request each time, retrying a starting server after its pause
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
                    await Clock.system.sleep(pause, call.signal);
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

/** Read a server's answered failure. */
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
