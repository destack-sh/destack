import { schema } from "@destack/schema";

/** The fields of a message's headers in order, each `set-cookie` its own pair. */
const HeaderList = schema.array(schema.tuple([schema.string(), schema.string()]));

/** The head of an HTTP request a stream forwards, sent with the stream's SYN. */
export const RequestHead = schema.object({
    /** The request's method. */
    method: schema.string().min(1),
    /** The request's URL, as the relay received it. */
    url: schema.string().url(),
    /** The request's headers. */
    headers: HeaderList,
});
/** The head of an HTTP request a stream forwards. */
export type RequestHead = schema.Infer<typeof RequestHead>;

/** The head of the HTTP response answering a stream's request, sent with an ACK. */
export const ResponseHead = schema.object({
    /** The response's status. */
    status: schema.number().int().min(200).max(599),
    /** The response's headers. */
    headers: HeaderList,
});
/** The head of the HTTP response answering a stream's request. */
export type ResponseHead = schema.Infer<typeof ResponseHead>;

/** The statuses of responses without a body (RFC 9110 15.3.5, 15.4.5). */
const BODILESS_STATUSES: ReadonlySet<number> = new Set([204, 205, 304]);

/** The headers of one connection, which a proxy drops (RFC 9110 7.6.1). */
const HOP_BY_HOP: ReadonlySet<string> = new Set([
    "connection",
    "keep-alive",
    "proxy-connection",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
]);

/** The methods of requests without a body. */
const BODILESS_METHODS: ReadonlySet<string> = new Set(["GET", "HEAD"]);

/** The heads of HTTP messages as streams carry them. */
export const Head = {
    /** Read a request's head. */
    request(request: Request): RequestHead {
        return { method: request.method, url: request.url, headers: entries(request.headers) };
    },

    /** Read a response's head. */
    response(response: Response): ResponseHead {
        return { status: response.status, headers: entries(response.headers) };
    },

    /** Build the headers a head lists. */
    headers(head: RequestHead | ResponseHead): Headers {
        const headers = new Headers();
        for (const [name, value] of head.headers) {
            headers.append(name, value);
        }

        return headers;
    },

    /** Whether a request's method carries a body. */
    hasRequestBody(head: RequestHead): boolean {
        return !BODILESS_METHODS.has(head.method.toUpperCase());
    },

    /** Whether a response's status carries a body. */
    hasResponseBody(head: ResponseHead): boolean {
        return !BODILESS_STATUSES.has(head.status);
    },
};

/** List the end-to-end headers in order, keeping each `set-cookie` apart as the Fetch standard combines the rest. */
function entries(headers: Headers): [string, string][] {
    // drop the connection's own headers and those it lists
    const listed = (headers.get("connection") ?? "")
        .split(",")
        .map((name) => name.trim().toLowerCase());
    const isEndToEnd = (name: string) => !HOP_BY_HOP.has(name) && !listed.includes(name);

    // keep each set-cookie apart
    const combined = [...headers].filter(([name]) => name !== "set-cookie" && isEndToEnd(name));
    const cookies = headers.getSetCookie().map((value): [string, string] => ["set-cookie", value]);

    return [...combined, ...cookies];
}
