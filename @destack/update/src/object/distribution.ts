import type { Select } from "@destack/db";
import { contained, principal } from "@destack/access";
import { argument, plural, t } from "@destack/locale";
import { defineCommand, defineObject, device, field } from "@destack/object";
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
        title: t`${plural(argument("count"), { one: "Distribution", other: "Distributions" })}`,
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
        access: {
            // let the device its owner acts as read, stage and record it
            permissions: {
                read: contained(principal.device),
                update: contained(principal.device),
                record: contained(principal.device),
            },
        },
    },
    (method) => ({
        create: method.create("record", { visibility: "package" }),
        /** Download and verify the feed's latest release if newer than the installed one. */
        stage: method.mutation({ title: t`Stage`, permission: "update", prepared: Staging }),
        /** Restart the distribution into the staged release. */
        activate: method.mutation({ title: t`Activate`, permission: "update", prepared: Version }),
    }),
);

/** The command staging the newest release of a current distribution, when one is newer. */
export const checkForUpdates = defineCommand({
    name: "check-for-updates",
    title: t`Check for Updates…`,
    object: distribution,
    method: "stage",
    when: { status: "current" },
});

/** The command restarting a distribution into the release it staged. */
export const restartToUpdate = defineCommand({
    name: "restart-to-update",
    title: t`Restart to Update`,
    object: distribution,
    method: "activate",
    when: { status: "staged" },
});

/** A distribution as its table stores it. */
export type DistributionRow = Select<typeof distribution.table>;
