import { eq } from "@destack/db";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { Credential } from "../src/credential/credential.ts";
import { user } from "../src/object/user.ts";
import { client } from "../src/object/client.ts";
import { nth, Identifier, present, schema } from "@destack/schema";
import { deviceAuthorization, verification } from "../src/authentication/index.ts";
import { outcome, AccountFixture, ISSUER, type Person } from "../src/test/index.ts";
import {
    at,
    claims,
    DeviceGrant,
    KeySet,
    locationOf,
    Redirect,
    Refusal,
    Tokens,
} from "./fixture.ts";
import { JWKS_PATH } from "../src/authentication/path.ts";
import { EMAIL_CODE, JWT_BEARER, TOKEN_EXCHANGE } from "../src/oauth/index.ts";
import { Pkce } from "@destack/access/token";
import { PackageId } from "@destack/package";
import accountPackage from "../src/package.ts";

/** The claims of an ID token the tests read. */
const IdClaims = schema.looseObject({
    /** The issuer. */
    iss: schema.string(),
    /** The user. */
    sub: schema.string(),
    /** The client. */
    aud: schema.string(),
    /** The nonce the client sent. */
    nonce: schema.string().exactOptional(),
});
/** The claims of an ID token the tests read. */
type IdClaims = schema.Infer<typeof IdClaims>;

/** The header of a JSON Web Token, with its signing key's identifier. */
const TokenHeader = schema.looseObject({
    /** The signing key's identifier. */
    kid: schema.string(),
});

/** The provider metadata the tests read (OpenID Connect Discovery). */
const Discovery = schema.looseObject({
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
});

/** The three dot-separated parts of a JSON Web Token: header, claims and signature. */
const TokenParts = schema.tuple([schema.string(), schema.string(), schema.string()]);

/** The page the application receives codes at. */
const REDIRECT = "https://notes.test/callback";

/** The account package as an OAuth resource, the audience of a device's tokens. */
const ACCOUNT_RESOURCE = PackageId.resource(accountPackage.id);

/** The device authorization grant (RFC 8628). */
const DEVICE_CODE = "urn:ietf:params:oauth:grant-type:device_code";

/** Sign a user in to another application, then revoke the consent. */
test("sign in to another application through the authorization code flow with PKCE", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("ada@example.com", "ada");
    await chooseProfile(owner);

    // register a public client
    const registered = await owner.client.oauthClient.create({
        scope: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "Notes",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code", "refresh_token"],
    });

    // discover the provider at its issuer
    const discovered = Discovery.parse(
        await (await provider(fixture, "/.well-known/openid-configuration")).json(),
    );
    expect({
        issuer: discovered.issuer,
        authorization: discovered.authorization_endpoint,
        token: discovered.token_endpoint,
        userinfo: discovered.userinfo_endpoint,
        keys: discovered.jwks_uri,
        device: discovered.device_authorization_endpoint,
        grants: discovered.grant_types_supported,
        challenges: discovered.code_challenge_methods_supported,
        claims: discovered.claims_supported,
    }).toEqual({
        issuer: ISSUER,
        authorization: `${ISSUER}/oauth2/authorize`,
        token: `${ISSUER}/oauth2/token`,
        userinfo: `${ISSUER}/oauth2/userinfo`,
        keys: `${ISSUER}${JWKS_PATH}`,
        device: `${ISSUER}/device/code`,
        grants: [
            "authorization_code",
            "refresh_token",
            JWT_BEARER,
            TOKEN_EXCHANGE,
            EMAIL_CODE,
            DEVICE_CODE,
        ],
        challenges: ["S256"],
        claims: [
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
    });

    // authorize with a PKCE challenge and send the signed-in user to the consent page
    const verifier = crypto
        .getRandomValues(new Uint8Array(32))
        .toBase64({ alphabet: "base64url", omitPadding: true });
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(verifier));
    const query = new URLSearchParams({
        response_type: "code",
        client_id: registered.clientId,
        redirect_uri: REDIRECT,
        scope: "openid profile email offline_access",
        state: "state-1",
        nonce: "nonce-1",
        code_challenge: new Uint8Array(digest).toBase64({
            alphabet: "base64url",
            omitPadding: true,
        }),
        code_challenge_method: "S256",
    });
    const authorized = await owner.browser.fetch(`/oauth2/authorize?${query}`);
    const consentPage = new URL(locationOf(authorized));
    expect([authorized.status, `${consentPage.origin}${consentPage.pathname}`]).toEqual([
        302,
        `${ISSUER}/consent`,
    ]);

    // consent and return the user to the application with a code for its state from this issuer
    const consented = await owner.browser.fetch("/oauth2/consent", {
        accept: true,
        oauth_query: consentPage.search.slice(1),
    });
    const callback = new URL(Redirect.parse(await consented.json()).url);
    expect({
        page: `${callback.origin}${callback.pathname}`,
        state: callback.searchParams.get("state"),
        issuer: callback.searchParams.get("iss"),
    }).toEqual({ page: REDIRECT, state: "state-1", issuer: ISSUER });

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
    const tokens = Tokens.parse(await redeemed.json());
    expect([redeemed.status, tokens.token_type, tokens.scope]).toEqual([
        200,
        "Bearer",
        "openid profile email offline_access",
    ]);

    // verify the ID token for this client and nonce with the published keys
    const identity = await verifyToken(fixture, present(tokens.id_token, "the ID token"));
    expect({
        iss: identity.iss,
        sub: identity.sub,
        aud: identity.aud,
        nonce: identity.nonce,
    }).toEqual({ iss: ISSUER, sub: owner.id, aud: registered.clientId, nonce: "nonce-1" });

    // read the user's claims
    const userinfo = async (accessToken: string) => {
        const response = await provider(fixture, "/oauth2/userinfo", undefined, {
            authorization: `Bearer ${accessToken}`,
        });

        const body: unknown = await response.json();

        return [response.status, body];
    };
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
    const replayed = await redeem(code);
    expect([
        replayed.status,
        Refusal.parse(await replayed.json()).error,
        (await userinfo(tokens.access_token))[0],
    ]).toEqual([400, "invalid_grant", 401]);

    // authorize again without consent and refuse a code without its verifier
    const authorize = async () => {
        const response = await owner.browser.fetch(`/oauth2/authorize?${query}`);

        return new URL(locationOf(response));
    };
    const remembered = await authorize();
    const forged = await redeem(
        present(remembered.searchParams.get("code"), "the authorization code"),
        "forged".repeat(8),
    );
    expect([
        `${remembered.origin}${remembered.pathname}`,
        forged.status,
        Refusal.parse(await forged.json()).error,
    ]).toEqual([REDIRECT, 401, "invalid_request"]);

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
    const renewed = Tokens.parse(await refreshed.json());
    expect(refreshed.status).toBe(200);

    // revoke the consent to end the client's tokens
    const { items: consents } = await owner.client.oauthConsent.list({ userId: owner.id });
    expect(consents.map(({ clientId, scopes }) => ({ clientId, scopes }))).toEqual([
        { clientId: registered.clientId, scopes: ["openid", "profile", "email", "offline_access"] },
    ]);
    await owner.client.oauthConsent.revoke({
        userId: owner.id,
        id: nth(consents, 0).id,
        requestId: RequestId.create(Date.now()),
    });
    const ended = await refresh(present(renewed.refresh_token, "the renewed refresh token"));
    expect([
        ended.status,
        Refusal.parse(await ended.json()).error,
        (await userinfo(renewed.access_token))[0],
    ]).toEqual([400, "invalid_grant", 401]);
});

test("refuse a refresh token issued before a revoked consent was granted again", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("ada@example.com", "ada");

    // register a public client and authorize it with PKCE, consenting once
    const registered = await owner.client.oauthClient.create({
        scope: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "Notes",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code", "refresh_token"],
    });
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
    const grant = async (): Promise<string> => {
        // consent on the page the authorization sends the person to, and redeem the code
        const authorized = await owner.browser.fetch(`/oauth2/authorize?${query}`);
        const consentPage = new URL(locationOf(authorized));
        const consented = await owner.browser.fetch("/oauth2/consent", {
            accept: true,
            oauth_query: consentPage.search.slice(1),
        });
        const callback = new URL(Redirect.parse(await consented.json()).url);
        const redeemed = await provider(fixture, "/oauth2/token", {
            grant_type: "authorization_code",
            code: present(callback.searchParams.get("code"), "the authorization code"),
            redirect_uri: REDIRECT,
            client_id: registered.clientId,
            code_verifier: verifier,
        });

        return present(Tokens.parse(await redeemed.json()).refresh_token, "the refresh token");
    };
    const refresh = async (token: string) => {
        const response = await provider(fixture, "/oauth2/token", {
            grant_type: "refresh_token",
            refresh_token: token,
            client_id: registered.clientId,
        });
        await response.text();

        return response.status;
    };
    const before = await grant();

    // revoke the consent, then grant it again
    const { items } = await owner.client.oauthConsent.list({ userId: owner.id });
    const consent = nth(items, 0);
    await owner.client.oauthConsent.revoke({
        userId: owner.id,
        id: consent.id,
        requestId: RequestId.create(Date.now()),
    });
    const after = await grant();

    // refuse the token issued before the second grant, and refresh the one issued after it
    expect([await refresh(before), await refresh(after)]).toEqual([400, 200]);
});

/** Sign a device in to a confidential application through the device authorization flow. */
test("sign a device in to a confidential application through the device authorization flow", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("ada@example.com");

    // register a confidential client in an account from a secret only its registrar knows
    const { accountId } = await fixture.createSpace(owner);
    const secret = await Credential.create("oauth-client");
    const registered = await owner.client.oauthClient.create({
        scope: accountId,
        requestId: RequestId.create(Date.now()),
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
    const grant = DeviceGrant.parse(await requested.json());
    const reviewed = await owner.browser.fetch(`/device?user_code=${grant.user_code}`);
    const approved = await owner.browser.fetch("/device/approve", {
        userCode: grant.user_code,
    });
    expect([requested.status, reviewed.status, approved.status]).toEqual([200, 200, 200]);
    expect(
        await fixture.storage.database
            .select({
                oauthClientId: deviceAuthorization.oauthClientId,
                status: deviceAuthorization.status,
            })
            .from(deviceAuthorization)
            .where(eq(deviceAuthorization.deviceCode, grant.device_code)),
    ).toEqual([{ oauthClientId: registered.clientId, status: "approved" }]);

    // refuse the code without the client's secret
    const exchange = (fields: Record<string, string>) =>
        provider(fixture, "/oauth2/token", {
            grant_type: DEVICE_CODE,
            device_code: grant.device_code,
            ...fields,
        });
    const unauthenticated = await exchange({ ...credentials, client_secret: "dst_ocs_wrong" });
    const issued = await exchange(credentials);
    const tokens = Tokens.parse(await issued.json());
    expect([
        [unauthenticated.status, Refusal.parse(await unauthenticated.json()).error],
        [issued.status, tokens.scope],
    ]).toEqual([
        [400, "invalid_client"],
        [200, "openid profile offline_access"],
    ]);
    const verified = await verifyToken(fixture, present(tokens.id_token, "the ID token"));
    expect(verified.aud).toBe(registered.clientId);

    // refuse further tokens once the user is suspended
    await fixture.storage.database
        .update(user.table)
        .set({ suspendedAt: Date.now() })
        .where(eq(user.table.id, owner.id));
    const suspended = await provider(fixture, "/oauth2/token", {
        ...credentials,
        grant_type: "refresh_token",
        refresh_token: present(tokens.refresh_token, "the refresh token"),
    });
    expect([suspended.status, await suspended.json()]).toEqual([
        401,
        { message: "user is no longer authorized" },
    ]);
});

/** Sign a device in as a new client through the browser once the person allows it, with PKCE. */
test("sign a device in as a new client through the browser once the person allows it, with a loopback redirect and PKCE", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("ada@example.com");
    const signedOut = fixture.browser();

    // authorize a fresh client with a challenge on the loopback
    const clientId = Identifier.create("client");
    const verifier = "v".repeat(43);
    const redirect = "http://127.0.0.1:53682/callback";
    const query = new URLSearchParams({
        response_type: "code",
        client_id: clientId,
        redirect_uri: redirect,
        scope: "openid profile email offline_access",
        resource: ACCOUNT_RESOURCE,
        state: "state-1",
        code_challenge: await Pkce.challenge(verifier),
        code_challenge_method: "S256",
    });
    const away = await signedOut.fetch(`/oauth2/authorize?${query}`);
    const asked = await owner.browser.fetch(`/oauth2/authorize?${query}`);
    const signIn = new URL(locationOf(away));
    const consentPage = new URL(locationOf(asked));
    expect({
        away: [away.status, `${signIn.origin}${signIn.pathname}`],
        asked: [asked.status, `${consentPage.origin}${consentPage.pathname}`],
    }).toEqual({
        away: [302, `${ISSUER}/sign-in`],
        asked: [302, `${ISSUER}/consent`],
    });

    // return a denial to the client, then a code once the person allows it
    const decide = async (action: "approve" | "deny") => {
        const decided = await owner.browser.fetch(
            `/consent${consentPage.search}`,
            new URLSearchParams({ action }),
        );

        return new URL(locationOf(decided));
    };
    const denied = await decide("deny");
    const approved = await decide("approve");
    expect({
        denied: [`${denied.origin}${denied.pathname}`, denied.searchParams.get("error")],
        approved: [`${approved.origin}${approved.pathname}`, approved.searchParams.get("state")],
        issuer: approved.searchParams.get("iss"),
    }).toEqual({
        denied: [redirect, "access_denied"],
        approved: [redirect, "state-1"],
        issuer: ISSUER,
    });

    // refuse and spend a code without its verifier, redeem a fresh one, and refuse it replayed
    const redeem = (code: string, codeVerifier = verifier) =>
        provider(fixture, "/oauth2/token", {
            grant_type: "authorization_code",
            code,
            redirect_uri: redirect,
            client_id: clientId,
            code_verifier: codeVerifier,
            resource: ACCOUNT_RESOURCE,
        });
    const code = present(approved.searchParams.get("code"), "the authorization code");
    const forged = await redeem(code, "f".repeat(43));
    const fresh = present((await decide("approve")).searchParams.get("code"), "the fresh code");
    const redeemed = await redeem(fresh);
    const replayed = await redeem(fresh);
    const tokens = Tokens.parse(await redeemed.json());
    expect([
        [forged.status, Refusal.parse(await forged.json()).error],
        [redeemed.status, tokens.token_type],
        [replayed.status, Refusal.parse(await replayed.json()).error],
    ]).toEqual([
        [401, "invalid_request"],
        [200, "Bearer"],
        [400, "invalid_grant"],
    ]);

    // act as the person through the new client, registered as theirs at consent
    const { sub, client_id: holder, caller } = claims(tokens.access_token);
    const [registered] = await fixture.storage.database
        .select({ id: client.table.id, scope: client.table.scope })
        .from(client.table)
        .where(eq(client.table.id, clientId));
    expect([sub, holder, caller.credential.kind, registered]).toEqual([
        owner.id,
        clientId,
        "session",
        { id: clientId, scope: owner.id },
    ]);
});

/** Include the assurance of the session that approved a device's client in its tokens. */
test("include the second factor of the session approving a device's client in its tokens, and send a person asked to sign in again to the sign-in page", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("ada@example.com");
    await fixture.elevate(owner.id);

    // authorize a fresh client, and once more asking the person to sign in again
    const clientId = Identifier.create("client");
    const verifier = "s".repeat(43);
    const redirect = "http://127.0.0.1:53682/callback";
    const query = new URLSearchParams({
        response_type: "code",
        client_id: clientId,
        redirect_uri: redirect,
        scope: "openid offline_access",
        resource: ACCOUNT_RESOURCE,
        state: "state-2",
        code_challenge: await Pkce.challenge(verifier),
        code_challenge_method: "S256",
    });
    const asked = new URL(locationOf(await owner.browser.fetch(`/oauth2/authorize?${query}`)));
    const again = new URLSearchParams({
        ...Object.fromEntries(query),
        prompt: "login",
        max_age: "60",
    });
    const reauthenticate = new URL(
        locationOf(await owner.browser.fetch(`/oauth2/authorize?${again}`)),
    );

    // consent and redeem the code for tokens with the session's second factor
    const consented = await owner.browser.fetch(
        `/consent${asked.search}`,
        new URLSearchParams({ action: "approve" }),
    );
    const callback = new URL(locationOf(consented));
    const redeemed = await provider(fixture, "/oauth2/token", {
        grant_type: "authorization_code",
        code: present(callback.searchParams.get("code"), "the authorization code"),
        redirect_uri: redirect,
        client_id: clientId,
        code_verifier: verifier,
        resource: ACCOUNT_RESOURCE,
    });
    const { caller } = claims(Tokens.parse(await redeemed.json()).access_token);
    expect({
        reauthenticate: `${reauthenticate.origin}${reauthenticate.pathname}`,
        assurance: caller.credential.assurance,
    }).toEqual({
        reauthenticate: `${ISSUER}/sign-in`,
        assurance: { level: 2, authenticatedAt: caller.credential.assurance?.authenticatedAt },
    });
});

/** Exchange a mailed code for a person's tokens on a device's new client at the token endpoint. */
test("sign a person in with a mailed code at the token endpoint as a new client, and refuse a wrong, a spent or an expired code", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("ada@example.com");
    const browser = fixture.browser();

    // mail a code, and exchange it for the person's tokens on a new client
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
    const { otp } = at(await fixture.codes(), -1);
    const wrong = await redeem(otp === "000000" ? "111111" : "000000");
    const redeemed = await redeem(otp);
    const spent = await redeem(otp);
    const tokens = Tokens.parse(await redeemed.json());
    const { sub, client_id: holder, caller } = claims(tokens.access_token);
    const [registered] = await fixture.storage.database
        .select({ id: client.table.id, scope: client.table.scope })
        .from(client.table)
        .where(eq(client.table.id, clientId));
    expect({
        wrong: [wrong.status, Refusal.parse(await wrong.json()).error],
        redeemed: [redeemed.status, tokens.scope, tokens.refresh_token !== undefined],
        token: [
            sub,
            holder,
            caller.credential.kind,
            caller.credential.id,
            caller.credential.assurance?.level,
        ],
        registered,
        spent: [spent.status, Refusal.parse(await spent.json()).error],
    }).toEqual({
        wrong: [400, "invalid_grant"],
        redeemed: [200, "openid profile offline_access", true],
        token: [owner.id, clientId, "client", clientId, 1],
        registered: { id: clientId, scope: owner.id },
        spent: [400, "invalid_grant"],
    });

    // refuse a code past its expiry
    await send();
    await fixture.storage.database
        .update(verification)
        .set({ expiresAt: Date.now() - 1 })
        .where(eq(verification.identifier, "sign-in-otp-ada@example.com"));
    const expired = await redeem(at(await fixture.codes(), -1).otp);
    expect([expired.status, Refusal.parse(await expired.json())]).toEqual([
        400,
        { error: "invalid_grant", error_description: "the code is wrong or expired" },
    ]);
});

/** Rename and disable a registered client, refuse authorizing it while disabled, and delete it. */
test("update, disable and delete an oauth client", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signInWithLink("ada@example.com");

    // register, rename and disable a public client
    const registered = await owner.client.oauthClient.create({
        scope: owner.id,
        requestId: RequestId.create(Date.now()),
        name: "Notes",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code"],
    });
    const updated = await owner.client.oauthClient.update({
        scope: owner.id,
        id: registered.id,
        requestId: RequestId.create(Date.now()),
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
        code_challenge: await Pkce.challenge("verifier".repeat(8)),
        code_challenge_method: "S256",
    });
    const refused = await owner.browser.fetch(`/oauth2/authorize?${query}`);
    expect([refused.status, refused.headers.get("location"), await refused.text()]).toEqual([
        302,
        `${ISSUER}/error?error=client_disabled&error_description=client+is+disabled`,
        "",
    ]);

    // delete the client
    const deleted = await owner.client.oauthClient.delete({
        scope: owner.id,
        id: registered.id,
        requestId: RequestId.create(Date.now()),
        revision: updated.revision,
    });
    expect(deleted).toEqual({});
    expect(
        await outcome(owner.client.oauthClient.get({ scope: owner.id, id: registered.id })),
    ).toBe(`NOT_FOUND: no oauth-client ${registered.id}`);
});

/** Choose a person's locale and time zone and refuse an unknown zone. */
async function chooseProfile(person: Person): Promise<void> {
    // change the time zone and the locale
    const current = await person.client.user.get({ id: person.id });
    const change = { id: person.id, revision: current.revision };
    expect(
        await outcome(
            person.client.user.update({
                ...change,
                requestId: RequestId.create(Date.now()),
                timeZone: "Mars/Olympus",
            }),
        ),
    ).toBe("BAD_REQUEST: unknown time zone Mars/Olympus");
    await person.client.user.update({
        ...change,
        requestId: RequestId.create(Date.now()),
        timeZone: "Europe/London",
        locale: "en-GB",
    });
}

/** Decode one base64url part of a JSON Web Token as JSON. */
function decode(segment: string): unknown {
    return JSON.parse(
        new TextDecoder().decode(Uint8Array.fromBase64(segment, { alphabet: "base64url" })),
    );
}

/** Send a request to the provider as another application does: a form post or a read. */
function provider(
    fixture: AccountFixture,
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

/** Verify an ES256 JSON Web Token with the provider's published key, returning its claims. */
async function verifyToken(fixture: AccountFixture, token: string): Promise<IdClaims> {
    // find the published key with the header's key identifier
    const [header, payload, signature] = TokenParts.parse(token.split("."));
    const { kid } = TokenHeader.parse(decode(header));
    const { keys } = KeySet.parse(await (await provider(fixture, JWKS_PATH)).json());
    const key = present(
        keys.find((entry) => entry.kid === kid),
        "the published signing key",
    );

    // check the signature over the header and claims
    const imported = await crypto.subtle.importKey(
        "jwk",
        key,
        { name: "ECDSA", namedCurve: "P-256" },
        false,
        ["verify"],
    );
    const isSigned = await crypto.subtle.verify(
        { name: "ECDSA", hash: "SHA-256" },
        imported,
        Uint8Array.fromBase64(signature, { alphabet: "base64url" }),
        new TextEncoder().encode(`${header}.${payload}`),
    );
    expect(isSigned).toBe(true);

    return IdClaims.parse(decode(payload));
}
