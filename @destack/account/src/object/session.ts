import { type Select } from "@destack/db";
import { type MethodContext, defineAuditAction, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { Revocation } from "../credential/revocation.ts";
import { client } from "./client.ts";
import { user } from "./user.ts";

/** The verified methods of an authentication ceremony. */
export const AUTHENTICATION_METHODS = [
    "magic-link",
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
        permissions: ["read", "revoke", "attach"],
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
    },
    (method) => ({
        revoke: method.mutation({ permission: "revoke" }),
        revokeOthers: method.mutation({ permission: "revoke" }),
        attach: method.update("attach", { visibility: "package", fields: ["clientId"] }),
    }),
).implement({
    methods: {
        revoke: (call) => {
            // withdraw the target now, refusing one withdrawn before
            Revocation.requireUnrevoked(call.target, call.object.name);

            return call.update({ revokedAt: call.now });
        },
        revokeOthers: async (call) => {
            await revokeOthers(call, call.target.id);

            return call.target;
        },
        attach: async (call, next) => {
            // refuse attaching a revoked session
            Revocation.requireUnrevoked(call.target, call.object.name);

            return next();
        },
    },
});
/** A persisted session record. */
export type Session = Select<typeof session.table>;

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
