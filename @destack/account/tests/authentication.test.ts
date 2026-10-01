import { AuditCaller } from "@destack/audit";
import { identity, passkey, session, twoFactor } from "../src/object/authentication.ts";
import { testCallKey } from "@destack/service/test";
import { user } from "../src/object/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { eq } from "@destack/db";
import { identifier } from "@destack/schema";
import {
    createAuthenticator,
    type AuthenticatorOptions,
} from "../src/authentication/authentication.ts";
import { parseEnvelope } from "better-auth/crypto";
import { AccountAuthentication } from "../src/authentication/caller.ts";
import { ActiveSession } from "../src/authentication/session.ts";
import { verification } from "../src/stack/index.ts";
import { openAccountDatabase } from "./database.ts";
import { Browser, PasskeyAuthenticator, authenticatorCode } from "./browser.ts";
import { AccountFixture } from "./fixture.ts";
import { principal } from "@destack/access";
import { RequestId } from "@destack/service/request";

/** Sign in through a delivered link, reject reuse, and revoke the persistent session. */
test("sign in and revoke a session", async () => {
    await using fixture = await AccountFixture.open();
    const database = fixture.opened.database;
    const authenticator = fixture.authenticator;
    const messages = fixture.links;

    // request a sign-in link through the HTTP route a browser calls
    const requested = await authenticator.handler(
        new Request("http://localhost:3210/auth/sign-in/magic-link", {
            method: "POST",
            headers: { "content-type": "application/json", origin: "http://localhost:3210" },
            body: JSON.stringify({
                email: "florian@example.com",
                name: "Florian",
                callbackURL: "/",
            }),
        }),
    );
    expect([requested.status, await requested.json()]).toEqual([200, { status: true }]);
    expect(messages.map(({ email }) => email)).toEqual(["florian@example.com"]);

    // retain only a hashed link token, then consume it exactly once
    const token = new URL(messages[0].url).searchParams.get("token");
    const challenges = await database.select().from(verification);
    expect(challenges).toHaveLength(1);
    expect(challenges[0].identifier).not.toBe(token);
    const verified = await authenticator.handler(new Request(messages[0].url));
    expect(verified.status).toBe(302);
    const cookies = verified.headers
        .getSetCookie()
        .map((cookie) => cookie.split(";")[0])
        .join("; ");
    const replay = await authenticator.handler(new Request(messages[0].url));
    expect(
        new URL(replay.headers.get("location")!, "http://localhost:3210").searchParams.get("error"),
    ).toBe("INVALID_TOKEN");
    expect(await database.select().from(verification)).toEqual([]);

    // inspect the actual shared user and session records
    const [person] = await database.select().from(user.table);
    const [login] = await database.select().from(session.table);
    expect(identifier("user").parse(person.id)).toBe(person.id);
    expect(identifier("session").parse(login.id)).toBe(login.id);
    expect({
        name: person.name,
        email: person.email,
        verified: person.emailVerified,
        userId: login.userId,
    }).toEqual({
        name: "Florian",
        email: "florian@example.com",
        verified: true,
        userId: person.id,
    });
    const result = await authenticator.api.getSession({
        headers: new Headers({ cookie: cookies }),
    });
    expect(result?.user.id).toBe(person.id);
    const verifiedSession = await ActiveSession.read(login.id, database);
    expect(verifiedSession.authenticationMethod).toBe("magic-link");
    await expect(verifiedSession.requireAuthentication(database, true)).rejects.toMatchObject({
        statusCode: 403,
        body: {
            code: "WEBAUTHN_REQUIRED",
            message: "recent WebAuthn authentication is required",
        },
    });

    // reject foreign browser origins before changing persistent state
    const foreign = await authenticator.handler(
        new Request("http://localhost:3210/auth/sign-out", {
            method: "POST",
            headers: { cookie: cookies, origin: "https://attacker.example" },
        }),
    );
    expect([foreign.status, await foreign.json()]).toEqual([
        403,
        { code: "INVALID_ORIGIN", message: "Invalid origin" },
    ]);
    expect(await database.select({ id: session.table.id }).from(session.table)).toEqual([
        { id: login.id },
    ]);

    // apply administrative revocation without relying on cookie expiry
    await database
        .update(session.table)
        .set({ revokedAt: Date.now() })
        .where(eq(session.table.id, login.id));
    const revoked = await authenticator.handler(
        new Request("http://localhost:3210/auth/get-session", {
            headers: { cookie: cookies },
        }),
    );
    expect([revoked.status, await revoked.json()]).toEqual([
        401,
        { message: "session is no longer authorized" },
    ]);

    // sign out through the browser protocol and reject the previously valid cookie
    const signedOut = await authenticator.handler(
        new Request("http://localhost:3210/auth/sign-out", {
            method: "POST",
            headers: { cookie: cookies, origin: "http://localhost:3210" },
        }),
    );
    expect([signedOut.status, await signedOut.json()]).toEqual([200, { success: true }]);
    expect(
        await authenticator.api.getSession({ headers: new Headers({ cookie: cookies }) }),
    ).toBeNull();
    expect(await database.select().from(session.table)).toEqual([]);

    // retain each authentication request and its outcome without credential contents
    const calls = await fixture.delivered();
    expect(
        calls
            .filter((call) => call.method === "authentication.request")
            .map((call) => ({
                endpoint: call.execution.targets.endpoint!.id,
                outcome: call.execution.outcome,
            })),
    ).toEqual([
        { endpoint: "/sign-in/magic-link", outcome: { kind: "success" } },
        { endpoint: "/magic-link/verify", outcome: { kind: "success" } },
        {
            endpoint: "/magic-link/verify",
            outcome: {
                kind: "failure",
                error: {
                    code: "AUTHENTICATION_REJECTED",
                    status: 302,
                    message: "authentication rejected",
                },
            },
        },
        {
            endpoint: "/sign-out",
            outcome: {
                kind: "failure",
                error: {
                    code: "AUTHENTICATION_REJECTED",
                    status: 403,
                    message: "authentication rejected",
                },
            },
        },
        { endpoint: "/sign-out", outcome: { kind: "success" } },
    ]);
    expect(
        calls
            .filter((call) => call.method !== "authentication.request")
            .map((call) => ({
                method: call.method,
                actor: AuditCaller.actor(call.execution.context.caller),
                targets: call.execution.targets,
                details: call.execution.details,
                outcome: call.execution.outcome,
            })),
    ).toEqual([
        {
            method: "session.create",
            actor: { type: "subject", subject: principal.user.reference("universe", person.id) },
            targets: {
                user: { type: "user", id: person.id },
                session: { type: "session", id: login.id },
            },
            details: { method: "/magic-link/verify" },
            outcome: { kind: "success" },
        },
    ]);
});

/** Require the enrolled factor after passwordless sign-in and consume recovery codes once. */
test("require the second factor after passwordless sign-in", async () => {
    const opened = await openAccountDatabase();
    const messages: { email: string; url: string }[] = [];
    const codes: { email: string; otp: string; type: string }[] = [];
    const key = { version: 1, value: "first-account-encryption-key-32-characters-minimum" };
    const options: AuthenticatorOptions = {
        database: opened.database,
        callKey: testCallKey,
        origin: "http://localhost:3210",
        trustedOrigins: ["http://localhost:3210"],
        ipAddress: { disableIpTracking: true },
        secret: "local-account-test-secret-32-characters-minimum",
        secrets: [key],
        providers: {},
        signInUri: "http://localhost:3210/sign-in",
        consentUri: "http://localhost:3210/consent",
        verificationUri: "http://localhost:3210/device",
        secondFactorUri: "http://localhost:3210/sign-in/two-factor",
        handleUri: "http://localhost:3210/sign-in/handle",
        service: async () => {
            throw new Error("the test serves no account service");
        },
        sendMagicLink: async (message) => {
            messages.push(message);
        },
        sendCode: async (message) => {
            codes.push(message);
        },
    };
    let authenticator = createAuthenticator(options);
    let browser = new Browser(options.origin, authenticator.handler);

    try {
        // sign in using a delivered email code and enroll a real authenticator
        const sent = await browser.fetch("/auth/email-otp/send-verification-otp", {
            email: "florian@example.com",
            type: "sign-in",
        });
        expect([sent.status, await sent.json()]).toEqual([200, { success: true }]);
        const signedIn = await browser.fetch("/auth/sign-in/email-otp", {
            email: "florian@example.com",
            otp: codes[0].otp,
            name: "Florian",
        });
        expect(signedIn.status).toBe(200);
        const enrollment = await browser.fetch("/auth/two-factor/enable", { method: "totp" });
        expect(enrollment.status).toBe(200);
        const { totpURI, backupCodes } = await enrollment.json();
        const enrolled = await browser.fetch("/auth/two-factor/verify-totp", {
            code: authenticatorCode(totpURI),
        });
        expect(enrolled.status).toBe(200);
        await browser.fetch("/auth/sign-out", {});

        // retain encrypted credentials in our table across deployment key rotation
        const [factor] = await opened.database.select().from(twoFactor.table);
        expect([
            parseEnvelope(factor.secret)?.version,
            parseEnvelope(factor.backupCodes)?.version,
        ]).toEqual([1, 1]);
        authenticator = createAuthenticator({
            ...options,
            secrets: [
                { version: 2, value: "second-account-encryption-key-32-characters-minimum" },
                key,
            ],
        });
        browser = new Browser(options.origin, authenticator.handler);

        // stop a magic link at the second factor, carrying its callback, without a usable session
        await browser.fetch("/auth/sign-in/magic-link", {
            email: "florian@example.com",
            callbackURL: "/",
        });
        const challenge = await browser.fetch(messages[0].url);
        expect([challenge.status, challenge.headers.get("location")]).toEqual([
            302,
            "http://localhost:3210/sign-in/two-factor?callbackURL=http%3A%2F%2Flocalhost%3A3210%2F",
        ]);
        expect(await (await browser.fetch("/auth/get-session")).json()).toBeNull();

        // race two valid challenges against one recovery code in the shared database
        const contenders = [
            new Browser(options.origin, authenticator.handler),
            new Browser(options.origin, authenticator.handler),
        ];
        for (const contender of contenders) {
            await contender.fetch("/auth/sign-in/magic-link", {
                email: "florian@example.com",
                callbackURL: "/",
            });
            await contender.fetch(messages[messages.length - 1].url);
        }
        const attempts = await Promise.all(
            contenders.map((contender) =>
                contender.fetch("/auth/two-factor/verify-backup-code", { code: backupCodes[1] }),
            ),
        );
        // complete a conflicted attempt after the winning transaction has consumed the code
        for (let index = 0; index < attempts.length; index++) {
            if (attempts[index].status === 409) {
                expect(await attempts[index].json()).toEqual({
                    message: "Failed to verify backup code. Please try again.",
                });
                await contenders[index].fetch("/auth/sign-in/magic-link", {
                    email: "florian@example.com",
                    callbackURL: "/",
                });
                await contenders[index].fetch(messages[messages.length - 1].url);
                attempts[index] = await contenders[index].fetch(
                    "/auth/two-factor/verify-backup-code",
                    {
                        code: backupCodes[1],
                    },
                );
            }
        }
        expect(
            attempts.map((response) => response.status).sort((left, right) => left - right),
        ).toEqual([200, 401]);
        const rejected = attempts.find((response) => response.status === 401)!;
        expect(await rejected.json()).toEqual({
            code: "INVALID_BACKUP_CODE",
            message: "Invalid backup code",
        });

        // a recovery code completes the ceremony, then cannot be used again
        const recovered = await browser.fetch("/auth/two-factor/verify-backup-code", {
            code: backupCodes[0],
        });
        expect(recovered.status, await recovered.clone().text()).toBe(200);
        const [consumed] = await opened.database.select().from(twoFactor.table);
        expect(parseEnvelope(consumed.backupCodes)?.version).toBe(2);
        expect((await (await browser.fetch("/auth/get-session")).json()).user.email).toBe(
            "florian@example.com",
        );

        // approve a native client and redeem its browser-confirmed grant exactly once
        const native = new Browser(options.origin, authenticator.handler);
        const offered = await native.fetch("/auth/device/code", { client_id: "destack-daemon" });
        expect(offered.status, await offered.clone().text()).toBe(200);
        const grant = await offered.json();
        expect(new URL(grant.verification_uri_complete).origin).toBe("http://localhost:3210");
        const reviewed = await browser.fetch(
            `/auth/device?user_code=${encodeURIComponent(grant.user_code)}`,
        );
        expect([reviewed.status, await reviewed.json()]).toEqual([
            200,
            {
                user_code: grant.user_code,
                status: "pending",
                client_id: "destack-daemon",
                scope: null,
            },
        ]);
        const approved = await browser.fetch("/auth/device/approve", { userCode: grant.user_code });
        expect([approved.status, await approved.json()]).toEqual([200, { success: true }]);
        const redemption = {
            grant_type: "urn:ietf:params:oauth:grant-type:device_code",
            client_id: "destack-daemon",
            device_code: grant.device_code,
        };
        const exchanged = await native.fetch("/auth/device/token", redemption);
        expect(exchanged.status, await exchanged.clone().text()).toBe(200);
        const credential = await exchanged.json();
        const [nativeSession] = await opened.database
            .select()
            .from(session.table)
            .where(eq(session.table.token, credential.access_token));
        expect(
            (
                await AccountAuthentication.authenticate(
                    new Request("http://localhost:3210/accounts", {
                        headers: { authorization: `Bearer ${credential.access_token}` },
                    }),
                    authenticator,
                )
            ).credential,
        ).toEqual({ kind: "session", id: nativeSession.id, userId: nativeSession.userId });
        const reused = await native.fetch("/auth/device/token", redemption);
        expect([reused.status, await reused.json()]).toEqual([
            400,
            { error: "invalid_grant", error_description: "Invalid device code" },
        ]);

        // native session revocation must take effect before authorization too
        await opened.database
            .update(session.table)
            .set({ revokedAt: Date.now() })
            .where(eq(session.table.id, nativeSession.id));
        await expect(
            AccountAuthentication.authenticate(
                new Request("http://localhost:3210/accounts", {
                    headers: { authorization: `Bearer ${credential.access_token}` },
                }),
                authenticator,
            ),
        ).rejects.toMatchObject({
            code: "UNAUTHORIZED",
            status: 401,
            message: "the person is not signed in",
        });
        await browser.fetch("/auth/sign-out", {});
        await browser.fetch("/auth/sign-in/magic-link", {
            email: "florian@example.com",
            callbackURL: "/",
        });
        await browser.fetch(messages[messages.length - 1].url);
        const replay = await browser.fetch("/auth/two-factor/verify-backup-code", {
            code: backupCodes[0],
        });
        expect([replay.status, await replay.json()]).toEqual([
            401,
            { code: "INVALID_BACKUP_CODE", message: "Invalid backup code" },
        ]);
        expect(await (await browser.fetch("/auth/get-session")).json()).toBeNull();
    } finally {
        await opened.close();
    }
});

/** List a user's sessions and revoke one through the account service. */
test("list and revoke a session over the account service", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const other = await fixture.signIn("owner@example.com");

    // list both sessions of the user
    const listed = await owner.client.session.list({ userId: owner.id });
    expect(listed.items.map((item) => [item.userId, item.revokedAt])).toEqual([
        [owner.id, null],
        [owner.id, null],
    ]);

    // revoke the other browser's session to sign it out
    const current = await (await other.browser.fetch("/auth/get-session")).json();
    const revoked = await owner.client.session.revoke({
        userId: owner.id,
        id: current.session.id,
        requestId: RequestId.create(),
    });
    expect(revoked.revokedAt).toBe(revoked.updatedAt);
    expect(await (await other.browser.fetch("/auth/get-session")).json()).toEqual({
        message: "session is no longer authorized",
    });

    // revoke every other session of the user except the calling one
    const third = await fixture.signIn("owner@example.com");
    const own = await (await owner.browser.fetch("/auth/get-session")).json();
    await owner.client.session.revokeOthers({
        userId: owner.id,
        id: own.session.id,
        requestId: RequestId.create(),
    });
    expect([
        (await owner.browser.fetch("/auth/get-session")).status,
        (await third.browser.fetch("/auth/get-session")).status,
    ]).toEqual([200, 401]);

    // leave the session objects the one path to list, revoke and rename, closing Better Auth's own endpoints
    const closed = await Promise.all([
        owner.browser.fetch("/auth/list-sessions"),
        owner.browser.fetch("/auth/revoke-sessions", {}),
        owner.browser.fetch("/auth/update-user", { name: "Renamed" }),
    ]);
    expect(closed.map((response) => response.status)).toEqual([404, 404, 404]);
});

/** Register a passkey from a fresh session, then sign in with it alone at the highest assurance. */
test("register a passkey and sign in with it", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const authenticator = await PasskeyAuthenticator.generate("http://localhost:3210");

    // register the passkey through the browser ceremony
    const creation = await owner.browser.fetch("/auth/passkey/generate-register-options");
    const registered = await owner.browser.fetch("/auth/passkey/verify-registration", {
        response: await authenticator.register(await creation.json()),
        name: "laptop",
    });
    expect(registered.status, await registered.clone().text()).toBe(200);
    expect(
        await fixture.opened.database
            .select({ name: passkey.table.name, userId: passkey.table.userId })
            .from(passkey.table),
    ).toEqual([{ name: "laptop", userId: owner.id }]);

    // sign in with the passkey in another browser, verified by the user
    const browser = fixture.browser();
    const request = await browser.fetch("/auth/passkey/generate-authenticate-options");
    const signedIn = await browser.fetch("/auth/passkey/verify-authentication", {
        response: await authenticator.authenticate(await request.json()),
    });
    expect(signedIn.status, await signedIn.clone().text()).toBe(200);
    const current = await (await browser.fetch("/auth/get-session")).json();
    expect(current.user.id).toBe(owner.id);

    // rate the session phishing resistant
    const active = await ActiveSession.read(current.session.id, fixture.opened.database);
    expect(active.authenticationMethod).toBe("webauthn");
    await active.requireAuthentication(fixture.opened.database, true);
});

/** Sign in through GitHub's and Google's OAuth callbacks, creating verified users and their provider identities. */
test("sign in with github and google", async () => {
    await using fixture = await AccountFixture.open({
        providers: {
            github: { clientId: "github-client", clientSecret: "github-secret" },
            google: { clientId: "google-client", clientSecret: "google-secret" },
        },
    });
    answerProviders();

    // complete each provider's authorization code callback in a browser of its own
    const signIn = async (provider: "github" | "google") => {
        const browser = fixture.browser();
        const started = await browser.fetch("/auth/sign-in/social", { provider, callbackURL: "/" });
        const page = new URL((await started.json()).url);
        const state = page.searchParams.get("state")!;
        const returned = await browser.fetch(
            `/auth/callback/${provider}?code=${provider}-code&state=${encodeURIComponent(state)}`,
        );
        const current = await (await browser.fetch("/auth/get-session")).json();
        const active = await ActiveSession.read(current.session.id, fixture.opened.database);

        return {
            page: `${page.origin}${page.pathname}`,
            redirect: [returned.status, returned.headers.get("location")],
            email: current.user.email,
            verified: current.user.emailVerified,
            method: active.authenticationMethod,
        };
    };
    expect([await signIn("github"), await signIn("google")]).toEqual([
        {
            page: "https://github.com/login/oauth/authorize",
            redirect: [302, "/"],
            email: "octo@example.com",
            verified: true,
            method: "oauth",
        },
        {
            page: "https://accounts.google.com/o/oauth2/v2/auth",
            redirect: [302, "/"],
            email: "ada@example.com",
            verified: true,
            method: "oauth",
        },
    ]);

    // keep each provider identity without its signed ID token
    expect(
        await fixture.opened.database
            .select({
                providerId: identity.table.providerId,
                providerUserId: identity.table.providerUserId,
                idToken: identity.table.idToken,
            })
            .from(identity.table),
    ).toEqual([
        { providerId: "github", providerUserId: "1", idToken: null },
        { providerId: "google", providerUserId: "google-subject", idToken: null },
    ]);
});

/** Answer the GitHub and Google endpoints Better Auth reaches during callbacks, restoring the real fetch afterwards. */
function answerProviders(): void {
    // an ID token Google would sign for the configured client
    const encode = (value: object) =>
        new TextEncoder()
            .encode(JSON.stringify(value))
            .toBase64({ alphabet: "base64url", omitPadding: true });
    const now = Math.floor(Date.now() / 1000);
    const token = [
        encode({ alg: "RS256", kid: "google-key", typ: "JWT" }),
        encode({
            iss: "https://accounts.google.com",
            aud: "google-client",
            sub: "google-subject",
            email: "ada@example.com",
            email_verified: true,
            name: "Ada",
            iat: now,
            exp: now + 3600,
        }),
        "signature",
    ].join(".");

    // answer each provider endpoint by URL
    const answers: Readonly<Record<string, unknown>> = {
        "https://github.com/login/oauth/access_token": {
            access_token: "github-access-token",
            token_type: "bearer",
            scope: "read:user,user:email",
        },
        "https://api.github.com/user": { id: 1, login: "octocat", name: "Octo Cat", email: null },
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
    const original = globalThis.fetch;
    globalThis.fetch = (async (input: RequestInfo | URL) => {
        const url = input instanceof Request ? input.url : String(input);
        if (!Object.hasOwn(answers, url)) {
            throw new Error(`unexpected request to ${url}`);
        }

        return Response.json(answers[url]);
    }) as typeof fetch;
    onTestFinished(() => {
        globalThis.fetch = original;
    });
}
