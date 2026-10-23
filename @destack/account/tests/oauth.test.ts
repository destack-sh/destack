import { eq } from "@destack/db";
import { RequestId } from "@destack/sync";
import { expect, test, refusal } from "@destack/test";
import { Credential } from "../src/credential/credential.ts";
import { client } from "../src/object/client.ts";
import { session } from "../src/object/session.ts";
import { nth, Identifier, type JsonValue, present, schema } from "@destack/schema";
import { deviceAuthorization, verification } from "../src/authentication/index.ts";
import {
    ISSUER,
    OPERATOR_EMAIL,
    TestAccountStore,
    TestBrowser,
    TestJwt,
    type TestPerson,
} from "../src/test/index.ts";
import { ANY_NUMBER, ANY_STRING, DeviceGrant, Redirect, Tokens } from "./fixture.ts";
import { JWKS_PATH } from "../src/authentication/path.ts";
import { EMAIL_CODE, JWT_BEARER, TOKEN_EXCHANGE } from "../src/oauth/index.ts";
import { Pkce, TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { PackageId } from "@destack/package";
import accountPackage from "../src/package.ts";

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
        expires_at: ANY_NUMBER,
        scope,
        ...(extra.refresh === true ? { refresh_token: ANY_STRING } : {}),
        ...(extra.id === true ? { id_token: ANY_STRING } : {}),
    };
}

/** Register a public client of a person returning to the application's page. */
function registerNotes(fixture: TestAccountStore, person: TestPerson) {
    return person.client(accountPackage).oauthClient.create({
        scope: person.id,
        requestId: RequestId.create(fixture.clock.now()),
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
        requestId: RequestId.create(fixture.clock.now()),
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
            device_authorization_endpoint: `${ISSUER}/device/code`,
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
                "sub",
                "iss",
                "aud",
                "exp",
                "iat",
                "sid",
                "scope",
                "azp",
                "name",
                "picture",
                "given_name",
                "family_name",
                "email",
                "email_verified",
                "preferred_username",
                "locale",
                "zoneinfo",
            ],
        },
    ]);

    // authorize with a PKCE challenge, which sends the signed-in user to the consent page
    const verifier = Pkce.verifier();
    const query = new URLSearchParams({
        response_type: "code",
        client_id: registered.clientId,
        redirect_uri: REDIRECT,
        scope: "openid profile email offline_access",
        state: "state-1",
        nonce: "nonce-1",
        code_challenge: await Pkce.challenge(verifier),
        code_challenge_method: "S256",
    });
    const authorized = await browser.fetch(`/oauth2/authorize?${query}`);
    const consentPage = new URL(TestBrowser.location(authorized));
    expect([authorized.status, `${consentPage.origin}${consentPage.pathname}`]).toEqual([
        302,
        `${ISSUER}/consent`,
    ]);

    // consent and return the user to the application with a code for its state from this issuer
    const consented = await browser.fetch("/oauth2/consent", {
        accept: true,
        oauth_query: consentPage.search.slice(1),
    });
    const callback = new URL(Redirect.parse(await consented.json()).url);
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

    // refuse a replayed code and end the tokens it issued
    const invalidGrant = [400, { error: "invalid_grant", error_description: "invalid code" }];
    expect([await answered(await redeem(code)), (await userinfo(tokens.access_token))[0]]).toEqual([
        invalidGrant,
        401,
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
        [401, { error: "invalid_request", error_description: "code verification failed" }],
    ]);

    // refresh the tokens of another authorization
    const refresh = (token: string) =>
        provider(fixture, "/oauth2/token", {
            grant_type: "refresh_token",
            refresh_token: token,
            client_id: registered.clientId,
        });
    const another = present((await authorize()).searchParams.get("code"), "the authorization code");
    const fresh = Tokens.parse(await (await redeem(another)).json());
    const refreshed = await refresh(present(fresh.refresh_token, "the refresh token"));
    const renewed = Tokens.parse(await refreshed.clone().json());
    expect(await answered(refreshed)).toEqual([
        200,
        issued("openid profile email offline_access", ACCESS_SECONDS, { refresh: true, id: true }),
    ]);

    // revoke the consent to end the client's tokens
    const { items: consents } = await owner
        .client(accountPackage)
        .oauthConsent.list({ userId: owner.id });
    expect(consents.map(({ clientId, scopes }) => ({ clientId, scopes }))).toEqual([
        { clientId: registered.clientId, scopes: ["openid", "profile", "email", "offline_access"] },
    ]);
    await owner.client(accountPackage).oauthConsent.revoke({
        userId: owner.id,
        id: nth(consents, 0).id,
        requestId: RequestId.create(fixture.clock.now()),
    });
    const ended = await refresh(present(renewed.refresh_token, "the renewed refresh token"));
    expect([await answered(ended), (await userinfo(renewed.access_token))[0]]).toEqual([
        [400, { error: "invalid_grant", error_description: "session not found" }],
        401,
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

    // revoke the consent, then grant it again
    const { items } = await owner.client(accountPackage).oauthConsent.list({ userId: owner.id });
    await owner.client(accountPackage).oauthConsent.revoke({
        userId: owner.id,
        id: nth(items, 0).id,
        requestId: RequestId.create(fixture.clock.now()),
    });
    const after = await grant();

    // refuse the token issued before the second grant, and refresh the one issued after it
    expect([await refresh(before), await refresh(after)]).toEqual([
        [400, { error: "invalid_grant", error_description: "session not found" }],
        [
            200,
            issued("openid profile email offline_access", ACCESS_SECONDS, {
                refresh: true,
                id: true,
            }),
        ],
    ]);
});

test("sign a device in to a confidential application through the device authorization flow", async () => {
    // register a confidential client in an account from a secret only its registrar knows
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser: ownerBrowser } =
        await fixture.signInWithLink("ada@example.com");
    const { id: accountId } = await fixture.createAccount(owner);
    const secret = await Credential.create("oauth-client");
    const registered = await owner.client(accountPackage).oauthClient.create({
        scope: accountId,
        requestId: RequestId.create(fixture.clock.now()),
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
    const requested = await provider(fixture, "/device/code", {
        ...credentials,
        scope: "openid profile offline_access",
    });
    const grant = DeviceGrant.parse(await requested.clone().json());
    const reviewed = await ownerBrowser.fetch(`/device?user_code=${grant.user_code}`);
    const approved = await ownerBrowser.fetch("/device/approve", {
        userCode: grant.user_code,
    });
    expect([
        await answered(requested),
        reviewed.status,
        await answered(approved),
        await fixture.database
            .select({
                oauthClientId: deviceAuthorization.oauthClientId,
                status: deviceAuthorization.status,
            })
            .from(deviceAuthorization)
            .where(eq(deviceAuthorization.deviceCode, grant.device_code)),
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
        [200, { success: true }],
        [{ oauthClientId: registered.clientId, status: "approved" }],
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
        [400, { error: "invalid_client", error_description: "invalid client_secret" }],
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
        requestId: RequestId.create(fixture.clock.now()),
    });
    const suspended = await provider(fixture, "/oauth2/token", {
        ...credentials,
        grant_type: "refresh_token",
        refresh_token: present(tokens.refresh_token, "the refresh token"),
    });
    expect(await answered(suspended)).toEqual([401, { message: "user is no longer authorized" }]);
});

test("sign a device in as a new client through the browser once the person allows it, with a loopback redirect and PKCE", async () => {
    // authorize a fresh client with a challenge on the loopback, signed out and signed in
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const { person: owner, browser: ownerBrowser } =
        await fixture.signInWithLink("ada@example.com");
    const signedOut = fixture.browser();
    const clientId = Identifier.create("client");
    const verifier = Pkce.verifier();
    const query = new URLSearchParams({
        response_type: "code",
        client_id: clientId,
        redirect_uri: LOOPBACK,
        scope: "openid profile email offline_access",
        resource: ACCOUNT_RESOURCE,
        state: "state-1",
        code_challenge: await Pkce.challenge(verifier),
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

    // return a denial to the client, then a code once the person allows it
    const decide = async (action: "approve" | "deny") => {
        const decided = await ownerBrowser.fetch(
            `/consent${consentPage.search}`,
            new URLSearchParams({ action }),
        );

        return new URL(TestBrowser.location(decided));
    };
    const denied = await decide("deny");
    const approved = await decide("approve");
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
                error_description: "User denied access",
                state: "state-1",
                iss: ISSUER,
            },
        ],
        approved: [LOOPBACK, { code: ANY_STRING, state: "state-1", iss: ISSUER }],
    });

    // refuse and spend a code without its verifier, redeem a fresh one, and refuse it replayed
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
    const fresh = present((await decide("approve")).searchParams.get("code"), "the fresh code");
    const redeemed = await redeem(fresh);
    const replayed = await redeem(fresh);
    const tokens = Tokens.parse(await redeemed.clone().json());
    expect([await answered(forged), await answered(redeemed), await answered(replayed)]).toEqual([
        [401, { error: "invalid_request", error_description: "code verification failed" }],
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

test("include the second factor of the session approving a device's client in its tokens", async () => {
    // authorize a fresh client in a browser the person stepped up in
    await using fixture = await TestAccountStore.open();
    const { person, browser } = await fixture.signInWithLink("ada@example.com");
    await fixture.elevate(person, browser);
    const redeemed = await fixture.authorize(browser, {
        clientId: Identifier.create("client"),
        redirectUri: LOOPBACK,
        scope: "openid offline_access",
        state: "state-2",
        resource: ACCOUNT_RESOURCE,
    });

    // carry the approving session's second factor into the tokens
    const { caller } = TestJwt.claims(Tokens.parse(await redeemed.json()).access_token);
    const approving = await fixture.database
        .select({ authenticatedAt: session.table.authenticatedAt })
        .from(session.table)
        .where(eq(session.table.id, schema.identifier("session").parse(caller.credential.id)));
    expect(caller.credential.assurance).toEqual({
        level: 2,
        authenticatedAt: nth(approving, 0).authenticatedAt,
    });
});

test("send a signed-in person asked to sign in again within an age to the sign-in page", async () => {
    // authorize a fresh client asking the person to sign in again
    await using fixture = await TestAccountStore.open();
    const { browser } = await fixture.signInWithLink("ada@example.com");
    const query = new URLSearchParams({
        response_type: "code",
        client_id: Identifier.create("client"),
        redirect_uri: LOOPBACK,
        scope: "openid offline_access",
        resource: ACCOUNT_RESOURCE,
        state: "state-2",
        code_challenge: await Pkce.challenge(Pkce.verifier()),
        code_challenge_method: "S256",
        prompt: "login",
        max_age: "60",
    });
    const asked = await browser.fetch(`/oauth2/authorize?${query}`);

    // send the browser to the sign-in page
    const page = new URL(TestBrowser.location(asked));
    expect([asked.status, `${page.origin}${page.pathname}`]).toEqual([302, `${ISSUER}/sign-in`]);
});

test("sign a person in with a mailed code at the token endpoint as a new client, and refuse a wrong, a spent or an expired code", async () => {
    // mail a code, and exchange it for the person's tokens on a new client
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const owner = await fixture.signIn("ada@example.com");
    const browser = fixture.browser();
    const clientId = Identifier.create("client");
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

    // refuse a code past its expiry, which Better Auth checks on the wall clock the fixture's clock trails
    await send();
    await fixture.database
        .update(verification)
        .set({ expiresAt: fixture.clock.now() - 1 })
        .where(eq(verification.identifier, "sign-in-otp-ada@example.com"));
    expect(await answered(await redeem(await fixture.code()))).toEqual(wrongCode);
});

test("update, disable and delete an oauth client", async () => {
    // register, rename and disable a public client
    await using fixture = await TestAccountStore.open();
    const { person: owner, browser } = await fixture.signInWithLink("ada@example.com");
    const registered = await owner.client(accountPackage).oauthClient.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.clock.now()),
        name: "Notes",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code"],
    });
    const updated = await owner.client(accountPackage).oauthClient.update({
        scope: owner.id,
        id: registered.id,
        requestId: RequestId.create(fixture.clock.now()),
        revision: registered.revision,
        name: "Notebook",
        disabled: true,
    });
    expect(updated).toEqual({
        ...registered,
        name: "Notebook",
        disabled: true,
        revision: 2,
        updatedAt: updated.updatedAt,
    });

    // refuse authorizing the disabled client
    const query = new URLSearchParams({
        response_type: "code",
        client_id: registered.clientId,
        redirect_uri: REDIRECT,
        scope: "openid",
        state: "state-1",
        code_challenge: await Pkce.challenge(Pkce.verifier()),
        code_challenge_method: "S256",
    });
    const refused = await browser.fetch(`/oauth2/authorize?${query}`);
    expect([refused.status, TestBrowser.location(refused), await refused.text()]).toEqual([
        302,
        `${ISSUER}/error?error=client_disabled&error_description=client+is+disabled`,
        "",
    ]);

    // delete the client
    const deleted = await owner.client(accountPackage).oauthClient.delete({
        scope: owner.id,
        id: registered.id,
        requestId: RequestId.create(fixture.clock.now()),
        revision: updated.revision,
    });
    expect(deleted).toEqual({});
    expect(
        await refusal(
            owner.client(accountPackage).oauthClient.get({ scope: owner.id, id: registered.id }),
        ),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", `oauth-client ${registered.id} not found`]);
});
