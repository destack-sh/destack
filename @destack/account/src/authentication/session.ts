import { user } from "../object/user.ts";
import { device } from "../object/device.ts";
import { APIError, createAuthMiddleware, getSessionFromCtx, isAPIError } from "better-auth/api";
import { and, eq, gt, isNull, isNotNull, or, type DatabaseConnection } from "@destack/db";
import type { AuthenticationAssurance } from "@destack/access";
import { identifier, schema } from "@destack/schema";
import {
    AUTHENTICATION_METHODS,
    passkey,
    session,
    type Session,
} from "../object/authentication.ts";
import { Bearer } from "@destack/service/authentication";
import type { BetterAuthPlugin } from "better-auth";

/** The maximum age of a ceremony used to change authentication credentials. */
const FRESHNESS_MILLISECONDS = 5 * 60 * 1000;

/** Authentication changes requiring a recent ceremony. */
const SENSITIVE = new Set([
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
    "/native/token",
]);

/** Completed authentication protocols and their verified methods. */
const METHODS: Readonly<Record<string, NonNullable<Session["authenticationMethod"]>>> = {
    "/magic-link/verify": "magic-link",
    "/sign-in/email-otp": "email-otp",
    "/device/token": "device",
    "/native/token": "device",
    "/two-factor/verify-totp": "totp",
    "/two-factor/verify-backup-code": "recovery",
    "/passkey/verify-authentication": "webauthn",
};

/** The session and user Better Auth resolves, with the columns an active session holds. */
const ResolvedSession = schema.looseObject({
    session: schema.looseObject({
        id: identifier("session"),
        expiresAt: schema.date(),
        authenticatedAt: schema.number().nullable(),
        authenticationMethod: schema.enum(AUTHENTICATION_METHODS).nullable(),
    }),
    user: schema.looseObject({
        id: identifier("user"),
        twoFactorEnabled: schema.boolean(),
        email: schema.string(),
        emailVerified: schema.boolean(),
    }),
});

/** Apply session policy after credential plugins resolve native and browser credentials. */
export function sessionPolicy(database: DatabaseConnection): BetterAuthPlugin {
    return {
        id: "destack-session",
        hooks: {
            before: [{ matcher: () => true, handler: sessionGuard(database) }],
            after: [{ matcher: () => true, handler: sessionAuthentication() }],
        },
    };
}

/** Enforce shared suspension and revocation before authentication endpoints run. */
function sessionGuard(database: DatabaseConnection) {
    return createAuthMiddleware(async (context) => {
        // skip sign-out and the OAuth endpoints authenticating clients themselves
        const path = context.path;
        if (path === "/sign-out" || (path !== undefined && CLIENT_ENDPOINTS.has(path))) {
            return;
        }
        // resolve opaque native tokens directly before checking shared session policy
        const headers = context.request?.headers ?? context.headers;
        const bearer = headers === undefined ? undefined : Bearer.read(headers);
        if (headers?.has("authorization") && bearer === undefined) {
            throw new APIError("UNAUTHORIZED", { message: "invalid session credential" });
        }
        const current =
            bearer === undefined
                ? await getSessionFromCtx(context, { disableRefresh: true })
                : await context.context.internalAdapter.findSession(bearer);
        if (
            bearer !== undefined &&
            (!current || current.session.expiresAt.getTime() <= Date.now())
        ) {
            throw new APIError("UNAUTHORIZED", { message: "session is no longer authorized" });
        }
        if (!current) {
            return;
        }
        context.context.session = current;

        // check authoritative revocation
        const active = await ActiveSession.read(current.session.id, database);

        // require the enrolled factor before changing credentials or their recovery methods
        if (context.path !== undefined && SENSITIVE.has(context.path)) {
            await active.requireAuthentication(database);
        }
    });
}

/** Record the method only after the authentication protocol succeeds. */
function sessionAuthentication() {
    return createAuthMiddleware(async (context) => {
        // ignore registration, failures, and challenges that have not issued a session
        if (context.path === undefined) {
            return;
        }
        const path = context.path;
        const method = path.startsWith("/callback/") ? "oauth" : METHODS[path];
        const result = context.context.returned;
        if (!method || (isAPIError(result) && result.statusCode >= 400)) {
            return;
        }
        const isVerification = method === "totp" || method === "webauthn" || method === "recovery";
        const current = isVerification
            ? (context.context.newSession ?? context.context.session)
            : context.context.newSession;
        if (!current) {
            return;
        }

        // retain the verified method through Better Auth's session store
        await context.context.internalAdapter.updateSession(current.session.token, {
            authenticatedAt: Date.now(),
            authenticationMethod: method,
        });
    });
}

/** A live session of an active user on an active device. */
export class ActiveSession {
    /** The session. */
    readonly id: Session["id"];
    /** The signed-in user. */
    readonly userId: Session["userId"];
    /** The session expiry time. */
    readonly expiresAt: number;
    /** The last completed authentication ceremony. */
    readonly authenticatedAt: number | null;
    /** The verified method of the last ceremony. */
    readonly authenticationMethod: Session["authenticationMethod"];
    /** Whether the user enrolled TOTP. */
    readonly twoFactorEnabled: boolean;
    /** The user's email address. */
    readonly email: string;
    /** Whether the user verified the email address. */
    readonly emailVerified: boolean;

    /** Hold a read session. */
    private constructor(row: Omit<ActiveSession, "assurance" | "requireAuthentication">) {
        // take the session's and its user's columns
        this.id = row.id;
        this.userId = row.userId;
        this.expiresAt = row.expiresAt;
        this.authenticatedAt = row.authenticatedAt;
        this.authenticationMethod = row.authenticationMethod;
        this.twoFactorEnabled = row.twoFactorEnabled;
        this.email = row.email;
        this.emailVerified = row.emailVerified;
    }

    /** Read an active session with its user's standing. */
    static async read(sessionId: string, database: DatabaseConnection): Promise<ActiveSession> {
        // read the session, its user and its device together
        const [active] = await database
            .select({
                id: session.table.id,
                userId: session.table.userId,
                expiresAt: session.table.expiresAt,
                authenticatedAt: session.table.authenticatedAt,
                authenticationMethod: session.table.authenticationMethod,
                twoFactorEnabled: user.table.twoFactorEnabled,
                email: user.table.email,
                emailVerified: user.table.emailVerified,
            })
            .from(session.table)
            .innerJoin(user.table, eq(user.table.id, session.table.userId))
            .leftJoin(device.table, eq(device.table.id, session.table.deviceId))
            .where(
                and(
                    eq(session.table.id, identifier("session").parse(sessionId)),
                    gt(session.table.expiresAt, Date.now()),
                    isNull(session.table.revokedAt),
                    isNull(user.table.suspendedAt),
                    isNull(user.table.deletionRequestedAt),
                    or(
                        isNull(session.table.deviceId),
                        and(isNotNull(device.table.id), isNull(device.table.revokedAt)),
                    ),
                ),
            )
            .limit(1);
        if (!active) {
            throw new APIError("UNAUTHORIZED", { message: "session is no longer authorized" });
        }

        // require the user's private fields, which the user's home database always has
        const { twoFactorEnabled, email, emailVerified } = active;
        if (twoFactorEnabled === null || email === null || emailVerified === null) {
            throw new TypeError(`user ${active.userId} has its private fields concealed`);
        }

        return new ActiveSession({ ...active, twoFactorEnabled, email, emailVerified });
    }

    /** Hold a session Better Auth resolved after the session guard found it active. */
    static of(resolved: unknown): ActiveSession {
        const { session: row, user: person } = ResolvedSession.parse(resolved);

        return new ActiveSession({
            id: row.id,
            userId: person.id,
            expiresAt: row.expiresAt.getTime(),
            authenticatedAt: row.authenticatedAt,
            authenticationMethod: row.authenticationMethod,
            twoFactorEnabled: person.twoFactorEnabled,
            email: person.email,
            emailVerified: person.emailVerified,
        });
    }

    /** Rate the last ceremony from 1 to 3. */
    get assurance(): AuthenticationAssurance | undefined {
        // rate only a completed ceremony
        const method = this.authenticationMethod;
        if (this.authenticatedAt === null || method === null) {
            return undefined;
        }
        const level =
            method === "webauthn" ? 3 : method === "totp" || method === "recovery" ? 2 : 1;

        return { level, authenticatedAt: this.authenticatedAt };
    }

    /** Require a recent ceremony with the user's enrolled factor, phishing resistant when asked. */
    async requireAuthentication(
        database: DatabaseConnection,
        phishingResistant = false,
    ): Promise<void> {
        // require a completed ceremony within the sensitive-operation window
        const method = this.authenticationMethod;
        const time = this.authenticatedAt;
        const now = Date.now();
        if (method === null || time === null || time > now || now - time > FRESHNESS_MILLISECONDS) {
            throw new APIError("FORBIDDEN", {
                code: "REAUTHENTICATION_REQUIRED",
                message: "recent authentication is required",
            });
        }

        // WebAuthn authentication requires verified user presence and the expected origin
        if (phishingResistant && method !== "webauthn") {
            throw new APIError("FORBIDDEN", {
                code: "WEBAUTHN_REQUIRED",
                message: "recent WebAuthn authentication is required",
            });
        }

        // require an enrolled factor
        const [credential] = await database
            .select({ id: passkey.table.id })
            .from(passkey.table)
            .where(eq(passkey.table.userId, this.userId))
            .limit(1);
        const isVerified = method === "totp" || method === "webauthn" || method === "recovery";
        if ((this.twoFactorEnabled || credential) && !isVerified) {
            throw new APIError("FORBIDDEN", {
                code: "REAUTHENTICATION_REQUIRED",
                message: "recent authentication with an enrolled factor is required",
            });
        }
    }
}

/** Reject new sessions for suspended or deleting users. */
export async function authorizeSignIn(userId: string, database: DatabaseConnection): Promise<void> {
    const [active] = await database
        .select({ id: user.table.id })
        .from(user.table)
        .where(
            and(
                eq(user.table.id, identifier("user").parse(userId)),
                isNull(user.table.suspendedAt),
                isNull(user.table.deletionRequestedAt),
            ),
        )
        .limit(1);
    if (!active) {
        throw new APIError("UNAUTHORIZED", { message: "user is no longer authorized" });
    }
}
