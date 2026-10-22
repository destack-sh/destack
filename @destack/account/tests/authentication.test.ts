import { AuditCaller, JournalEntry, journal } from "@destack/object";
import { Subject, RequestId } from "@destack/sync";
import { Account, account, passkey, session, twoFactor, userLogin } from "../src/object/index.ts";
import { user } from "../src/object/user.ts";
import { expect, onTestFinished, single, test } from "@destack/test";
import { eq, gte } from "@destack/db";
import { Identifier, type JsonObject, present, schema } from "@destack/schema";
import type { BetterAuthAuthenticator } from "../src/better-auth/authenticator.ts";
import { parseEnvelope } from "better-auth/crypto";
import { ActiveSession } from "../src/better-auth/active-session.ts";
import { verification } from "../src/stack/index.ts";

import { PasskeyAuthenticator, authenticatorCode } from "./authenticator.ts";
import { AccountFixture, Browser, ISSUER, ORIGIN, SIGN_UP } from "../src/test/index.ts";
import {
    at,
    claims,
    DeviceGrant,
    Enrollment,
    locationOf,
    Redirect,
    Refusal,
    Tokens,
} from "./fixture.ts";
import { principal } from "@destack/access";
import { PackageId } from "@destack/package";
import accountPackage from "../src/package.ts";

/** The session Better Auth reports for a browser's cookie. */
const CurrentSession = schema.looseObject({
    /** The session. */
    session: schema.looseObject({
        /** The session's identifier. */
        id: schema.string(),
    }),
    /** The signed-in user. */
    user: schema.looseObject({
        /** The user's identifier. */
        id: schema.string(),
        /** The user's email address. */
        email: schema.string(),
        /** Whether the user proved the email address. */
        emailVerified: schema.boolean(),
    }),
});

/** The options a WebAuthn ceremony starts with. */
const CeremonyOptions = schema.looseObject({
    /** The challenge the authenticator signs. */
    challenge: schema.string(),
});

/** Sign in through a delivered link, reject reuse, and revoke the persistent session. */
test("sign in and revoke a session", async () => {
    await using fixture = await AccountFixture.open();
    const database = fixture.storage.database;
    const authenticator = fixture.authenticator;
    const started = Date.now();

    // request a sign-in link through the HTTP route a browser calls
    const requested = await fixture.fetch(
        new Request(`${ISSUER}/sign-in/magic-link`, {
            method: "POST",
            headers: { "content-type": "application/json", origin: ORIGIN },
            body: JSON.stringify({
                email: "florian@example.com",
                callbackURL: "/",
                metadata: { ...SIGN_UP, name: "Florian", handle: "florian", residencyId: "eu" },
            }),
        }),
    );
    expect([requested.status, await requested.json()]).toEqual([200, { status: true }]);
    const messages = await fixture.links();
    expect(messages.map(({ email }) => email)).toEqual(["florian@example.com"]);

    // carry the sign-up in the link's query
    const link = at(messages, 0).url;
    const query = new URL(link).searchParams;
    query.delete("token");
    expect(Object.fromEntries(query)).toEqual({
        callbackURL: "/",
        ...SIGN_UP,
        name: "Florian",
        handle: "florian",
        residencyId: "eu",
    });

    // retain only a hashed link token
    const token = new URL(link).searchParams.get("token");
    const challenge = single(await database.select().from(verification));

    // store the SHA-256 digest of the link token in base64url
    const digest = await crypto.subtle.digest(
        "SHA-256",
        new TextEncoder().encode(present(token, "the link token")),
    );
    expect(challenge.identifier).toBe(
        new Uint8Array(digest).toBase64({ alphabet: "base64url", omitPadding: true }),
    );
    const verified = await fixture.fetch(new Request(link));
    expect(verified.status).toBe(302);
    const cookies = verified.headers
        .getSetCookie()
        .map((cookie) => cookie.split(";")[0])
        .join("; ");
    const replay = await fixture.fetch(new Request(link));
    expect(new URL(locationOf(replay), ORIGIN).searchParams.get("error")).toBe("INVALID_TOKEN");
    expect(await database.select().from(verification)).toEqual([]);

    // inspect the actual shared user and session records
    const person = single(
        await database.select().from(user.table).where(eq(user.table.email, "florian@example.com")),
    );
    const login = single(
        await database.select().from(session.table).where(eq(session.table.userId, person.id)),
    );
    expect(schema.identifier("user").parse(person.id)).toBe(person.id);
    expect(schema.identifier("session").parse(login.id)).toBe(login.id);
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
    const result = await authenticator.auth.api.getSession({
        headers: new Headers({ cookie: cookies }),
    });
    expect(result?.user.id).toBe(person.id);
    const verifiedSession = await ActiveSession.read(login.id, database, Date.now());
    expect(verifiedSession.authenticationMethod).toBe("magic-link");

    // reject foreign browser origins before changing persistent state
    const foreign = await fixture.fetch(
        new Request(`${ISSUER}/sign-out`, {
            method: "POST",
            headers: { cookie: cookies, origin: "https://attacker.example" },
        }),
    );
    expect([foreign.status, await foreign.json()]).toEqual([
        403,
        { code: "INVALID_ORIGIN", message: "Invalid origin" },
    ]);
    expect(
        await database
            .select({ id: session.table.id })
            .from(session.table)
            .where(eq(session.table.userId, person.id)),
    ).toEqual([{ id: login.id }]);

    // apply administrative revocation without relying on cookie expiry
    await database
        .update(session.table)
        .set({ revokedAt: Date.now() })
        .where(eq(session.table.id, login.id));
    const revoked = await fixture.fetch(
        new Request(`${ISSUER}/get-session`, {
            headers: { cookie: cookies },
        }),
    );
    expect([revoked.status, await revoked.json()]).toEqual([
        401,
        { message: "session is no longer authorized" },
    ]);

    // sign out through the browser protocol and reject the previously valid cookie
    const signedOut = await fixture.fetch(
        new Request(`${ISSUER}/sign-out`, {
            method: "POST",
            headers: { cookie: cookies, origin: ORIGIN },
        }),
    );
    expect([signedOut.status, await signedOut.json()]).toEqual([200, { success: true }]);
    expect(
        await authenticator.auth.api.getSession({ headers: new Headers({ cookie: cookies }) }),
    ).toBeNull();
    expect(
        await database.select().from(session.table).where(eq(session.table.userId, person.id)),
    ).toEqual([]);

    // retain each authentication request and its outcome without credential contents
    const rows = await database
        .select()
        .from(journal)
        .where(gte(journal.startedAt, started))
        .orderBy(journal.startedAt, journal.position);
    const calls = rows.map((row) => JournalEntry.parse(row.call));
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
                error: { code: "UNAUTHORIZED", message: "authentication rejected" },
            },
        },
        {
            endpoint: "/sign-out",
            outcome: {
                kind: "failure",
                error: { code: "FORBIDDEN", message: "authentication rejected" },
            },
        },
        { endpoint: "/sign-out", outcome: { kind: "success" } },
    ]);

    // retain the person's personal account and session beside the system's sending of the sign-in mail
    const personal = single(
        await database
            .select({ id: account.table.id })
            .from(account.table)
            .where(eq(account.table.scope, person.id)),
    );
    expect(
        calls
            .filter(
                (call) =>
                    call.method !== "authentication.request" &&
                    Subject.same(
                        AuditCaller.actor(call.execution.context.caller).subject,
                        principal.user.reference("universe", person.id),
                    ),
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
            actor: { subject: principal.user.reference("universe", person.id) },
            target: { type: "account", id: personal.id },
            details: {},
            outcome: { kind: "success" },
        },
        {
            method: "session.create",
            actor: { subject: principal.user.reference("universe", person.id) },
            target: { type: "session", id: login.id },
            details: { path: "/magic-link/verify" },
            outcome: { kind: "success" },
        },
    ]);
});

/** Sign a person stored with an unproven email in with a mailed code, proving the email. */
test("sign a person with an unproven email in with a mailed code, proving the email", async () => {
    await using fixture = await AccountFixture.open();
    const database = fixture.storage.database;
    const browser = fixture.browser();

    // store a person whose email nobody proved yet
    const id = schema.identifier("user").parse("user-01996ab0-0000-7000-8000-0000000000e1");
    const email = "ada@example.com";
    await database.insert(user.table).values({
        id,
        name: "Ada",
        email,
        ...SIGN_UP,
        homeId: Account.home(Account.personal(id)),
        createdAt: 1,
        updatedAt: 1,
    });

    // send a code and sign in with it
    const sent = await browser.fetch("/email-otp/send-verification-otp", {
        email,
        type: "sign-in",
    });
    const signedIn = await browser.fetch("/sign-in/email-otp", {
        email,
        otp: at(await fixture.codes(), 0).otp,
    });

    // expect the person signed in with the email proven
    const [person] = await database.select().from(user.table).where(eq(user.table.id, id));
    expect({
        statuses: [sent.status, signedIn.status],
        verified: person?.emailVerified,
        challenges: await database.select().from(verification),
    }).toEqual({ statuses: [200, 200], verified: true, challenges: [] });
});

/** Sign in with the latest of several mailed codes, as a person asking for a new code does. */
test("sign a person in with the latest of two mailed codes", async () => {
    await using fixture = await AccountFixture.open();
    const browser = fixture.browser();
    const email = "bea@example.com";
    await fixture.person(email);

    // send a code, leave it unused and send another
    const send = () =>
        browser.fetch("/email-otp/send-verification-otp", { email, type: "sign-in" });
    const sent = [(await send()).status, (await send()).status];

    // sign in with the second code
    const signedIn = await browser.fetch("/sign-in/email-otp", {
        email,
        otp: at(await fixture.codes(), 1).otp,
    });
    expect([...sent, signedIn.status]).toEqual([200, 200, 200]);
});

/** Require the enrolled factor after passwordless sign-in and consume recovery codes once. */
test("require the second factor after passwordless sign-in", async () => {
    await using fixture = await AccountFixture.open();
    const database = fixture.storage.database;
    let browser = fixture.browser();

    // store the person, then sign in using a mailed code and enroll a real authenticator
    const person = await fixture.person("florian@example.com");
    const sent = await browser.fetch("/email-otp/send-verification-otp", {
        email: "florian@example.com",
        type: "sign-in",
    });
    expect([sent.status, await sent.json()]).toEqual([200, { success: true }]);
    const signedIn = await browser.fetch("/sign-in/email-otp", {
        email: "florian@example.com",
        otp: at(await fixture.codes(), -1).otp,
    });
    expect(signedIn.status).toBe(200);
    const enrollment = await browser.fetch("/two-factor/enable", { method: "totp" });
    expect(enrollment.status).toBe(200);
    const { totpURI, backupCodes } = Enrollment.parse(await enrollment.json());
    const enrolled = await browser.fetch("/two-factor/verify-totp", {
        code: authenticatorCode(totpURI),
    });
    expect(enrolled.status).toBe(200);
    await browser.fetch("/sign-out", {});

    // retain encrypted credentials in our table across a rotation of the root key
    const factor = single(await database.select().from(twoFactor.table));
    expect([
        parseEnvelope(factor.secret)?.version,
        parseEnvelope(factor.backupCodes)?.version,
    ]).toEqual([1, 1]);
    const authenticator = await fixture.rotate();
    browser = browse(authenticator);

    // stop a magic link at the second factor
    await browser.fetch("/sign-in/magic-link", {
        email: "florian@example.com",
        callbackURL: "/",
    });
    const challenge = await browser.fetch(at(await fixture.links(), -1).url);
    const stopped = new URL(locationOf(challenge), ORIGIN);
    expect([challenge.status, `${stopped.origin}${stopped.pathname}`]).toEqual([
        302,
        `${ISSUER}/sign-in/two-factor`,
    ]);
    expect(await (await browser.fetch("/get-session")).json()).toBeNull();

    // race two valid challenges against one recovery code in the shared database
    const contenders = [browse(authenticator), browse(authenticator)];
    for (const contender of contenders) {
        await contender.fetch("/sign-in/magic-link", {
            email: "florian@example.com",
            callbackURL: "/",
        });
        await contender.fetch(at(await fixture.links(), -1).url);
    }
    const attempts = await Promise.all(
        contenders.map((contender) =>
            contender.fetch("/two-factor/verify-backup-code", { code: backupCodes[1] }),
        ),
    );
    // complete a conflicted attempt after the winning transaction has consumed the code
    for (const [index, contender] of contenders.entries()) {
        const attempt = at(attempts, index);
        if (attempt.status === 409) {
            expect(await attempt.json()).toEqual({
                message: "Failed to verify backup code. Please try again.",
            });
            await contender.fetch("/sign-in/magic-link", {
                email: "florian@example.com",
                callbackURL: "/",
            });
            await contender.fetch(at(await fixture.links(), -1).url);
            attempts[index] = await contender.fetch("/two-factor/verify-backup-code", {
                code: backupCodes[1],
            });
        }
    }
    expect(
        attempts.map((response) => response.status).toSorted((left, right) => left - right),
    ).toEqual([200, 401]);
    const rejected = present(
        attempts.find((response) => response.status === 401),
        "the rejected attempt",
    );
    expect(await rejected.json()).toEqual({
        code: "INVALID_BACKUP_CODE",
        message: "Invalid backup code",
    });

    // complete the ceremony with a recovery code
    const recovered = await browser.fetch("/two-factor/verify-backup-code", {
        code: backupCodes[0],
    });
    expect(recovered.status, await recovered.clone().text()).toBe(200);
    const consumed = single(await database.select().from(twoFactor.table));
    expect(parseEnvelope(consumed.backupCodes)?.version).toBe(2);
    const recoveredSession = CurrentSession.parse(
        await (await browser.fetch("/get-session")).json(),
    );
    expect(recoveredSession.user.email).toBe("florian@example.com");

    // approve a device's new client in the recovered session, and redeem its code exactly once as the client
    const device = browse(authenticator);
    const clientId = Identifier.create("client");
    const offered = await device.fetch(
        "/device/code",
        new URLSearchParams({
            client_id: clientId,
            scope: "openid offline_access",
            resource: PackageId.resource(accountPackage.id),
        }),
    );
    expect(offered.status, await offered.clone().text()).toBe(200);
    const grant = DeviceGrant.parse(await offered.json());
    const approved = await browser.fetch(
        `/device?${new URLSearchParams({ user_code: grant.user_code })}`,
        new URLSearchParams({ action: "approve" }),
    );
    expect(approved.status).toBe(200);
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
    expect(exchanged.status, await exchanged.clone().text()).toBe(200);
    const tokens = Tokens.parse(await exchanged.json());
    const reused = await redeem();
    expect({
        caller: claims(tokens.access_token).caller.credential,
        reused: [reused.status, Refusal.parse(await reused.json()).error],
    }).toEqual({
        caller: { kind: "client", id: clientId },
        reused: [400, "invalid_grant"],
    });

    // refuse refreshing the device's tokens once the person revokes its client, which no longer identifies
    await person.client.client.revoke({
        userId: person.id,
        id: clientId,
        requestId: RequestId.create(Date.now()),
    });
    const refreshed = await device.fetch(
        "/oauth2/token",
        new URLSearchParams({
            grant_type: "refresh_token",
            client_id: clientId,
            refresh_token: present(tokens.refresh_token, "the refresh token"),
        }),
    );
    expect([refreshed.status, Refusal.parse(await refreshed.json()).error]).toEqual([
        400,
        "invalid_client",
    ]);

    // refuse the spent recovery code in a new sign-in
    await browser.fetch("/sign-in/magic-link", {
        email: "florian@example.com",
        callbackURL: "/",
    });
    await browser.fetch(at(await fixture.links(), -1).url);
    const replay = await browser.fetch("/two-factor/verify-backup-code", {
        code: backupCodes[0],
    });
    expect([replay.status, await replay.json()]).toEqual([
        401,
        { code: "INVALID_BACKUP_CODE", message: "Invalid backup code" },
    ]);
    expect(await (await browser.fetch("/get-session")).json()).toBeNull();
});

/** List a user's sessions and revoke one through the account package. */
test("list and revoke a session over the account package", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("owner@example.com");
    const other = await fixture.signInWithLink("owner@example.com");

    // list the sessions of both browsers and both clients of the user
    const listed = await owner.client.session.list({ userId: owner.id });
    expect(listed.items.map((item) => [item.userId, item.revokedAt])).toEqual([
        [owner.id, null],
        [owner.id, null],
        [owner.id, null],
        [owner.id, null],
    ]);

    // revoke the other browser's session to sign it out
    const current = CurrentSession.parse(await (await other.browser.fetch("/get-session")).json());
    const revoked = await owner.client.session.revoke({
        userId: owner.id,
        id: current.session.id,
        requestId: RequestId.create(Date.now()),
    });
    expect(revoked.revokedAt).toBe(revoked.updatedAt);
    expect(await (await other.browser.fetch("/get-session")).json()).toEqual({
        message: "session is no longer authorized",
    });

    // revoke every other session of the user except the calling one
    const third = await fixture.signInWithLink("owner@example.com");
    const own = CurrentSession.parse(await (await owner.browser.fetch("/get-session")).json());
    await owner.client.session.revokeOthers({
        userId: owner.id,
        id: own.session.id,
        requestId: RequestId.create(Date.now()),
    });
    expect([
        (await owner.browser.fetch("/get-session")).status,
        (await third.browser.fetch("/get-session")).status,
    ]).toEqual([200, 401]);

    // close Better Auth's endpoints the session objects replace
    const closed = await Promise.all([
        owner.browser.fetch("/list-sessions"),
        owner.browser.fetch("/revoke-sessions", {}),
        owner.browser.fetch("/update-user", { name: "Renamed" }),
    ]);
    expect(closed.map((response) => response.status)).toEqual([404, 404, 404]);
});

/** Register a passkey from a fresh session, then sign in with it alone at the highest assurance. */
test("register a passkey and sign in with it", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("owner@example.com");
    const authenticator = await PasskeyAuthenticator.generate(ORIGIN);

    // register the passkey through the browser ceremony
    const creation = await owner.browser.fetch("/passkey/generate-register-options");
    const registered = await owner.browser.fetch("/passkey/verify-registration", {
        response: await authenticator.register(CeremonyOptions.parse(await creation.json())),
        name: "laptop",
    });
    expect(registered.status, await registered.clone().text()).toBe(200);
    expect(
        await fixture.storage.database
            .select({ name: passkey.table.name, userId: passkey.table.userId })
            .from(passkey.table),
    ).toEqual([{ name: "laptop", userId: owner.id }]);

    // sign in with the passkey in another browser
    const browser = fixture.browser();
    const request = await browser.fetch("/passkey/generate-authenticate-options");
    const signedIn = await browser.fetch("/passkey/verify-authentication", {
        response: await authenticator.authenticate(CeremonyOptions.parse(await request.json())),
    });
    expect(signedIn.status, await signedIn.clone().text()).toBe(200);
    const current = CurrentSession.parse(await (await browser.fetch("/get-session")).json());
    expect(current.user.id).toBe(owner.id);

    // record the passkey as the session's ceremony
    const active = await ActiveSession.read(
        current.session.id,
        fixture.storage.database,
        Date.now(),
    );
    expect(active.authenticationMethod).toBe("webauthn");
});

/** Sign in through GitHub's and Google's OAuth callbacks, named by the profile else the sign-up. */
test("sign in with github and google", async () => {
    await using fixture = await AccountFixture.open({
        signIn: {
            providers: {
                github: { clientId: "github-client", clientSecret: "github-secret" },
                google: { clientId: "google-client", clientSecret: "google-secret" },
            },
        },
    });
    answerProviders();

    // complete each provider's authorization code callback in a separate browser
    const signIn = async (provider: "github" | "google") => {
        const browser = fixture.browser();
        const started = await browser.fetch("/sign-in/social", {
            provider,
            callbackURL: "/",
            additionalData: { ...SIGN_UP, name: "Octo", handle: provider, residencyId: "eu" },
        });
        const page = new URL(Redirect.parse(await started.json()).url);
        const state = present(page.searchParams.get("state"), "the state");
        const returned = await browser.fetch(
            `/callback/${provider}?code=${provider}-code&state=${encodeURIComponent(state)}`,
        );
        const current = CurrentSession.parse(await (await browser.fetch("/get-session")).json());
        const active = await ActiveSession.read(
            current.session.id,
            fixture.storage.database,
            Date.now(),
        );
        const [person] = await fixture.storage.database
            .select({
                name: user.table.name,
                timeZone: user.table.timeZone,
                locale: user.table.locale,
            })
            .from(user.table)
            .where(eq(user.table.id, schema.identifier("user").parse(current.user.id)));

        return {
            page: `${page.origin}${page.pathname}`,
            redirect: [returned.status, returned.headers.get("location")],
            email: current.user.email,
            verified: current.user.emailVerified,
            method: active.authenticationMethod,
            signUp: person,
        };
    };
    expect([await signIn("github"), await signIn("google")]).toEqual([
        {
            page: "https://github.com/login/oauth/authorize",
            redirect: [302, "/"],
            email: "octo@example.com",
            verified: true,
            method: "oauth",
            signUp: { name: "Octo", ...SIGN_UP },
        },
        {
            page: "https://accounts.google.com/o/oauth2/v2/auth",
            redirect: [302, "/"],
            email: "ada@example.com",
            verified: true,
            method: "oauth",
            signUp: { name: "Ada", timeZone: "Europe/London", locale: "en-GB" },
        },
    ]);

    // store each provider identity without its signed ID token
    expect(
        await fixture.storage.database
            .select({
                providerId: userLogin.table.providerId,
                providerUserId: userLogin.table.providerUserId,
                idToken: userLogin.table.idToken,
            })
            .from(userLogin.table),
    ).toEqual([
        { providerId: "github", providerUserId: "1", idToken: null },
        { providerId: "google", providerUserId: "google-subject", idToken: null },
    ]);
});

/** Answer the GitHub and Google endpoints Better Auth calls during callbacks. */
function answerProviders(): void {
    // an ID token Google would sign for the configured client
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
            zoneinfo: "Europe/London",
            locale: "en-gb",
            iat: now,
            exp: now + 3600,
        }),
        "signature",
    ].join(".");

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
    const original = globalThis.fetch;
    const answer = async (input: RequestInfo | URL) => {
        const url = input instanceof Request ? input.url : String(input);
        if (!Object.hasOwn(answers, url)) {
            throw new Error(`unexpected request to ${url}`);
        }

        return Response.json(answers[url]);
    };
    globalThis.fetch = Object.assign(answer, original);
    onTestFinished(() => {
        globalThis.fetch = original;
    });
}

/** Encode a JSON value as an unpadded base64url token part. */
function encode(value: object): string {
    return new TextEncoder()
        .encode(JSON.stringify(value))
        .toBase64({ alphabet: "base64url", omitPadding: true });
}

/** Open a browser of an authenticator's routes below the issuer, as the oauth adapter has them. */
function browse(authenticator: BetterAuthAuthenticator): Browser {
    // send each request with its path below the issuer's
    const prefix = new URL(ISSUER).pathname;
    const send = (request: Request) => {
        const url = new URL(request.url);
        const below = `${ORIGIN}${url.pathname.slice(prefix.length)}${url.search}`;

        return authenticator.handler(new Request(below, request));
    };

    return new Browser(ISSUER, send, SIGN_UP.locale);
}
