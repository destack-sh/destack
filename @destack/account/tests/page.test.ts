import { inArray } from "@destack/db";
import { Identifier } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { expect, test } from "@destack/test";
import { authenticatorCode } from "./authenticator.ts";
import type { Browser } from "../src/test/index.ts";
import { AccountFixture, ISSUER } from "../src/test/index.ts";
import { DEVICE_CLIENT_NAME, user } from "../src/object/index.ts";
import { at, DeviceGrant, Enrollment, locationOf, Tokens } from "./fixture.ts";

/** The path of the issuer's pages, which their forms post to. */
const PAGES = new URL(ISSUER).pathname;

/** The session cookie Better Auth sets. */
const SESSION_COOKIE = "better-auth.session_token";

/** The policy of every page, admitting the time zone script by its SHA-256 hash. */
const POLICY =
    "default-src 'none'; script-src 'sha256-eZ9dPUSZvzZuzsdc+Rt5v76cmD+uc20qSM6f7h4EPAs='; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'";

/** Sign a device client's browser in on the sign-in page by an emailed link and code. */
test("sign in on the sign-in page by link and by code", async () => {
    await using fixture = await AccountFixture.open();
    await fixture.person("ada@example.com");

    // send a signed-out browser from a device client's authorization to the sign-in page
    const browser = fixture.browser();
    const authorize = `/oauth2/authorize?${new URLSearchParams({
        response_type: "code",
        client_id: Identifier.create("client"),
        redirect_uri: "http://127.0.0.1:53682/callback",
        state: "state-1",
        code_challenge: "c".repeat(43),
        code_challenge_method: "S256",
    })}`;
    const away = await browser.fetch(authorize);
    const page = new URL(locationOf(away));
    const query = page.searchParams;
    expect([away.status, `${page.origin}${page.pathname}`, query.has("sig")]).toEqual([
        302,
        `${ISSUER}/sign-in`,
        true,
    ]);

    // render the email form running only the time zone script
    const form = await browser.fetch(`/sign-in${page.search}`);
    expect({
        status: form.status,
        type: form.headers.get("content-type"),
        policy: form.headers.get("content-security-policy"),
        main: await main(form),
    }).toEqual({
        status: 200,
        type: "text/html; charset=utf-8",
        policy: POLICY,
        main: signInForm(`${PAGES}/sign-in?${escape(query.toString())}`),
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
    const followed = await browser.fetch(at(await fixture.links(), -1).url);
    expect([
        followed.status,
        followed.headers.get("location"),
        browser.cookies.has(SESSION_COOKIE),
    ]).toEqual([302, `${ISSUER}/sign-in?${query}`, true]);

    // return a replayed link to the sign-in page with its error and callback
    const stranger = fixture.browser();
    const replayed = await stranger.fetch(at(await fixture.links(), -1).url);
    const failed = new URL(locationOf(replayed));
    expect([replayed.status, failed.href]).toEqual([
        302,
        `${ISSUER}/sign-in?${query}&error=INVALID_TOKEN`,
    ]);
    expect(await main(await stranger.fetch(failed.href))).toEqual(
        signInForm(`${PAGES}/sign-in?${escape(query.toString())}`).replace(
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
        `<form method="post" action="${PAGES}/sign-in?${escape(query.toString())}">`,
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
    const code = at(await fixture.codes(), -1);
    const wrong = await other.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "verify", email: "ada@example.com", otp: "000000" }),
    );
    const signedIn = await other.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "verify", email: "ada@example.com", otp: code.otp }),
    );
    expect({
        code: [code.email, code.purpose],
        wrong: [wrong.status, await main(wrong)],
        signedIn: [
            signedIn.status,
            signedIn.headers.get("location"),
            other.cookies.has(SESSION_COOKIE),
        ],
    }).toEqual({
        code: ["ada@example.com", "sign-in"],
        wrong: [400, `<main><h1>Enter your code</h1><p role="alert">Invalid OTP</p>${codeForm}`],
        signedIn: [303, `${PAGES}/sign-in?${query}`, true],
    });

    // escape the query a page passes
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
        hostile: signInForm(`${PAGES}/sign-in?callbackURL=%2F%22%3E%3Cscript%3E`),
        foreign: [
            400,
            `<main><h1>Sign-in failed</h1><p role="alert">The callback URL is not trusted.</p></main>`,
        ],
    });
});

/** Sign people up with their name, handle, residency, time zone and language. */
test("sign up on the sign-up page with a name in the posted time zone and the browser's language", async () => {
    await using fixture = await AccountFixture.open();
    const browser = fixture.browser("x y, fr-ch;q=0.9, en;q=0.5");
    const chosen = {
        email: "ada@example.com",
        name: "Ada Lovelace",
        handle: "ada",
        residency: "us",
    } as const;

    // refuse a sign-up without a time zone or with an unknown one, mailing nothing
    const missing = await browser.fetch(
        "/sign-up",
        new URLSearchParams({ action: "link", ...chosen }),
    );
    const unknown = await browser.fetch(
        "/sign-up",
        new URLSearchParams({ action: "link", ...chosen, timeZone: "Mars/Olympus" }),
    );
    const refused = signUpForm({ ...chosen, alert: "Choose your time zone." });

    // refuse a sign-up with an empty or overlong name, mailing nothing
    const long = "a".repeat(201);
    const empty = await browser.fetch(
        "/sign-up",
        new URLSearchParams({ action: "link", ...chosen, name: "", timeZone: "UTC" }),
    );
    const overlong = await browser.fetch(
        "/sign-up",
        new URLSearchParams({ action: "link", ...chosen, name: long, timeZone: "UTC" }),
    );
    const unnamed = "A name has 1 to 200 characters.";
    expect({
        missing: [missing.status, await main(missing)],
        unknown: [unknown.status, await main(unknown)],
        empty: [empty.status, await main(empty)],
        overlong: [overlong.status, await main(overlong)],
        links: await fixture.links(),
    }).toEqual({
        missing: [400, refused],
        unknown: [400, refused],
        empty: [400, signUpForm({ ...chosen, name: "", alert: unnamed })],
        overlong: [400, signUpForm({ ...chosen, name: long, alert: unnamed })],
        links: [],
    });

    // refuse a link sign-up past the page without a time zone and a locale when it is opened
    const direct = await browser.fetch("/sign-in/magic-link", {
        email: "eve@example.com",
        callbackURL: "/",
    });
    const opened = await browser.fetch(at(await fixture.links(), -1).url);
    const failed = new URL(locationOf(opened));
    expect([direct.status, opened.status, Object.fromEntries(failed.searchParams)]).toEqual([
        200,
        302,
        {
            error: "INVALID_SIGN_UP",
            error_description: "a sign-up needs a known time zone and a locale",
        },
    ]);

    // sign up by a link in the posted time zone and the browser's first well-formed language
    await browser.fetch(
        "/sign-up",
        new URLSearchParams({ action: "link", ...chosen, timeZone: "America/New_York" }),
    );
    await browser.fetch(at(await fixture.links(), -1).url);

    // sign up by a code in the pages' own language for a browser asking for none
    const other = fixture.browser("*");
    const code = {
        email: "bob@example.com",
        name: "Bob",
        handle: "bob",
        residency: "eu",
        timeZone: "Asia/Tokyo",
    };
    await other.fetch("/sign-up", new URLSearchParams({ action: "code", ...code }));
    const { otp } = at(await fixture.codes(), -1);
    await other.fetch("/sign-up", new URLSearchParams({ action: "verify", otp, ...code }));

    // store each new user's name, time zone and locale
    const users = await fixture.storage.database
        .select({
            email: user.table.email,
            name: user.table.name,
            timeZone: user.table.timeZone,
            locale: user.table.locale,
        })
        .from(user.table)
        .where(inArray(user.table.email, ["ada@example.com", "bob@example.com"]))
        .orderBy(user.table.email);
    expect(users).toEqual([
        {
            email: "ada@example.com",
            name: "Ada Lovelace",
            timeZone: "America/New_York",
            locale: "fr-CH",
        },
        { email: "bob@example.com", name: "Bob", timeZone: "Asia/Tokyo", locale: "en" },
    ]);
});

/** Offer the configured providers on the sign-in page, sending the browser to the chosen one. */
test("sign in with a provider on the sign-in page", async () => {
    await using fixture = await AccountFixture.open({
        signIn: {
            providers: { github: { clientId: "github-client", clientSecret: "github-secret" } },
        },
    });
    const browser = fixture.browser();

    // offer the provider below the email form
    const query = new URLSearchParams({ callbackURL: "/spaces" });
    const form = await browser.fetch(`/sign-in?${query}`);
    expect(await main(form)).toEqual(
        signInForm(`${PAGES}/sign-in?callbackURL=%2Fspaces`).replace(
            "</form></main>",
            [
                "</form>",
                `<form method="post" action="${PAGES}/sign-in?callbackURL=%2Fspaces">`,
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
    const page = new URL(locationOf(started));
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
    const owner = await fixture.signInWithLink("ada@example.com");

    // register and authorize an application and send the user to the consent page
    const redirect = "https://notes.test/callback";
    const client = await owner.client.oauthClient.create({
        scope: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "Notes <&>",
        redirectUris: [redirect],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code", "refresh_token"],
    });
    const authorized = await owner.browser.fetch(
        `/oauth2/authorize?${new URLSearchParams({
            response_type: "code",
            client_id: client.clientId,
            redirect_uri: redirect,
            scope: "openid email",
            state: "state-1",
            code_challenge: "c".repeat(43),
            code_challenge_method: "S256",
        })}`,
    );
    const page = new URL(locationOf(authorized));

    // show the application's escaped name and the scopes it asks for
    const shown = await owner.browser.fetch(page.href);
    expect([shown.status, await main(shown)]).toEqual([
        200,
        [
            "<main><h1>Authorize</h1>",
            "<p><strong>Notes &lt;&amp;&gt;</strong> asks to:</p>",
            "<ul><li><code>openid</code></li><li><code>email</code></li></ul>",
            `<form method="post" action="${PAGES}/consent?${escape(page.searchParams.toString())}">`,
            `<button name="action" value="approve">Allow</button>`,
            `<button name="action" value="deny">Deny</button>`,
            "</form></main>",
        ].join(""),
    ]);

    // allow and return the browser to the application with a code for its state
    const allowed = await owner.browser.fetch(
        page.href,
        new URLSearchParams({ action: "approve" }),
    );
    const callback = new URL(locationOf(allowed));
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
        issuer: ISSUER,
        hasCode: true,
    });
});

/** Sign in and approve a device client's code on the verification page. */
test("approve a device code on the verification page", async () => {
    await using fixture = await AccountFixture.open();
    const native = fixture.browser();
    const browser = fixture.browser();
    await fixture.person("ada@example.com");

    // request a device code as a device's new client
    const deviceClient = Identifier.create("client");
    const offered = await native.fetch("/device/code", { client_id: deviceClient });
    const grant = DeviceGrant.parse(await offered.json());
    const complete = new URL(grant.verification_uri_complete);
    expect(`${complete.origin}${complete.pathname}`).toBe(`${ISSUER}/device`);

    // ask for the code without one
    const empty = await browser.fetch("/device");
    expect(await main(empty)).toEqual(
        [
            "<main><h1>Approve a device</h1>",
            `<form method="get" action="${PAGES}/device">`,
            `<label for="user_code">Code shown on your device</label>`,
            `<input id="user_code" name="user_code" autocomplete="off" required autofocus />`,
            "<button>Continue</button>",
            "</form></main>",
        ].join(""),
    );

    // send a signed-out browser to sign in
    const here = `/device${complete.search}`;
    const away = await browser.fetch(here);
    const callback = new URLSearchParams({ callbackURL: `${PAGES}${here}` });
    expect([away.status, away.headers.get("location")]).toEqual([
        303,
        `${PAGES}/sign-in?${callback}`,
    ]);
    await signIn(fixture, browser, callback);

    // show the requested sign-in and approve it
    const shown = await browser.fetch(here);
    expect(await main(shown)).toEqual(
        [
            "<main><h1>Approve a device</h1>",
            `<p> Sign in to <strong>${DEVICE_CLIENT_NAME}</strong> with the code <strong>${grant.user_code}</strong>? </p>`,
            `<form method="post" action="${PAGES}${escape(here)}">`,
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

    // redeem the approved code as the device's client
    const redeemed = await native.fetch(
        "/oauth2/token",
        new URLSearchParams({
            grant_type: "urn:ietf:params:oauth:grant-type:device_code",
            device_code: grant.device_code,
            client_id: deviceClient,
        }),
    );
    expect([redeemed.status, Tokens.parse(await redeemed.json()).token_type]).toEqual([
        200,
        "Bearer",
    ]);
});

/** Complete an enrolled authenticator's code on the second-factor page. */
test("verify an authenticator code on the second-factor page", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("ada@example.com");

    // enroll an authenticator
    const enrollment = await owner.browser.fetch("/two-factor/enable", {});
    const { totpURI } = Enrollment.parse(await enrollment.json());
    await owner.browser.fetch("/two-factor/verify-totp", { code: authenticatorCode(totpURI) });

    // stop a link's sign-in at the second-factor page
    const browser = fixture.browser();
    const query = new URLSearchParams({ callbackURL: "/spaces" });
    await browser.fetch(
        `/sign-in?${query}`,
        new URLSearchParams({ action: "link", email: "ada@example.com" }),
    );
    const challenged = await browser.fetch(at(await fixture.links(), -1).url);
    const page = new URL(locationOf(challenged));
    const next = new URLSearchParams({ callbackURL: `${ISSUER}/sign-in?${query}` });
    expect([challenged.status, page.href]).toEqual([302, `${ISSUER}/sign-in/two-factor?${next}`]);

    // show the code form
    const shown = await browser.fetch(page.href);
    expect(await main(shown)).toEqual(
        [
            "<main><h1>Verify it is you</h1>",
            `<form method="post" action="${PAGES}/sign-in/two-factor?${escape(next.toString())}">`,
            `<label for="code">Code from your authenticator app, or a backup code</label>`,
            `<input id="code" name="code" autocomplete="one-time-code" required autofocus />`,
            `<button name="action" value="totp">Verify</button>`,
            `<button name="action" value="backup">Use backup code</button>`,
            "</form></main>",
        ].join(""),
    );

    // verify the authenticator's code
    const verified = await browser.fetch(
        page.href,
        new URLSearchParams({ action: "totp", code: authenticatorCode(totpURI) }),
    );
    expect([
        verified.status,
        verified.headers.get("location"),
        browser.cookies.has(SESSION_COOKIE),
    ]).toEqual([303, `${ISSUER}/sign-in?${query}`, true]);
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
    await browser.fetch(at(await fixture.links(), -1).url);
}

/** Write the main element of the sign-up form with its values. */
function signUpForm(values: {
    email: string;
    name: string;
    handle: string;
    residency: "eu" | "us";
    alert: string;
}): string {
    const option = (value: "eu" | "us", label: string) =>
        value === values.residency
            ? `<option value="${value}" selected>${label}</option>`
            : `<option value="${value}">${label}</option>`;

    return [
        "<main><h1>Sign up to Destack</h1>",
        `<p role="alert">${values.alert}</p>`,
        `<form method="post" action="${PAGES}/sign-up">`,
        `<label for="email">Email</label>`,
        `<input id="email" name="email" type="email" autocomplete="email" required autofocus value="${values.email}" />`,
        `<label for="name">Name</label>`,
        `<input id="name" name="name" autocomplete="name" maxlength="200" required value="${values.name}" />`,
        `<label for="handle">Handle</label>`,
        `<input id="handle" name="handle" autocapitalize="none" spellcheck="false" required value="${values.handle}" />`,
        `<label for="residency">Where your data lives</label>`,
        `<select id="residency" name="residency" required>`,
        option("eu", "European Union"),
        option("us", "United States"),
        "</select>",
        timeZoneField(),
        `<button name="action" value="link">Email me a sign-up link</button>`,
        `<button name="action" value="code">Email me a code</button>`,
        "</form></main>",
    ].join("");
}

/** Read a page's main element with whitespace collapsed. */
async function main(response: Response): Promise<string> {
    const text = await response.text();
    const element = text.slice(text.indexOf("<main>"), text.indexOf("</main>") + "</main>".length);

    return element.replace(/\s+/gu, " ").replace(/> </gu, "><");
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

/** Write the sign-up form's time zone field, a list of zones without scripts. */
function timeZoneField(): string {
    const options = Intl.supportedValuesOf("timeZone").map(
        (zone) => `<option value="${zone}">${zone}</option>`,
    );

    return [
        `<noscript><label> Time zone <select name="timeZone" required>`,
        `<option value="">Choose your time zone</option>`,
        ...options,
        `</select></label></noscript>`,
        `<input type="hidden" name="timeZone" />`,
    ].join("");
}

/** Escape a value as the pages write it into attributes. */
function escape(value: string): string {
    return value.replaceAll("&", "&amp;");
}
