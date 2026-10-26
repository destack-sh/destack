import { principal } from "@destack/access";
import { and, eq } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { MemoryKeychain } from "@destack/identity";
import { ObjectClient } from "@destack/object/client";
import { present, schema } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, single, test } from "@destack/test";
import { account } from "../object/account.ts";
import { client } from "../object/client.ts";
import { SignUp } from "../better-auth/sign-up.ts";
import { TestAccountStore, TestBrowser, ISSUER, type TestPerson } from "../test/index.ts";
import { Issuer } from "./issuer.ts";
import { ClientKey } from "./client-key.ts";
import { SessionStore } from "./session.ts";
import { login, Login } from "./login.ts";
import { NativeSignIn } from "./sign-in.ts";
import { SimulatedClock } from "@destack/environment/simulation";

/** The time the native device's clock reads, in UTC epoch milliseconds: 2026-01-01. */
const NOW = 1_767_225_600_000;

/** The native device the fixtures sign people in on. */
const DEVICE = schema.identifier("device").parse("device-01996ab0-0000-7000-8000-0000000000e1");

/** The native device as the creator of its logins. */
const WRITER = principal.device.reference(Scope.universe.id, DEVICE);

/** The person signed in on the issuer's pages. */
const EMAIL = "ada@example.com";

/** Allow a browser sign-in on the issuer's consent page, returning the loopback redirect. */
async function consent(browser: TestBrowser, url: string): Promise<string> {
    // follow the authorization to the consent page and allow it
    const page = TestBrowser.location(await browser.fetch(url));
    const allowed = await browser.fetch(page, new URLSearchParams({ action: "approve" }));

    return TestBrowser.location(allowed);
}

/** The login a sign-in writes for a person on the native device. */
function loginRow(signedIn: { id: string; clientId: string }, person: TestPerson) {
    return {
        id: signedIn.id,
        scope: DEVICE,
        issuer: ISSUER,
        subjectId: person.id,
        clientId: signedIn.clientId,
        name: EMAIL,
        status: "authenticated",
        tags: {},
        createdAt: NOW,
        ...ObjectReference.columns("createdBy", WRITER),
        updatedAt: NOW,
        ...ObjectReference.columns("updatedBy", WRITER),
        revision: 1,
    };
}

/** Read the clients of a person, oldest first. */
async function nativeClients(accounts: TestAccountStore, person: TestPerson) {
    const rows = await accounts.database
        .select({ id: client.table.id, revokedAt: client.table.revokedAt })
        .from(client.table)
        .where(and(eq(client.table.scope, person.id), eq(client.table.kind, "native")))
        .orderBy(client.table.createdAt, client.table.id);

    return rows.map((row) => ({ id: row.id, isRevoked: row.revokedAt !== null }));
}

/** Sign people in on a native device against the fixture's issuer. */
async function native(accounts: TestAccountStore) {
    // store the laptop's logins in a local database on its own clock, in the accounts' world
    const laptop = accounts.simulation.environment("laptop");
    const device = { ...laptop, clock: new SimulatedClock(NOW) };
    const database = await connectBunSqlite(":memory:", [], { environment: device });
    onTestFinished(() => database.close());
    const subject = principal.device.reference(Scope.universe.id, DEVICE);
    const logins = await ObjectClient.open({
        database,
        objects: { login },
        environment: device,
        scope: DEVICE,
        caller: subject,
        device: subject,
        endpoint: {
            url: "http://local.invalid/",
            fetch: async () => {
                throw new TypeError("local objects cannot call a server");
            },
        },
        reconnect: () => {
            throw new TypeError("local objects move nowhere");
        },
    });
    onTestFinished(() => logins.close());

    // call the laptop's own loopback through its simulated network
    const { network } = laptop;

    // sign people in with the keychain's refresh tokens
    const issuer = new Issuer(ISSUER, (request) => accounts.fetch(request), laptop);
    const sessions = new SessionStore(new MemoryKeychain());
    const signIn = new NativeSignIn({
        issuer,
        sessions,
        keys: new ClientKey(new MemoryKeychain(), laptop),
        logins,
        environment: laptop,
        client: {
            name: "laptop",
            platform: "macOS",
            architecture: "arm64",
            version: "2026.9.0",
        },
    });

    return { signIn, issuer, logins, sessions, network, laptop };
}

test("ask the issuer for a fresh sign-in within the challenged age in a step-up, which takes every factor the person enrolled", async () => {
    await using accounts = await TestAccountStore.open();
    const { signIn } = await native(accounts);

    // open the browser at the issuer and stop the sign-in before it returns
    const opened: URL[] = [];
    const stopping = new AbortController();
    const signedIn = signIn.browser(
        async (url) => {
            opened.push(new URL(url));
            stopping.abort();
        },
        { assurance: 2, maxAge: 900_000 },
        stopping.signal,
    );
    const stopped = await signedIn.then(
        () => "signed in",
        (error: unknown) => error,
    );
    expect(stopped).toBe(stopping.signal.reason);

    // ask for a fresh sign-in within fifteen minutes
    const asked = single(opened).searchParams;
    expect([asked.get("prompt"), asked.get("max_age")]).toEqual(["login", "900"]);
});

test("sign a person in through a browser redirect with PKCE as a new client, and sign them out revoking its refresh token", async () => {
    await using accounts = await TestAccountStore.open();
    const { person: owner, browser: ownerBrowser } = await accounts.signInWithLink(EMAIL);
    const { signIn, issuer, logins, sessions, network } = await native(accounts);
    const before = await nativeClients(accounts, owner);

    // allow the sign-in on the consent page and return to the loopback redirect
    const callbacks: string[] = [];
    const pages: Response[] = [];
    const signedIn = await signIn.browser(async (url) => {
        const callback = await consent(ownerBrowser, url);
        callbacks.push(callback);
        pages.push(await network.fetch(new Request(callback)));
    });
    const callback = single(callbacks);
    const page = single(pages);

    // expect a login of the new client, storing its refresh token in the keychain
    const person = { issuer: ISSUER, subjectId: owner.id };
    const refreshToken = present(await sessions.load(person), "the stored refresh token");
    expect({
        page: [page.status, await page.text()],
        rows: await logins.query.login.findMany(),
        clients: await nativeClients(accounts, owner),
    }).toEqual({
        page: [200, "Signed in. You can close this window."],
        rows: [loginRow(signedIn, owner)],
        clients: [...before, { id: signedIn.clientId, isRevoked: false }],
    });

    // refuse a second use of the redirect, whose listener closed with the sign-in
    const origin = new URL(callback).origin;
    await expect(network.fetch(new Request(callback))).rejects.toThrow(
        `fetch failed: nothing serves ${origin}`,
    );

    // sign out, forgetting the refresh token the issuer no longer refreshes
    await signIn.signOut(signedIn);
    const registration = schema.identifier("client").parse(signedIn.clientId);
    expect({
        rows: await logins.query.login.findMany(),
        session: await sessions.load(person),
        refreshed: await refusal(issuer.refresh(registration, refreshToken)),
    }).toEqual({
        rows: [],
        session: undefined,
        refreshed: [
            "BAD_REQUEST",
            "INVALID_GRANT",
            "the token endpoint refused the request: invalid_grant: invalid refresh token",
        ],
    });
});

test("fail a browser sign-in whose redirect names another issuer", async () => {
    await using accounts = await TestAccountStore.open();
    const { signIn, logins, network } = await native(accounts);

    // answer the redirect with another issuer
    const pages: Response[] = [];
    const failed = await refusal(
        signIn.browser(async (url) => {
            const asked = new URL(url).searchParams;
            const callback = new URL(present(asked.get("redirect_uri"), "the redirect"));
            callback.search = new URLSearchParams({
                code: "code-1",
                state: present(asked.get("state"), "the state"),
                iss: "https://mixup.test",
            }).toString();
            pages.push(await network.fetch(new Request(callback)));
        }),
    );

    // fail the sign-in without a login
    const page = single(pages);
    expect({
        page: [page.status, await page.text()],
        failed,
        rows: await logins.query.login.findMany(),
    }).toEqual({
        page: [400, "Sign-in failed. Start it again from Destack."],
        failed: ["UNAUTHORIZED", "SIGN_IN_INCOMPLETE", "the sign-in did not complete"],
        rows: [],
    });
});

test("sign a person in with a device code once they approve it, and replace the client an earlier sign-in here registered", async () => {
    await using accounts = await TestAccountStore.open();
    const { person: owner, browser } = await accounts.signInWithLink(EMAIL);
    const { signIn, logins } = await native(accounts);
    const before = await nativeClients(accounts, owner);

    // wait while the person has not decided yet
    const waiting = await signIn.startDevice();
    const pending = await waiting.poll();

    // approve each code on the verification page before its first poll, as the issuer paces polls
    const signInWithCode = () =>
        signIn.device(async (started) => {
            await browser.fetch(started.url, new URLSearchParams({ action: "approve" }));
        });
    const first = await signInWithCode();
    const second = await signInWithCode();

    // expect one login of the second client, the first client revoked
    const verification = new URL(waiting.url);
    expect({
        page: `${verification.origin}${verification.pathname}`,
        pending,
        rows: await logins.query.login.findMany(),
        clients: await nativeClients(accounts, owner),
    }).toEqual({
        page: `${ISSUER}/device`,
        pending: 5000,
        rows: [{ ...loginRow(second, owner), revision: 2, id: first.id }],
        clients: [
            ...before,
            { id: first.clientId, isRevoked: true },
            { id: second.clientId, isRevoked: false },
        ],
    });
});

test("sign a person in with a mailed code as a new client, refusing a wrong code", async () => {
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn(EMAIL);
    const { signIn, logins } = await native(accounts);
    const before = await nativeClients(accounts, owner);

    // mail a code, refuse a wrong one, and sign in with the mailed one
    const started = await signIn.startEmail(EMAIL);
    const otp = await accounts.code();
    const wrong = await refusal(started.verify(otp === "000000" ? "111111" : "000000"));
    const signedIn = await started.verify(otp);
    expect({
        wrong,
        rows: await logins.query.login.findMany(),
        clients: await nativeClients(accounts, owner),
    }).toEqual({
        wrong: [
            "BAD_REQUEST",
            "INVALID_GRANT",
            "the token endpoint refused the request: invalid_grant: the code is wrong or expired",
        ],
        rows: [loginRow(signedIn, owner)],
        clients: [...before, { id: signedIn.clientId, isRevoked: false }],
    });
});

test("sign a new person up with a mailed code and their handle, refusing a new email without one", async () => {
    await using accounts = await TestAccountStore.open();
    const { signIn, logins } = await native(accounts);
    const email = "bea@example.com";
    const signUp = SignUp.parse({
        name: "Bea",
        handle: "bea",
        residencyId: "eu",
        timeZone: "Europe/Zurich",
        locale: "en",
    });

    // refuse a new email's code without a handle, then sign the person up with one
    const refused = await refusal((await signIn.startEmail(email)).verify(await accounts.code()));
    await (await signIn.startEmail(email, signUp)).verify(await accounts.code());

    // read the login and the personal account the sign-up created together
    const signedIn = await logins.query.login.findFirst();
    const personal = await accounts.database
        .select({ scope: account.table.scope, kind: account.table.kind })
        .from(account.table)
        .where(eq(account.table.handle, "bea"));
    expect({ refused, personal }).toEqual({
        refused: [
            "BAD_REQUEST",
            "INVALID_GRANT",
            `the token endpoint refused the request: invalid_grant: nobody signed up as ${email}: sign up with a handle`,
        ],
        personal: [{ scope: present(signedIn, "the login").subjectId, kind: "personal" }],
    });
});

test("read a person's account through tokens refreshed after a mailed code signed them in, without a browser session", async () => {
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn(EMAIL);
    const { signIn, sessions, laptop } = await native(accounts);

    // sign in with the mailed code and read the person's account through a refreshed token
    const started = await signIn.startEmail(EMAIL);
    const signedIn = await started.verify(await accounts.code());
    const person = new Login(signedIn, sessions, (request) => accounts.fetch(request), laptop);
    expect(await person.account()).toBe(owner.account);
});
