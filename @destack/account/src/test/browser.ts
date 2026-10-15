import type { Fetch } from "@destack/runtime";

/** Retain browser cookies across real authentication HTTP requests. */
export class Browser {
    /** The URL the browser's paths are relative to: the issuer's pages and endpoints. */
    readonly base: string;
    /** Send a request to the served origin. */
    readonly send: Fetch;
    /** The languages the browser asks for, as its Accept-Language header. */
    readonly language: string;
    /** Cookies received from the base's origin. */
    readonly cookies = new Map<string, string>();

    /** Connect a browser asking for some languages to the pages below a URL. */
    constructor(base: string, send: Fetch, language: string) {
        this.base = base;
        this.send = send;
        this.language = language;
    }

    /** Send a request to a path below the base or to a URL, posting a JSON body or a form. */
    async fetch(path: string, body?: object | URLSearchParams): Promise<Response> {
        // send the request with the kept cookies, a form as it is and any other body as JSON
        const isForm = body instanceof URLSearchParams;
        const payload = isForm || body === undefined ? body : JSON.stringify(body);
        const url = URL.canParse(path) ? path : `${this.base}${path}`;
        const request = new Request(url, {
            method: body === undefined ? "GET" : "POST",
            headers: {
                ...(body === undefined ? {} : { origin: new URL(this.base).origin }),
                "content-type": isForm ? "application/x-www-form-urlencoded" : "application/json",
                "accept-language": this.language,
                cookie: [...this.cookies].map(([name, value]) => `${name}=${value}`).join("; "),
            },
            ...(payload === undefined ? {} : { body: payload }),
        });
        const sent = await this.send(request);

        // read the whole body like a browser to end the request
        const bytes = await sent.arrayBuffer();
        const response = new Response(bytes.byteLength === 0 ? null : bytes, sent);

        // replace or expire cookies as a browser would
        for (const cookie of response.headers.getSetCookie()) {
            const end = cookie.indexOf(";");
            const assignment = end === -1 ? cookie : cookie.slice(0, end);
            const separator = assignment.indexOf("=");
            const name = assignment.slice(0, separator);
            const value = assignment.slice(separator + 1);

            // forget an expired cookie
            if (/max-age=0(?:;|$)/iu.test(cookie)) {
                this.cookies.delete(name);
            }
            // keep any other
            else {
                this.cookies.set(name, value);
            }
        }

        return response;
    }
}
