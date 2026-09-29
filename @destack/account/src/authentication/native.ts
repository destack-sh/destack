import type { BetterAuthPlugin } from "better-auth";
import { APIError, createAuthEndpoint, getSessionFromCtx } from "better-auth/api";
import { schema } from "@destack/schema";
import { Digest } from "../object/digest.ts";

/** How long an authorization code lives, within RFC 6749 4.1.2's ten minutes. */
const CODE_MILLISECONDS = 60 * 1000;

/** The random bytes of an authorization code, 256 bits. */
const CODE_BYTES = 32;

/** The loopback addresses of a native redirect, per RFC 8252 7.3. */
const LOOPBACK_HOSTS = new Set(["127.0.0.1", "[::1]"]);

/** The prefix of the verification records holding authorization codes. */
const CODE_PREFIX = "native-code:";

/** A native client's authorization request, as RFC 6749 4.1.1 and RFC 7636 4.3 define it. */
const AuthorizationRequest = schema.object({
    response_type: schema.literal("code"),
    client_id: schema.string(),
    redirect_uri: schema.string(),
    state: schema.string().min(1),
    code_challenge: schema.string().regex(/^[A-Za-z0-9_-]{43}$/),
    code_challenge_method: schema.literal("S256"),
});

/** The grant an authorization code carries. */
const Grant = schema.object({
    /** The approving user. */
    userId: schema.string(),
    /** The native client. */
    clientId: schema.string(),
    /** The loopback redirect the code was sent to. */
    redirectUri: schema.string(),
    /** The S256 PKCE challenge. */
    challenge: schema.string(),
});

/** Sign native clients in through a browser and a loopback redirect. */
export function nativeAuthorization(options: {
    /** The native client Destack ships. */
    readonly client: string;
    /** The browser page where users sign in. */
    readonly signInUri: string;
    /** The browser page where users approve a native client's sign-in. */
    readonly consentUri: string;
}) {
    /** Refuse an unknown client or a redirect other than a loopback address. */
    const requireClient = (request: schema.Infer<typeof AuthorizationRequest>) => {
        if (request.client_id !== options.client || !isLoopback(request.redirect_uri)) {
            throw new APIError("BAD_REQUEST", {
                error: "invalid_request",
                error_description: "unknown native client or redirect",
            });
        }
    };

    return {
        id: "destack-native-authorization",
        endpoints: {
            nativeAuthorize: createAuthEndpoint(
                "/native/authorize",
                { method: "GET", query: AuthorizationRequest },
                async (context) => {
                    // refuse an unknown client or redirect
                    const query = context.query;
                    requireClient(query);

                    // send a signed-out browser to sign in, returning here afterwards
                    const here = `${context.context.baseURL}/native/authorize?${new URLSearchParams(query)}`;
                    const session = await getSessionFromCtx(context, { disableRefresh: true });
                    if (session === null) {
                        const page = new URL(options.signInUri, context.context.baseURL);
                        page.searchParams.set("callbackURL", here);
                        throw context.redirect(page.href);
                    }

                    // ask the person to approve the client on the consent page
                    const page = new URL(options.consentUri, context.context.baseURL);
                    page.search = new URLSearchParams({ ...query, flow: "native" }).toString();
                    throw context.redirect(page.href);
                },
            ),
            nativeApprove: createAuthEndpoint(
                "/native/approve",
                {
                    method: "POST",
                    body: schema.object({
                        /** Whether the person allowed the client. */
                        accept: schema.boolean(),
                        /** The client's authorization request. */
                        query: schema.string(),
                    }),
                },
                async (context) => {
                    // require the approving person and a known client
                    const session = await getSessionFromCtx(context, { disableRefresh: true });
                    if (session === null) {
                        throw new APIError("UNAUTHORIZED", { message: "sign in to approve" });
                    }
                    const request = AuthorizationRequest.parse(
                        Object.fromEntries(new URLSearchParams(context.body.query)),
                    );
                    requireClient(request);

                    // return a denial to the client
                    const callback = new URL(request.redirect_uri);
                    callback.searchParams.set("state", request.state);
                    callback.searchParams.set("iss", context.context.options.baseURL as string);
                    if (!context.body.accept) {
                        callback.searchParams.set("error", "access_denied");

                        return context.json({ url: callback.href });
                    }

                    // keep the code's grant for its exchange, and return the code to the client
                    const code = crypto
                        .getRandomValues(new Uint8Array(CODE_BYTES))
                        .toBase64({ alphabet: "base64url", omitPadding: true });
                    await context.context.internalAdapter.createVerificationValue({
                        identifier: `${CODE_PREFIX}${await Digest.hex(code)}`,
                        value: JSON.stringify({
                            userId: session.user.id,
                            clientId: request.client_id,
                            redirectUri: request.redirect_uri,
                            challenge: request.code_challenge,
                        }),
                        expiresAt: new Date(Date.now() + CODE_MILLISECONDS),
                    });
                    callback.searchParams.set("code", code);

                    return context.json({ url: callback.href });
                },
            ),
            nativeToken: createAuthEndpoint(
                "/native/token",
                {
                    method: "POST",
                    body: schema.object({
                        grant_type: schema.literal("authorization_code"),
                        code: schema.string().min(1),
                        redirect_uri: schema.string(),
                        client_id: schema.string(),
                        code_verifier: schema.string().regex(/^[A-Za-z0-9._~-]{43,128}$/),
                    }),
                    metadata: {
                        allowedMediaTypes: [
                            "application/x-www-form-urlencoded",
                            "application/json",
                        ],
                    },
                },
                async (context) => {
                    // take the code once and refuse an expired one
                    const body = context.body;
                    const consumed = await context.context.internalAdapter.consumeVerificationValue(
                        `${CODE_PREFIX}${await Digest.hex(body.code)}`,
                    );
                    const grant =
                        consumed === null || consumed.expiresAt.getTime() <= Date.now()
                            ? undefined
                            : Grant.parse(JSON.parse(consumed.value));

                    // require the code's client, redirect and verifier
                    const isGranted =
                        grant !== undefined &&
                        grant.clientId === body.client_id &&
                        grant.redirectUri === body.redirect_uri &&
                        grant.challenge === (await Digest.base64url(body.code_verifier));
                    if (!isGranted) {
                        throw new APIError("BAD_REQUEST", {
                            error: "invalid_grant",
                            error_description: "invalid authorization code",
                        });
                    }

                    // sign the client in with a session of the approving user
                    const user = await context.context.internalAdapter.findUserById(grant.userId);
                    const session =
                        user === null
                            ? null
                            : await context.context.internalAdapter.createSession(user.id);
                    if (user === null || session === null) {
                        throw new APIError("BAD_REQUEST", {
                            error: "invalid_grant",
                            error_description: "the approving user cannot sign in",
                        });
                    }
                    context.context.setNewSession({ session, user });
                    context.setHeader("Cache-Control", "no-store");

                    return context.json({
                        access_token: session.token,
                        token_type: "Bearer",
                        expires_in: Math.floor((session.expiresAt.getTime() - Date.now()) / 1000),
                    });
                },
            ),
        },
    } satisfies BetterAuthPlugin;
}

/** Report whether a redirect points to a loopback address over HTTP. */
function isLoopback(uri: string): boolean {
    // parse the redirect and refuse anything but a URL
    let url: URL;
    try {
        url = new URL(uri);
    } catch {
        return false;
    }

    return url.protocol === "http:" && LOOPBACK_HOSTS.has(url.hostname) && url.hash === "";
}
