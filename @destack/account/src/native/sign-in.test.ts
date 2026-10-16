import { principal } from "@destack/access";
import { and, eq } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { MemoryKeychain } from "@destack/identity";
import { ObjectClient } from "@destack/object/client";
import { Clock, present, schema } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { expect, onTestFinished, test, TestClock } from "@destack/test";
import { account } from "../object/account.ts";
import { client } from "../object/client.ts";
import { SignUp } from "../better-auth/sign-up.ts";
import { AccountFixture, type Browser, ISSUER, outcome, type Person } from "../test/index.ts";
import { Issuer } from "./issuer.ts";
import { ClientKey } from "./client-key.ts";
import { login } from "./login.ts";
import { SessionKeychain } from "./session-keychain.ts";
import { PersonClient } from "./person.ts";
import { NativeSignIn } from "./sign-in.ts";

/** The time the native client's clock reads, in UTC epoch milliseconds: 2026-01-01. */
const NOW = 1_767_225_600_000;

/** The native client the fixtures sign people in on. */
const CLIENT = schema.identifier("client").parse("client-01996ab0-0000-7000-8000-0000000000e1");

/** The native client as the creator of its logins. */
const WRITER = principal.client.reference(Scope.universe.id, CLIENT);

/** The person signed in on the issuer's pages. */
const EMAIL = "ada@example.com";

test("ask the issuer for the challenged assurance and recency in a step-up sign-in", async () => {
    await using accounts = await AccountFixture.open();
    const { signIn } = await native(accounts);

    // open the browser at the issuer and stop before it returns
    let opened: URL | undefined;
    const stopping = new AbortController();
    const signedIn = signIn.browser(
        async (url) => {
            opened = new URL(url);
            stopping.abort();
        },
        { assurance: 2, maxAge: 900_000 },
        stopping.signal,
    );
    await signedIn.catch(() => undefined);

    // expect a fresh sign-in within fifteen minutes asked for
    const asked = opened?.searchParams;
    expect([asked?.get("prompt"), asked?.get("max_age")]).toEqual(["login", "900"]);
});

test("sign a person in through a browser redirect with PKCE as a new client, and sign them out revoking its refresh token", async () => {
    await using accounts = await AccountFixture.open();
    const owner = await accounts.signInWithLink(EMAIL);
    const { signIn, issuer, logins, sessions } = await native(accounts);
    const before = await nativeClients(accounts, owner);

    // allow the sign-in on the consent page and return to the loopback redirect
    let callback = "";
    let answered: Response | undefined;
    const signedIn = await signIn.browser(async (url) => {
        callback = await consent(owner.browser, url);
        answered = await fetch(callback);
    });

    // expect a login of the new client, keeping its refresh token in the keychain
    const person = { issuer: ISSUER, subject: owner.id };
    const refreshToken = await sessions.load(person);
    expect({
        page: [answered?.status, await answered?.text()],
        rows: await logins.query.login.findMany(),
        clients: await nativeClients(accounts, owner),
        isRetained: refreshToken !== undefined,
    }).toEqual({
        page: [200, "Signed in. You can close this window."],
        rows: [loginRow(signedIn, owner)],
        clients: [...before, { id: signedIn.registrationId, isRevoked: false }],
        isRetained: true,
    });

    // refuse a second use of the redirect, whose listener closed with the sign-in
    const again = await fetch(callback).then(
        () => "answered",
        () => "refused",
    );

    // sign out, forgetting the refresh token the issuer no longer refreshes
    await signIn.signOut(signedIn);
    const registration = schema.identifier("client").parse(signedIn.registrationId);
    expect({
        again,
        rows: await logins.query.login.findMany(),
        session: await sessions.load(person),
        refreshed: await outcome(
            issuer.refresh(registration, present(refreshToken, "the refresh token")),
        ),
    }).toEqual({
        again: "refused",
        rows: [],
        session: undefined,
        refreshed: "UNAUTHORIZED: the issuer refused the refresh token: invalid refresh token",
    });
});

test("fail a browser sign-in another issuer answers", async () => {
    await using accounts = await AccountFixture.open();
    const { signIn, logins } = await native(accounts);

    // answer the redirect with another issuer
    let answered: Response | undefined;
    const failed = await outcome(
        signIn.browser(async (url) => {
            const asked = new URL(url).searchParams;
            const callback = new URL(present(asked.get("redirect_uri"), "the redirect"));
            callback.search = new URLSearchParams({
                code: "code-1",
                state: present(asked.get("state"), "the state"),
                iss: "https://mixup.test",
            }).toString();
            answered = await fetch(callback);
        }),
    );

    // fail the sign-in without a login
    expect({
        page: [answered?.status, await answered?.text()],
        failed,
        rows: await logins.query.login.findMany(),
    }).toEqual({
        page: [400, "Sign-in failed. Start it again from Destack."],
        failed: "UNAUTHORIZED: the sign-in did not complete",
        rows: [],
    });
});

test("sign a person in with a device code once they approve it, and replace the client an earlier sign-in here registered", async () => {
    await using accounts = await AccountFixture.open();
    const owner = await accounts.signInWithLink(EMAIL);
    const { signIn, logins } = await native(accounts);
    const before = await nativeClients(accounts, owner);

    // wait while the person has not decided yet
    const waiting = await signIn.startDevice();
    const pending = await waiting.poll();

    // approve each code on the verification page before its first poll, as the issuer paces polls
    const signInWithCode = async () => {
        const started = await signIn.startDevice();
        await owner.browser.fetch(started.url, new URLSearchParams({ action: "approve" }));
        const granted = await started.poll();
        if (typeof granted === "number") {
            throw new TypeError("the approved sign-in waits");
        }

        return granted;
    };
    const first = await signInWithCode();
    const second = await signInWithCode();

    // keep one login of the second client, the first client revoked
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
            { id: first.registrationId, isRevoked: true },
            { id: second.registrationId, isRevoked: false },
        ],
    });
});

test("sign a person in with a mailed code as a new client, refusing a wrong code", async () => {
    await using accounts = await AccountFixture.open();
    const owner = await accounts.person(EMAIL);
    const { signIn, logins } = await native(accounts);
    const before = await nativeClients(accounts, owner);

    // mail a code, refuse a wrong one, and sign in with the mailed one
    const started = await signIn.startEmail(EMAIL);
    const { otp } = present((await accounts.codes()).at(-1), "the mailed code");
    const wrong = await outcome(started.verify(otp === "000000" ? "111111" : "000000"));
    const signedIn = await started.verify(otp);
    expect({
        wrong,
        rows: await logins.query.login.findMany(),
        clients: await nativeClients(accounts, owner),
    }).toEqual({
        wrong: "UNAUTHORIZED: the issuer refused the code: the code is wrong or expired",
        rows: [loginRow(signedIn, owner)],
        clients: [...before, { id: signedIn.registrationId, isRevoked: false }],
    });
});

test("sign a new person up with a mailed code and their handle, refusing a new email without one", async () => {
    await using accounts = await AccountFixture.open();
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
    const refused = await outcome((await signIn.startEmail(email)).verify(await accounts.code()));
    await (await signIn.startEmail(email, signUp)).verify(await accounts.code());

    // read the login and the personal account the sign-up created together
    const signedIn = await logins.query.login.findFirst();
    const personal = await accounts.database
        .select({ scope: account.table.scope, kind: account.table.kind })
        .from(account.table)
        .where(eq(account.table.handle, "bea"));
    expect({ refused, personal }).toEqual({
        refused: `UNAUTHORIZED: the issuer refused the code: nobody signed up as ${email}: sign up with a handle`,
        personal: [{ scope: present(signedIn, "the login").subjectId, kind: "personal" }],
    });
});

test("act as a person a mailed code signed in, refreshing their client's tokens without a browser session", async () => {
    await using accounts = await AccountFixture.open();
    const owner = await accounts.person(EMAIL);
    const { signIn, sessions } = await native(accounts);

    // sign in with the mailed code and read the person's account through a refreshed token
    const started = await signIn.startEmail(EMAIL);
    const signedIn = await started.verify(await accounts.code());
    const person = await PersonClient.open(
        signedIn,
        sessions,
        (request) => accounts.fetch(request),
        Clock.system,
    );
    expect(await person.account()).toBe(owner.account);
});

/** Allow a browser sign-in on the issuer's consent page, returning the loopback redirect. */
async function consent(browser: Browser, url: string): Promise<string> {
    // follow the authorization to the consent page and allow it
    const asked = await browser.fetch(url);
    const page = present(asked.headers.get("location"), "the consent page");
    const allowed = await browser.fetch(page, new URLSearchParams({ action: "approve" }));

    return present(allowed.headers.get("location"), "the loopback redirect");
}

/** The login a sign-in writes for a person on the native client. */
function loginRow(signedIn: { id: string; registrationId: string }, person: Person) {
    return {
        id: signedIn.id,
        scope: CLIENT,
        issuer: ISSUER,
        subjectId: person.id,
        registrationId: signedIn.registrationId,
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

/** Read the native clients of a person, oldest first. */
async function nativeClients(accounts: AccountFixture, person: Person) {
    const rows = await accounts.database
        .select({ id: client.table.id, revokedAt: client.table.revokedAt })
        .from(client.table)
        .where(and(eq(client.table.scope, person.id), eq(client.table.kind, "native")))
        .orderBy(client.table.createdAt, client.table.id);

    return rows.map((row) => ({ id: row.id, isRevoked: row.revokedAt !== null }));
}

/** Sign people in on a native client against the fixture's issuer. */
async function native(accounts: AccountFixture) {
    // keep the client's logins in a local database
    const database = await connectBunSqlite(":memory:");
    onTestFinished(() => database.close());
    const subject = principal.client.reference(Scope.universe.id, CLIENT);
    const logins = await ObjectClient.open({
        database,
        objects: { login },
        package: login.package,
        clock: new TestClock(NOW),
        scope: CLIENT,
        caller: subject,
        client: subject,
        endpoint: {
            url: "http://local.invalid/",
            fetch: async () => {
                throw new TypeError("local objects reach no server");
            },
        },
        reconnect: () => {
            throw new TypeError("local objects move nowhere");
        },
    });
    onTestFinished(() => logins.close());

    // sign people in with the keychain's refresh tokens
    const issuer = new Issuer(ISSUER, (request) => accounts.fetch(request));
    const sessions = new SessionKeychain(new MemoryKeychain());
    const signIn = new NativeSignIn({
        issuer,
        sessions,
        keys: new ClientKey(new MemoryKeychain(), Clock.system),
        logins,
        clock: Clock.system,
        client: {
            name: "laptop",
            platform: "macOS",
            architecture: "arm64",
            version: "2026.9.0",
        },
    });

    return { signIn, issuer, logins, sessions };
}
