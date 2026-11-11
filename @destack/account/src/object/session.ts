import { AccessContext, accessGrant, Caller, principal } from "@destack/access";
import { type Select, type Snapshot, type Table } from "@destack/db";
import { type MethodContext, defineAuditAction, defineObject, field, grant } from "@destack/object";
import { schema } from "@destack/schema";
import { type Call, RequestId, Scope } from "@destack/sync";
import type { ObjectStore } from "@destack/object/server";
import { Revocation } from "../credential/revocation.ts";
import { client } from "./client.ts";
import { user } from "./user.ts";
import { argument, plural, t } from "@destack/locale";

/** The verified methods of an authentication ceremony. */
export const AUTHENTICATION_METHODS = [
    "email-otp",
    "oauth",
    "totp",
    "webauthn",
    "recovery",
] as const;

/** A user's signed-in session. */
export const session = defineObject(
    {
        name: "session",
        plural: "sessions",
        title: t`${plural(argument("count"), { one: "Session", other: "Sessions" })}`,
        scope: user,
        fields: {
            /** The authenticated user. */
            user: field.reference(user, { delete: "cascade" }),
            /** The client the person signed in through; revoking it ends the session. */
            client: field.reference(() => client).optional(),
            /** The session credential stored by Better Auth. */
            token: field.string().sensitive("deterministic"),
            /** The client IP observed through trusted ingress. */
            ipAddress: field.string().optional(),
            /** The reported client user agent. */
            userAgent: field.string().optional(),
            /** The approximate country inferred from the observed address. */
            country: field.string().optional(),
            /** The approximate city inferred from the observed address. */
            city: field.string().optional(),
            /** The last completed authentication ceremony. */
            authenticatedAt: field.time().optional(),
            /** The verified method used in the last authentication ceremony. */
            authenticationMethod: field.enum(AUTHENTICATION_METHODS).optional(),
            /** The session expiry time. */
            expiresAt: field.time(),
            /** The time access was revoked. */
            revokedAt: field.time().optional(),
        },
        constraints: {
            token: { kind: "unique", on: ["token"] },
            user: { kind: "index", on: ["userId"] },
            client: { kind: "index", on: ["clientId"] },
            expiry: { kind: "index", on: ["expiresAt"] },
            expiryOrder: {
                kind: "check",
                condition: { expiresAt: { gt: { field: "createdAt" } } },
            },
            authenticationTime: {
                kind: "check",
                condition: {
                    OR: [
                        { authenticationMethod: { isNull: true } },
                        { authenticatedAt: { isNotNull: true } },
                    ],
                },
            },
        },
        access: {
            permissions: ["read", "revoke", "link"],
        },
    },
    (method) => ({
        revoke: method.mutation({ title: t`Revoke`, permission: "revoke" }),
        revokeOthers: method.mutation({ title: t`Revoke others`, permission: "revoke" }),
        link: method.update("link", { visibility: "package", fields: ["clientId"] }),
    }),
).implement({
    methods: {
        revoke: async (call) => {
            // withdraw the target now, refusing one withdrawn before
            Revocation.requireUnrevoked(call.target, call.object.name);
            const revoked = await call.update({ revokedAt: call.now });

            // revoke the grants its person gave for this sign-in alone, as the person
            const reader = { read: <Read extends Table>(table: Read) => call.read(table) };
            const calls = await Session.grantRevocations(reader, call.target.id);
            if (calls.length > 0) {
                const person = principal.user.reference(Scope.universe.id, call.target.userId);
                await call.mutate(AccessContext.of(Caller.of(person), call.now), calls);
            }

            return revoked;
        },
        revokeOthers: async (call) => {
            await revokeOthers(call, call.target.id);

            return call.target;
        },
        link: async (call, next) => {
            // refuse linking a revoked session
            Revocation.requireUnrevoked(call.target, call.object.name);

            return next();
        },
    },
});
/** A persisted session record. */
export type Session = Select<typeof session.table>;

/** A person's sign-in, which ends the grants they gave for it alone. */
export const Session = {
    /** Revoke sessions as their people, each revoking the grants given for it alone. */
    async revoke(
        objects: Pick<ObjectStore, "mutate" | "environment">,
        sessions: readonly Pick<Session, "id" | "scope" | "userId">[],
        now: number,
    ): Promise<void> {
        for (const row of sessions) {
            const person = principal.user.reference(Scope.universe.id, row.userId);
            await objects.mutate(AccessContext.of(Caller.of(person), now), {
                id: RequestId.create(objects.environment),
                calls: [session.call("revoke", session.target(row))],
            });
        }
    },

    /** List the calls revoking the grants a person gave for one sign-in alone. */
    async grantRevocations(reader: Pick<Snapshot, "read">, id: Session["id"]): Promise<Call[]> {
        const bound = await reader.read(accessGrant).findMany({ where: { sessionId: id } });

        return bound.map((row) => grant.call("revoke", grant.target(row)));
    },
};

/** End every active session of the target's user but the target. */
async function revokeOthers(context: MethodContext, current: Session["id"]): Promise<void> {
    // read the other active sessions
    const others = await context.read(session).findMany({
        where: {
            scope: schema.identifier("user").parse(context.scope),
            id: { ne: current },
            revokedAt: { isNull: true },
        },
    });

    // revoke them in one mutation
    await context.mutate(
        context.caller,
        others.map((other) => session.call("revoke", session.target(other))),
    );
}

/** A session issued after all required authentication ceremonies complete. */
export const sessionCreate = defineAuditAction({
    name: "session.create",
    target: schema.object({ type: schema.literal("session"), id: schema.identifier("session") }),
    details: schema.object({
        /** The path of the endpoint issuing the session. */
        path: schema.string().exactOptional(),
    }),
});
