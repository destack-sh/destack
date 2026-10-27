import { expect, test } from "@destack/test";
import { Entitlement, type EntitlementTerms, type EntitlementUse } from "../src/object/index.ts";
import { PackageId } from "@destack/package";

/** The package declaring the platform's features. */
const PRICING = PackageId.parse("package-019f5530-8000-7000-8000-000000000503");

/** The uses a machine decides for a space: its four resources and two tenancies. */
const USES: readonly EntitlementUse[] = [
    "database",
    "storage",
    "compute",
    "network",
    { tenancy: "shared" },
    { tenancy: "dedicated" },
];

/** A metered entitlement to a resource with a limit of 1000, blocked at it or within it. */
function metered(
    resource: "database" | "storage" | "network",
    state: "blocked" | "within",
): EntitlementTerms {
    return { packageId: PRICING, feature: resource, resource, value: null, limit: 1000, state };
}

test("decide a use from an account's entitlements: unknown until replicated, blocked at its limit, unentitled without a grant, and by tenancy", () => {
    // entitle blocked database and network, storage within its limit, and the shared and local tenancies
    const rows = [
        metered("database", "blocked"),
        metered("storage", "within"),
        metered("network", "blocked"),
        {
            packageId: PRICING,
            feature: "tenancy",
            resource: "tenancy" as const,
            value: ["shared", "local"],
            limit: null,
            state: null,
        },
    ];

    // decide each use before the entitlements replicate, then from them
    expect({
        pending: USES.map((use) => Entitlement.decide(undefined, use).status),
        replicated: USES.map((use) => Entitlement.decide(rows, use)),
    }).toEqual({
        pending: ["unknown", "unknown", "unknown", "unknown", "unknown", "unknown"],
        replicated: [
            { status: "false", reason: "Blocked", message: "database is at its limit of 1000" },
            { status: "true", reason: "Within", message: "storage within the plan" },
            {
                status: "false",
                reason: "Unentitled",
                message: "no plan of the account grants compute",
            },
            { status: "false", reason: "Blocked", message: "network is at its limit of 1000" },
            { status: "true", reason: "Within", message: "shared tenancy within the plan" },
            {
                status: "false",
                reason: "Unentitled",
                message: "no plan of the account grants dedicated tenancy",
            },
        ],
    });
});

test("keep a space's chosen retention within its plan's days, and as chosen where no plan sets them", () => {
    const plan = (days: number): EntitlementTerms => ({
        packageId: PRICING,
        feature: "retention",
        resource: "retention",
        value: days,
        limit: null,
        state: null,
    });
    expect([
        Entitlement.retention([plan(7)], 30),
        Entitlement.retention([plan(365)], 30),
        Entitlement.retention([metered("database", "within")], 30),
    ]).toEqual([7, 30, 30]);
});
