import { defineAuditAction } from "@destack/audit";
import { identifier, schema } from "@destack/schema";
import { daemonPackage } from "./action.ts";

/** Record login sign-in under its verified request or execution identity. */
export const signInLogin = defineAuditAction({
    package: daemonPackage,
    name: "login.sign-in",
    version: 1,
    targets: schema.object({
        login: schema.object({ type: schema.literal("login"), id: identifier("login") }),
    }),
    details: schema.object({
        /** Authentication authority used by the login. */
        issuer: schema.httpUrl(),
        /** Last known session issued by the authentication authority. */
        sessionId: identifier("session").nullable(),
    }),
});

/** Record login sign-out under its verified request or execution identity. */
export const signOutLogin = defineAuditAction({
    package: daemonPackage,
    name: "login.sign-out",
    version: 1,
    targets: schema.object({
        login: schema.object({ type: schema.literal("login"), id: identifier("login") }),
    }),
    details: schema.object({
        /** Authentication authority used by the login. */
        issuer: schema.httpUrl(),
        /** Last known session issued by the authentication authority. */
        sessionId: identifier("session").nullable(),
    }),
});
