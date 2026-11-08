import { copyRequest } from "./request.ts";

/** The header a proxy writes the hops of a request into (RFC 7239). */
const FORWARDED_HEADER = "forwarded";

/** A token of RFC 7230, which a parameter value need not quote. */
const TOKEN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/u;

/** The parameters this codec reads and writes. */
const PARAMETERS = ["for", "host", "proto"] as const;

/** The names of the parameters this codec reads. */
const PARAMETER_NAMES: ReadonlySet<string> = new Set(PARAMETERS);

/** One hop of a forwarded request: its client, the host it asked for and its protocol. */
export interface ForwardedElement {
    /** The node the hop received the request from, such as `192.0.2.43` or `[2001:db8::1]`. */
    readonly for?: string;
    /** The host the client asked for. */
    readonly host?: string;
    /** The protocol the client asked with, such as https. */
    readonly proto?: string;
}

/** RFC 7239's Forwarded header: the hops a request took, the first one's nearest the client. */
export const Forwarded = {
    /** Read a header's elements, skipping parameters other than for, host and proto. */
    parse(header: string | null): ForwardedElement[] {
        // read each comma-separated element's semicolon-separated pairs, unquoting quoted values
        if (header === null || header.trim() === "") {
            return [];
        }

        return splitOutside(header, ",").map((element) => {
            const pairs: Record<string, string> = {};
            for (const pair of splitOutside(element, ";")) {
                const separator = pair.indexOf("=");
                const name = pair.slice(0, separator).trim().toLowerCase();
                const value = pair.slice(separator + 1).trim();
                if (separator > 0 && PARAMETER_NAMES.has(name)) {
                    pairs[name] = unquote(value);
                }
            }

            return pairs;
        });
    },

    /** Write elements as a header, quoting values that are no tokens. */
    format(elements: readonly ForwardedElement[]): string {
        return elements
            .map((element) =>
                PARAMETERS.flatMap((name) => {
                    const value = element[name];

                    return value === undefined ? [] : [`${name}=${quote(value)}`];
                }).join(";"),
            )
            .join(", ");
    },

    /** Read the first hop of a request, the one nearest its client, empty without a header. */
    first(request: Request): ForwardedElement {
        return Forwarded.parse(request.headers.get(FORWARDED_HEADER))[0] ?? {};
    },

    /** Read the host a request's client asked for: its first hop's, else the request's own. */
    host(request: Request): string {
        return Forwarded.first(request).host ?? new URL(request.url).host;
    },

    /** Copy a request at the host and protocol its client asked for, as its first hop has them. */
    restore(request: Request): Request {
        // keep a request no front door forwarded as it is
        const { host, proto } = Forwarded.first(request);
        if (host === undefined) {
            return request;
        }

        // ask for the forwarded host, with the protocol and host header the client sent
        const url = new URL(request.url);
        url.host = host;
        url.protocol = proto === undefined ? url.protocol : `${proto}:`;
        const headers = new Headers(request.headers);
        headers.set("host", host);

        return copyRequest(request, { headers }, url.href);
    },

    /** Copy a request as a front door receives it: a trusted peer's hop, else the peer alone. */
    receive(
        request: Request,
        peer: string | undefined,
        isTrusted: (peer: string) => boolean,
    ): Request {
        // keep the last hop a trusted proxy appended, as earlier ones are its client's to forge
        if (peer === undefined || isTrusted(peer)) {
            const last = Forwarded.parse(request.headers.get(FORWARDED_HEADER)).at(-1);

            return last === undefined ? request : Forwarded.replace(request, last);
        }

        return Forwarded.replace(request, { for: peer });
    },

    /** Copy a request with its header replaced by one hop, as its front door writes it. */
    replace(request: Request, element: ForwardedElement): Request {
        const headers = new Headers(request.headers);
        headers.set(FORWARDED_HEADER, Forwarded.format([element]));

        return copyRequest(request, { headers });
    },
};

/** Split a text at a separator outside quoted strings. */
function splitOutside(text: string, separator: string): string[] {
    // track quotes and escapes, splitting only between quoted strings
    const parts: string[] = [];
    let current = "";
    let isQuoted = false;
    let isEscaped = false;
    for (const character of text) {
        if (isEscaped) {
            isEscaped = false;
        } else if (character === "\\" && isQuoted) {
            isEscaped = true;
        } else if (character === '"') {
            isQuoted = !isQuoted;
        } else if (character === separator && !isQuoted) {
            parts.push(current);
            current = "";
            continue;
        }
        current += character;
    }
    parts.push(current);

    return parts.map((part) => part.trim()).filter((part) => part !== "");
}

/** Read a parameter value, unquoting and unescaping a quoted string. */
function unquote(value: string): string {
    return value.startsWith('"') && value.endsWith('"') && value.length >= 2
        ? value.slice(1, -1).replaceAll(/\\(.)/gu, "$1")
        : value;
}

/** Write a parameter value, quoting one that is no token. */
function quote(value: string): string {
    return TOKEN.test(value) ? value : `"${value.replaceAll(/["\\]/gu, "\\$&")}"`;
}
