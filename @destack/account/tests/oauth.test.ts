import { eq } from "@destack/db";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { Credential } from "../src/object/credential.ts";
import { user } from "../src/object/user.ts";
import { session } from "../src/object/authentication.ts";
import { identifier } from "@destack/schema";
import { deviceAuthorization } from "../src/stack/authentication/device.ts";
import { Browser } from "./browser.ts";
import { AccountFixture, outcome, type Person } from "./fixture.ts";
import { Digest } from "../src/object/digest.ts";

/** The fixture's origin, the provider's issuer. */
const ORIGIN = "http://localhost:3210";

/** The page the application receives codes at. */
const REDIRECT = "https://notes.test/callback";

/** The device authorization grant (RFC 8628). */
const DEVICE_CODE = "urn:ietf:params:oauth:grant-type:device_code";

/** Sign a user in to another application: discover the provider, authorize with PKCE, consent, redeem the code, read the user's claims, refresh, and end it all by revoking the consent. */
test("sign in to another application through the authorization code flow with PKCE", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("ada@example.com");
    await chooseProfile(owner);

    // register a public client
    const client = await owner.client.oauthClient.create({
        scope: owner.id,
        requestId: RequestId.create(),
        name: "Notes",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code", "refresh_token"],
    });

    // discover the provider at its issuer
    const discovered = await (await provider(fixture, "/.well-known/openid-configuration")).json();
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
        issuer: ORIGIN,
        authorization: `${ORIGIN}/auth/oauth2/authorize`,
        token: `${ORIGIN}/auth/oauth2/token`,
        userinfo: `${ORIGIN}/auth/oauth2/userinfo`,
        keys: `${ORIGIN}/auth/jwks`,
        device: `${ORIGIN}/auth/device/code`,
        grants: ["authorization_code", "refresh_token", DEVICE_CODE],
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
        client_id: client.clientId,
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
    const authorized = await owner.browser.fetch(`/auth/oauth2/authorize?${query}`);
    const consentPage = new URL(authorized.headers.get("location")!);
    expect([authorized.status, `${consentPage.origin}${consentPage.pathname}`]).toEqual([
        302,
        `${ORIGIN}/consent`,
    ]);

    // consent and return the user to the application with a code for its state from this issuer
    const consented = await owner.browser.fetch("/auth/oauth2/consent", {
        accept: true,
        oauth_query: consentPage.search.slice(1),
    });
    const callback = new URL((await consented.json()).url);
    expect({
        page: `${callback.origin}${callback.pathname}`,
        state: callback.searchParams.get("state"),
        issuer: callback.searchParams.get("iss"),
    }).toEqual({ page: REDIRECT, state: "state-1", issuer: ORIGIN });

    // redeem the code with its verifier
    const redeem = (code: string, codeVerifier = verifier) =>
        provider(fixture, "/auth/oauth2/token", {
            grant_type: "authorization_code",
            code,
            redirect_uri: REDIRECT,
            client_id: client.clientId,
            code_verifier: codeVerifier,
        });
    const redeemed = await redeem(callback.searchParams.get("code")!);
    const tokens = await redeemed.json();
    expect([redeemed.status, tokens.token_type, tokens.scope]).toEqual([
        200,
        "Bearer",
        "openid profile email offline_access",
    ]);

    // verify the ID token for this client and nonce with the published keys
    const identity = await verifyToken(fixture, tokens.id_token);
    expect({
        iss: identity.iss,
        sub: identity.sub,
        aud: identity.aud,
        nonce: identity.nonce,
    }).toEqual({ iss: ORIGIN, sub: owner.id, aud: client.clientId, nonce: "nonce-1" });

    // read the user's claims, with the handle, locale and time zone as OpenID Connect claims
    const userinfo = async (accessToken: string) => {
        const response = await provider(fixture, "/auth/oauth2/userinfo", undefined, {
            authorization: `Bearer ${accessToken}`,
        });

        return [response.status, await response.json()];
    };
    expect(await userinfo(tokens.access_token)).toEqual([
        200,
        {
            sub: owner.id,
            name: "ada@example.com",
            email: "ada@example.com",
            email_verified: true,
            preferred_username: "ada",
            locale: "de-AT",
            zoneinfo: "Europe/Vienna",
        },
    ]);

    // refuse a replayed code and end the tokens it issued
    const replayed = await redeem(callback.searchParams.get("code")!);
    expect([
        replayed.status,
        (await replayed.json()).error,
        (await userinfo(tokens.access_token))[0],
    ]).toEqual([400, "invalid_grant", 401]);

    // authorize again without consent and refuse a code without its verifier
    const authorize = async () => {
        const response = await owner.browser.fetch(`/auth/oauth2/authorize?${query}`);

        return new URL(response.headers.get("location")!);
    };
    const remembered = await authorize();
    const forged = await redeem(remembered.searchParams.get("code")!, "forged".repeat(8));
    expect([
        `${remembered.origin}${remembered.pathname}`,
        forged.status,
        (await forged.json()).error,
    ]).toEqual([REDIRECT, 401, "invalid_request"]);

    // refresh the tokens of another authorization
    const refresh = (token: string) =>
        provider(fixture, "/auth/oauth2/token", {
            grant_type: "refresh_token",
            refresh_token: token,
            client_id: client.clientId,
        });
    const fresh = await (await redeem((await authorize()).searchParams.get("code")!)).json();
    const refreshed = await refresh(fresh.refresh_token);
    const renewed = await refreshed.json();
    expect(refreshed.status).toBe(200);

    // revoke the consent to end the client's tokens
    const { items: consents } = await owner.client.oauthConsent.list({ userId: owner.id });
    expect(consents.map(({ clientId, scopes }) => ({ clientId, scopes }))).toEqual([
        { clientId: client.clientId, scopes: ["openid", "profile", "email", "offline_access"] },
    ]);
    await owner.client.oauthConsent.revoke({
        userId: owner.id,
        id: consents[0]!.id,
        requestId: RequestId.create(),
        revision: consents[0]!.revision,
    });
    const ended = await refresh(renewed.refresh_token);
    expect([
        ended.status,
        (await ended.json()).error,
        (await userinfo(renewed.access_token))[0],
    ]).toEqual([400, "invalid_grant", 401]);
});

/** Sign a device in to a confidential application through the device authorization flow, authenticating the client by its secret. */
test("sign a device in to a confidential application through the device authorization flow", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("ada@example.com");

    // register a confidential client in an account from a secret only its registrar knows
    const { accountId } = await fixture.createSpace(owner);
    const secret = await Credential.create("oauth-client");
    const client = await owner.client.oauthClient.create({
        scope: accountId,
        requestId: RequestId.create(),
        name: "Television",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "client_secret_post",
        clientSecret: secret.digest,
        grantTypes: [DEVICE_CODE, "refresh_token"],
    });
    expect([Object.hasOwn(client, "clientSecret"), client.userId]).toEqual([false, owner.id]);

    // request a device code as the client for the user to approve in the browser
    const credentials = { client_id: client.clientId, client_secret: secret.secret };
    const requested = await provider(fixture, "/auth/device/code", {
        ...credentials,
        scope: "openid profile offline_access",
    });
    const grant = await requested.json();
    const reviewed = await owner.browser.fetch(`/auth/device?user_code=${grant.user_code}`);
    const approved = await owner.browser.fetch("/auth/device/approve", {
        userCode: grant.user_code,
    });
    expect([requested.status, reviewed.status, approved.status]).toEqual([200, 200, 200]);
    expect(
        await fixture.opened.database
            .select({
                oauthClientId: deviceAuthorization.oauthClientId,
                status: deviceAuthorization.status,
            })
            .from(deviceAuthorization)
            .where(eq(deviceAuthorization.deviceCode, grant.device_code)),
    ).toEqual([{ oauthClientId: client.clientId, status: "approved" }]);

    // refuse the code at the native endpoint and without the client's secret, then issue tokens
    const exchange = (fields: Record<string, string>) =>
        provider(fixture, "/auth/oauth2/token", {
            grant_type: DEVICE_CODE,
            device_code: grant.device_code,
            ...fields,
        });
    const native = await fixture.browser().fetch("/auth/device/token", {
        grant_type: DEVICE_CODE,
        device_code: grant.device_code,
        client_id: "destack-daemon",
    });
    const unauthenticated = await exchange({ ...credentials, client_secret: "dst_ocs_wrong" });
    const issued = await exchange(credentials);
    const tokens = await issued.json();
    expect([
        [native.status, (await native.json()).error],
        [unauthenticated.status, (await unauthenticated.json()).error],
        [issued.status, tokens.scope],
    ]).toEqual([
        [400, "invalid_grant"],
        [400, "invalid_client"],
        [200, "openid profile offline_access"],
    ]);
    expect((await verifyToken(fixture, tokens.id_token)).aud).toBe(client.clientId);

    // refuse further tokens once the user is suspended
    await fixture.opened.database
        .update(user.table)
        .set({ suspendedAt: Date.now() })
        .where(eq(user.table.id, owner.id));
    const suspended = await provider(fixture, "/auth/oauth2/token", {
        ...credentials,
        grant_type: "refresh_token",
        refresh_token: tokens.refresh_token,
    });
    expect([suspended.status, await suspended.json()]).toEqual([
        401,
        { message: "user is no longer authorized" },
    ]);
});

/** Sign a native client in through the browser once the person allows it: a loopback redirect with PKCE, redeemed once for a session. */
test("sign a native client in through the browser once the person allows it, with a loopback redirect and PKCE", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("ada@example.com");
    const signedOut = fixture.browser();

    // authorize with a challenge on the loopback, sending a signed-out browser to sign in first
    const verifier = "v".repeat(43);
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(verifier));
    const redirect = "http://127.0.0.1:53682/callback";
    const query = new URLSearchParams({
        response_type: "code",
        client_id: "destack-daemon",
        redirect_uri: redirect,
        state: "state-1",
        code_challenge: new Uint8Array(digest).toBase64({
            alphabet: "base64url",
            omitPadding: true,
        }),
        code_challenge_method: "S256",
    });
    const authorize = (browser: Browser, parameters = query) =>
        browser.fetch(`/auth/native/authorize?${parameters}`);
    const decide = async (browser: Browser, action: "approve" | "deny") => {
        const asked = new URL((await authorize(browser)).headers.get("location")!);

        return browser.fetch(`${asked.pathname}${asked.search}`, new URLSearchParams({ action }));
    };
    const [away, foreign, asked] = [
        await authorize(signedOut),
        await authorize(
            owner.browser,
            new URLSearchParams({
                ...Object.fromEntries(query),
                redirect_uri: "https://evil.test/",
            }),
        ),
        await authorize(owner.browser),
    ];
    const signIn = new URL(away.headers.get("location")!);
    const consent = new URL(asked.headers.get("location")!);
    expect({
        away: [away.status, `${signIn.origin}${signIn.pathname}`],
        callbackURL: signIn.searchParams.get("callbackURL"),
        foreign: foreign.status,
        asked: [asked.status, consent.origin, consent.searchParams.get("flow")],
        code: consent.searchParams.get("code"),
    }).toEqual({
        away: [302, `${ORIGIN}/sign-in`],
        callbackURL: `${ORIGIN}/auth/native/authorize?${query}`,
        foreign: 400,
        asked: [302, ORIGIN, "native"],
        code: null,
    });

    // return a denial to the client, and a code once the person allows it
    const denied = new URL((await decide(owner.browser, "deny")).headers.get("location")!);
    const approved = await decide(owner.browser, "approve");
    const callback = new URL(approved.headers.get("location")!);
    expect({
        denied: [`${denied.origin}${denied.pathname}`, denied.searchParams.get("error")],
        approved: [approved.status, `${callback.origin}${callback.pathname}`],
        state: callback.searchParams.get("state"),
        issuer: callback.searchParams.get("iss"),
    }).toEqual({
        denied: [redirect, "access_denied"],
        approved: [303, redirect],
        state: "state-1",
        issuer: ORIGIN,
    });

    // refuse and spend the code without its verifier, then redeem a fresh one for a session
    const redeem = (code: string, codeVerifier = verifier) =>
        provider(fixture, "/auth/native/token", {
            grant_type: "authorization_code",
            code,
            redirect_uri: redirect,
            client_id: "destack-daemon",
            code_verifier: codeVerifier,
        });
    const forged = await redeem(callback.searchParams.get("code")!, "f".repeat(43));
    const spent = await redeem(callback.searchParams.get("code")!);
    const fresh = new URL((await decide(owner.browser, "approve")).headers.get("location")!);
    const redeemed = await redeem(fresh.searchParams.get("code")!);
    const replayed = await redeem(fresh.searchParams.get("code")!);
    const tokens = await redeemed.json();
    expect([
        [forged.status, (await forged.json()).error],
        [spent.status, (await spent.json()).error],
        [redeemed.status, tokens.token_type],
        [replayed.status, (await replayed.json()).error],
    ]).toEqual([
        [400, "invalid_grant"],
        [400, "invalid_grant"],
        [200, "Bearer"],
        [400, "invalid_grant"],
    ]);

    // act as the user with the session, recorded as a native sign-in
    const native = fixture.connect(() => ({ authorization: `Bearer ${tokens.access_token}` }));
    const { subject, credential } = await native.authentication.current();
    const [created] = await fixture.opened.database
        .select({ method: session.table.authenticationMethod })
        .from(session.table)
        .where(eq(session.table.id, identifier("session").parse(credential.id)));
    expect([subject.id, credential.kind, created!.method]).toEqual([owner.id, "session", "device"]);
});

/** Rename and disable a registered client, refuse authorizing it while disabled, and delete it. */
test("update, disable and delete an oauth client", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("ada@example.com");

    // register a public client, then rename and disable it
    const client = await owner.client.oauthClient.create({
        scope: owner.id,
        requestId: RequestId.create(),
        name: "Notes",
        redirectUris: [REDIRECT],
        tokenEndpointAuthMethod: "none",
        grantTypes: ["authorization_code"],
    });
    const updated = await owner.client.oauthClient.update({
        scope: owner.id,
        id: client.id,
        requestId: RequestId.create(),
        revision: client.revision,
        name: "Notebook",
        disabled: true,
    });
    expect(updated).toEqual({
        ...client,
        name: "Notebook",
        disabled: true,
        revision: 2,
        updatedAt: updated.updatedAt,
    });

    // refuse authorizing the disabled client
    const query = new URLSearchParams({
        response_type: "code",
        client_id: client.clientId,
        redirect_uri: REDIRECT,
        scope: "openid",
        state: "state-1",
        code_challenge: await Digest.base64url("verifier".repeat(8)),
        code_challenge_method: "S256",
    });
    const refused = await owner.browser.fetch(`/auth/oauth2/authorize?${query}`);
    expect([refused.status, refused.headers.get("location"), await refused.text()]).toEqual([
        302,
        `${ORIGIN}/auth/error?error=client_disabled&error_description=client+is+disabled`,
        "",
    ]);

    // delete the client, after which it is gone
    const deleted = await owner.client.oauthClient.delete({
        scope: owner.id,
        id: client.id,
        requestId: RequestId.create(),
        revision: updated.revision,
    });
    expect(deleted).toEqual({});
    expect(await outcome(owner.client.oauthClient.get({ scope: owner.id, id: client.id }))).toBe(
        `NOT_FOUND: no oauth-client ${client.id}`,
    );
});

/** Choose a person's handle, locale and time zone and refuse an unknown zone. */
async function chooseProfile(person: Person): Promise<void> {
    // claim the handle with the personal account, after choosing a residency
    const chosen = await person.client.user.get({ id: person.id });
    await person.client.user.update({
        id: person.id,
        requestId: RequestId.create(),
        revision: chosen.revision,
        residency: "eu",
    });
    await person.client.account.create({
        scope: person.id,
        requestId: RequestId.create(),
        handle: "ada",
        name: "Ada",
        defaultResidency: "eu",
        kind: "personal",
    });

    // set the locale and time zone
    const current = await person.client.user.get({ id: person.id });
    const change = { id: person.id, revision: current.revision, locale: "de-AT" };
    await expect(
        person.client.user.update({
            ...change,
            requestId: RequestId.create(),
            timeZone: "Mars/Olympus",
        }),
    ).rejects.toMatchObject({ code: "BAD_REQUEST", message: "unknown time zone Mars/Olympus" });
    await person.client.user.update({
        ...change,
        requestId: RequestId.create(),
        timeZone: "Europe/Vienna",
    });
}

/** Send a request to the provider as another application does: a form post, or a read with its headers. */
function provider(
    fixture: AccountFixture,
    path: string,
    form?: Record<string, string>,
    headers: Record<string, string> = {},
): Promise<Response> {
    return fixture.server.fetch(
        new Request(new URL(path, ORIGIN), {
            method: form === undefined ? "GET" : "POST",
            headers:
                form === undefined
                    ? headers
                    : { ...headers, "content-type": "application/x-www-form-urlencoded" },
            body: form === undefined ? undefined : new URLSearchParams(form),
        }),
    );
}

/** Verify an ES256 JSON Web Token with the key the provider publishes under its identifier, returning its claims. */
async function verifyToken(
    fixture: AccountFixture,
    token: string,
): Promise<Record<string, unknown>> {
    // find the published key with the header's key identifier
    const [header, claims, signature] = token.split(".") as [string, string, string];
    const decode = (part: string) =>
        JSON.parse(
            new TextDecoder().decode(Uint8Array.fromBase64(part, { alphabet: "base64url" })),
        );
    const { keys } = await (await provider(fixture, "/auth/jwks")).json();
    const key = keys.find((entry: { kid: string }) => entry.kid === decode(header).kid);

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
        new TextEncoder().encode(`${header}.${claims}`),
    );
    expect(isSigned).toBe(true);

    return decode(claims);
}
