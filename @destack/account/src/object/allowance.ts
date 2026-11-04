import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";

/** The platform resources a feature allows, which machines and spaces enforce. */
export const ENTITLEMENT_RESOURCES = [
    "database",
    "storage",
    "compute",
    "network",
    "tenancy",
    "domain",
    "retention",
] as const;

/** A platform resource a feature allows. */
export type EntitlementResource = (typeof ENTITLEMENT_RESOURCES)[number];

/** What a feature grants: access alone, a fixed value, or usage of meters up to a limit. */
export const FEATURE_KINDS = ["boolean", "static", "metered"] as const;

/** Where a metered usage stands against its limit and its subscription's budget. */
export const USAGE_STATES = ["within", "near", "over", "blocked"] as const;

/** Where a metered usage stands. */
export type UsageState = (typeof USAGE_STATES)[number];

/** What an account may use of one feature, as its entitlement keeps it. */
export const Allowance = schema.object({
    /** The package declaring the feature. */
    packageId: PackageId,
    /** The feature's name in its package. */
    feature: schema.string().min(1),
    /** What the feature grants. */
    kind: schema.enum(FEATURE_KINDS),
    /** The value of a static feature. */
    value: schema.json().nullable(),
    /** The usage limit of a metered feature, unlimited without one. */
    limit: schema.number().int().nullable(),
    /** When a metered feature's usage resets, absent for usage that never resets. */
    resetAt: schema.number().int().nullable(),
    /** Where the usage of a metered feature stands against its limit and budget. */
    state: schema.enum(USAGE_STATES).nullable(),
    /** The platform resource the feature allows, absent for a package's feature. */
    resource: schema.enum(ENTITLEMENT_RESOURCES).nullable(),
});
/** What an account may use of one feature. */
export type Allowance = schema.Infer<typeof Allowance>;
