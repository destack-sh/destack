import type { Fetch } from "@destack/object";
import type { AuthContext } from "better-auth";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { CallError, claimTable } from "@destack/object";
import { present, schema } from "@destack/schema";
import { alert, html, type Markup, redirect, render, timeZoneField } from "./markup.ts";
import { SignUp } from "./sign-up.ts";
import { Account } from "../object/account.ts";
import { HANDLE_LENGTH, type HandleAvailability } from "../address/handle.ts";
import { DisplayName, NAME_LENGTH } from "../object/display-name.ts";
import { Residency, type ResidencyCode } from "../object/residency.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";
import { AUTHORIZATION_PATH } from "../authentication/path.ts";
import type { AuthorizationServer } from "../oauth/server.ts";
import { Decision } from "../oauth/authorization.ts";

/** The query parameters Better Auth adds to the sign-in page after a failed sign-in. */
const ERROR_PARAMETERS = ["error", "error_description"];

/** The refusal messages and statuses of a typed handle that is taken or invalid. */
const HANDLE_REFUSALS: Readonly<
    Record<Exclude<HandleAvailability, "available">, [string, number]>
> = {
    taken: ["This handle is taken.", 409],
    invalid: [
        `A handle has 1 to ${HANDLE_LENGTH} lowercase letters, digits and single hyphens between them.`,
        400,
    ],
};

/** The refusal message of a typed name that is empty or too long. */
const NAME_REFUSAL = `A name has 1 to ${NAME_LENGTH} characters.`;

/** A Better Auth answer with the URL the browser goes to next. */
const Redirection = schema.looseObject({ url: schema.string().min(1) });

/** A Better Auth or OAuth error answer. */
const Failure = schema.looseObject({
    message: schema.string().exactOptional(),
    error_description: schema.string().exactOptional(),
    error: schema.string().exactOptional(),
});

/** A Better Auth answer to a first-factor sign-in. */
const SignedIn = schema.looseObject({ twoFactorRedirect: schema.boolean().exactOptional() });

/** A Better Auth answer with the browser's session, null when signed out. */
const Session = schema
    .looseObject({
        user: schema.looseObject({ email: schema.string() }),
        session: schema.looseObject({ createdAt: schema.string() }),
    })
    .nullable();

/** An emailed sign-in: the address, the form that asked for it and the sign-up it includes. */
interface EmailSignIn {
    /** The email the link or code goes to. */
    readonly email: string;
    /** The page whose form asked for it. */
    readonly page: URL;
    /** The sign-up of a new person, absent for a sign-in. */
    readonly signUp?: SignUp;
    /** Answer the asking form again with a notice. */
    readonly refuse: (notice: Markup, status: number) => Promise<Response>;
}

/** The values of the sign-up form. */
interface SignUpForm {
    /** The email. */
    readonly email: string;
    /** The display name. */
    readonly name: string;
    /** The handle. */
    readonly handle: string;
    /** The selected residency, null before a selection. */
    readonly residency: ResidencyCode | null;
}

/** The issuer's hosted sign-in, sign-up, consent, device verification and second-factor pages. */
export class SignInPage {
    /** The URL of the Better Auth routes, the origin and base path. */
    readonly endpoint: string;
    /** The page where people sign in. */
    readonly signIn: URL;
    /** The page where people sign up with their name, handle and residency. */
    readonly signUp: URL;
    /** The page where people approve another application's access. */
    readonly consent: URL;
    /** The page where people approve a device's sign-in. */
    readonly verification: URL;
    /** The page where people complete a second factor. */
    readonly secondFactor: URL;
    /** The Better Auth context, with its trusted origins and social providers. */
    readonly context: Promise<Pick<AuthContext, "isTrustedOrigin" | "socialProviders">>;
    /** Send a request to the Better Auth routes. */
    readonly send: Fetch;
    /** The account database with the residencies and the handles accounts claim. */
    readonly database: DatabaseConnection;
    /** The authorization server, whose requests and device sign-ins the pages decide. */
    readonly server: AuthorizationServer;

    /** Serve the pages at their URLs over the Better Auth routes. */
    constructor(
        options: Pick<
            SignInPage,
            | "endpoint"
            | "signIn"
            | "signUp"
            | "consent"
            | "verification"
            | "secondFactor"
            | "context"
            | "send"
            | "database"
            | "server"
        >,
    ) {
        // store the routes and the database the pages read
        this.endpoint = options.endpoint;
        this.context = options.context;
        this.send = options.send;
        this.database = options.database;
        this.server = options.server;

        // store the page URLs
        this.signIn = options.signIn;
        this.signUp = options.signUp;
        this.consent = options.consent;
        this.verification = options.verification;
        this.secondFactor = options.secondFactor;
    }

    /** Report whether a path is one of the pages. */
    handles(path: string): boolean {
        return [this.signIn, this.signUp, this.consent, this.verification, this.secondFactor].some(
            (page) => page.pathname === path,
        );
    }

    /** Show a page, or submit its form. */
    async serve(request: Request): Promise<Response> {
        // read the query the page passes along
        const url = new URL(request.url);
        const query = new URLSearchParams(url.search);
        const error = query.get("error");
        for (const name of ERROR_PARAMETERS) {
            query.delete(name);
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
            const answered = isSubmit
                ? this.#submitSignIn(request, query)
                : this.#showSignIn(request, query, error);

            return answered.catch((refused: unknown) => refusal("Sign-in failed", refused));
        }
        // sign up
        else if (url.pathname === this.signUp.pathname) {
            return isSubmit
                ? this.#submitSignUp(request, query)
                : this.#showSignUp(request, query, error);
        }
        // complete the second factor
        else if (url.pathname === this.secondFactor.pathname) {
            return isSubmit
                ? this.#submitSecondFactor(request, query)
                : this.#secondFactorForm(query, html``, 200);
        }
        // consent to an application
        else if (url.pathname === this.consent.pathname) {
            const answered = isSubmit
                ? this.#submitConsent(request, query)
                : this.#showConsent(request, query);

            return answered.catch((refused: unknown) => refusal("Authorization failed", refused));
        }
        // approve a device
        else {
            const answered = isSubmit
                ? this.#submitVerification(request, query)
                : this.#showVerification(request, query);

            return answered.catch((refused: unknown) => refusal("Approval failed", refused));
        }
    }

    /** Show the sign-in form, or continue a signed-in person. */
    async #showSignIn(
        request: Request,
        query: URLSearchParams,
        error: string | null,
    ): Promise<Response> {
        // ask for the email of a signed-out person, and of one an authorization asks to sign in again
        const session = await this.#session(request);
        const requestUri = query.get("request_uri");
        const review =
            requestUri === null
                ? undefined
                : await this.server.authorization.review(requestUri, request.headers);
        if (session === null || review?.isLoginDue === true) {
            const notice = error === null ? html`` : alert(`Sign-in failed: ${error}.`);

            return this.#signInForm(query, notice, 200);
        }

        // continue an authorization as the signed-in person
        const greeting = html`<p>Signed in as <strong>${session.user.email}</strong>.</p>`;
        if (review !== undefined) {
            return redirect(this.#continuation(query), []);
        }
        // return a signed-in person to the callback
        else if (query.has("callbackURL")) {
            return redirect(present(query.get("callbackURL"), "the callback URL"), []);
        }
        // confirm the sign-in
        else {
            return render("Signed in", greeting);
        }
    }

    /** Submit an email, a code or a provider of a sign-in. */
    async #submitSignIn(request: Request, query: URLSearchParams): Promise<Response> {
        // read the chosen action
        const form = await request.formData();
        const action = field(form, "action");
        const emailed = (): EmailSignIn => ({
            email: field(form, "email"),
            page: this.signIn,
            refuse: (notice, status) => this.#signInForm(query, notice, status),
        });

        // email a sign-in link returning to the continuation
        if (action === "link") {
            return this.#sendLink(request, query, emailed());
        }
        // email a one-use code
        else if (action === "code") {
            return this.#sendCode(request, query, emailed());
        }
        // sign in with the emailed code
        else if (action === "verify") {
            return this.#verifyCode(request, query, emailed(), field(form, "otp"));
        }
        // sign in with a social provider
        else if (action === "provider") {
            return this.#startProvider(request, query, field(form, "provider"));
        }
        // refuse an action the form does not offer
        else {
            throw AccountError.of("ACTION_UNKNOWN", `unknown sign-in action: ${action}`);
        }
    }

    /** Email a sign-in link returning to the continuation, with a new person's sign-up. */
    async #sendLink(
        request: Request,
        query: URLSearchParams,
        emailed: EmailSignIn,
    ): Promise<Response> {
        // send the link
        const response = await this.#forward(request, "/sign-in/magic-link", {
            email: emailed.email,
            callbackURL: escapeLink(target(this.signIn, query)),
            errorCallbackURL: escapeLink(target(emailed.page, query)),
            ...(emailed.signUp === undefined ? {} : { metadata: emailed.signUp }),
        });
        if (!response.ok) {
            return emailed.refuse(alert(await failure(response)), response.status);
        }

        // tell whether the link signs in or up
        const kind = emailed.signUp === undefined ? "sign-in" : "sign-up";

        return render(
            "Check your email",
            html`<p>We sent a ${kind} link to <strong>${emailed.email}</strong>.</p>`,
        );
    }

    /** Email a one-use code and ask for it. */
    async #sendCode(
        request: Request,
        query: URLSearchParams,
        emailed: EmailSignIn,
    ): Promise<Response> {
        // send the code
        const response = await this.#forward(request, "/email-otp/send-verification-otp", {
            email: emailed.email,
            type: "sign-in",
        });
        if (!response.ok) {
            return emailed.refuse(alert(await failure(response)), response.status);
        }

        return this.#codeForm(emailed, query, html``, 200);
    }

    /** Sign in with an emailed code, creating a new person and their personal account. */
    async #verifyCode(
        request: Request,
        query: URLSearchParams,
        emailed: EmailSignIn,
        otp: string,
    ): Promise<Response> {
        // verify the code
        const response = await this.#forward(request, "/sign-in/email-otp", {
            email: emailed.email,
            otp,
            ...emailed.signUp,
        });
        if (!response.ok) {
            return this.#codeForm(emailed, query, alert(await failure(response)), response.status);
        }

        return this.#complete(response, query);
    }

    /** Sign in with a social provider, redirecting to it. */
    async #startProvider(
        request: Request,
        query: URLSearchParams,
        provider: string,
    ): Promise<Response> {
        // start the provider's sign-in
        const response = await this.#forward(request, "/sign-in/social", {
            provider,
            callbackURL: target(this.signIn, query),
            errorCallbackURL: target(this.signIn, query),
        });
        if (!response.ok) {
            return this.#signInForm(query, alert(await failure(response)), response.status);
        }

        return follow(response);
    }

    /** Ask a signed-out person for their email, name, handle, residency and time zone. */
    async #showSignUp(
        request: Request,
        query: URLSearchParams,
        error: string | null,
    ): Promise<Response> {
        // continue a signed-in person
        if ((await this.#session(request)) !== null) {
            return redirect(target(this.signIn, query), []);
        }

        // ask for the sign-up
        const notice = error === null ? html`` : alert(`Sign-up failed: ${error}.`);
        const choice = { email: "", name: "", handle: "", residency: null };

        return this.#signUpForm(query, choice, notice, 200);
    }

    /** Submit a sign-up by link or code, or the code of one. */
    async #submitSignUp(request: Request, query: URLSearchParams): Promise<Response> {
        // read the choice
        const form = await request.formData();
        const action = field(form, "action");
        const residencies = await Residency.list(this.database);
        const choice = {
            email: field(form, "email"),
            name: field(form, "name"),
            handle: field(form, "handle"),
            residency: parseResidency(field(form, "residency"), residencies),
        };

        // refuse a sign-up without a known time zone
        const person = SignUp.person.safeParse({
            timeZone: form.get("timeZone"),
            locale: SignUp.locale(request.headers),
        });
        if (!person.success) {
            return this.#signUpForm(query, choice, alert("Choose your time zone."), 400);
        }

        // refuse a missing or overlong name
        if (!DisplayName.safeParse(choice.name).success) {
            return this.#signUpForm(query, choice, alert(NAME_REFUSAL), 400);
        }

        // refuse a taken or invalid handle with its reason
        const claims = Snapshot.live(this.database).read(claimTable);
        const availability = await Account.availability(claims, choice.handle);
        if (availability !== "available") {
            const [reason, status] = HANDLE_REFUSALS[availability];

            return this.#signUpForm(query, choice, alert(reason), status);
        }

        // send the sign-up in the emailed link or code
        const emailed: EmailSignIn = {
            email: choice.email,
            page: this.signUp,
            signUp: SignUp.parse({
                ...person.data,
                name: choice.name,
                handle: choice.handle,
                residencyId: choice.residency,
            }),
            refuse: (notice, status) => this.#signUpForm(query, choice, notice, status),
        };

        // email a sign-up link returning to the continuation
        if (action === "link") {
            return this.#sendLink(request, query, emailed);
        }
        // email a one-use code
        else if (action === "code") {
            return this.#sendCode(request, query, emailed);
        }
        // sign up with the emailed code
        else if (action === "verify") {
            return this.#verifyCode(request, query, emailed, field(form, "otp"));
        }
        // refuse an action the form does not offer
        else {
            throw AccountError.of("ACTION_UNKNOWN", `unknown sign-up action: ${action}`);
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
            throw AccountError.of("ACTION_UNKNOWN", `unknown second-factor action: ${action}`);
        }

        // verify the code and return to the continuation
        const response = await this.#forward(request, path, { code: field(form, "code") });
        if (!response.ok) {
            return this.#secondFactorForm(query, alert(await failure(response)), response.status);
        }

        return redirect(parameter(query, "callbackURL"), response.headers.getSetCookie());
    }

    /** Address the authorization a page's query continues. */
    #continuation(query: URLSearchParams): string {
        const continued = new URLSearchParams({
            client_id: parameter(query, "client_id"),
            request_uri: parameter(query, "request_uri"),
        });

        return `${this.endpoint}${AUTHORIZATION_PATH}?${continued}`;
    }

    /** Show the application and the scopes it asks for, or send a person to sign in first. */
    async #showConsent(request: Request, query: URLSearchParams): Promise<Response> {
        // send a person who must sign in to the sign-in page
        const {
            party,
            request: asked,
            isLoginDue,
        } = await this.server.authorization.review(
            parameter(query, "request_uri"),
            request.headers,
        );
        if (isLoginDue) {
            return redirect(target(this.signIn, query), []);
        }

        // ask to approve or deny the scopes
        return render(
            "Authorize",
            html`<p><strong>${party.name}</strong> asks to:</p>
                ${list(asked.scopes)}
                <form method="post" action="${target(this.consent, query)}">
                    <button name="action" value="approve">Allow</button>
                    <button name="action" value="deny">Deny</button>
                </form>`,
        );
    }

    /** Approve or deny the scopes, and return to the application with the decision. */
    async #submitConsent(request: Request, query: URLSearchParams): Promise<Response> {
        // decide on the stored request as the signed-in person
        await this.#requireOrigin(request);
        const form = await request.formData();
        const decision = parseDecision(field(form, "action"));
        const location = await this.server.authorization.decide(
            parameter(query, "request_uri"),
            request.headers,
            decision,
        );

        return redirect(location, []);
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

        // sign in first
        const session = await this.#session(request);
        if (session === null) {
            const continuation = new URLSearchParams({
                callbackURL: target(this.verification, query),
            });

            return redirect(target(this.signIn, continuation), []);
        }

        // ask to approve a pending sign-in by the client's name
        const { party, row, decision } = await this.server.device.review(code);
        if (decision.status === "pending") {
            return render(
                "Approve a device",
                html`<p>
                        Sign in to <strong>${party.name}</strong> with the code
                        <strong>${code}</strong>?
                    </p>
                    ${row.scopes.length === 0 ? html`` : list(row.scopes)}
                    <form method="post" action="${target(this.verification, query)}">
                        <button name="action" value="approve">Approve</button>
                        <button name="action" value="deny">Deny</button>
                    </form>`,
            );
        }

        return renderDevice(decision.status);
    }

    /** Approve or deny a device's sign-in. */
    async #submitVerification(request: Request, query: URLSearchParams): Promise<Response> {
        // decide on the sign-in of the code as the signed-in person
        await this.#requireOrigin(request);
        const form = await request.formData();
        const decision = parseDecision(field(form, "action"));
        await this.server.device.decide(parameter(query, "user_code"), request.headers, decision);

        return renderDevice(decision === "approve" ? "approved" : "denied");
    }

    /** Refuse a form posted from outside the trusted origins, as a forged request. */
    async #requireOrigin(request: Request): Promise<void> {
        const origin = request.headers.get("origin");
        const context = await this.context;
        if (origin === null || !context.isTrustedOrigin(origin)) {
            throw AccountError.of(
                "ORIGIN_UNTRUSTED",
                `the form was posted from ${origin ?? "nowhere"}`,
            );
        }
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

    /** Answer the form for an emailed code, posting back its email and sign-up. */
    #codeForm(
        emailed: EmailSignIn,
        query: URLSearchParams,
        notice: Markup,
        status: number,
    ): Promise<Response> {
        // store the email and the sign-up as hidden fields
        const { email, signUp } = emailed;
        const hidden = signUp === undefined ? { email } : { email, ...hiddenFields(signUp) };
        const fields = Object.entries(hidden).map(
            ([name, value]) => html`<input type="hidden" name="${name}" value="${value}" />`,
        );

        return render(
            "Enter your code",
            html`${notice}
                <p>We sent a code to <strong>${email}</strong>.</p>
                <form method="post" action="${target(emailed.page, query)}">
                    ${fields}
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
    #secondFactorForm(query: URLSearchParams, notice: Markup, status: number): Promise<Response> {
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

    /** Answer the sign-up form with the typed values and the universe's residencies. */
    async #signUpForm(
        query: URLSearchParams,
        choice: SignUpForm,
        notice: Markup,
        status: number,
    ): Promise<Response> {
        // offer the residencies, preserving the selection
        const residencies = await Residency.list(this.database);
        const options = residencies.map(({ id, name }) =>
            id === choice.residency
                ? html`<option value="${id}" selected>${name}</option>`
                : html`<option value="${id}">${name}</option>`,
        );

        return render(
            "Sign up to Destack",
            html`${notice}
                <form method="post" action="${target(this.signUp, query)}">
                    <label for="email">Email</label>
                    <input
                        id="email"
                        name="email"
                        type="email"
                        autocomplete="email"
                        required
                        autofocus
                        value="${choice.email}"
                    />
                    <label for="name">Name</label>
                    <input
                        id="name"
                        name="name"
                        autocomplete="name"
                        maxlength="${String(NAME_LENGTH)}"
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
                    <label for="residency">Where your data lives</label>
                    <select id="residency" name="residency" required>
                        ${options}
                    </select>
                    ${timeZoneField()}
                    <button name="action" value="link">Email me a sign-up link</button>
                    <button name="action" value="code">Email me a code</button>
                </form>`,
            status,
        );
    }

    /** Return a first-factor sign-in to its continuation. */
    async #complete(response: Response, query: URLSearchParams): Promise<Response> {
        // read the outcome and the session's cookies
        const body = SignedIn.parse(await response.json());
        const cookies = response.headers.getSetCookie();

        // ask for the enrolled second factor
        if (body.twoFactorRedirect === true) {
            const next = new URLSearchParams({ callbackURL: target(this.signIn, query) });

            return redirect(target(this.secondFactor, next), cookies);
        }

        return redirect(target(this.signIn, query), cookies);
    }

    /** Read the browser's session. */
    async #session(
        request: Request,
    ): Promise<{ user: { email: string }; session: { createdAt: string } } | null> {
        // read the session through Better Auth
        const response = await this.#forward(request, "/get-session");

        // treat a revoked credential as signed out
        if (response.status === 401) {
            return null;
        }
        // refuse any other failure
        else if (!response.ok) {
            throw AccountError.of(
                "SESSION_READ_FAILED",
                `session read failed: ${await failure(response)}`,
            );
        }

        return Session.parse(await response.json());
    }

    /** Send a browser's form to a Better Auth route as JSON. */
    #forward(request: Request, path: string, body?: object): Promise<Response> {
        // store the browser's headers and ask for JSON
        const headers = new Headers(request.headers);
        headers.delete("content-length");
        headers.set("accept", "application/json");
        headers.set("content-type", "application/json");

        return this.send(
            new Request(`${this.endpoint}${path}`, {
                method: body === undefined ? "GET" : "POST",
                headers,
                ...(body === undefined ? {} : { body: JSON.stringify(body) }),
            }),
        );
    }
}

/** Send the browser to the URL of a Better Auth answer, with the answer's cookies. */
async function follow(response: Response): Promise<Response> {
    const { url } = Redirection.parse(await response.json());

    return redirect(url, response.headers.getSetCookie());
}

/** Answer a refusal as a page with its message, passing on any other failure. */
function refusal(title: string, error: unknown): Promise<Response> {
    // pass on a failure that is no refusal
    const refused = CallError.refusal(error);
    if (refused === undefined) {
        throw error;
    }

    return render(title, alert(refused.message), refused.status);
}

/** Answer the outcome of a device sign-in. */
function renderDevice(status: "approved" | "denied"): Promise<Response> {
    return status === "approved"
        ? render("Device approved", html`<p>Return to your device to continue.</p>`)
        : render("Device denied", html`<p>The device was not signed in.</p>`);
}

/** List the scopes of a request. */
function list(scopes: readonly string[]): Markup {
    return html`<ul>
        ${scopes.map((name) => html`<li><code>${name}</code></li>`)}
    </ul>`;
}

/** Write the path and query of a page. */
function target(page: URL, query: URLSearchParams): string {
    return query.size === 0 ? page.pathname : `${page.pathname}?${query}`;
}

/** Escape a URL's percent signs against a second decoding. */
function escapeLink(url: string): string {
    return url.replaceAll("%", "%25");
}

/** Write a sign-up as the hidden fields of a code form, which the form posts back. */
function hiddenFields(signUp: SignUp): Record<string, string> {
    return {
        timeZone: signUp.timeZone,
        name: signUp.name,
        handle: signUp.handle,
        residency: signUp.residencyId,
    };
}

/** Read a chosen residency among the universe's. */
function parseResidency(
    value: string,
    residencies: readonly Pick<Residency, "id" | "name">[],
): ResidencyCode {
    // refuse an unknown residency
    const residency = residencies.find((known) => known.id === value);
    if (residency === undefined) {
        throw AccountError.of("RESIDENCY_UNKNOWN", `unknown residency: ${value}`);
    }

    return residency.id;
}

/** Read whether a form approves or denies, refusing any other action. */
function parseDecision(action: string): Decision {
    const decision = Decision.safeParse(action);
    if (!decision.success) {
        throw AccountError.of("ACTION_UNKNOWN", `unknown approval action: ${action}`);
    }

    return decision.data;
}

/** Read a text field of a submitted form. */
function field(form: { readonly get: (name: string) => unknown }, name: string): string {
    // refuse a missing field
    const value = form.get(name);
    if (typeof value !== "string") {
        throw AccountError.of("FORM_FIELD_MISSING", `missing form field: ${name}`);
    }

    return value;
}

/** Read a query parameter a page requires. */
function parameter(query: URLSearchParams, name: string): string {
    // refuse a missing parameter
    const value = query.get(name);
    if (value === null) {
        throw AccountError.of("QUERY_PARAMETER_MISSING", `missing query parameter: ${name}`);
    }

    return value;
}

/** Read the message of a failed Better Auth response. */
async function failure(response: Response): Promise<string> {
    // read the error body
    const body = Failure.parse(await response.json());

    // read a Better Auth error's message
    if (body.message !== undefined) {
        return body.message;
    }
    // read an OAuth error's description
    else if (body.error_description !== undefined) {
        return body.error_description;
    }
    // read an OAuth error's code (RFC 6749 5.2)
    else if (body.error !== undefined) {
        return body.error;
    }
    // refuse an error without any
    else {
        throw AccountError.of(
            "AUTHENTICATION_FAILED",
            `authentication failed without a message: ${response.status}`,
        );
    }
}
