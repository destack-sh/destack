import { user, User } from "../object/user.ts";
import { client } from "../object/client.ts";
import { APIError } from "better-auth/api";
import { and, eq, gt, isNull, isNotNull, or, type DatabaseConnection } from "@destack/db";
import type { AuthenticationAssurance } from "@destack/access";
import { schema } from "@destack/schema";
import { passkey } from "../object/passkey.ts";
import { AUTHENTICATION_METHODS, session, type Session } from "../object/session.ts";

/** The maximum age of a ceremony used to change authentication credentials. */
const FRESHNESS_MILLISECONDS = 5 * 60 * 1000;

/** The assurance level each verified method proves. */
const ASSURANCE_LEVELS: Readonly<Record<NonNullable<Session["authenticationMethod"]>, 1 | 2 | 3>> =
    {
        "magic-link": 1,
        "email-otp": 1,
        oauth: 1,
        totp: 2,
        recovery: 2,
        webauthn: 3,
    };

/** The session and user Better Auth resolves, with the columns an active session has. */
const BetterAuthSession = schema.looseObject({
    session: schema.looseObject({
        id: schema.identifier("session"),
        expiresAt: schema.date(),
        authenticatedAt: schema.number().nullable(),
        authenticationMethod: schema.enum(AUTHENTICATION_METHODS).nullable(),
    }),
    user: schema.looseObject({
        id: schema.identifier("user"),
        twoFactorEnabled: schema.boolean(),
        email: schema.string(),
        emailVerified: schema.boolean(),
    }),
});

/** A live session of an active user on an active client. */
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

    /** Store a read session. */
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

    /** Read a session Better Auth resolved after the session guard found it active. */
    static of(resolved: unknown): ActiveSession {
        const { session: row, user: person } = BetterAuthSession.parse(resolved);

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

    /** Read an active session with its user's standing. */
    static async read(
        sessionId: string,
        database: DatabaseConnection,
        now: number,
    ): Promise<ActiveSession> {
        // read the session, its user and its client together
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
            .leftJoin(client.table, eq(client.table.id, session.table.clientId))
            .where(
                and(
                    eq(session.table.id, schema.identifier("session").parse(sessionId)),
                    gt(session.table.expiresAt, now),
                    isNull(session.table.revokedAt),
                    User.isActive(),
                    or(
                        isNull(session.table.clientId),
                        and(isNotNull(client.table.id), isNull(client.table.revokedAt)),
                    ),
                ),
            )
            .limit(1);
        if (active === undefined) {
            throw new APIError("UNAUTHORIZED", { message: "session is no longer authorized" });
        }

        // require the user's private fields
        const { twoFactorEnabled, email, emailVerified } = active;
        if (twoFactorEnabled === null || email === null || emailVerified === null) {
            throw new TypeError(`user ${active.userId} has its private fields concealed`);
        }

        return new ActiveSession({ ...active, twoFactorEnabled, email, emailVerified });
    }

    /** Rate the last ceremony from 1 to 3, none before one completed. */
    get assurance(): AuthenticationAssurance | undefined {
        // rate only a completed ceremony
        const method = this.authenticationMethod;
        if (this.authenticatedAt === null || method === null) {
            return undefined;
        }

        return { level: ASSURANCE_LEVELS[method], authenticatedAt: this.authenticatedAt };
    }

    /** Report whether a method proves an enrolled factor. */
    static isEnrolledFactor(method: NonNullable<Session["authenticationMethod"]>): boolean {
        return ASSURANCE_LEVELS[method] >= 2;
    }

    /** Require a recent ceremony with the user's enrolled factor. */
    async requireAuthentication(database: DatabaseConnection, now: number): Promise<void> {
        // require a completed ceremony within the sensitive-operation window
        const method = this.authenticationMethod;
        const time = this.authenticatedAt;
        if (method === null || time === null || time > now || now - time > FRESHNESS_MILLISECONDS) {
            throw new APIError("FORBIDDEN", {
                code: "REAUTHENTICATION_REQUIRED",
                message: "recent authentication is required",
            });
        }

        // require an enrolled factor
        const [credential] = await database
            .select({ id: passkey.table.id })
            .from(passkey.table)
            .where(eq(passkey.table.userId, this.userId))
            .limit(1);
        const isEnrolled = this.twoFactorEnabled || credential !== undefined;
        if (isEnrolled && !ActiveSession.isEnrolledFactor(method)) {
            throw new APIError("FORBIDDEN", {
                code: "REAUTHENTICATION_REQUIRED",
                message: "recent authentication with an enrolled factor is required",
            });
        }
    }
}
