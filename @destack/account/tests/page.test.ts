import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { authenticatorCode, type Browser } from "./browser.ts";
import { AccountFixture } from "./fixture.ts";

/** The fixture's origin, the issuer serving the pages. */
const ORIGIN = "http://localhost:3210";

/** The session cookie Better Auth sets. */
const SESSION_COOKIE = "better-auth.session_token";

/** Sign the daemon's browser in on the sign-in page by an emailed link and by an emailed code, returning to the sign-in page. */
test("sign in on the sign-in page by link and by code", async () => {
    await using fixture = await AccountFixture.open();

    // send a signed-out browser from the daemon's authorization to the sign-in page
    const browser = fixture.browser();
    const authorize = `/auth/native/authorize?${new URLSearchParams({
        response_type: "code",
        client_id: "destack-daemon",
        redirect_uri: "http://127.0.0.1:53682/callback",
        state: "state-1",
        code_challenge: "c".repeat(43),
        code_challenge_method: "S256",
    })}`;
    const away = await browser.fetch(authorize);
    const page = new URL(away.headers.get("location")!);
    const query = new URLSearchParams({ callbackURL: `${ORIGIN}${authorize}` });
    expect([away.status, page.href]).toEqual([302, `${ORIGIN}/sign-in?${query}`]);

    // render the email form without scripts, posting back with the callback
    const form = await browser.fetch(`/sign-in${page.search}`);
    expect({
        status: form.status,
        type: form.headers.get("content-type"),
        policy: form.headers.get("content-security-policy"),
        main: await main(form),
    }).toEqual({
        status: 200,
        type: "text/html; charset=utf-8",
        policy: "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'",
        main: signInForm(`/sign-in?${escape(query.toString())}`),
    });

    // email a link that signs the browser in and returns to the sign-in page
    const sent = await browser.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "link", email: "ada@example.com" }),
    );
    expect([sent.status, await main(sent)]).toEqual([
        200,
        "<main><h1>Check your email</h1><p>We sent a sign-in link to <strong>ada@example.com</strong>.</p></main>",
    ]);
    const followed = await browser.fetch(fixture.links.at(-1)!.url);
    expect([
        followed.status,
        followed.headers.get("location"),
        browser.cookies.has(SESSION_COOKIE),
    ]).toEqual([302, `${ORIGIN}/sign-in?${query}`, true]);

    // return a replayed link to the sign-in page with its error and callback
    const stranger = fixture.browser();
    const replayed = await stranger.fetch(fixture.links.at(-1)!.url);
    const failed = new URL(replayed.headers.get("location")!);
    expect([replayed.status, failed.href]).toEqual([
        302,
        `${ORIGIN}/sign-in?${query}&error=INVALID_TOKEN`,
    ]);
    expect(await main(await stranger.fetch(`${failed.pathname}${failed.search}`))).toEqual(
        signInForm(`/sign-in?${escape(query.toString())}`).replace(
            "<form",
            `<p role="alert">Sign-in failed: INVALID_TOKEN.</p><form`,
        ),
    );

    // email a code that signs another browser in and returns to the sign-in page
    const other = fixture.browser();
    const asked = await other.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "code", email: "ada@example.com" }),
    );
    const codeForm = [
        "<p>We sent a code to <strong>ada@example.com</strong>.</p>",
        `<form method="post" action="/sign-in?${escape(query.toString())}">`,
        `<input type="hidden" name="email" value="ada@example.com" />`,
        `<label for="otp">Code</label>`,
        `<input id="otp" name="otp" inputmode="numeric" autocomplete="one-time-code" required autofocus />`,
        `<button name="action" value="verify">Sign in</button>`,
        "</form></main>",
    ].join("");
    expect([asked.status, await main(asked)]).toEqual([
        200,
        `<main><h1>Enter your code</h1>${codeForm}`,
    ]);
    const code = fixture.codes.at(-1)!;
    const wrong = await other.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "verify", email: "ada@example.com", otp: "000000" }),
    );
    const signedIn = await other.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "verify", email: "ada@example.com", otp: code.otp }),
    );
    expect({
        code: [code.email, code.type],
        wrong: [wrong.status, await main(wrong)],
        signedIn: [
            signedIn.status,
            signedIn.headers.get("location"),
            other.cookies.has(SESSION_COOKIE),
        ],
    }).toEqual({
        code: ["ada@example.com", "sign-in"],
        wrong: [400, `<main><h1>Enter your code</h1><p role="alert">Invalid OTP</p>${codeForm}`],
        signedIn: [303, `/sign-in?${query}`, true],
    });

    // send a person without a handle on to choose one
    const onboarded = await other.fetch(`/sign-in?${query}`);
    expect([onboarded.status, onboarded.headers.get("location")]).toEqual([
        303,
        `/sign-in/handle?${query}`,
    ]);

    // escape the query a page carries, and refuse a callback outside the trusted origins
    const hostile = await fixture
        .browser()
        .fetch(`/sign-in?${new URLSearchParams({ callbackURL: '/"><script>' })}`);
    const foreign = await fixture
        .browser()
        .fetch(`/sign-in?${new URLSearchParams({ callbackURL: "https://evil.test/" })}`);
    expect({
        hostile: await main(hostile),
        foreign: [foreign.status, await main(foreign)],
    }).toEqual({
        hostile: signInForm("/sign-in?callbackURL=%2F%22%3E%3Cscript%3E"),
        foreign: [
            400,
            `<main><h1>Sign-in failed</h1><p role="alert">The callback URL is not trusted.</p></main>`,
        ],
    });
});

/** Choose a handle after the first sign-in: suggested, refused with its reason when taken, then claimed with the personal account before the callback. */
test("choose a handle on the handle page after the first sign-in", async () => {
    await using fixture = await AccountFixture.open();

    // let another person hold the handle ada-taken
    const holder = await fixture.signIn("holder@example.com");
    const current = await holder.client.user.get({ id: holder.id });
    await holder.client.user.update({
        id: holder.id,
        requestId: RequestId.create(),
        revision: current.revision,
        residency: "eu",
    });
    await holder.client.account.create({
        kind: "personal",
        scope: holder.id,
        requestId: RequestId.create(),
        handle: "ada-taken",
        name: "Holder",
    });

    // sign in for the first time and land on the handle page
    const browser = fixture.browser();
    const query = new URLSearchParams({ callbackURL: "/spaces" });
    await signIn(fixture, browser, query);
    const onboarded = await browser.fetch(`/sign-in?${query}`);
    expect([onboarded.status, onboarded.headers.get("location")]).toEqual([
        303,
        "/sign-in/handle?callbackURL=%2Fspaces",
    ]);

    // suggest the email's handle, and ask for the name and the residency
    const shown = await browser.fetch("/sign-in/handle?callbackURL=%2Fspaces");
    expect([shown.status, await main(shown)]).toEqual([
        200,
        handleForm({ name: "", handle: "ada", selected: null, alert: null }),
    ]);

    // refuse a taken handle with its reason and keep the typed input
    const refused = await browser.fetch(
        "/sign-in/handle?callbackURL=%2Fspaces",
        new URLSearchParams({ name: "Ada", handle: "ada-taken", residency: "us" }),
    );
    expect([refused.status, await main(refused)]).toEqual([
        409,
        handleForm({
            name: "Ada",
            handle: "ada-taken",
            selected: "us",
            alert: "This handle is taken.",
        }),
    ]);

    // claim a free handle with the personal account, then continue to the callback
    const claimed = await browser.fetch(
        "/sign-in/handle?callbackURL=%2Fspaces",
        new URLSearchParams({ name: "Ada", handle: "ada", residency: "us" }),
    );
    const continued = await browser.fetch(claimed.headers.get("location")!);
    const client = fixture.connect(() => ({
        origin: ORIGIN,
        cookie: [...browser.cookies].map(([name, value]) => `${name}=${value}`).join("; "),
    }));
    const { subject, profile } = await client.authentication.current();
    const { items } = await client.account.list({ scope: subject.id });
    expect({
        claimed: [claimed.status, claimed.headers.get("location")],
        continued: [continued.status, continued.headers.get("location")],
        profile,
        accounts: items.map(({ kind, handle, name, defaultResidency }) => ({
            kind,
            handle,
            name,
            defaultResidency,
        })),
        residency: (await client.user.get({ id: subject.id })).residency,
    }).toEqual({
        claimed: [303, "/sign-in?callbackURL=%2Fspaces"],
        continued: [303, "/spaces"],
        profile: { subject, name: "Ada", handle: "ada" },
        accounts: [{ kind: "personal", handle: "ada", name: "Ada", defaultResidency: "us" }],
        residency: "us",
    });
});

/** Offer the configured providers on the sign-in page, sending the browser to the chosen one. */
test("sign in with a provider on the sign-in page", async () => {
    await using fixture = await AccountFixture.open({
        providers: { github: { clientId: "github-client", clientSecret: "github-secret" } },
    });
    const browser = fixture.browser();

    // offer the provider below the email form
    const query = new URLSearchParams({ callbackURL: "/spaces" });
    const form = await browser.fetch(`/sign-in?${query}`);
    expect(await main(form)).toEqual(
        signInForm("/sign-in?callbackURL=%2Fspaces").replace(
            "</form></main>",
            [
                "</form>",
                `<form method="post" action="/sign-in?callbackURL=%2Fspaces">`,
                `<input type="hidden" name="action" value="provider" />`,
                `<button name="provider" value="github"> Continue with GitHub </button>`,
                "</form></main>",
            ].join(""),
        ),
    );

    // send the browser to the provider with the sign-in's state
    const started = await browser.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "provider", provider: "github" }),
    );
    const page = new URL(started.headers.get("location")!);
    expect({
        status: started.status,
        page: `${page.origin}${page.pathname}`,
        client: page.searchParams.get("client_id"),
        hasState: browser.cookies.has("better-auth.state"),
    }).toEqual({
        status: 303,
        page: "https://github.com/login/oauth/authorize",
        client: "github-client",
        hasState: true,
    });
});

/** Approve another application's scopes on the consent page, returning to it with a code. */
test("approve an application on the consent page", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("ada@example.com");

    // register and authorize an application and send the user to the consent page
    const redirect = "https://notes.test/callback";
    const client = await owner.client.oauthClient.create({
        scope: owner.id,
        requestId: RequestId.create(),
        name: "Notes <&>",
        redirectUris: [redirect],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code", "refresh_token"],
    });
    const authorized = await owner.browser.fetch(
        `/auth/oauth2/authorize?${new URLSearchParams({
            response_type: "code",
            client_id: client.clientId,
            redirect_uri: redirect,
            scope: "openid email",
            state: "state-1",
            code_challenge: "c".repeat(43),
            code_challenge_method: "S256",
        })}`,
    );
    const page = new URL(authorized.headers.get("location")!);

    // show the application's escaped name and the scopes it asks for
    const shown = await owner.browser.fetch(`${page.pathname}${page.search}`);
    expect([shown.status, await main(shown)]).toEqual([
        200,
        [
            "<main><h1>Authorize</h1>",
            "<p><strong>Notes &lt;&amp;&gt;</strong> asks to:</p>",
            "<ul><li><code>openid</code></li><li><code>email</code></li></ul>",
            `<form method="post" action="/consent?${escape(page.searchParams.toString())}">`,
            `<button name="action" value="approve">Allow</button>`,
            `<button name="action" value="deny">Deny</button>`,
            "</form></main>",
        ].join(""),
    ]);

    // allow and return the browser to the application with a code for its state
    const allowed = await owner.browser.fetch(
        `${page.pathname}${page.search}`,
        new URLSearchParams({ action: "approve" }),
    );
    const callback = new URL(allowed.headers.get("location")!);
    expect({
        status: allowed.status,
        page: `${callback.origin}${callback.pathname}`,
        state: callback.searchParams.get("state"),
        issuer: callback.searchParams.get("iss"),
        hasCode: callback.searchParams.has("code"),
    }).toEqual({
        status: 303,
        page: redirect,
        state: "state-1",
        issuer: ORIGIN,
        hasCode: true,
    });
});

/** Sign in and approve the daemon's device code on the verification page. */
test("approve a device code on the verification page", async () => {
    await using fixture = await AccountFixture.open();
    const daemon = fixture.browser();
    const browser = fixture.browser();

    // request a device code as the daemon
    const offered = await daemon.fetch("/auth/device/code", { client_id: "destack-daemon" });
    const grant = await offered.json();
    const complete = new URL(grant.verification_uri_complete);
    expect(`${complete.origin}${complete.pathname}`).toBe(`${ORIGIN}/device`);

    // ask for the code without one
    const empty = await browser.fetch("/device");
    expect(await main(empty)).toEqual(
        [
            "<main><h1>Approve a device</h1>",
            `<form method="get" action="/device">`,
            `<label for="user_code">Code shown on your device</label>`,
            `<input id="user_code" name="user_code" autocomplete="off" required autofocus />`,
            "<button>Continue</button>",
            "</form></main>",
        ].join(""),
    );

    // send a signed-out browser to sign in, returning to the code
    const here = `/device${complete.search}`;
    const away = await browser.fetch(here);
    const callback = new URLSearchParams({ callbackURL: here });
    expect([away.status, away.headers.get("location")]).toEqual([303, `/sign-in?${callback}`]);
    await signIn(fixture, browser, callback);

    // show the requested sign-in, then approve it
    const shown = await browser.fetch(here);
    expect(await main(shown)).toEqual(
        [
            "<main><h1>Approve a device</h1>",
            `<p> Sign in to <strong>destack-daemon</strong> with the code <strong>${grant.user_code}</strong>? </p>`,
            `<form method="post" action="${escape(here)}">`,
            `<button name="action" value="approve">Approve</button>`,
            `<button name="action" value="deny">Deny</button>`,
            "</form></main>",
        ].join(""),
    );
    const approved = await browser.fetch(here, new URLSearchParams({ action: "approve" }));
    expect([approved.status, await main(approved)]).toEqual([
        200,
        "<main><h1>Device approved</h1><p>Return to your device to continue.</p></main>",
    ]);

    // redeem the approved code as the daemon
    const redeemed = await daemon.fetch("/auth/device/token", {
        grant_type: "urn:ietf:params:oauth:grant-type:device_code",
        device_code: grant.device_code,
        client_id: "destack-daemon",
    });
    expect([redeemed.status, (await redeemed.json()).token_type]).toEqual([200, "Bearer"]);
});

/** Complete an enrolled authenticator's code on the second-factor page, returning to the magic link's sign-in page. */
test("verify an authenticator code on the second-factor page", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("ada@example.com");

    // enroll an authenticator
    const enrollment = await owner.browser.fetch("/auth/two-factor/enable", {});
    const { totpURI } = await enrollment.json();
    await owner.browser.fetch("/auth/two-factor/verify-totp", { code: authenticatorCode(totpURI) });

    // stop a link's sign-in at the second-factor page, carrying its callback
    const browser = fixture.browser();
    const query = new URLSearchParams({ callbackURL: "/spaces" });
    await browser.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "link", email: "ada@example.com" }),
    );
    const challenged = await browser.fetch(fixture.links.at(-1)!.url);
    const page = new URL(challenged.headers.get("location")!);
    const next = new URLSearchParams({ callbackURL: `${ORIGIN}/sign-in?${query}` });
    expect([challenged.status, page.href]).toEqual([302, `${ORIGIN}/sign-in/two-factor?${next}`]);

    // show the code form
    const shown = await browser.fetch(`${page.pathname}${page.search}`);
    expect(await main(shown)).toEqual(
        [
            "<main><h1>Verify it is you</h1>",
            `<form method="post" action="/sign-in/two-factor?${escape(next.toString())}">`,
            `<label for="code">Code from your authenticator app, or a backup code</label>`,
            `<input id="code" name="code" autocomplete="one-time-code" required autofocus />`,
            `<button name="action" value="totp">Verify</button>`,
            `<button name="action" value="backup">Use backup code</button>`,
            "</form></main>",
        ].join(""),
    );

    // verify the authenticator's code, returning to the sign-in page signed in
    const verified = await browser.fetch(
        `${page.pathname}${page.search}`,
        new URLSearchParams({ action: "totp", code: authenticatorCode(totpURI) }),
    );
    expect([
        verified.status,
        verified.headers.get("location"),
        browser.cookies.has(SESSION_COOKIE),
    ]).toEqual([303, `${ORIGIN}/sign-in?${query}`, true]);
});

/** Sign a browser in on the sign-in page by an emailed link. */
async function signIn(
    fixture: AccountFixture,
    browser: Browser,
    query: URLSearchParams,
): Promise<void> {
    await browser.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "link", email: "ada@example.com" }),
    );
    await browser.fetch(fixture.links.at(-1)!.url);
}

/** Write the main element of the handle form with its values. */
function handleForm(values: {
    name: string;
    handle: string;
    selected: "eu" | "us" | null;
    alert: string | null;
}): string {
    const option = (value: "eu" | "us", label: string) =>
        value === values.selected
            ? `<option value="${value}" selected>${label}</option>`
            : `<option value="${value}">${label}</option>`;

    return [
        "<main><h1>Choose your handle</h1>",
        values.alert === null ? "" : `<p role="alert">${values.alert}</p>`,
        `<form method="post" action="/sign-in/handle?callbackURL=%2Fspaces">`,
        `<label for="name">Name</label>`,
        `<input id="name" name="name" autocomplete="name" required value="${values.name}" />`,
        `<label for="handle">Handle</label>`,
        `<input id="handle" name="handle" autocapitalize="none" spellcheck="false" required value="${values.handle}" />`,
        `<label for="residency">Where your data lives</label>`,
        `<select id="residency" name="residency" required>`,
        option("eu", "European Union"),
        option("us", "United States"),
        "</select><button>Continue</button></form></main>",
    ].join("");
}

/** Read a page's main element with whitespace collapsed. */
async function main(response: Response): Promise<string> {
    const text = await response.text();
    const element = text.slice(text.indexOf("<main>"), text.indexOf("</main>") + "</main>".length);

    return element.replace(/\s+/g, " ").replace(/> </g, "><");
}

/** Write the main element of the sign-in form posting to an action. */
function signInForm(action: string): string {
    return [
        "<main><h1>Sign in to Destack</h1>",
        `<form method="post" action="${action}">`,
        `<label for="email">Email</label>`,
        `<input id="email" name="email" type="email" autocomplete="email" required autofocus />`,
        `<button name="action" value="link">Email me a sign-in link</button>`,
        `<button name="action" value="code">Email me a code</button>`,
        "</form></main>",
    ].join("");
}

/** Escape a value as the pages write it into attributes. */
function escape(value: string): string {
    return value.replaceAll("&", "&amp;");
}
