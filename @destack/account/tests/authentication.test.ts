import { Failure } from "@destack/error";
import { AuditCaller, JournalEntry, journal } from "@destack/object";
import { ObjectReference, Subject, RequestId } from "@destack/sync";
import { Account, account, passkey, session, twoFactor, userLogin } from "../src/object/index.ts";
import { user } from "../src/object/user.ts";
import { expect, onTestFinished, single, test } from "@destack/test";
import { eq, gte } from "@destack/db";
import { Simulation } from "@destack/environment/simulation";
import { Identifier, type JsonObject, type JsonValue, present, schema } from "@destack/schema";
import { parseEnvelope } from "better-auth/crypto";
import { ActiveSession } from "../src/better-auth/active-session.ts";
import { verification } from "../src/authentication/index.ts";
import {
    authenticatorCode,
    Enrollment,
    ISSUER,
    ORIGIN,
    PasskeyAuthenticator,
    SIGN_UP,
    TestAccountStore,
    TestBrowser,
    TestJwt,
} from "../src/test/index.ts";
import { DeviceGrant, Redirect, Tokens } from "./fixture.ts";
import { principal } from "@destack/access";
import { PackageId } from "@destack/package";
import accountPackage from "../src/package.ts";

/** The port HTTPS origins listen on. */
const HTTPS_PORT = 443;

/** The email of the person each scenario signs in. */
const EMAIL = "florian@example.com";

/** The session Better Auth reports for a browser's cookie, without its times and tokens. */
const CurrentSession = schema.object({
    /** The session. */
    session: schema
        .object({
            /** The session's identifier. */
            id: schema.string(),
            /** The signed-in user. */
            userId: schema.string(),
        })
        .strip(),
    /** The signed-in user. */
    user: schema
        .object({
            /** The user's identifier. */
            id: schema.string(),
            /** The user's email address. */
            email: schema.string(),
            /** Whether the user proved the email address. */
            emailVerified: schema.boolean(),
        })
        .strip(),
});

/** The options a WebAuthn ceremony starts with. */
const CeremonyOptions = schema.looseObject({
    /** The challenge the authenticator signs. */
    challenge: schema.string(),
});

/** Order rows by their identifiers. */
function byId(left: { readonly id: string }, right: { readonly id: string }): number {
    return left.id.localeCompare(right.id);
}

/** Read a response as its status and JSON body. */
async function answered(response: Response): Promise<readonly [number, JsonValue]> {
    return [response.status, await response.json()];
}

/** Read the version of the root key that encrypted a stored secret. */
function envelope(encrypted: string): number {
    return present(parseEnvelope(encrypted), "the secret's envelope").version;
}

/** Read the session a browser's cookie holds. */
async function current(browser: TestBrowser) {
    return CurrentSession.parse(await (await browser.fetch("/get-session")).json());
}

/** Sign a browser in by the person's mailed link. */
async function signInByLink(fixture: TestAccountStore, browser: TestBrowser): Promise<Response> {
    await browser.fetch("/sign-in/magic-link", { email: EMAIL, callbackURL: "/" });

    return browser.fetch(await fixture.link());
}

/** Enroll an authenticator in a browser a mailed code signs in, then sign the browser out. */
async function enroll(fixture: TestAccountStore, browser: TestBrowser) {
    // sign in with a mailed code
    const person = await fixture.signIn(EMAIL);
    await browser.fetch("/email-otp/send-verification-otp", { email: EMAIL, type: "sign-in" });
    await browser.fetch("/sign-in/email-otp", { email: EMAIL, otp: await fixture.code() });

    // enable an authenticator and verify its first code
    const enrollment = await browser.fetch("/two-factor/enable", { method: "totp" });
    const enrolled = Enrollment.parse(await enrollment.json());
    await browser.fetch("/two-factor/verify-totp", {
        code: authenticatorCode(enrolled.totpURI, fixture.clock.now()),
    });
    await browser.fetch("/sign-out", {});

    return { ...enrolled, userId: person.id };
}

/** Serve the GitHub and Google endpoints Better Auth calls in callbacks, at the fixture's time. */
function answerProviders(fixture: TestAccountStore): void {
    // an ID token Google would sign for the configured client
    const now = Math.floor(fixture.clock.now() / 1000);
    const token = TestJwt.encode({
        iss: "https://accounts.google.com",
        aud: "google-client",
        sub: "google-subject",
        email: "ada@example.com",
        email_verified: true,
        name: "Ada",
        zoneinfo: "Europe/London",
        locale: "en-gb",
        iat: now,
        exp: now + 3600,
    });

    // answer each provider endpoint by URL
    const answers: JsonObject = {
        "https://github.com/login/oauth/access_token": {
            access_token: "github-access-token",
            token_type: "bearer",
            scope: "read:user,user:email",
        },
        "https://api.github.com/user": { id: 1, login: "octocat", name: null, email: null },
        "https://api.github.com/user/emails": [
            { email: "octo@example.com", primary: true, verified: true },
        ],
        "https://oauth2.googleapis.com/token": {
            access_token: "google-access-token",
            expires_in: 3600,
            id_token: token,
            token_type: "Bearer",
            scope: "openid email profile",
        },
    };

    // serve each provider's host on a simulated network the library's global fetch reaches while installed
    const simulation = new Simulation({ seed: 1, start: fixture.clock.now() });
    const hosts = new Set(Object.keys(answers).map((url) => new URL(url).hostname));
    for (const hostname of hosts) {
        simulation.network.of(hostname).serve({
            hostname,
            port: HTTPS_PORT,
            fetch: (request) => {
                const url = new URL(request.url);

                return Response.json(answers[`${url.origin}${url.pathname}`]);
            },
        });
    }
    simulation.install();
    onTestFinished(() => simulation.uninstall());
}

test("sign a person up by a mailed link once, and end the browser's session from the command line and by signing out", async () => {
    await using fixture = await TestAccountStore.open();
    const { database, authenticator } = fixture;
    const started = fixture.clock.now();

    // request a sign-in link through the HTTP route a browser calls
    const requested = await fixture.fetch(
        new Request(`${ISSUER}/sign-in/magic-link`, {
            method: "POST",
            headers: { "content-type": "application/json", origin: ORIGIN },
            body: JSON.stringify({
                email: EMAIL,
                callbackURL: "/",
                metadata: { ...SIGN_UP, name: "Florian", handle: "florian", residencyId: "eu" },
            }),
        }),
    );
    const messages = await fixture.links();
    const link = single(messages).url;
    const query = new URL(link).searchParams;
    const token = present(query.get("token"), "the link token");
    query.delete("token");
    expect([
        await answered(requested),
        messages.map(({ email }) => email),
        Object.fromEntries(query),
    ]).toEqual([
        [200, { status: true }],
        [EMAIL],
        { callbackURL: "/", ...SIGN_UP, name: "Florian", handle: "florian", residencyId: "eu" },
    ]);

    // store the SHA-256 digest of the link token in base64url alone
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(token));
    expect(single(await database.select().from(verification)).identifier).toBe(
        new Uint8Array(digest).toBase64({ alphabet: "base64url", omitPadding: true }),
    );

    // verify the link once, refusing its replay
    const verified = await fixture.fetch(new Request(link));
    const cookies = verified.headers
        .getSetCookie()
        .map((cookie) => cookie.split(";")[0])
        .join("; ");
    const replay = await fixture.fetch(new Request(link));
    expect([
        [verified.status, TestBrowser.location(verified)],
        [replay.status, TestBrowser.location(replay)],
        await database.select().from(verification),
    ]).toEqual([[302, `${ORIGIN}/`], [302, `${ORIGIN}/?error=INVALID_TOKEN`], []]);

    // store the person and their session, which the cookie signs in
    const person = single(
        await database
            .select({
                id: user.table.id,
                name: user.table.name,
                email: user.table.email,
                emailVerified: user.table.emailVerified,
            })
            .from(user.table)
            .where(eq(user.table.email, EMAIL)),
    );
    const login = single(
        await database
            .select({ id: session.table.id, userId: session.table.userId })
            .from(session.table)
            .where(eq(session.table.userId, person.id)),
    );
    const signedIn = present(
        await authenticator.auth.api.getSession({ headers: new Headers({ cookie: cookies }) }),
        "the cookie's session",
    );
    const active = await ActiveSession.read(login.id, database, fixture.clock.now());
    expect([person, login, signedIn.user.id, active.authenticationMethod]).toEqual([
        { id: person.id, name: "Florian", email: EMAIL, emailVerified: true },
        { id: login.id, userId: person.id },
        person.id,
        "magic-link",
    ]);

    // revoke the browser's session from the person's command line, without waiting for the cookie to expire
    const revoking = fixture.clock.now();
    const commandLine = await fixture.signIn(EMAIL);
    await commandLine.client(accountPackage).session.revoke({
        userId: commandLine.id,
        id: login.id,
        requestId: RequestId.create(fixture.clock.now()),
    });
    const revoked = await fixture.fetch(
        new Request(`${ISSUER}/get-session`, { headers: { cookie: cookies } }),
    );
    expect(await answered(revoked)).toEqual([401, { message: "session is no longer authorized" }]);

    // sign out through the browser protocol and reject the previously valid cookie
    const signedOut = await fixture.fetch(
        new Request(`${ISSUER}/sign-out`, {
            method: "POST",
            headers: { cookie: cookies, origin: ORIGIN },
        }),
    );
    expect([
        await answered(signedOut),
        await authenticator.auth.api.getSession({ headers: new Headers({ cookie: cookies }) }),
        await database.select().from(session.table).where(eq(session.table.id, login.id)),
    ]).toEqual([[200, { success: true }], null, []]);

    // retain each authentication request and its outcome without credential contents
    const rows = await database
        .select()
        .from(journal)
        .where(gte(journal.startedAt, started))
        .orderBy(journal.startedAt, journal.position);
    const calls = rows.map((row) => JournalEntry.parse(row.call));
    const browsing = rows
        .filter((row) => row.startedAt < revoking)
        .map((row) => JournalEntry.parse(row.call));
    expect(
        calls
            .filter((call) => call.method === "authentication.request")
            .map((call) => ({
                endpoint: call.execution.target.id,
                outcome: call.execution.outcome,
            })),
    ).toEqual([
        { endpoint: "/sign-in/magic-link", outcome: { kind: "success" } },
        { endpoint: "/magic-link/verify", outcome: { kind: "success" } },
        {
            endpoint: "/magic-link/verify",
            outcome: {
                kind: "failure",
                error: Failure.http("UNAUTHORIZED", "authentication rejected"),
            },
        },
        { endpoint: "/email-otp/send-verification-otp", outcome: { kind: "success" } },
        { endpoint: "/oauth2/token", outcome: { kind: "success" } },
        { endpoint: "/oauth2/token", outcome: { kind: "success" } },
        { endpoint: "/sign-out", outcome: { kind: "success" } },
    ]);

    // retain the person's personal account and session from the browser's sign-in, beside the system's sending of the mail
    const personal = single(
        await database
            .select({ id: account.table.id })
            .from(account.table)
            .where(eq(account.table.scope, person.id)),
    );
    const subject = principal.user.reference("universe", person.id);
    expect(
        browsing
            .filter(
                (call) =>
                    call.method !== "authentication.request" &&
                    Subject.same(AuditCaller.actor(call.execution.context.caller).subject, subject),
            )
            .map((call) => ({
                method: call.method,
                actor: AuditCaller.actor(call.execution.context.caller),
                target: call.execution.target,
                details: call.execution.details,
                outcome: call.execution.outcome,
            })),
    ).toEqual([
        {
            method: "account.create",
            actor: { subject },
            target: { type: "account", id: personal.id },
            details: {},
            outcome: { kind: "success" },
        },
        {
            method: "session.create",
            actor: { subject },
            target: { type: "session", id: login.id },
            details: { path: "/magic-link/verify" },
            outcome: { kind: "success" },
        },
    ]);
});

test("sign a person with an unproven email in with a mailed code, proving the email", async () => {
    // store a person whose email nobody proved yet
    await using fixture = await TestAccountStore.open();
    const browser = fixture.browser();
    const id = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-0000000000e1");
    const email = "ada@example.com";
    await fixture.database.insert(user.table).values({
        id,
        name: "Ada",
        email,
        ...SIGN_UP,
        homeId: Account.home(Account.personal(id)),
        createdAt: fixture.clock.now(),
        updatedAt: fixture.clock.now(),
    });

    // send a code and sign in with it
    const sent = await browser.fetch("/email-otp/send-verification-otp", {
        email,
        type: "sign-in",
    });
    const signedIn = await browser.fetch("/sign-in/email-otp", {
        email,
        otp: await fixture.code(),
    });

    // sign the person in with the email proven, spending the code
    expect({
        statuses: [sent.status, signedIn.status],
        person: await fixture.database
            .select({ id: user.table.id, emailVerified: user.table.emailVerified })
            .from(user.table)
            .where(eq(user.table.id, id)),
        session: (await current(browser)).user,
        challenges: await fixture.database.select().from(verification),
    }).toEqual({
        statuses: [200, 200],
        person: [{ id, emailVerified: true }],
        session: { id, email, emailVerified: true },
        challenges: [],
    });
});

test("sign a person in with the latest of two mailed codes", async () => {
    // send a code, leave it unused and send another
    await using fixture = await TestAccountStore.open();
    const browser = fixture.browser();
    const person = await fixture.signIn("person@example.com");
    const send = () =>
        browser.fetch("/email-otp/send-verification-otp", {
            email: person.email,
            type: "sign-in",
        });
    const sent = [await answered(await send()), await answered(await send())];

    // sign in with the second code
    await browser.fetch("/sign-in/email-otp", { email: person.email, otp: await fixture.code() });
    expect([sent, (await current(browser)).user]).toEqual([
        [
            [200, { success: true }],
            [200, { success: true }],
        ],
        { id: person.id, email: person.email, emailVerified: true },
    ]);
});

test("stop a passwordless sign-in at the second factor, and complete it with a recovery code once", async () => {
    // enroll an authenticator, storing its secret and recovery codes encrypted
    await using fixture = await TestAccountStore.open();
    const { backupCodes, userId } = await enroll(fixture, fixture.browser());
    const factorOf = async () =>
        single(
            await fixture.database
                .select()
                .from(twoFactor.table)
                .where(eq(twoFactor.table.userId, userId)),
        );
    const factor = await factorOf();

    // stop a magic link at the second factor on a sign-in over a rotated root key
    const browser = fixture.browser(SIGN_UP.locale, await fixture.rotate());
    const challenge = await signInByLink(fixture, browser);
    const stopped = new URL(TestBrowser.location(challenge), ORIGIN);
    const unfinished: unknown = await (await browser.fetch("/get-session")).json();

    // complete the sign-in with a recovery code, rewriting the remaining codes
    await browser.fetch("/two-factor/verify-backup-code", { code: backupCodes[0] });
    const consumed = await factorOf();
    expect({
        encrypted: [envelope(factor.secret), envelope(factor.backupCodes)],
        challenge: [challenge.status, `${stopped.origin}${stopped.pathname}`],
        unfinished,
        consumed: envelope(consumed.backupCodes),
        session: (await current(browser)).user.email,
    }).toEqual({
        encrypted: [1, 1],
        challenge: [302, `${ISSUER}/sign-in/two-factor`],
        unfinished: null,
        consumed: 2,
        session: EMAIL,
    });
});

test("refuse a spent recovery code in a later sign-in", async () => {
    // spend a recovery code in one sign-in
    await using fixture = await TestAccountStore.open();
    const { backupCodes } = await enroll(fixture, fixture.browser());
    const first = fixture.browser();
    await signInByLink(fixture, first);
    const spent = await first.fetch("/two-factor/verify-backup-code", { code: backupCodes[0] });

    // refuse it in another
    const second = fixture.browser();
    await signInByLink(fixture, second);
    const replay = await second.fetch("/two-factor/verify-backup-code", { code: backupCodes[0] });
    expect([
        spent.status,
        await answered(replay),
        await (await second.fetch("/get-session")).json(),
    ]).toEqual([200, [401, { code: "INVALID_BACKUP_CODE", message: "Invalid backup code" }], null]);
});

test("approve a device's new client in a signed-in session, and redeem its code once as the client", async () => {
    // request a device code as a new client, and approve it in the person's signed-in browser
    await using fixture = await TestAccountStore.open();
    const { browser } = await fixture.signInWithLink(EMAIL);
    const device = fixture.browser();
    const clientId = Identifier.create("client");
    const offered = await device.fetch(
        "/device/code",
        new URLSearchParams({
            client_id: clientId,
            scope: "openid offline_access",
            resource: PackageId.resource(accountPackage.id),
        }),
    );
    const grant = DeviceGrant.parse(await offered.json());
    const approved = await browser.fetch(
        `/device?${new URLSearchParams({ user_code: grant.user_code })}`,
        new URLSearchParams({ action: "approve" }),
    );

    // redeem the code as the client once, its tokens naming the client
    const redeem = () =>
        device.fetch(
            "/oauth2/token",
            new URLSearchParams({
                grant_type: "urn:ietf:params:oauth:grant-type:device_code",
                client_id: clientId,
                device_code: grant.device_code,
            }),
        );
    const exchanged = await redeem();
    const tokens = Tokens.parse(await exchanged.json());
    expect({
        approved: approved.status,
        credential: TestJwt.claims(tokens.access_token).caller.credential.kind,
        reused: await answered(await redeem()),
    }).toEqual({
        approved: 200,
        credential: "client",
        reused: [400, { error: "invalid_grant", error_description: "Invalid device code" }],
    });
});

test("refuse refreshing a device's tokens once the person revokes its client", async () => {
    // sign a device's new client in by a device code the person approves
    await using fixture = await TestAccountStore.open();
    const { person, browser } = await fixture.signInWithLink(EMAIL);
    const device = fixture.browser();
    const clientId = Identifier.create("client");
    const offered = await device.fetch(
        "/device/code",
        new URLSearchParams({
            client_id: clientId,
            scope: "openid offline_access",
            resource: PackageId.resource(accountPackage.id),
        }),
    );
    const grant = DeviceGrant.parse(await offered.json());
    await browser.fetch(
        `/device?${new URLSearchParams({ user_code: grant.user_code })}`,
        new URLSearchParams({ action: "approve" }),
    );
    const exchanged = await device.fetch(
        "/oauth2/token",
        new URLSearchParams({
            grant_type: "urn:ietf:params:oauth:grant-type:device_code",
            client_id: clientId,
            device_code: grant.device_code,
        }),
    );
    const tokens = Tokens.parse(await exchanged.json());

    // revoke the client, which no longer identifies
    await person.client(accountPackage).client.revoke({
        userId: person.id,
        id: clientId,
        requestId: RequestId.create(fixture.clock.now()),
    });
    const refreshed = await device.fetch(
        "/oauth2/token",
        new URLSearchParams({
            grant_type: "refresh_token",
            client_id: clientId,
            refresh_token: present(tokens.refresh_token, "the refresh token"),
        }),
    );
    expect(await answered(refreshed)).toEqual([
        400,
        { error: "invalid_client", error_description: "missing client" },
    ]);
});

test("list and revoke a session over the account package", async () => {
    // sign the owner in on two browsers, each beside a native client
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const { person: owner, browser: ownerBrowser } = await fixture.signInWithLink(EMAIL);
    const { browser: otherBrowser } = await fixture.signInWithLink(EMAIL);
    const sessions = owner.client(accountPackage).session;

    // list the sessions of both browsers and both clients of the user
    const listed = await sessions.list({ userId: owner.id });
    const stored = await fixture.database
        .select({ id: session.table.id })
        .from(session.table)
        .where(eq(session.table.userId, owner.id));
    expect(
        listed.items.map(({ id, userId, revokedAt }) => ({ id, userId, revokedAt })).toSorted(byId),
    ).toEqual(stored.map(({ id }) => ({ id, userId: owner.id, revokedAt: null })).toSorted(byId));

    // revoke the other browser's session to sign it out
    const other = (await current(otherBrowser)).session.id;
    const before = present(
        listed.items.find((item) => item.id === other),
        "the other browser's session",
    );
    const revoked = await sessions.revoke({
        userId: owner.id,
        id: other,
        requestId: RequestId.create(fixture.clock.now()),
    });
    expect([revoked, await answered(await otherBrowser.fetch("/get-session"))]).toEqual([
        {
            ...before,
            ...ObjectReference.columns("updatedBy", owner.subject),
            revokedAt: fixture.clock.now(),
            updatedAt: fixture.clock.now(),
            revision: before.revision + 1,
        },
        [401, { message: "session is no longer authorized" }],
    ]);

    // revoke every other session of the user except the calling one
    const { browser: thirdBrowser } = await fixture.signInWithLink(EMAIL);
    const calling = (await current(ownerBrowser)).session.id;
    await sessions.revokeOthers({
        userId: owner.id,
        id: calling,
        requestId: RequestId.create(fixture.clock.now()),
    });
    expect([
        (await current(ownerBrowser)).session.id,
        await answered(await thirdBrowser.fetch("/get-session")),
    ]).toEqual([calling, [401, { message: "session is no longer authorized" }]]);

    // close Better Auth's endpoints the session objects replace
    const closed = await Promise.all([
        ownerBrowser.fetch("/list-sessions"),
        ownerBrowser.fetch("/revoke-sessions", {}),
        ownerBrowser.fetch("/update-user", { name: "Renamed" }),
    ]);
    expect(closed.map((response) => response.status)).toEqual([404, 404, 404]);
});

test("register a passkey and sign in with it", async () => {
    // register the passkey through the browser ceremony
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser: ownerBrowser } = await fixture.signInWithLink(EMAIL);
    const authenticator = await PasskeyAuthenticator.generate(ORIGIN);
    const creation = await ownerBrowser.fetch("/passkey/generate-register-options");
    const registered = await ownerBrowser.fetch("/passkey/verify-registration", {
        response: await authenticator.register(CeremonyOptions.parse(await creation.json())),
        name: "laptop",
    });

    // sign in with the passkey in another browser
    const browser = fixture.browser();
    const request = await browser.fetch("/passkey/generate-authenticate-options");
    const signedIn = await browser.fetch("/passkey/verify-authentication", {
        response: await authenticator.authenticate(CeremonyOptions.parse(await request.json())),
    });
    const signed = await current(browser);
    const active = await ActiveSession.read(
        signed.session.id,
        fixture.database,
        fixture.clock.now(),
    );

    // store the passkey, and record it as the session's ceremony
    expect({
        statuses: [registered.status, signedIn.status],
        passkeys: await fixture.database
            .select({ name: passkey.table.name, userId: passkey.table.userId })
            .from(passkey.table),
        user: signed.user,
        method: active.authenticationMethod,
    }).toEqual({
        statuses: [200, 200],
        passkeys: [{ name: "laptop", userId: owner.id }],
        user: { id: owner.id, email: EMAIL, emailVerified: true },
        method: "webauthn",
    });
});

test("sign people up with github and google, taking their name, time zone and locale from the profile or the sign-up", async () => {
    // complete each provider's authorization code callback in a separate browser
    await using fixture = await TestAccountStore.open({
        signIn: {
            providers: {
                github: { clientId: "github-client", clientSecret: "github-secret" },
                google: { clientId: "google-client", clientSecret: "google-secret" },
            },
        },
    });
    answerProviders(fixture);
    const signUp = async (provider: "github" | "google") => {
        // follow the provider's page back to the callback with the sign-in's state
        const browser = fixture.browser();
        const started = await browser.fetch("/sign-in/social", {
            provider,
            callbackURL: "/",
            additionalData: { ...SIGN_UP, name: "Octo", handle: provider, residencyId: "eu" },
        });
        const page = new URL(Redirect.parse(await started.json()).url);
        const state = present(page.searchParams.get("state"), "the state");
        await browser.fetch(
            `/callback/${provider}?code=${provider}-code&state=${encodeURIComponent(state)}`,
        );

        // read the person the callback signed up
        const signedUp = await current(browser);

        return fixture.database
            .select({
                name: user.table.name,
                timeZone: user.table.timeZone,
                locale: user.table.locale,
            })
            .from(user.table)
            .where(eq(user.table.id, schema.identifier("user").parse(signedUp.user.id)));
    };

    // store each person's sign-up, and each provider identity without its signed ID token
    expect({
        github: await signUp("github"),
        google: await signUp("google"),
        logins: await fixture.database
            .select({
                providerId: userLogin.table.providerId,
                providerUserId: userLogin.table.providerUserId,
                idToken: userLogin.table.idToken,
            })
            .from(userLogin.table),
    }).toEqual({
        github: [{ name: "Octo", ...SIGN_UP }],
        google: [{ name: "Ada", timeZone: "Europe/London", locale: "en-GB" }],
        logins: [
            { providerId: "github", providerUserId: "1", idToken: null },
            { providerId: "google", providerUserId: "google-subject", idToken: null },
        ],
    });
});
