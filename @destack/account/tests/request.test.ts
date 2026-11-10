import { accessGrant, principal } from "@destack/access";
import { Identifier, present, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { RequestId } from "@destack/sync";
import { and, eq, isNull } from "@destack/db";
import { expect, single, test } from "@destack/test";
import {
    CONSENT_PATH,
    ConsentList,
    HOME_LOCATION_PATH,
    TokenResponse,
} from "../src/authentication/index.ts";
import accountPackage from "../src/package.ts";
import {
    Account,
    DEVICE_CLIENT_NAME,
    HOME_SPACE,
    placement,
    session,
} from "../src/object/index.ts";
import { DOMAINS, InstallationOrigin } from "../src/address/index.ts";
import { ISSUER, ORIGIN, TestAccountStore, TestBrowser, TestDirectory } from "../src/test/index.ts";
import { DeviceGrant } from "./fixture.ts";

/** The path of the stored authorization requests below the issuer. */
const AUTHORIZATION_REQUESTS = "/oauth2/requests/authorization";

/** The path of the pending device sign-ins below the issuer. */
const DEVICE_REQUESTS = "/oauth2/requests/device";

/** A refusal's reason and message, as the issuer answers a call it refuses. */
const Refusal = schema.object({ reason: schema.string(), message: schema.string() }).loose();

/** Read a refusal by its status, error and description. */
async function refused(response: Response): Promise<readonly [number, string, string]> {
    const { reason, message } = Refusal.parse(await response.json());

    return [response.status, reason, message];
}

/** Post a JSON body from another site, as a forged decision would. */
function forged(browser: TestBrowser, path: string, body: object): Promise<Response> {
    return browser.send(
        new Request(`${ISSUER}${path}`, {
            method: "POST",
            headers: { origin: "https://elsewhere.test", "content-type": "application/json" },
            body: JSON.stringify(body),
        }),
    );
}

test("ask a sign-in at the level a request's acr values ask, refusing one the person's methods never reach", async () => {
    // register a client for ada, signed in by a mailed code alone
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser: owned } = await fixture.signInBrowser("ada@example.com");
    const redirect = "https://notes.test/callback";
    const client = await owner.client(accountPackage).application.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.store.environment),
        name: "Notes",
        redirectUris: [redirect],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code", "refresh_token"],
    });
    const authorize = async (acr: string) => {
        const query = new URLSearchParams({
            response_type: "code",
            client_id: client.clientId,
            redirect_uri: redirect,
            scope: "openid",
            state: "state-1",
            code_challenge: "c".repeat(43),
            code_challenge_method: "S256",
            acr_values: acr,
        });
        const location = new URL(
            TestBrowser.location(await owned.fetch(`/oauth2/authorize?${query}`)),
        );

        return [`${location.origin}${location.pathname}`, location.searchParams.get("error")];
    };

    // meet the code's level, refuse a factor's she never enrolled, and refuse an unknown value
    expect([await authorize("aal1"), await authorize("aal2"), await authorize("gold")]).toEqual([
        [`${ORIGIN}/consent`, null],
        [redirect, "unmet_authentication_requirements"],
        [redirect, "invalid_request"],
    ]);
});

test("send a browser to the sign-in view, then to the consent view, and approve a client from the issuer's own origin alone", async () => {
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser: owned } = await fixture.signInBrowser("ada@example.com");
    const visitor = fixture.browser();

    // register a client and authorize it, signed out and signed in
    const redirect = "https://notes.test/callback";
    const client = await owner.client(accountPackage).application.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.store.environment),
        name: "Notes",
        redirectUris: [redirect],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code", "refresh_token"],
    });
    const authorize = `/oauth2/authorize?${new URLSearchParams({
        response_type: "code",
        client_id: client.clientId,
        redirect_uri: redirect,
        scope: "openid email",
        state: "state-1",
        code_challenge: "c".repeat(43),
        code_challenge_method: "S256",
    })}`;
    const signIn = new URL(TestBrowser.location(await visitor.fetch(authorize)));
    const consent = new URL(TestBrowser.location(await owned.fetch(authorize)));
    const requestUri = present(consent.searchParams.get("request_uri"), "the request's reference");

    // read the stored request, refuse its decision from another site, then approve it
    const read: unknown = await (
        await owned.fetch(
            `${AUTHORIZATION_REQUESTS}?${new URLSearchParams({ request_uri: requestUri })}`,
        )
    ).json();
    const forgery = await forged(owned, AUTHORIZATION_REQUESTS, {
        requestUri,
        decision: "approve",
    });
    const decided = await owned.fetch(AUTHORIZATION_REQUESTS, { requestUri, decision: "approve" });
    const { location } = schema.object({ location: schema.string() }).parse(await decided.json());
    const callback = new URL(location);

    // send the signed-out browser to sign in and the signed-in one to consent, both at the universe's origin
    expect({
        signIn: [`${signIn.origin}${signIn.pathname}`, signIn.searchParams.get("client_id")],
        consent: `${consent.origin}${consent.pathname}`,
        read,
        forgery: await refused(forgery),
        callback: [
            `${callback.origin}${callback.pathname}`,
            callback.searchParams.get("state"),
            callback.searchParams.has("code"),
        ],
    }).toEqual({
        signIn: [`${ORIGIN}/sign-in`, client.clientId],
        consent: `${ORIGIN}/consent`,
        read: {
            client: { id: client.clientId, name: "Notes" },
            scopes: ["openid", "email"],
            isLoginDue: false,
            grant: null,
        },
        forgery: [403, "ORIGIN_UNTRUSTED", "the decision came from https://elsewhere.test"],
        callback: [redirect, "state-1", true],
    });
});

test("send a device's person to the verification view, refuse a signed-out read, and approve the device", async () => {
    await using fixture = await TestAccountStore.open();
    const native = fixture.browser();
    const { browser } = await fixture.signInBrowser("ada@example.com");

    // request a device code for the account package as a device's new client
    const deviceClient = Identifier.create("client", fixture.environment);
    const offered = await native.fetch(
        "/oauth2/device_authorization",
        new URLSearchParams({
            client_id: deviceClient,
            resource: PackageId.resource(accountPackage.id),
        }),
    );
    const grant = DeviceGrant.parse(await offered.json());
    const verification = new URL(grant.verification_uri_complete);

    // refuse a signed-out read, read it signed in, and approve it
    const query = new URLSearchParams({ user_code: grant.user_code });
    const signedOut = await fixture.browser().fetch(`${DEVICE_REQUESTS}?${query}`);
    const read: unknown = await (await browser.fetch(`${DEVICE_REQUESTS}?${query}`)).json();
    const decided: unknown = await (
        await browser.fetch(DEVICE_REQUESTS, { userCode: grant.user_code, decision: "approve" })
    ).json();

    // redeem the approved code as the device's client
    const redeemed = await native.fetch(
        "/oauth2/token",
        new URLSearchParams({
            grant_type: "urn:ietf:params:oauth:grant-type:device_code",
            device_code: grant.device_code,
            client_id: deviceClient,
        }),
    );
    expect({
        verification: `${verification.origin}${verification.pathname}`,
        signedOut: await refused(signedOut),
        read,
        decided,
        redeemed: [redeemed.status, TokenResponse.parse(await redeemed.json()).token_type],
    }).toEqual({
        verification: `${ORIGIN}/verification`,
        signedOut: [401, "SIGN_IN_REQUIRED", "the person is not signed in"],
        read: {
            client: { id: deviceClient, name: DEVICE_CLIENT_NAME },
            scopes: [],
            status: "pending",
        },
        decided: { status: "approved" },
        redeemed: [200, "Bearer"],
    });
});

test("send an authorization refused before its redirect exists to the sign-in view with its error", async () => {
    await using fixture = await TestAccountStore.open();
    const browser = fixture.browser();

    // authorize a client the issuer does not know
    const unknown = Identifier.create("client", fixture.environment);
    const answered = await browser.fetch(
        `/oauth2/authorize?${new URLSearchParams({ response_type: "code", client_id: unknown, redirect_uri: "https://notes.test/callback" })}`,
    );
    const location = new URL(TestBrowser.location(answered));

    // show the refusal on the sign-in view instead of the client's redirect
    expect([
        answered.status,
        `${location.origin}${location.pathname}`,
        location.searchParams.get("error"),
    ]).toEqual([302, `${ORIGIN}/sign-in`, "invalid_request"]);
});

test("mail a known person's code with a link entering it on the sign-in view, which signs them in", async () => {
    await using fixture = await TestAccountStore.open();
    await fixture.signInBrowser("ada@example.com");

    // ask for a code in a new browser, then sign in with the code the link carries
    const browser = fixture.browser();
    await browser.fetch("/email-otp/send-verification-otp", {
        email: "ada@example.com",
        type: "sign-in",
    });
    const link = new URL(present((await fixture.links()).at(-1), "the mailed link").url);
    const signedIn = await browser.fetch("/sign-in/email-otp", {
        email: link.searchParams.get("email"),
        otp: link.searchParams.get("code"),
    });

    expect({
        page: `${link.origin}${link.pathname}`,
        query: Object.fromEntries(link.searchParams),
        signedIn: signedIn.status,
        isSignedIn: browser.cookies.has("better-auth.session_token"),
    }).toEqual({
        page: `${ORIGIN}/sign-in`,
        query: { email: "ada@example.com", code: await fixture.code() },
        signedIn: 200,
        isSignedIn: true,
    });
});

test("answer a sign-up's handle check and the residencies with the one the visitor's country is offered", async () => {
    await using fixture = await TestAccountStore.open();
    await fixture.signInBrowser("ada@example.com", "ada");
    const browser = fixture.browser();

    // check a free, a taken and an invalid handle, then list the residencies from Germany and from Japan
    const check = async (handle: string): Promise<unknown> =>
        await (await browser.fetch(`/sign-up/handle?${new URLSearchParams({ handle })}`)).json();
    const residencies = async (country: string): Promise<unknown> =>
        await (
            await browser.send(
                new Request(`${ISSUER}/sign-up/residency`, {
                    headers: { "cf-ipcountry": country },
                }),
            )
        ).json();

    expect({
        free: await check("bea"),
        taken: await check("ada"),
        invalid: await check("-"),
        germany: await residencies("DE"),
        elsewhere: await residencies("JP"),
    }).toEqual({
        free: { handle: "bea", availability: "available" },
        taken: { handle: "ada", availability: "taken" },
        invalid: { handle: "-", availability: "invalid" },
        germany: {
            residencies: [
                { code: "eu", name: "European Union" },
                { code: "us", name: "United States" },
            ],
            suggested: "eu",
        },
        elsewhere: {
            residencies: [
                { code: "eu", name: "European Union" },
                { code: "us", name: "United States" },
            ],
            suggested: null,
        },
    });
});

test("refuse reviewing a request without its reference, and deciding one by a malformed body", async () => {
    await using fixture = await TestAccountStore.open();
    const { browser } = await fixture.signInBrowser("ada@example.com");

    // read both requests with no reference, and decide one with no decision
    const authorization = await browser.fetch(AUTHORIZATION_REQUESTS);
    const device = await browser.fetch(DEVICE_REQUESTS);
    const decided = await browser.fetch(AUTHORIZATION_REQUESTS, { requestUri: "urn:request" });

    expect([await refused(authorization), await refused(device), await refused(decided)]).toEqual([
        [400, "QUERY_PARAMETER_MISSING", "the request has no request_uri"],
        [400, "QUERY_PARAMETER_MISSING", "the request has no user_code"],
        [400, "BODY_INVALID", "the request's body does not match"],
    ]);
});

test("locate a signed-in person's Home, starting while their home's machine answers nowhere, and refuse a signed-out browser", async () => {
    // sign a person up, which places their home at the residency's machine
    await using fixture = await TestAccountStore.open();
    const { person, browser } = await fixture.signInBrowser("ada@example.com", "ada");
    const serving: unknown = await (await browser.fetch(HOME_LOCATION_PATH)).json();

    // withdraw the endpoint of the machine serving their home, and locate it again
    const home = Account.home(Account.personal(person.id));
    const placed = single(
        await fixture.database
            .select()
            .from(placement.table)
            .where(eq(placement.table.spaceId, home)),
    );
    await TestDirectory.publish(
        fixture.database,
        placed.machineId,
        placed.scope,
        null,
        fixture.clock,
    );
    const starting: unknown = await (await browser.fetch(HOME_LOCATION_PATH)).json();
    const signedOut = await fixture.browser().fetch(HOME_LOCATION_PATH);

    expect({ serving, starting, signedOut: [signedOut.status, await signedOut.json()] }).toEqual({
        serving: { status: "serving", url: "https://home--home--ada.destack.space/" },
        starting: { status: "starting" },
        signedOut: [
            401,
            {
                code: "UNAUTHORIZED",
                reason: "SIGN_IN_REQUIRED",
                domain: "@destack/account",
                message: "the person is not signed in",
            },
        ],
    });
});

test("refuse an installation origin's unsigned request, and grant what its space's signed request asks for this sign-in alone once its person approves, asking again at the next sign-in", async () => {
    // ask the owner for notes' grant
    await using fixture = await TestAccountStore.open();
    const { owner, browser, notes, origin, asked, description, unsigned, consent, ask } =
        await askGrant(fixture);
    const requestUri = present(consent.searchParams.get("request_uri"), "the request's reference");

    // review it, then approve it for this sign-in alone
    const read: unknown = await (
        await browser.fetch(
            `${AUTHORIZATION_REQUESTS}?${new URLSearchParams({ request_uri: requestUri })}`,
        )
    ).json();
    const decided = await browser.fetch(AUTHORIZATION_REQUESTS, {
        requestUri,
        decision: "approve-once",
    });
    const { location } = schema.object({ location: schema.string() }).parse(await decided.json());
    const callback = new URL(location);
    const granted = single(
        await fixture.database
            .select()
            .from(accessGrant)
            .where(eq(accessGrant.delegateId, notes.id)),
    );
    const signedIn = single(
        await fixture.database
            .select({ id: session.table.id })
            .from(session.table)
            .where(and(eq(session.table.userId, owner.id), isNull(session.table.clientId))),
    );

    // sign the owner in again in another browser, which is asked again and may allow it once again
    const later = fixture.browser();
    await later.signIn(owner.email, () => fixture.code());
    const again = await ask(later);
    const decidedAgain = await later.fetch(AUTHORIZATION_REQUESTS, {
        requestUri: present(again.searchParams.get("request_uri"), "the second request"),
        decision: "approve-once",
    });

    // refuse the unsigned one on the sign-in view, show the grant to approve, and grant it within the session
    expect({
        unsigned: [
            unsigned.pathname,
            unsigned.searchParams.get("error"),
            unsigned.searchParams.get("error_description"),
        ],
        consent: `${consent.origin}${consent.pathname}`,
        read,
        callback: [
            `${callback.origin}${callback.pathname}`,
            callback.searchParams.get("state"),
            callback.searchParams.has("code"),
        ],
        granted: [granted.permissions, granted.sessionId === signedIn.id],
        again: [`${again.origin}${again.pathname}`, decidedAgain.status],
    }).toEqual({
        unsigned: ["/sign-in", "invalid_request", "an installation's origin signs its requests"],
        consent: `${ORIGIN}/consent`,
        read: {
            client: { id: origin, name: "notes" },
            scopes: ["openid", "profile"],
            isLoginDue: false,
            grant: {
                delegate: notes,
                installation: "notes",
                package: "@acme/notes",
                space: `${HOME_SPACE}--${owner.handle}`,
                permissions: [{ ...asked, within: "home", description, isGranted: false }],
            },
        },
        callback: [`${origin}/.destack/open`, "state-1", true],
        granted: [[asked], true],
        again: [`${ORIGIN}/consent`, 200],
    });
});

test("list a person's consent with the grant it gave an installation, and revoke both", async () => {
    // ask the owner for notes' grant and approve it lastingly
    await using fixture = await TestAccountStore.open();
    const { owner, browser, notes, origin, asked, description, consent } = await askGrant(fixture);
    const requestUri = present(consent.searchParams.get("request_uri"), "the request's reference");
    await browser.fetch(AUTHORIZATION_REQUESTS, { requestUri, decision: "approve" });
    const now = fixture.environment.clock.now();

    // list the owner's consents, revoke notes', and list them again
    const listed: unknown = await (await browser.fetch(CONSENT_PATH)).json();
    const revoked = await browser.fetch(
        `${CONSENT_PATH}?${new URLSearchParams({ client_id: origin })}`,
        undefined,
        "DELETE",
    );
    const after: unknown = await (await browser.fetch(CONSENT_PATH)).json();
    const grants = await fixture.database
        .select()
        .from(accessGrant)
        .where(eq(accessGrant.delegateId, notes.id));

    // keep the browser's own sign-in consent, and drop notes' with its grant
    const device = {
        client: { id: owner.clientId, name: "Destack app" },
        scopes: ["openid", "profile", "offline_access"],
        grantedAt: now,
        grant: null,
    };
    expect({ listed, revoked: revoked.status, after, grants }).toEqual({
        listed: {
            consents: [
                device,
                {
                    client: { id: origin, name: "notes" },
                    scopes: ["openid", "profile"],
                    grantedAt: now,
                    grant: {
                        delegate: notes,
                        installation: "notes",
                        package: "@acme/notes",
                        space: `${HOME_SPACE}--${owner.handle}`,
                        permissions: [{ ...asked, within: "home", description }],
                        lifetime: "lasting",
                    },
                },
            ],
        },
        revoked: 204,
        after: { consents: [device] },
        grants: [],
    });
});

test("list an app allowed for this sign-in alone, and remove its grant with its consent", async () => {
    // ask the owner for notes' grant and approve it for this sign-in alone
    await using fixture = await TestAccountStore.open();
    const { owner, browser, notes, origin, asked, description, consent } = await askGrant(fixture);
    const requestUri = present(consent.searchParams.get("request_uri"), "the request's reference");
    await browser.fetch(AUTHORIZATION_REQUESTS, { requestUri, decision: "approve-once" });

    // list it, then remove it
    const listed = ConsentList.parse(await (await browser.fetch(CONSENT_PATH)).json());
    await browser.fetch(
        `${CONSENT_PATH}?${new URLSearchParams({ client_id: origin })}`,
        undefined,
        "DELETE",
    );
    const grants = await fixture.database
        .select()
        .from(accessGrant)
        .where(eq(accessGrant.delegateId, notes.id));

    // show its grant until the sign-in ends, and revoke the grant with the consent
    expect({
        listed: listed.consents.find((each) => each.client.id === origin)?.grant,
        grants,
    }).toEqual({
        listed: {
            delegate: notes,
            installation: "notes",
            package: "@acme/notes",
            space: `${HOME_SPACE}--${owner.handle}`,
            permissions: [{ ...asked, within: "home", description }],
            lifetime: "sign-in",
        },
        grants: [],
    });
});

test("revoke an app's grant for this sign-in alone once the person signs out", async () => {
    // approve notes' grant for this sign-in alone
    await using fixture = await TestAccountStore.open();
    const notes = await askGrant(fixture);
    const once = present(notes.consent.searchParams.get("request_uri"), "notes' request");
    await notes.browser.fetch(AUTHORIZATION_REQUESTS, {
        requestUri: once,
        decision: "approve-once",
    });
    const granted = async () =>
        (await fixture.database.select().from(accessGrant)).map((row) => [
            row.delegateId,
            row.sessionId === null,
            row.expiresAt === null,
        ]);
    const before = await granted();

    // sign the browser out
    await notes.browser.fetch("/sign-out", {});

    // bind the grant to the sign-in and its expiry, then revoke it with the sign-out
    expect({ before, after: await granted() }).toEqual({
        before: [[notes.notes.id, false, false]],
        after: [],
    });
});

test("refuse a signed grant naming another installation than the origin asking", async () => {
    // sign notes' origin's request for a grant naming tasks
    await using fixture = await TestAccountStore.open();
    const { consent } = await askGrant(fixture, "tasks");

    // send the browser to the sign-in page with the refusal, no consent shown
    expect([
        consent.pathname,
        consent.searchParams.get("error"),
        consent.searchParams.get("error_description"),
    ]).toEqual([
        "/sign-in",
        "invalid_request_object",
        "the grant names another installation than the origin asking",
    ]);
});

/** Ask the owner for notes' grant as its origin does: unsigned, then signed by its space. */
async function askGrant(fixture: TestAccountStore, named = "notes") {
    // sign the owner in, and start an identity for their home, where notes is installed
    const { person: owner, browser } = await fixture.signInBrowser("ada@example.com");
    const { spaceId, machine } = await fixture.personalHome(owner);
    await fixture.startIdentity(spaceId, machine);
    const notes = principal.installation.reference(
        spaceId,
        Identifier.create("installation", fixture.environment),
    );
    const origin = `https://${InstallationOrigin.format({ name: "notes", space: HOME_SPACE, handle: owner.handle }, DOMAINS.space)}`;
    const asked = { ...session.permission("read"), scope: owner.id };
    const description = {
        package: accountPackage.id,
        id: "permission.session.read",
        source: "See sessions",
        values: {},
    };

    // ask unsigned, then sign the same request with the grant as the home's space
    const parameters = {
        response_type: "code",
        client_id: origin,
        redirect_uri: `${origin}/.destack/open`,
        scope: "openid profile",
        state: "state-1",
        code_challenge: "c".repeat(43),
        code_challenge_method: "S256",
    };
    const unsigned = new URL(
        TestBrowser.location(
            await browser.fetch(`/oauth2/authorize?${new URLSearchParams(parameters)}`),
        ),
    );
    const ask = async (asker: TestBrowser) => {
        const signed = await fixture.identityOf(notes).request(ISSUER, {
            ...parameters,
            authorization_details: [
                {
                    type: "destack_grant",
                    delegate: notes,
                    installation: named,
                    package: "@acme/notes",
                    space: `${HOME_SPACE}--${owner.handle}`,
                    permissions: [{ ...asked, within: "home", description }],
                },
            ],
        });
        const query = new URLSearchParams({ client_id: origin, request: signed });

        return new URL(TestBrowser.location(await asker.fetch(`/oauth2/authorize?${query}`)));
    };
    const consent = await ask(browser);

    return { owner, browser, notes, origin, asked, description, unsigned, consent, ask };
}
