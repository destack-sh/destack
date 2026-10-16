import { machine } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema, Version } from "@destack/schema";
import { CHANNELS } from "../release/index.ts";

/** What staging found: the installed release, and the release staged for activation. */
export const Staging = defineSchema(
    schema.object({
        /** The installed release's version. */
        installed: Version,
        /** The version of the release staged for activation, null while none newer is. */
        staged: Version.nullable(),
    }),
);
/** What staging found. */
export type Staging = schema.Infer<typeof Staging>;

/** This machine's Destack distribution: its channel, the release installed and the one staged. */
export const distribution = defineObject(
    {
        name: "distribution",
        plural: "distributions",
        scope: machine,
        fields: {
            /** The release channel the distribution follows. */
            channel: field.enum(CHANNELS),

            // observed
            /** The installed release's version. */
            installed: field.string(Version),
            /** The version of the release staged for activation, absent while none newer is. */
            staged: field.string(Version).optional(),
            /** Whether the release is current, a newer one is staged, or the restart runs. */
            status: field.enum(["current", "staged", "activating"]).default("current"),
            /** When the channel's repository was last checked. */
            checkedAt: field.time().optional(),
        },
        permissions: ["read", "update", "record"],
    },
    (method) => ({
        create: method.create("record", { isInternal: true }),
        /** Download and verify the channel's latest release if newer than the installed one. */
        stage: method.mutation({ permission: "update", prepared: Staging }),
        /** Restart the distribution into the staged release. */
        activate: method.mutation({ permission: "update", prepared: Version }),
    }),
);
/** A distribution as its table stores it. */
export type DistributionRow = Select<typeof distribution.table>;
