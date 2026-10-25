import { none } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import { user } from "./user.ts";

/** A device's sign-in a person approves in a browser by its user code (RFC 8628). */
export const deviceAuthorization = defineObject(
    {
        name: "device-authorization",
        plural: "deviceAuthorizations",
        scope: "universe",
        expiring: [{ after: { milliseconds: 0 }, from: "expiresAt" }],
        fields: {
            /** The SHA-256 digest of the device code the client polls with. */
            deviceDigest: field.string(Digest).sensitive("deterministic"),
            /** The SHA-256 digest of the user code the person enters. */
            userDigest: field.string(Digest).sensitive("deterministic"),
            /** The client asking: an application's public client identifier or a device's. */
            clientId: field.string(),
            /** The asked OAuth scopes. */
            scopes: field.json(schema.array(schema.string())),
            /** The package the tokens are for, as its resource (RFC 8707). */
            resource: field.string().optional(),
            /** The least wait between polls, in milliseconds. */
            interval: field.integer(),
            /** The last poll. */
            polledAt: field.time().optional(),
            /** Whether the person approved, denied or has yet to decide, decided once. */
            status: field.state({
                initial: "pending",
                transitions: {
                    approve: {
                        from: ["pending"],
                        to: "approved",
                        permission: "decide",
                        fields: ["userId"],
                        isInternal: true,
                    },
                    deny: {
                        from: ["pending"],
                        to: "denied",
                        permission: "decide",
                        isInternal: true,
                    },
                },
            }),
            /** The approving user, set exactly when approved. */
            user: field.reference(user, { delete: "cascade" }).optional(),
            /** When the codes lapse. */
            expiresAt: field.time(),
        },
        permissions: { issue: none(), poll: none(), decide: none(), redeem: none() },
        constraints: {
            deviceDigest: { kind: "unique", on: ["deviceDigest"] },
            userDigest: { kind: "unique", on: ["userDigest"] },
            expiry: { kind: "index", on: ["expiresAt"] },
            decision: {
                kind: "check",
                condition: {
                    OR: [
                        { status: "approved", userId: { isNotNull: true } },
                        { status: { ne: "approved" }, userId: { isNull: true } },
                    ],
                },
            },
        },
    },
    (method) => ({
        /** Issue a device's codes. */
        create: method.create("issue", {
            fields: [
                "deviceDigest",
                "userDigest",
                "clientId",
                "scopes",
                "resource",
                "interval",
                "expiresAt",
            ],
            isInternal: true,
            isPredicted: false,
        }),
        /** Record a poll, with the wait before the next poll. */
        poll: method.update("poll", {
            fields: ["polledAt", "interval"],
            isInternal: true,
            isPredicted: false,
        }),
        /** Remove the codes once the client takes its tokens. */
        delete: method.delete("redeem", { isInternal: true }),
    }),
);
/** A stored device authorization. */
export type DeviceAuthorization = Select<typeof deviceAuthorization.table>;

/** A person's decision on a device's sign-in: pending, approved by a user, or denied. */
export type DeviceDecision =
    | { readonly status: "pending" }
    | { readonly status: "approved"; readonly userId: Identifier<"user"> }
    | { readonly status: "denied" };

/** The device sign-ins people decide on by their user codes (RFC 8628). */
export const DeviceAuthorization = {
    /** Read a stored sign-in's decision. */
    decision(row: Pick<DeviceAuthorization, "status" | "userId">): DeviceDecision {
        // read an approval with its user
        if (row.status === "approved") {
            return { status: "approved", userId: present(row.userId, "the approving user") };
        }

        return { status: row.status };
    },
};
