import type { Fetch } from "@destack/object";
import { present, schema } from "@destack/schema";
import {
    AUTHORIZATION_REQUEST_PATH,
    AUTHORIZATION_PATH,
    DEVICE_REQUEST_PATH,
} from "../authentication/path.ts";

/** The redirects a browser follows at most, the Fetch standard's limit. */
const REDIRECTS = 20;

/** A decided request: where the browser goes next. */
const Decided = schema.object({ location: schema.string() });

/** A sign-in by a code: whether the person's second factor comes next. */
const SignedIn = schema.object({ twoFactorRedirect: schema.boolean().exactOptional() }).loose();

/** Retain browser cookies across real authentication HTTP requests. */
export class TestBrowser {
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

    /** Read the page a redirect response sends a browser to. */
    static location(response: Response): string {
        return present(response.headers.get("location"), "the redirect location");
    }

    /** Send a request to a path below the base or to a URL, posting a JSON body or a form. */
    async fetch(
        path: string,
        body?: object | URLSearchParams,
        method: "GET" | "POST" | "DELETE" = body === undefined ? "GET" : "POST",
    ): Promise<Response> {
        // send the request with the stored cookies, a form as it is and any other body as JSON
        const isForm = body instanceof URLSearchParams;
        const payload = isForm || body === undefined ? body : JSON.stringify(body);
        const url = URL.canParse(path) ? path : `${this.base}${path}`;
        const request = new Request(url, {
            method,
            headers: {
                ...(method === "GET" ? {} : { origin: new URL(this.base).origin }),
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
            // set any other
            else {
                this.cookies.set(name, value);
            }
        }

        return response;
    }

    /** Decide the stored request a consent view shows, answering where the browser goes next. */
    async consent(view: string, decision: "approve" | "deny"): Promise<string> {
        const requestUri = present(
            new URL(view).searchParams.get("request_uri"),
            "the request's reference",
        );
        const decided = await this.fetch(AUTHORIZATION_REQUEST_PATH, { requestUri, decision });

        return Decided.parse(await decided.json()).location;
    }

    /** Decide a device's pending sign-in by its code, as the verification view does. */
    async verify(userCode: string, decision: "approve" | "deny"): Promise<Response> {
        return this.fetch(DEVICE_REQUEST_PATH, { userCode, decision });
    }

    /** Sign in by a mailed code as the views do, reporting whether the second step is due. */
    async signIn(
        email: string,
        code: () => Promise<string>,
        signUp: Readonly<Record<string, string>> = {},
    ): Promise<boolean> {
        // mail a code, then sign in with it, signing a new person up with the fields
        await this.fetch("/email-otp/send-verification-otp", { email, type: "sign-in" });
        const signedIn = await this.fetch("/sign-in/email-otp", {
            email,
            otp: await code(),
            ...signUp,
        });

        return SignedIn.parse(await signedIn.json()).twoFactorRedirect === true;
    }

    /** Follow the issuer until it leaves, signing in and consenting on its views. */
    async authorize(
        url: string,
        person: {
            /** The email the code goes to. */
            readonly email: string;
            /** Read the code the issuer mailed last. */
            readonly code: () => Promise<string>;
            /** Read the authenticator's current code, absent without a second factor. */
            readonly authenticator?: () => Promise<string>;
        },
        isLeft: (url: URL) => boolean,
    ): Promise<string> {
        let next = url;
        for (let passed = 0; !isLeft(new URL(next)); passed += 1) {
            // refuse an issuer that never leaves
            if (passed >= REDIRECTS) {
                throw new TypeError(`the sign-in redirected more than ${REDIRECTS} times`);
            }

            // sign in on a sign-in view, approve a consent view, and follow any other page
            const page = new URL(next);
            if (page.pathname.endsWith("/sign-in")) {
                next = await this.#continue(page, person);
            } else if (page.pathname.endsWith("/consent")) {
                next = await this.consent(next, "approve");
            } else {
                next = TestBrowser.location(await this.fetch(next));
            }
        }

        return next;
    }

    /** Sign in on a sign-in view, answering the stored request it continues. */
    async #continue(view: URL, person: Parameters<TestBrowser["authorize"]>[1]): Promise<string> {
        // continue the stored request the view carries, as the sign-in view does
        const continuation = new URL(`${this.base}${AUTHORIZATION_PATH}`);
        for (const name of ["client_id", "request_uri"]) {
            continuation.searchParams.set(
                name,
                present(view.searchParams.get(name), `the view's ${name}`),
            );
        }

        // sign in by the mailed code, answering the second step with the authenticator's code where due
        const isChallenged = await this.signIn(person.email, person.code);
        if (isChallenged && person.authenticator !== undefined) {
            await this.fetch("/two-factor/verify-totp", { code: await person.authenticator() });
        }

        return continuation.href;
    }
}
