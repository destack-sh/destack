import { defineAuditAction } from "@destack/audit";
import { identifier, schema } from "@destack/schema";
import { daemonPackage } from "./action.ts";

/** Record preview start under its verified request or execution identity. */
export const startPreview = defineAuditAction({
    package: daemonPackage,
    name: "preview.start",
    version: 1,
    targets: schema.object({
        preview: schema.object({ type: schema.literal("preview"), id: identifier("preview") }),
    }),
    details: schema.object({
        /** Registered source checkout. */
        checkoutId: identifier("checkout"),
        /** Space authorizing the operation. */
        spaceId: identifier("space"),
        /** Selected retained universe login. */
        loginId: identifier("login"),
    }),
});

/** Record preview stop under its verified request or execution identity. */
export const stopPreview = defineAuditAction({
    package: daemonPackage,
    name: "preview.stop",
    version: 1,
    targets: schema.object({
        preview: schema.object({ type: schema.literal("preview"), id: identifier("preview") }),
    }),
    details: schema.object({
        /** Registered source checkout. */
        checkoutId: identifier("checkout"),
        /** Space authorizing the operation. */
        spaceId: identifier("space"),
        /** Selected retained universe login. */
        loginId: identifier("login"),
    }),
});
