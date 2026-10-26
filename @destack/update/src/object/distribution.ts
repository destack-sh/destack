import type { Select } from "@destack/db";
import { contained, principal } from "@destack/access";
import { defineObject, device, field } from "@destack/object";
import { schema, Version } from "@destack/schema";

/** What staging found: the installed release, and the release staged for activation. */
export const Staging = schema.object({
    /** The installed release's version. */
    installed: Version,
    /** The version of the release staged for activation, null while none newer is. */
    staged: Version.nullable(),
});
/** What staging found. */
export type Staging = schema.Infer<typeof Staging>;

/** This computer's Destack distribution: its installed release and the one staged. */
export const distribution = defineObject(
    {
        name: "distribution",
        plural: "distributions",
        scope: device,
        fields: {
            // observed
            /** The installed release's version. */
            installed: field.string(Version),
            /** The version of the release staged for activation, absent while none newer is. */
            staged: field.string(Version).optional(),
            /** Whether the release is current, a newer one is staged, or the restart runs. */
            status: field.enum(["current", "staged", "activating"]).default("current"),
            /** When the feed was last checked. */
            checkedAt: field.time().optional(),
        },
        // let the device its owner acts as read, stage and record it
        permissions: {
            read: contained(principal.device),
            update: contained(principal.device),
            record: contained(principal.device),
        },
    },
    (method) => ({
        create: method.create("record", { visibility: "package" }),
        /** Download and verify the feed's latest release if newer than the installed one. */
        stage: method.mutation({ permission: "update", prepared: Staging }),
        /** Restart the distribution into the staged release. */
        activate: method.mutation({ permission: "update", prepared: Version }),
    }),
);
/** A distribution as its table stores it. */
export type DistributionRow = Select<typeof distribution.table>;
