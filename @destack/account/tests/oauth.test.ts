import { eq } from "@destack/db";
import { RequestId } from "@destack/sync";
import { expect, test, refusal } from "@destack/test";
import { Credential } from "../src/credential/credential.ts";
import { client } from "../src/object/client.ts";
import { session } from "../src/object/session.ts";
import { nth, Identifier, type JsonValue, present, schema } from "@destack/schema";
import { verification } from "../src/authentication/index.ts";
import { deviceAuthorization } from "../src/object/device-authorization.ts";
import {
    ISSUER,
    OPERATOR_EMAIL,
    TestAccountStore,
    TestBrowser,
    TestJwt,
    type TestPerson,
} from "../src/test/index.ts";
import { ANY_STRING, DeviceGrant, Tokens } from "./fixture.ts";
import { JWKS_PATH } from "../src/authentication/path.ts";
import { EMAIL_CODE, JWT_BEARER, TOKEN_EXCHANGE } from "../src/oauth/index.ts";
import { Pkce, TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { PackageId } from "@destack/package";
import accountPackage from "../src/package.ts";

/** A second in milliseconds, the resolution Better Auth stores token times at. */
const SECOND = 1000;

/** The page the application receives codes at. */
const REDIRECT = "https://notes.test/callback";

/** The loopback redirect a device's new client receives its code at. */
const LOOPBACK = "http://127.0.0.1:53682/callback";

/** The account package as an OAuth resource, the audience of a device's tokens. */
const ACCOUNT_RESOURCE = PackageId.resource(accountPackage.id);

/** The device authorization grant (RFC 8628). */
const DEVICE_CODE = "urn:ietf:params:oauth:grant-type:device_code";

/** How long the provider's access tokens for registered clients last, in seconds. */
const ACCESS_SECONDS = 600;

/** How long a device's new client's access tokens for a package last, in seconds. */
const PACKAGE_SECONDS = TOKEN_LIFETIME_MILLISECONDS / 1000;

/** The claims of an ID token the account package sets, without Better Auth's times and hashes. */
const IdClaims = schema
    .object({
        /** The issuer. */
        iss: schema.string(),
        /** The user. */
        sub: schema.string(),
        /** The client. */
        aud: schema.string(),
        /** The nonce the client sent. */
        nonce: schema.string().exactOptional(),
    })
    .strip();

/** The session a browser's cookie holds. */
const BrowserSession = schema.looseObject({
    /** The session. */
    session: schema.looseObject({
        /** The session's identifier. */
        id: schema.string(),
    }),
});

/** The provider metadata the account package declares (OpenID Connect Discovery). */
const Discovery = schema
    .object({
        /** The issuer. */
        issuer: schema.string(),
        /** The authorization endpoint. */
        authorization_endpoint: schema.string(),
        /** The token endpoint. */
        token_endpoint: schema.string(),
        /** The user info endpoint. */
        userinfo_endpoint: schema.string(),
        /** The published signing keys. */
        jwks_uri: schema.string(),
        /** The device authorization endpoint. */
        device_authorization_endpoint: schema.string(),
        /** The grant types the server accepts. */
        grant_types_supported: schema.array(schema.string()),
        /** The PKCE challenge methods the server accepts. */
        code_challenge_methods_supported: schema.array(schema.string()),
        /** The claims the server can issue. */
        claims_supported: schema.array(schema.string()),
    })
    .strip();

/** Send a request to the provider as another application does: a form post or a read. */
function provider(
    fixture: TestAccountStore,
    path: string,
    form?: Record<string, string>,
    headers: Record<string, string> = {},
): Promise<Response> {
    return fixture.fetch(
        new Request(`${ISSUER}${path}`, {
            method: form === undefined ? "GET" : "POST",
            headers:
                form === undefined
                    ? headers
                    : { ...headers, "content-type": "application/x-www-form-urlencoded" },
            ...(form === undefined ? {} : { body: new URLSearchParams(form) }),
        }),
    );
}

/** Read a response as its status and JSON body. */
async function answered(response: Response): Promise<readonly [number, JsonValue]> {
    return [response.status, await response.json()];
}

/** Write the tokens the provider issues for scopes and seconds, secrets as any strings. */
function issued(
    scope: string,
    seconds: number,
    extra: { readonly refresh?: boolean; readonly id?: boolean } = {},
) {
    return {
        token_type: "Bearer",
        access_token: ANY_STRING,
        expires_in: seconds,
        scope,
        ...(extra.refresh === true ? { refresh_token: ANY_STRING } : {}),
        ...(extra.id === true ? { id_token: ANY_STRING } : {}),
    };
}

/** Register a public client of a person returning to the application's page. */
function registerNotes(fixture: TestAccountStore, person: TestPerson) {
    return person.client(accountPackage).application.create({
        scope: person.id,
        requestId: RequestId.create(fixture.environment),
        name: "Notes",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code", "refresh_token"],
    });
}

test("sign a user in to another application through the authorization code flow with PKCE, until they revoke the consent", async () => {
    // let the person choose their time zone and locale, and register a public client
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser } = await fixture.signInWithLink("ada@example.com", "ada");
    const current = await owner.client(accountPackage).user.get({ id: owner.id });
    await owner.client(accountPackage).user.update({
        id: owner.id,
        revision: current.revision,
        requestId: RequestId.create(fixture.environment),
        timeZone: "Europe/London",
        locale: "en-GB",
    });
    const registered = await registerNotes(fixture, owner);

    // discover the provider at its issuer, with the endpoints, grants and claims it adds
    const discovered = await provider(fixture, "/.well-known/openid-configuration");
    expect([discovered.status, Discovery.parse(await discovered.json())]).toEqual([
        200,
        {
            issuer: ISSUER,
            authorization_endpoint: `${ISSUER}/oauth2/authorize`,
            token_endpoint: `${ISSUER}/oauth2/token`,
            userinfo_endpoint: `${ISSUER}/oauth2/userinfo`,
            jwks_uri: `${ISSUER}${JWKS_PATH}`,
            device_authorization_endpoint: `${ISSUER}/oauth2/device_authorization`,
            grant_types_supported: [
                "authorization_code",
                "refresh_token",
                JWT_BEARER,
                TOKEN_EXCHANGE,
                EMAIL_CODE,
                DEVICE_CODE,
            ],
            code_challenge_methods_supported: ["S256"],
            claims_supported: [
                "iss",
                "sub",
                "aud",
                "exp",
                "iat",
                "auth_time",
                "nonce",
                "sid",
                "azp",
                "name",
                "picture",
                "email",
                "email_verified",
                "preferred_username",
                "locale",
                "zoneinfo",
            ],
        },
    ]);

    // authorize with a PKCE challenge, which sends the signed-in user to the consent page
    const verifier = Pkce.verifier(fixture.environment.random);
    const query = new URLSearchParams({
        response_type: "code",
        client_id: registered.clientId,
        redirect_uri: REDIRECT,
        scope: "openid profile email offline_access",
        state: "state-1",
        nonce: "nonce-1",
        code_challenge: await Pkce.challenge(verifier, fixture.environment.crypto),
        code_challenge_method: "S256",
    });
    const authorized = await browser.fetch(`/oauth2/authorize?${query}`);
    const consentPage = new URL(TestBrowser.location(authorized));
    expect([authorized.status, `${consentPage.origin}${consentPage.pathname}`]).toEqual([
        302,
        `${ISSUER}/consent`,
    ]);

    // consent and return the user to the application with a code for its state from this issuer
    const consented = await browser.fetch(
        consentPage.href,
        new URLSearchParams({ action: "approve" }),
    );
    const callback = new URL(TestBrowser.location(consented));
    expect([
        `${callback.origin}${callback.pathname}`,
        Object.fromEntries(callback.searchParams),
    ]).toEqual([REDIRECT, { code: ANY_STRING, state: "state-1", iss: ISSUER }]);

    // redeem the code with its verifier
    const redeem = (code: string, codeVerifier = verifier) =>
        provider(fixture, "/oauth2/token", {
            grant_type: "authorization_code",
            code,
            redirect_uri: REDIRECT,
            client_id: registered.clientId,
            code_verifier: codeVerifier,
        });
    const code = present(callback.searchParams.get("code"), "the authorization code");
    const redeemed = await redeem(code);
    const tokens = Tokens.parse(await redeemed.clone().json());
    expect(await answered(redeemed)).toEqual([
        200,
        issued("openid profile email offline_access", ACCESS_SECONDS, { refresh: true, id: true }),
    ]);

    // verify the ID token for this client and nonce with the published keys
    const idToken = await fixture.verifyIdToken(present(tokens.id_token, "the ID token"));
    expect(IdClaims.parse(idToken)).toEqual({
        iss: ISSUER,
        sub: owner.id,
        aud: registered.clientId,
        nonce: "nonce-1",
    });

    // read the user's claims
    const userinfo = async (accessToken: string) =>
        answered(
            await provider(fixture, "/oauth2/userinfo", undefined, {
                authorization: `Bearer ${accessToken}`,
            }),
        );
    expect(await userinfo(tokens.access_token)).toEqual([
        200,
        {
            sub: owner.id,
            name: "ada@example.com",
            email: "ada@example.com",
            email_verified: true,
            preferred_username: "ada",
            locale: "en-GB",
            zoneinfo: "Europe/London",
        },
    ]);

    // refuse a replayed code and end the refresh tokens it issued, the access token lapsing at its expiry
    const refresh = (token: string) =>
        provider(fixture, "/oauth2/token", {
            grant_type: "refresh_token",
            refresh_token: token,
            client_id: registered.clientId,
        });
    const invalidGrant = [400, { error: "invalid_grant", error_description: "invalid code" }];
    expect([
        await answered(await redeem(code)),
        await answered(await refresh(present(tokens.refresh_token, "the refresh token"))),
        (await userinfo(tokens.access_token))[0],
    ]).toEqual([
        invalidGrant,
        [400, { error: "invalid_grant", error_description: "invalid refresh token" }],
        200,
    ]);

    // authorize again without consent and refuse a code without its verifier
    const authorize = async () =>
        new URL(TestBrowser.location(await browser.fetch(`/oauth2/authorize?${query}`)));
    const remembered = await authorize();
    const forged = await redeem(
        present(remembered.searchParams.get("code"), "the authorization code"),
        "forged".repeat(8),
    );
    expect([`${remembered.origin}${remembered.pathname}`, await answered(forged)]).toEqual([
        REDIRECT,
        [
            400,
            {
                error: "invalid_grant",
                error_description: "the code verifier does not match its challenge",
            },
        ],
    ]);

    // refresh the tokens of another authorization
    const another = present((await authorize()).searchParams.get("code"), "the authorization code");
    const fresh = Tokens.parse(await (await redeem(another)).json());
    const refreshed = await refresh(present(fresh.refresh_token, "the refresh token"));
    const renewed = Tokens.parse(await refreshed.clone().json());
    expect(await answered(refreshed)).toEqual([
        200,
        issued("openid profile email offline_access", ACCESS_SECONDS, { refresh: true, id: true }),
    ]);

    // list the consents to the application and to the person's own client, and revoke the application's
    const { items: consents } = await owner
        .client(accountPackage)
        .oauthConsent.list({ userId: owner.id });
    expect(
        consents
            .map(({ clientId, scopes }) => ({ clientId, scopes }))
            .toSorted((left, right) => left.clientId.localeCompare(right.clientId)),
    ).toEqual(
        [
            {
                clientId: registered.clientId,
                scopes: ["openid", "profile", "email", "offline_access"],
            },
            { clientId: owner.clientId, scopes: ["openid", "profile", "offline_access"] },
        ].toSorted((left, right) => left.clientId.localeCompare(right.clientId)),
    );
    const revoked = present(
        consents.find(({ clientId }) => clientId === registered.clientId),
        "the application's consent",
    );
    await owner.client(accountPackage).oauthConsent.revoke({
        userId: owner.id,
        id: revoked.id,
        revision: revoked.revision,
        requestId: RequestId.create(fixture.environment),
    });
    const ended = await refresh(present(renewed.refresh_token, "the renewed refresh token"));
    expect([await answered(ended), (await userinfo(renewed.access_token))[0]]).toEqual([
        [400, { error: "invalid_grant", error_description: "invalid refresh token" }],
        200,
    ]);
});

test("refuse a refresh token issued before a revoked consent was granted again", async () => {
    // register a public client and authorize it with PKCE, consenting once
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser } = await fixture.signInWithLink("ada@example.com", "ada");
    const registered = await registerNotes(fixture, owner);
    const authorization = {
        clientId: registered.clientId,
        redirectUri: REDIRECT,
        scope: "openid profile email offline_access",
        state: "state-1",
        resource: null,
    };
    const grant = async () => {
        const redeemed = await fixture.authorize(browser, authorization);

        return present(Tokens.parse(await redeemed.json()).refresh_token, "the refresh token");
    };
    const refresh = async (token: string) =>
        answered(
            await provider(fixture, "/oauth2/token", {
                grant_type: "refresh_token",
                refresh_token: token,
                client_id: registered.clientId,
            }),
        );
    const before = await grant();

    // revoke the consent a second later, then grant it again a second after
    fixture.clock.advance(SECOND);
    const { items } = await owner.client(accountPackage).oauthConsent.list({ userId: owner.id });
    const revoked = present(
        items.find(({ clientId }) => clientId === registered.clientId),
        "the application's consent",
    );
    await owner.client(accountPackage).oauthConsent.revoke({
        userId: owner.id,
        id: revoked.id,
        revision: revoked.revision,
        requestId: RequestId.create(fixture.environment),
    });
    fixture.clock.advance(SECOND);
    const after = await grant();

    // refuse the token issued before the second grant, and refresh the one issued after it
    expect([await refresh(before), await refresh(after)]).toEqual([
        [400, { error: "invalid_grant", error_description: "invalid refresh token" }],
        [
            200,
            issued("openid profile email offline_access", ACCESS_SECONDS, {
                refresh: true,
                id: true,
            }),
        ],
    ]);
});

test("refuse a device authorization asking scopes beyond the client's", async () => {
    await using fixture = await TestAccountStore.open();
    const asked = await provider(fixture, "/oauth2/device_authorization", {
        client_id: Identifier.create("client", fixture.environment),
        scope: "openid admin",
    });
    expect(await answered(asked)).toEqual([
        400,
        { error: "invalid_scope", error_description: "the client may not ask for these scopes" },
    ]);
});

test("sign a device in to a confidential application through the device authorization flow", async () => {
    // register a confidential client in an account from a secret only its registrar knows
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser: ownerBrowser } =
        await fixture.signInWithLink("ada@example.com");
    const { id: accountId } = await fixture.createAccount(owner);
    const secret = await Credential.create("client-secret", fixture.environment);
    const registered = await owner.client(accountPackage).application.create({
        scope: accountId,
        requestId: RequestId.create(fixture.environment),
        name: "Television",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "client_secret_post",
        clientSecret: secret.digest,
        grantTypes: [DEVICE_CODE, "refresh_token"],
    });
    expect([Object.hasOwn(registered, "clientSecret"), registered.userId]).toEqual([
        false,
        owner.id,
    ]);

    // request a device code as the client for the user to approve in the browser
    const credentials = { client_id: registered.clientId, client_secret: secret.secret };
    const requested = await provider(fixture, "/oauth2/device_authorization", {
        ...credentials,
        scope: "openid profile offline_access",
    });
    const grant = DeviceGrant.parse(await requested.clone().json());
    const page = `/device?user_code=${grant.user_code}`;
    const reviewed = await ownerBrowser.fetch(page);
    const approved = await ownerBrowser.fetch(page, new URLSearchParams({ action: "approve" }));
    expect([
        await answered(requested),
        reviewed.status,
        approved.status,
        await fixture.database
            .select({
                clientId: deviceAuthorization.table.clientId,
                status: deviceAuthorization.table.status,
            })
            .from(deviceAuthorization.table),
    ]).toEqual([
        [
            200,
            {
                device_code: grant.device_code,
                user_code: grant.user_code,
                verification_uri: `${ISSUER}/device`,
                verification_uri_complete: `${ISSUER}/device?user_code=${grant.user_code}`,
                expires_in: 1800,
                interval: 5,
            },
        ],
        200,
        200,
        [{ clientId: registered.clientId, status: "approved" }],
    ]);

    // refuse the code without the client's secret, and issue the tokens with it
    const exchange = (fields: Record<string, string>) =>
        provider(fixture, "/oauth2/token", {
            grant_type: DEVICE_CODE,
            device_code: grant.device_code,
            ...fields,
        });
    const unauthenticated = await exchange({ ...credentials, client_secret: "dst_ocs_wrong" });
    const issuedTokens = await exchange(credentials);
    const tokens = Tokens.parse(await issuedTokens.clone().json());
    expect([await answered(unauthenticated), await answered(issuedTokens)]).toEqual([
        [401, { error: "invalid_client", error_description: "invalid client_secret" }],
        [200, issued("openid profile offline_access", ACCESS_SECONDS, { refresh: true, id: true })],
    ]);
    const idToken = await fixture.verifyIdToken(present(tokens.id_token, "the ID token"));
    expect(IdClaims.parse(idToken)).toEqual({
        iss: ISSUER,
        sub: owner.id,
        aud: registered.clientId,
    });

    // refuse further tokens once the platform's operator suspends the user
    const operator = await fixture.elevate(await fixture.signIn(OPERATOR_EMAIL));
    await operator.client(accountPackage).user.suspend({
        id: owner.id,
        requestId: RequestId.create(fixture.environment),
    });
    const suspended = await provider(fixture, "/oauth2/token", {
        ...credentials,
        grant_type: "refresh_token",
        refresh_token: present(tokens.refresh_token, "the refresh token"),
    });
    expect(await answered(suspended)).toEqual([
        400,
        { error: "invalid_grant", error_description: "the user may no longer sign in" },
    ]);
});

test("sign a device in as a new client through the browser once the person allows it, with a loopback redirect and PKCE", async () => {
    // authorize a fresh client with a challenge on the loopback, signed out and signed in
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const { person: owner, browser: ownerBrowser } =
        await fixture.signInWithLink("ada@example.com");
    const signedOut = fixture.browser();
    const clientId = Identifier.create("client", fixture.environment);
    const verifier = Pkce.verifier(fixture.environment.random);
    const query = new URLSearchParams({
        response_type: "code",
        client_id: clientId,
        redirect_uri: LOOPBACK,
        scope: "openid profile email offline_access",
        resource: ACCOUNT_RESOURCE,
        state: "state-1",
        code_challenge: await Pkce.challenge(verifier, fixture.environment.crypto),
        code_challenge_method: "S256",
    });
    const away = await signedOut.fetch(`/oauth2/authorize?${query}`);
    const asked = await ownerBrowser.fetch(`/oauth2/authorize?${query}`);
    const signIn = new URL(TestBrowser.location(away));
    const consentPage = new URL(TestBrowser.location(asked));
    expect({
        away: [away.status, `${signIn.origin}${signIn.pathname}`],
        asked: [asked.status, `${consentPage.origin}${consentPage.pathname}`],
    }).toEqual({
        away: [302, `${ISSUER}/sign-in`],
        asked: [302, `${ISSUER}/consent`],
    });

    // return a denial to the client, then a code once the person allows a new request
    const decide = async (page: URL, action: "approve" | "deny") => {
        const decided = await ownerBrowser.fetch(page.href, new URLSearchParams({ action }));

        return new URL(TestBrowser.location(decided));
    };
    const authorize = async () =>
        new URL(TestBrowser.location(await ownerBrowser.fetch(`/oauth2/authorize?${query}`)));
    const denied = await decide(consentPage, "deny");
    const approved = await decide(await authorize(), "approve");
    expect({
        denied: [`${denied.origin}${denied.pathname}`, Object.fromEntries(denied.searchParams)],
        approved: [
            `${approved.origin}${approved.pathname}`,
            Object.fromEntries(approved.searchParams),
        ],
    }).toEqual({
        denied: [
            LOOPBACK,
            {
                error: "access_denied",
                error_description: "the person denied access",
                state: "state-1",
                iss: ISSUER,
            },
        ],
        approved: [LOOPBACK, { code: ANY_STRING, state: "state-1", iss: ISSUER }],
    });

    // refuse and spend a code without its verifier, redeem a fresh one of the standing consent, and refuse it replayed
    const redeem = (code: string, codeVerifier = verifier) =>
        provider(fixture, "/oauth2/token", {
            grant_type: "authorization_code",
            code,
            redirect_uri: LOOPBACK,
            client_id: clientId,
            code_verifier: codeVerifier,
            resource: ACCOUNT_RESOURCE,
        });
    const code = present(approved.searchParams.get("code"), "the authorization code");
    const forged = await redeem(code, "f".repeat(43));
    const fresh = present((await authorize()).searchParams.get("code"), "the fresh code");
    const redeemed = await redeem(fresh);
    const replayed = await redeem(fresh);
    const tokens = Tokens.parse(await redeemed.clone().json());
    expect([await answered(forged), await answered(redeemed), await answered(replayed)]).toEqual([
        [
            400,
            {
                error: "invalid_grant",
                error_description: "the code verifier does not match its challenge",
            },
        ],
        [
            200,
            issued("openid profile email offline_access", PACKAGE_SECONDS, {
                refresh: true,
                id: true,
            }),
        ],
        [400, { error: "invalid_grant", error_description: "invalid code" }],
    ]);

    // act as the person through the new client in the browser's session, registered as theirs at consent
    const signedIn = BrowserSession.parse(await (await ownerBrowser.fetch("/get-session")).json());
    const approving = await fixture.database
        .select({ id: session.table.id, authenticatedAt: session.table.authenticatedAt })
        .from(session.table)
        .where(eq(session.table.id, schema.identifier("session").parse(signedIn.session.id)));
    const { id: sessionId, authenticatedAt } = nth(approving, 0);
    const issuedAt = Math.floor(fixture.clock.now() / 1000);
    expect([
        TestJwt.claims(tokens.access_token),
        await fixture.database
            .select({ id: client.table.id, scope: client.table.scope })
            .from(client.table)
            .where(eq(client.table.id, clientId)),
    ]).toEqual([
        {
            iss: ISSUER,
            sub: owner.id,
            aud: [ACCOUNT_RESOURCE, `${ISSUER}/oauth2/userinfo`],
            client_id: clientId,
            azp: clientId,
            sid: sessionId,
            scope: "openid profile email offline_access",
            jti: ANY_STRING,
            iat: issuedAt,
            exp: issuedAt + PACKAGE_SECONDS,
            caller: {
                subject: owner.subject,
                groups: [],
                contacts: [{ medium: "email", address: "ada@example.com" }],
                credential: {
                    kind: "session",
                    id: sessionId,
                    assurance: { level: 1, authenticatedAt },
                },
            },
        },
        [{ id: clientId, scope: owner.id }],
    ]);
});

test("include the second factor of the session approving a device's client in its tokens, and keep it in refreshed ones", async () => {
    // authorize a fresh client in a browser the person stepped up in
    await using fixture = await TestAccountStore.open();
    const { person, browser } = await fixture.signInWithLink("ada@example.com");
    await fixture.simulation.jump(fixture.clock.now() + 1000);
    await fixture.elevate(person, browser);
    const clientId = Identifier.create("client", fixture.environment);
    const redeemed = await fixture.authorize(browser, {
        clientId,
        redirectUri: LOOPBACK,
        scope: "openid offline_access",
        state: "state-2",
        resource: ACCOUNT_RESOURCE,
    });

    // refresh the tokens a minute later
    const tokens = Tokens.parse(await redeemed.json());
    await fixture.simulation.jump(fixture.clock.now() + 60 * SECOND);
    const refreshed = await provider(fixture, "/oauth2/token", {
        grant_type: "refresh_token",
        refresh_token: present(tokens.refresh_token, "the refresh token"),
        client_id: clientId,
    });
    const renewed = Tokens.parse(await refreshed.json());

    // carry the approving session's second factor into the tokens and their renewal
    const { caller } = TestJwt.claims(tokens.access_token);
    const approving = await fixture.database
        .select({ authenticatedAt: session.table.authenticatedAt })
        .from(session.table)
        .where(eq(session.table.id, schema.identifier("session").parse(caller.credential.id)));
    const assurance = { level: 2, authenticatedAt: nth(approving, 0).authenticatedAt };
    expect([
        caller.credential.assurance,
        TestJwt.claims(renewed.access_token).caller.credential.assurance,
    ]).toEqual([assurance, assurance]);
});

test("send a signed-in person asked to sign in again within an age to the sign-in page", async () => {
    // authorize a fresh client asking the person to sign in again, a second after they signed in
    await using fixture = await TestAccountStore.open();
    const { browser } = await fixture.signInWithLink("ada@example.com");
    await fixture.simulation.jump(fixture.clock.now() + 1000);
    const query = new URLSearchParams({
        response_type: "code",
        client_id: Identifier.create("client", fixture.environment),
        redirect_uri: LOOPBACK,
        scope: "openid offline_access",
        resource: ACCOUNT_RESOURCE,
        state: "state-2",
        code_challenge: await Pkce.challenge(
            Pkce.verifier(fixture.environment.random),
            fixture.environment.crypto,
        ),
        code_challenge_method: "S256",
        prompt: "login",
        max_age: "60",
    });
    const asked = await browser.fetch(`/oauth2/authorize?${query}`);

    // send the browser to the sign-in page
    const page = new URL(TestBrowser.location(asked));
    expect([asked.status, `${page.origin}${page.pathname}`]).toEqual([302, `${ISSUER}/sign-in`]);
});

test("continue an authorization asking a sign-in of no age once the person signed in again", async () => {
    // authorize a fresh client asking for a sign-in no older than zero seconds
    await using fixture = await TestAccountStore.open();
    const { browser } = await fixture.signInWithLink("ada@example.com");
    await fixture.simulation.jump(fixture.clock.now() + 1000);
    const clientId = Identifier.create("client", fixture.environment);
    const query = new URLSearchParams({
        response_type: "code",
        client_id: clientId,
        redirect_uri: LOOPBACK,
        scope: "openid",
        resource: ACCOUNT_RESOURCE,
        state: "state-3",
        code_challenge: await Pkce.challenge(
            Pkce.verifier(fixture.environment.random),
            fixture.environment.crypto,
        ),
        code_challenge_method: "S256",
        max_age: "0",
    });
    const asked = new URL(TestBrowser.location(await browser.fetch(`/oauth2/authorize?${query}`)));

    // sign in again by link, then continue the stored request
    await browser.fetch("/sign-in/magic-link", { email: "ada@example.com", callbackURL: "/" });
    await browser.fetch(await fixture.link());
    await fixture.simulation.jump(fixture.clock.now() + 1000);
    const continued = await browser.fetch(`/oauth2/authorize?${asked.searchParams}`);
    const next = new URL(TestBrowser.location(continued));

    // send the browser to sign in first, then on to consent
    expect([`${asked.origin}${asked.pathname}`, `${next.origin}${next.pathname}`]).toEqual([
        `${ISSUER}/sign-in`,
        `${ISSUER}/consent`,
    ]);
});

test("sign a person in with a mailed code at the token endpoint as a new client, and refuse a wrong, a spent or an expired code", async () => {
    // mail a code, and exchange it for the person's tokens on a new client
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const owner = await fixture.signIn("ada@example.com");
    const browser = fixture.browser();
    const clientId = Identifier.create("client", fixture.environment);
    const send = () =>
        browser.fetch("/email-otp/send-verification-otp", {
            email: "ada@example.com",
            type: "sign-in",
        });
    const redeem = (code: string) =>
        provider(fixture, "/oauth2/token", {
            grant_type: EMAIL_CODE,
            email: "ada@example.com",
            code,
            client_id: clientId,
            resource: ACCOUNT_RESOURCE,
        });
    await send();
    const otp = await fixture.code();
    const wrong = await redeem(otp === "000000" ? "111111" : "000000");
    const redeemed = await redeem(otp);
    const spent = await redeem(otp);
    const tokens = Tokens.parse(await redeemed.clone().json());

    // issue tokens of the new client in a new session, registered as the person's
    const registered = await fixture.database
        .select({ id: client.table.id, scope: client.table.scope })
        .from(client.table)
        .where(eq(client.table.id, clientId));
    const signedIn = await fixture.database
        .select({ id: session.table.id })
        .from(session.table)
        .where(eq(session.table.clientId, clientId));
    const issuedAt = Math.floor(fixture.clock.now() / 1000);
    const wrongCode = [
        400,
        { error: "invalid_grant", error_description: "the code is wrong or expired" },
    ];
    expect({
        wrong: await answered(wrong),
        redeemed: await answered(redeemed),
        claims: TestJwt.claims(tokens.access_token),
        registered,
        spent: await answered(spent),
    }).toEqual({
        wrong: wrongCode,
        redeemed: [
            200,
            issued("openid profile offline_access", PACKAGE_SECONDS, { refresh: true, id: true }),
        ],
        claims: {
            iss: ISSUER,
            sub: owner.id,
            aud: [ACCOUNT_RESOURCE, `${ISSUER}/oauth2/userinfo`],
            client_id: clientId,
            azp: clientId,
            sid: nth(signedIn, 0).id,
            scope: "openid profile offline_access",
            jti: ANY_STRING,
            iat: issuedAt,
            exp: issuedAt + PACKAGE_SECONDS,
            caller: {
                subject: owner.subject,
                groups: [],
                contacts: [{ medium: "email", address: "ada@example.com" }],
                credential: {
                    kind: "session",
                    id: nth(signedIn, 0).id,
                    assurance: { level: 1, authenticatedAt: fixture.clock.now() },
                },
            },
        },
        registered: [{ id: clientId, scope: owner.id }],
        spent: wrongCode,
    });

    // refuse a code past its expiry
    await send();
    await fixture.database
        .update(verification)
        .set({ expiresAt: fixture.clock.now() - 1 })
        .where(eq(verification.identifier, "sign-in-otp-ada@example.com"));
    expect(await answered(await redeem(await fixture.code()))).toEqual(wrongCode);
});

test("update, disable and delete an application", async () => {
    // register, rename and disable a public client
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser } = await fixture.signInWithLink("ada@example.com");
    const registered = await owner.client(accountPackage).application.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.environment),
        name: "Notes",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code"],
    });
    const updated = await owner.client(accountPackage).application.update({
        scope: owner.id,
        id: registered.id,
        requestId: RequestId.create(fixture.environment),
        revision: registered.revision,
        name: "Notebook",
        isDisabled: true,
    });
    expect(updated).toEqual({
        ...registered,
        name: "Notebook",
        isDisabled: true,
        revision: 2,
        updatedAt: updated.updatedAt,
    });

    // refuse authorizing the disabled client on a page, as its redirect is not to be trusted
    const query = new URLSearchParams({
        response_type: "code",
        client_id: registered.clientId,
        redirect_uri: REDIRECT,
        scope: "openid",
        state: "state-1",
        code_challenge: await Pkce.challenge(
            Pkce.verifier(fixture.environment.random),
            fixture.environment.crypto,
        ),
        code_challenge_method: "S256",
    });
    const refused = await browser.fetch(`/oauth2/authorize?${query}`);
    expect([refused.status, refused.headers.get("location")]).toEqual([400, null]);

    // delete the client
    const deleted = await owner.client(accountPackage).application.delete({
        scope: owner.id,
        id: registered.id,
        requestId: RequestId.create(fixture.environment),
        revision: updated.revision,
    });
    expect(deleted).toEqual({});
    expect(
        await refusal(
            owner.client(accountPackage).application.get({ scope: owner.id, id: registered.id }),
        ),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", `application ${registered.id} not found`]);
});

test("redeem a code by RFC 7636's verifier, keep a refresh token whose renewal is refused, rotate it at each use, revoke the family a spent token returns to, and refuse a lapsed code", async () => {
    // authorize a registered client with the challenge of RFC 7636 Appendix B
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const { person: owner, browser } = await fixture.signInWithLink("ada@example.com");
    const registered = await registerNotes(fixture, owner);
    const query = new URLSearchParams({
        response_type: "code",
        client_id: registered.clientId,
        redirect_uri: REDIRECT,
        scope: "openid offline_access",
        code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        code_challenge_method: "S256",
    });
    const consentPage = TestBrowser.location(await browser.fetch(`/oauth2/authorize?${query}`));
    const first = codeOf(
        TestBrowser.location(
            await browser.fetch(consentPage, new URLSearchParams({ action: "approve" })),
        ),
    );
    const redeem = (redeemed: string) =>
        provider(fixture, "/oauth2/token", {
            grant_type: "authorization_code",
            code: redeemed,
            redirect_uri: REDIRECT,
            client_id: registered.clientId,
            code_verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        });
    const tokens = Tokens.parse(await (await redeem(first)).json());

    // rotate the refresh token twice, then refuse the first one returning and every token of its family
    const refresh = async (token: string | undefined) =>
        provider(fixture, "/oauth2/token", {
            grant_type: "refresh_token",
            refresh_token: present(token, "the refresh token"),
            client_id: registered.clientId,
        });
    const narrowed = await answered(
        await provider(fixture, "/oauth2/token", {
            grant_type: "refresh_token",
            refresh_token: present(tokens.refresh_token, "the refresh token"),
            client_id: registered.clientId,
            scope: "offline_access",
        }),
    );
    const second = Tokens.parse(await (await refresh(tokens.refresh_token)).json());
    const third = Tokens.parse(await (await refresh(second.refresh_token)).json());
    const reused = await answered(await refresh(tokens.refresh_token));
    const revoked = await answered(await refresh(third.refresh_token));

    // refuse a fresh code once its minute passes
    const lapsing = codeOf(TestBrowser.location(await browser.fetch(`/oauth2/authorize?${query}`)));
    fixture.clock.advance(61 * SECOND);
    const lapsed = await answered(await redeem(lapsing));
    expect({ narrowed, reused, revoked, lapsed }).toEqual({
        narrowed: [
            400,
            { error: "invalid_target", error_description: "the request names no resource" },
        ],
        reused: [
            400,
            { error: "invalid_grant", error_description: "the refresh token was used before" },
        ],
        revoked: [400, { error: "invalid_grant", error_description: "invalid refresh token" }],
        lapsed: [400, { error: "invalid_grant", error_description: "the code expired" }],
    });
});

test("answer tokens uncached, introspect only the asking client's own standing tokens, renew offline after sign-out, and refuse repeated parameters and malformed basic credentials", async () => {
    // redeem a code of a registered client for its tokens
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser } = await fixture.signInWithLink("ada@example.com");
    const registered = await registerNotes(fixture, owner);
    const query = new URLSearchParams({
        response_type: "code",
        client_id: registered.clientId,
        redirect_uri: REDIRECT,
        scope: "openid offline_access",
        code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        code_challenge_method: "S256",
    });
    const consentPage = TestBrowser.location(await browser.fetch(`/oauth2/authorize?${query}`));
    const code = codeOf(
        TestBrowser.location(
            await browser.fetch(consentPage, new URLSearchParams({ action: "approve" })),
        ),
    );
    const redeemed = await provider(fixture, "/oauth2/token", {
        grant_type: "authorization_code",
        code,
        redirect_uri: REDIRECT,
        client_id: registered.clientId,
        code_verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
    });
    const tokens = Tokens.parse(await redeemed.clone().json());

    // introspect both tokens as their client and as another
    const stranger = Identifier.create("client", fixture.environment);
    const introspect = async (token: string | undefined, clientId: string) => {
        const answer = await provider(fixture, "/oauth2/introspect", {
            token: present(token, "the token"),
            client_id: clientId,
        });

        return schema.looseObject({ active: schema.boolean() }).parse(await answer.json()).active;
    };
    const activity = [
        await introspect(tokens.access_token, registered.clientId),
        await introspect(tokens.refresh_token, registered.clientId),
        await introspect(tokens.access_token, stranger),
        await introspect(tokens.refresh_token, stranger),
    ];

    // sign the person out, ending the session the access token rests on, and renew offline
    await browser.fetch("/sign-out", {});
    const ended = await introspect(tokens.access_token, registered.clientId);
    const [renewed] = await answered(
        await provider(fixture, "/oauth2/token", {
            grant_type: "refresh_token",
            refresh_token: present(tokens.refresh_token, "the refresh token"),
            client_id: registered.clientId,
        }),
    );

    // refuse a form giving a parameter twice
    const repeated = await fixture.fetch(
        new Request(`${ISSUER}/oauth2/token`, {
            method: "POST",
            headers: { "content-type": "application/x-www-form-urlencoded" },
            body: "grant_type=refresh_token&grant_type=authorization_code",
        }),
    );

    // refuse basic credentials that are no base64
    const malformed = await provider(
        fixture,
        "/oauth2/token",
        { grant_type: "refresh_token", refresh_token: present(tokens.refresh_token, "the token") },
        { authorization: "Basic %%%" },
    );
    expect({
        caching: redeemed.headers.get("cache-control"),
        activity,
        ended,
        renewed,
        repeated: await answered(repeated),
        malformed: await answered(malformed),
    }).toEqual({
        caching: "no-store",
        activity: [true, true, false, false],
        ended: false,
        renewed: 200,
        repeated: [
            400,
            { error: "invalid_request", error_description: "a parameter is given more than once" },
        ],
        malformed: [
            401,
            { error: "invalid_client", error_description: "malformed basic credentials" },
        ],
    });
});

/** Read the authorization code a redirect location carries. */
function codeOf(location: string): string {
    return present(new URL(location).searchParams.get("code"), "the authorization code");
}
