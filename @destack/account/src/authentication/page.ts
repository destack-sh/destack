import type { AuthContext } from "better-auth";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { connect } from "../client/client.ts";
import type { HandleAvailability } from "../object/handle.ts";
import { RESIDENCIES, type Residency } from "../object/region.ts";

/** The query parameters Better Auth adds to the sign-in page after a failed sign-in. */
const ERROR_PARAMETERS = ["error", "error_description"];

/** The headers of every page: HTML without scripts, never framed. */
const HEADERS = {
    "content-type": "text/html; charset=utf-8",
    "content-security-policy":
        "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'",
    "referrer-policy": "same-origin",
};

/** The stylesheet every page shares. */
const STYLE = `
:root { color-scheme: light dark; font-family: system-ui, sans-serif; line-height: 1.5; }
body { margin: 0; min-height: 100vh; display: grid; place-items: center; }
main { width: min(100% - 2rem, 24rem); }
form { display: grid; gap: 0.75rem; margin-block: 1.5rem; }
input, button { font: inherit; padding: 0.625rem 0.75rem; border-radius: 0.375rem; }
input { border: 1px solid GrayText; }
button { border: 0; background: CanvasText; color: Canvas; cursor: pointer; }
[role="alert"] { color: #d92d20; }
`;

/** The HTML escapes of the characters markup gives meaning to. */
const ESCAPES: Readonly<Record<string, string>> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
};

/** The names people read for each residency. */
const RESIDENCY_NAMES: Readonly<Record<Residency, string>> = {
    eu: "European Union",
    us: "United States",
};

/** The reasons a typed handle cannot be claimed, with their statuses. */
const REFUSALS: Readonly<Record<Exclude<HandleAvailability, "available">, [string, number]>> = {
    taken: ["This handle is taken.", 409],
    invalid: ["A handle has 1 to 63 lowercase letters, digits and inner hyphens.", 400],
};

/** A Better Auth answer with the URL the browser goes to next. */
const Redirection = schema.object({ url: schema.string().min(1) }).passthrough();

/** A Better Auth or OAuth error answer. */
const Failure = schema
    .object({
        message: schema.string().optional(),
        error_description: schema.string().optional(),
    })
    .passthrough();

/** A Better Auth answer describing an OAuth client. */
const Client = schema.object({ client_name: schema.string() }).passthrough();

/** A Better Auth answer describing a device code's sign-in. */
const DeviceReview = schema
    .object({
        status: schema.enum(["pending", "approved", "denied"]),
        client_id: schema.string(),
        scope: schema.string().nullable(),
    })
    .passthrough();

/** A Better Auth answer to a first-factor sign-in. */
const SignedIn = schema.object({ twoFactorRedirect: schema.boolean().optional() }).passthrough();

/** A Better Auth answer with the browser's session, null when signed out. */
const Session = schema
    .object({ user: schema.object({ email: schema.string() }).passthrough() })
    .passthrough()
    .nullable();

/** The values of the handle form. */
interface Choice {
    /** The display name. */
    readonly name: string;
    /** The handle. */
    readonly handle: string;
    /** The selected residency, null before a selection. */
    readonly residency: Residency | null;
}

/** The issuer's hosted sign-in, consent, device verification, second-factor and handle pages. */
export class SignInPages {
    /** The URL of the Better Auth routes, the origin and base path. */
    readonly endpoint: string;
    /** The page where people sign in. */
    readonly signIn: URL;
    /** The page where people approve another application's access. */
    readonly consent: URL;
    /** The page where people approve a device's sign-in. */
    readonly verification: URL;
    /** The page where people complete a second factor. */
    readonly secondFactor: URL;
    /** The page where people choose their handle after their first sign-in. */
    readonly handle: URL;
    /** The Better Auth context, with its trusted origins and social providers. */
    readonly context: Promise<Pick<AuthContext, "isTrustedOrigin" | "socialProviders">>;
    /** Send a request to the Better Auth routes. */
    readonly send: (request: Request) => Promise<Response>;
    /** Send a request to the account service in process. */
    readonly service: (request: Request) => Promise<Response>;

    /** Serve the pages at their URLs over the Better Auth routes. */
    constructor(
        options: Pick<
            SignInPages,
            | "endpoint"
            | "signIn"
            | "consent"
            | "verification"
            | "secondFactor"
            | "handle"
            | "context"
            | "send"
            | "service"
        >,
    ) {
        // keep the routes the pages call
        this.endpoint = options.endpoint;
        this.context = options.context;
        this.send = options.send;
        this.service = options.service;

        // keep the page URLs
        this.signIn = options.signIn;
        this.consent = options.consent;
        this.verification = options.verification;
        this.secondFactor = options.secondFactor;
        this.handle = options.handle;
    }

    /** Report whether a path is one of the pages. */
    handles(path: string): boolean {
        return [this.signIn, this.consent, this.verification, this.secondFactor, this.handle].some(
            (page) => page.pathname === path,
        );
    }

    /** Show a page, or submit its form. */
    async serve(request: Request): Promise<Response> {
        // read the query the page carries along, without a failed sign-in's error
        const url = new URL(request.url);
        const query = new URLSearchParams(url.search);
        const error = query.get("error");
        for (const parameter of ERROR_PARAMETERS) {
            query.delete(parameter);
        }

        // refuse a callback outside the trusted origins
        const callback = query.get("callbackURL");
        const context = await this.context;
        if (callback !== null && !context.isTrustedOrigin(callback, { allowRelativePaths: true })) {
            return render("Sign-in failed", alert("The callback URL is not trusted."), 400);
        }

        // refuse methods forms do not use
        const isSubmit = request.method === "POST";
        if (!isSubmit && request.method !== "GET") {
            return new Response(null, { status: 405, headers: { allow: "GET, POST" } });
        }

        // sign in
        if (url.pathname === this.signIn.pathname) {
            return isSubmit
                ? this.#submitSignIn(request, query)
                : this.#showSignIn(request, query, error);
        }
        // complete the second factor
        else if (url.pathname === this.secondFactor.pathname) {
            return isSubmit
                ? this.#submitSecondFactor(request, query)
                : this.#secondFactorForm(query, html``, 200);
        }
        // choose a handle
        else if (url.pathname === this.handle.pathname) {
            return isSubmit ? this.#submitHandle(request, query) : this.#showHandle(request, query);
        }
        // consent to an application
        else if (url.pathname === this.consent.pathname) {
            return isSubmit
                ? this.#submitConsent(request, query)
                : this.#showConsent(request, query);
        }
        // approve a device
        else {
            return isSubmit
                ? this.#submitVerification(request, query)
                : this.#showVerification(request, query);
        }
    }

    /** Show the sign-in form, or continue a signed-in person. */
    async #showSignIn(
        request: Request,
        query: URLSearchParams,
        error: string | null,
    ): Promise<Response> {
        // ask a signed-out person for their email
        const session = await this.#session(request);
        if (session === null) {
            const failed = error === null ? html`` : alert(`Sign-in failed: ${error}.`);

            return this.#signInForm(query, failed, 200);
        }

        // choose a handle before anything else
        const { profile } = await this.#reader(request).current();
        const signedIn = html`<p>Signed in as <strong>${session.user.email}</strong>.</p>`;
        if (profile.handle === null) {
            return redirect(target(this.handle, query), []);
        }
        // continue the provider's authorization as the signed-in person
        else if (query.has("sig")) {
            return render(
                "Continue",
                html`${signedIn}
                    <form method="post" action="${target(this.signIn, query)}">
                        <button name="action" value="continue">Continue</button>
                    </form>`,
            );
        }
        // return a signed-in person to the callback
        else if (query.has("callbackURL")) {
            return redirect(query.get("callbackURL")!, []);
        }
        // confirm the sign-in
        else {
            return render("Signed in", signedIn);
        }
    }

    /** Submit an email, a code, a provider or the continuation of a sign-in. */
    async #submitSignIn(request: Request, query: URLSearchParams): Promise<Response> {
        // read the chosen action
        const form = await request.formData();
        const action = field(form, "action");

        // email a sign-in link returning to the continuation
        if (action === "link") {
            const email = field(form, "email");
            const sent = await this.#forward(request, "/sign-in/magic-link", {
                email,
                callbackURL: linked(target(this.signIn, query)),
                errorCallbackURL: linked(target(this.signIn, query)),
            });
            if (!sent.ok) {
                return this.#signInForm(query, alert(await failure(sent)), sent.status);
            }

            return render(
                "Check your email",
                html`<p>We sent a sign-in link to <strong>${email}</strong>.</p>`,
            );
        }
        // email a one-use code
        else if (action === "code") {
            const email = field(form, "email");
            const sent = await this.#forward(request, "/email-otp/send-verification-otp", {
                email,
                type: "sign-in",
            });
            if (!sent.ok) {
                return this.#signInForm(query, alert(await failure(sent)), sent.status);
            }

            return this.#codeForm(query, email, html``, 200);
        }
        // sign in with the emailed code
        else if (action === "verify") {
            const email = field(form, "email");
            const signedIn = await this.#forward(request, "/sign-in/email-otp", {
                email,
                otp: field(form, "otp"),
            });
            if (!signedIn.ok) {
                return this.#codeForm(
                    query,
                    email,
                    alert(await failure(signedIn)),
                    signedIn.status,
                );
            }

            return this.#complete(signedIn, query);
        }
        // sign in with a social provider
        else if (action === "provider") {
            const started = await this.#forward(request, "/sign-in/social", {
                provider: field(form, "provider"),
                callbackURL: target(this.signIn, query),
                errorCallbackURL: target(this.signIn, query),
            });
            if (!started.ok) {
                return this.#signInForm(query, alert(await failure(started)), started.status);
            }
            const { url } = Redirection.parse(await started.json());

            return redirect(url, started.headers.getSetCookie());
        }
        // continue the provider's authorization
        else if (action === "continue") {
            const continued = await this.#forward(request, "/oauth2/continue", {
                selected: true,
                oauth_query: query.toString(),
            });
            if (!continued.ok) {
                return render("Sign-in failed", alert(await failure(continued)), continued.status);
            }
            const { url } = Redirection.parse(await continued.json());

            return redirect(url, continued.headers.getSetCookie());
        }
        // refuse an action the form does not offer
        else {
            throw new ServiceError("BAD_REQUEST", { message: `unknown sign-in action: ${action}` });
        }
    }

    /** Submit a code of the enrolled authenticator or a backup code. */
    async #submitSecondFactor(request: Request, query: URLSearchParams): Promise<Response> {
        // choose the endpoint of the code's kind
        const form = await request.formData();
        const action = field(form, "action");
        let path: string;
        // verify an authenticator code
        if (action === "totp") {
            path = "/two-factor/verify-totp";
        }
        // verify a backup code
        else if (action === "backup") {
            path = "/two-factor/verify-backup-code";
        }
        // refuse an action the form does not offer
        else {
            throw new ServiceError("BAD_REQUEST", {
                message: `unknown second-factor action: ${action}`,
            });
        }

        // verify the code, returning to the continuation
        const verified = await this.#forward(request, path, { code: field(form, "code") });
        if (!verified.ok) {
            return this.#secondFactorForm(query, alert(await failure(verified)), verified.status);
        }

        return redirect(parameter(query, "callbackURL"), verified.headers.getSetCookie());
    }

    /** Show the application and the scopes it asks for, or the native client asking to sign in. */
    async #showConsent(request: Request, query: URLSearchParams): Promise<Response> {
        // ask to approve a native client's sign-in on this device
        if (query.get("flow") === "native") {
            const session = await this.#session(request);

            return render(
                "Sign in on this device",
                html`<p>
                        The Destack app on this device asks to sign in as
                        <strong>${session?.user.email ?? "you"}</strong>.
                    </p>
                    <p>Allow it only if you started this sign-in yourself.</p>
                    <form method="post" action="${target(this.consent, query)}">
                        <button name="action" value="approve">Allow</button>
                        <button name="action" value="deny">Deny</button>
                    </form>`,
            );
        }

        // read the application's public name
        const client = await this.#forward(
            request,
            `/oauth2/public-client?${new URLSearchParams({ client_id: parameter(query, "client_id") })}`,
        );
        if (!client.ok) {
            return render("Authorization failed", alert(await failure(client)), client.status);
        }
        const { client_name: name } = Client.parse(await client.json());

        // ask to approve or deny the scopes
        return render(
            "Authorize",
            html`<p><strong>${name}</strong> asks to:</p>
                ${list(parameter(query, "scope"))}
                <form method="post" action="${target(this.consent, query)}">
                    <button name="action" value="approve">Allow</button>
                    <button name="action" value="deny">Deny</button>
                </form>`,
        );
    }

    /** Approve or deny the scopes or the native sign-in, returning to the application. */
    async #submitConsent(request: Request, query: URLSearchParams): Promise<Response> {
        // decide on the native sign-in, or on the scopes of the signed query
        const form = await request.formData();
        const accept = approval(field(form, "action"));
        const isNative = query.get("flow") === "native";
        const native = new URLSearchParams(query);
        native.delete("flow");
        const consented = isNative
            ? await this.#forward(request, "/native/approve", {
                  accept,
                  query: native.toString(),
              })
            : await this.#forward(request, "/oauth2/consent", {
                  accept,
                  oauth_query: query.toString(),
              });
        if (!consented.ok) {
            return render(
                "Authorization failed",
                alert(await failure(consented)),
                consented.status,
            );
        }
        const { url } = Redirection.parse(await consented.json());

        return redirect(url, consented.headers.getSetCookie());
    }

    /** Ask for a device's code, then show the sign-in it requests. */
    async #showVerification(request: Request, query: URLSearchParams): Promise<Response> {
        // ask for the code the device shows
        const code = query.get("user_code");
        if (code === null) {
            return render(
                "Approve a device",
                html`<form method="get" action="${this.verification.pathname}">
                    <label for="user_code">Code shown on your device</label>
                    <input id="user_code" name="user_code" autocomplete="off" required autofocus />
                    <button>Continue</button>
                </form>`,
            );
        }

        // sign in first, returning here
        const session = await this.#session(request);
        if (session === null) {
            const here = new URLSearchParams({ callbackURL: target(this.verification, query) });

            return redirect(target(this.signIn, here), []);
        }

        // read the sign-in the code requests
        const reviewed = await this.#forward(
            request,
            `/device?${new URLSearchParams({ user_code: code })}`,
        );
        if (!reviewed.ok) {
            return render("Approval failed", alert(await failure(reviewed)), reviewed.status);
        }
        const device = DeviceReview.parse(await reviewed.json());

        // ask to approve a pending sign-in
        if (device.status === "pending") {
            const scopes = device.scope === null ? html`` : list(device.scope);

            return render(
                "Approve a device",
                html`<p>
                        Sign in to <strong>${device.client_id}</strong> with the code
                        <strong>${code}</strong>?
                    </p>
                    ${scopes}
                    <form method="post" action="${target(this.verification, query)}">
                        <button name="action" value="approve">Approve</button>
                        <button name="action" value="deny">Deny</button>
                    </form>`,
            );
        }

        return decided(device.status);
    }

    /** Approve or deny a device's sign-in. */
    async #submitVerification(request: Request, query: URLSearchParams): Promise<Response> {
        // decide on the sign-in of the code
        const form = await request.formData();
        const isApproved = approval(field(form, "action"));
        const decision = await this.#forward(
            request,
            isApproved ? "/device/approve" : "/device/deny",
            {
                userCode: parameter(query, "user_code"),
            },
        );
        if (!decision.ok) {
            return render("Approval failed", alert(await failure(decision)), decision.status);
        }

        return decided(isApproved ? "approved" : "denied");
    }

    /** Answer the sign-in form. */
    async #signInForm(query: URLSearchParams, notice: Markup, status: number): Promise<Response> {
        // offer the configured social providers
        const { socialProviders } = await this.context;
        const providers =
            socialProviders.length === 0
                ? html``
                : html`<form method="post" action="${target(this.signIn, query)}">
                      <input type="hidden" name="action" value="provider" />
                      ${socialProviders.map(
                          (provider) =>
                              html`<button name="provider" value="${provider.id}">
                                  Continue with ${provider.name}
                              </button>`,
                      )}
                  </form>`;

        return render(
            "Sign in to Destack",
            html`${notice}
                <form method="post" action="${target(this.signIn, query)}">
                    <label for="email">Email</label>
                    <input
                        id="email"
                        name="email"
                        type="email"
                        autocomplete="email"
                        required
                        autofocus
                    />
                    <button name="action" value="link">Email me a sign-in link</button>
                    <button name="action" value="code">Email me a code</button>
                </form>
                ${providers}`,
            status,
        );
    }

    /** Answer the form for an emailed code. */
    #codeForm(query: URLSearchParams, email: string, notice: Markup, status: number): Response {
        return render(
            "Enter your code",
            html`${notice}
                <p>We sent a code to <strong>${email}</strong>.</p>
                <form method="post" action="${target(this.signIn, query)}">
                    <input type="hidden" name="email" value="${email}" />
                    <label for="otp">Code</label>
                    <input
                        id="otp"
                        name="otp"
                        inputmode="numeric"
                        autocomplete="one-time-code"
                        required
                        autofocus
                    />
                    <button name="action" value="verify">Sign in</button>
                </form>`,
            status,
        );
    }

    /** Answer the second-factor form. */
    #secondFactorForm(query: URLSearchParams, notice: Markup, status: number): Response {
        return render(
            "Verify it is you",
            html`${notice}
                <form method="post" action="${target(this.secondFactor, query)}">
                    <label for="code">Code from your authenticator app, or a backup code</label>
                    <input id="code" name="code" autocomplete="one-time-code" required autofocus />
                    <button name="action" value="totp">Verify</button>
                    <button name="action" value="backup">Use backup code</button>
                </form>`,
            status,
        );
    }

    /** Suggest a handle to a signed-in person who has none. */
    async #showHandle(request: Request, query: URLSearchParams): Promise<Response> {
        // sign in first, returning here through the sign-in page
        const session = await this.#session(request);
        if (session === null) {
            return redirect(target(this.signIn, query), []);
        }

        // continue a person who chose a handle
        const reader = this.#reader(request);
        const { subject, profile } = await reader.current();
        if (profile.handle !== null) {
            return redirect(target(this.signIn, query), []);
        }

        // prefill the name, the suggested handle and a chosen residency
        const person = await reader.user(subject.id);
        const { handle } = await reader.suggestHandle(subject.id);
        const chosen = person.residency ?? null;
        const choice = { name: person.name, handle, residency: chosen };

        return this.#handleForm(query, choice, chosen, html``, 200);
    }

    /** Claim the typed handle with the personal account, after choosing the residency. */
    async #submitHandle(request: Request, query: URLSearchParams): Promise<Response> {
        // read the choice of the signed-in person
        const form = await request.formData();
        const account = this.#account(request);
        const { subject } = await account.authentication.current();
        const person = await account.user.get({ id: subject.id });
        const chosen = person.residency ?? null;
        const residency = chosen ?? parseResidency(field(form, "residency"));
        const choice = { name: field(form, "name"), handle: field(form, "handle"), residency };

        // refuse a taken or invalid handle with its reason
        const { availability } = await account.user.checkHandle({
            id: subject.id,
            handle: choice.handle,
        });
        if (availability !== "available") {
            const [reason, status] = REFUSALS[availability];

            return this.#handleForm(query, choice, chosen, alert(reason), status);
        }

        // set the person's name and the residency, chosen once
        await account.user.update({
            id: subject.id,
            requestId: RequestId.create(),
            revision: person.revision,
            name: choice.name,
            residency,
        });

        // claim the handle with the personal account, then continue
        await account.account.create({
            kind: "personal",
            scope: subject.id,
            requestId: RequestId.create(),
            handle: choice.handle,
            name: choice.name,
        });

        return redirect(target(this.signIn, query), []);
    }

    /** Answer the handle form. */
    #handleForm(
        query: URLSearchParams,
        choice: Choice,
        chosen: Residency | null,
        notice: Markup,
        status: number,
    ): Response {
        // ask for a residency until one is chosen and keep the selection
        const options = RESIDENCIES.map((value) =>
            value === choice.residency
                ? html`<option value="${value}" selected>${RESIDENCY_NAMES[value]}</option>`
                : html`<option value="${value}">${RESIDENCY_NAMES[value]}</option>`,
        );
        const residency =
            chosen === null
                ? html`<label for="residency">Where your data lives</label>
                      <select id="residency" name="residency" required>
                          ${options}
                      </select>`
                : html``;

        return render(
            "Choose your handle",
            html`${notice}
                <form method="post" action="${target(this.handle, query)}">
                    <label for="name">Name</label>
                    <input
                        id="name"
                        name="name"
                        autocomplete="name"
                        required
                        value="${choice.name}"
                    />
                    <label for="handle">Handle</label>
                    <input
                        id="handle"
                        name="handle"
                        autocapitalize="none"
                        spellcheck="false"
                        required
                        value="${choice.handle}"
                    />
                    ${residency}
                    <button>Continue</button>
                </form>`,
            status,
        );
    }

    /** Return a first-factor sign-in to its continuation, through the second factor when enrolled. */
    async #complete(signedIn: Response, query: URLSearchParams): Promise<Response> {
        // read the outcome and the session's cookies
        const body = SignedIn.parse(await signedIn.json());
        const cookies = signedIn.headers.getSetCookie();

        // ask for the enrolled second factor
        if (body.twoFactorRedirect === true) {
            const next = new URLSearchParams({ callbackURL: target(this.signIn, query) });

            return redirect(target(this.secondFactor, next), cookies);
        }

        return redirect(target(this.signIn, query), cookies);
    }

    /** Read the browser's session. */
    async #session(request: Request): Promise<{ user: { email: string } } | null> {
        const current = await this.#forward(request, "/get-session");

        // treat a revoked credential as signed out
        if (current.status === 401) {
            return null;
        }
        // refuse any other failure
        else if (!current.ok) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `session read failed: ${await failure(current)}`,
            });
        }

        return Session.parse(await current.json());
    }

    /** Connect to the account service as the browser's session, for a form the browser posted with its origin. */
    #account(request: Request) {
        // keep the browser's headers without its body's
        const headers = new Headers(request.headers);
        headers.delete("content-length");
        headers.delete("content-type");
        headers.delete("accept");

        return connect({
            url: this.signIn.origin,
            headers: () => Object.fromEntries(headers),
            fetch: (sent) => this.service(sent),
        });
    }

    /** Read the signed-in person as the page itself, on a navigation that carries no origin. */
    #reader(request: Request) {
        // read as the page's own origin, exposing only reads
        const headers = new Headers(request.headers);
        headers.delete("content-length");
        headers.delete("content-type");
        headers.delete("accept");
        headers.set("origin", this.signIn.origin);
        const account = connect({
            url: this.signIn.origin,
            headers: () => Object.fromEntries(headers),
            fetch: (sent) => this.service(sent),
        });

        return {
            current: () => account.authentication.current(),
            user: (id: string) => account.user.get({ id }),
            suggestHandle: (id: string) => account.user.suggestHandle({ id }),
        };
    }

    /** Send a browser's form to a Better Auth route as JSON, with its cookies, origin and address. */
    #forward(request: Request, path: string, body?: object): Promise<Response> {
        // keep the browser's headers, asking for JSON
        const headers = new Headers(request.headers);
        headers.delete("content-length");
        headers.set("accept", "application/json");
        headers.set("content-type", "application/json");

        return this.send(
            new Request(`${this.endpoint}${path}`, {
                method: body === undefined ? "GET" : "POST",
                headers,
                body: body === undefined ? undefined : JSON.stringify(body),
            }),
        );
    }
}

/** HTML the pages wrote or escaped. */
class Markup {
    /** The HTML text. */
    readonly text: string;

    /** Hold HTML text. */
    constructor(text: string) {
        this.text = text;
    }
}

/** Build markup, escaping every interpolated value that is no markup. */
function html(
    strings: TemplateStringsArray,
    ...values: readonly (string | Markup | readonly Markup[])[]
): Markup {
    let text = strings[0]!;
    for (const [index, value] of values.entries()) {
        const written =
            value instanceof Markup
                ? value.text
                : typeof value === "string"
                  ? escape(value)
                  : value.map((item) => item.text).join("");
        text += written + strings[index + 1];
    }

    return new Markup(text);
}

/** Escape a value for HTML text and attributes. */
function escape(value: string): string {
    return value.replace(/[&<>"']/g, (character) => ESCAPES[character]!);
}

/** Announce a message to the person. */
function alert(message: string): Markup {
    return html`<p role="alert">${message}</p>`;
}

/** Answer a page. */
function render(
    title: string,
    main: Markup,
    status = 200,
    cookies: readonly string[] = [],
): Response {
    // write the document
    const document = html`<!doctype html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>${title}</title>
                <style>
                    ${new Markup(STYLE)}
                </style>
            </head>
            <body>
                <main>
                    <h1>${title}</h1>
                    ${main}
                </main>
            </body>
        </html>`;

    // keep the cookies the routes set
    const headers = new Headers(HEADERS);
    for (const cookie of cookies) {
        headers.append("set-cookie", cookie);
    }

    return new Response(document.text, { status, headers });
}

/** Send the browser on after a form with the cookies the routes set. */
function redirect(location: string, cookies: readonly string[]): Response {
    const headers = new Headers({ location });
    for (const cookie of cookies) {
        headers.append("set-cookie", cookie);
    }

    return new Response(null, { status: 303, headers });
}

/** Answer a decided device sign-in. */
function decided(status: "approved" | "denied"): Response {
    return status === "approved"
        ? render("Device approved", html`<p>Return to your device to continue.</p>`)
        : render("Device denied", html`<p>The device was not signed in.</p>`);
}

/** List the space-separated scopes of a request. */
function list(scope: string): Markup {
    return html`<ul>
        ${scope.split(" ").map((name) => html`<li><code>${name}</code></li>`)}
    </ul>`;
}

/** Write the path and query of a page. */
function target(page: URL, query: URLSearchParams): string {
    return query.size === 0 ? page.pathname : `${page.pathname}?${query}`;
}

// NOTE #Suspicious: Better Auth's magic link decodes its callbacks twice and splits nested queries
/** Escape a URL's percent signs against the magic link's second decoding. */
function linked(url: string): string {
    return url.replaceAll("%", "%25");
}

/** Read a chosen residency. */
function parseResidency(value: string): Residency {
    const residency = RESIDENCIES.find((known) => known === value);
    if (residency === undefined) {
        throw new ServiceError("BAD_REQUEST", { message: `unknown residency: ${value}` });
    }

    return residency;
}

/** Read whether a form approves or denies. */
function approval(action: string): boolean {
    // approve
    if (action === "approve") {
        return true;
    }
    // deny
    else if (action === "deny") {
        return false;
    }
    // refuse any other action
    else {
        throw new ServiceError("BAD_REQUEST", { message: `unknown approval action: ${action}` });
    }
}

/** Read a text field of a submitted form. */
function field(form: { get(name: string): unknown }, name: string): string {
    const value = form.get(name);
    if (typeof value !== "string") {
        throw new ServiceError("BAD_REQUEST", { message: `missing form field: ${name}` });
    }

    return value;
}

/** Read a query parameter a page requires. */
function parameter(query: URLSearchParams, name: string): string {
    const value = query.get(name);
    if (value === null) {
        throw new ServiceError("BAD_REQUEST", { message: `missing query parameter: ${name}` });
    }

    return value;
}

/** Read the message of a failed Better Auth response. */
async function failure(response: Response): Promise<string> {
    const body = Failure.parse(await response.json());

    // read a Better Auth error's message
    if (body.message !== undefined) {
        return body.message;
    }
    // read an OAuth error's description
    else if (body.error_description !== undefined) {
        return body.error_description;
    }
    // refuse an error without either
    else {
        throw new ServiceError("INTERNAL_SERVER_ERROR", {
            message: `authentication failed without a message: ${response.status}`,
        });
    }
}
