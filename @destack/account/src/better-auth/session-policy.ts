import { createAuthMiddleware, getSessionFromCtx, isAPIError } from "better-auth/api";
import type { DatabaseConnection } from "@destack/db";
import type { Session } from "../object/session.ts";
import type { BetterAuthPlugin } from "better-auth";
import { ActiveSession } from "./active-session.ts";
import type { Clock } from "@destack/environment";

/** Authentication changes requiring a recent ceremony. */
const SENSITIVE_PATHS = new Set([
    "/two-factor/enable",
    "/two-factor/disable",
    "/two-factor/generate-backup-codes",
    "/passkey/generate-register-options",
    "/passkey/verify-registration",
    "/passkey/delete-passkey",
    "/link-social",
    "/unlink-account",
    "/change-email",
    "/device/approve",
]);

/** The protocol endpoints that authenticate clients instead of sessions. */
const CLIENT_ENDPOINTS = new Set([
    "/oauth2/token",
    "/oauth2/userinfo",
    "/oauth2/introspect",
    "/oauth2/revoke",
]);

/** Completed authentication protocols and their verified methods. */
const METHOD_PATHS: Readonly<Record<string, NonNullable<Session["authenticationMethod"]>>> = {
    "/magic-link/verify": "magic-link",
    "/sign-in/email-otp": "email-otp",
    "/two-factor/verify-totp": "totp",
    "/two-factor/verify-backup-code": "recovery",
    "/passkey/verify-authentication": "webauthn",
};

/** Apply session policy after credential plugins resolve native and browser credentials. */
export function betterAuthSessionPolicy(
    database: DatabaseConnection,
    clock: Clock,
): BetterAuthPlugin {
    return {
        id: "destack-session",
        hooks: {
            before: [{ matcher: () => true, handler: requireActiveSession(database, clock) }],
            after: [{ matcher: () => true, handler: recordAuthenticationMethod(clock) }],
        },
    };
}

/** Enforce shared suspension and revocation before authentication endpoints run. */
function requireActiveSession(database: DatabaseConnection, clock: Clock) {
    return createAuthMiddleware(async (context) => {
        // skip sign-out and the OAuth endpoints authenticating clients themselves
        const path = context.path;
        if (path === "/sign-out" || (path !== undefined && CLIENT_ENDPOINTS.has(path))) {
            return;
        }

        // resolve the session cookie
        const current = await getSessionFromCtx(context, { disableRefresh: true });

        // pass a request without a session
        if (current === null) {
            return;
        }

        // cache the resolved session for the endpoint
        context.context.session = current;

        // check authoritative revocation, and require the enrolled factor for a sensitive change
        const now = clock.now();
        const active = await ActiveSession.read(current.session.id, database, now);
        if (path !== undefined && SENSITIVE_PATHS.has(path)) {
            await active.requireAuthentication(database, now);
        }
    });
}

/** Record the method only after the authentication protocol succeeds. */
function recordAuthenticationMethod(clock: Clock) {
    return createAuthMiddleware(async (context) => {
        // ignore registration, failures and challenges without a session
        const path = context.path;
        if (path === undefined) {
            return;
        }

        // take the verified method of a completed protocol
        const method = path.startsWith("/callback/")
            ? "oauth"
            : Object.hasOwn(METHOD_PATHS, path)
              ? METHOD_PATHS[path]
              : undefined;
        const result = context.context.returned;
        if (method === undefined || (isAPIError(result) && result.statusCode >= 400)) {
            return;
        }

        // find the session the ceremony created or verified
        const current = ActiveSession.isEnrolledFactor(method)
            ? (context.context.newSession ?? context.context.session)
            : context.context.newSession;
        if (current === null || current === undefined) {
            return;
        }

        // retain the verified method through Better Auth's session store
        await context.context.internalAdapter.updateSession(current.session.token, {
            authenticatedAt: clock.now(),
            authenticationMethod: method,
        });
    });
}
