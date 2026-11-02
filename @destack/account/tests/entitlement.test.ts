import { expect, test } from "@destack/test";
import { Entitlement, type EntitlementTerms, type EntitlementUse } from "../src/object/index.ts";
import { Package, PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

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

test("keep a space's chosen retention where a plan grants custom retention, and cap it at the setting's default otherwise", () => {
    const custom: EntitlementTerms = {
        packageId: PRICING,
        feature: "customRetention",
        resource: "retention",
        value: null,
        limit: null,
        state: null,
    };
    // cap by a retention setting's whole definition, as event stores pass it
    const { definition } = defineSetting(
        {
            name: "telemetry.retention",
            title: "Telemetry retention",
            description: "How many days logs, traces and metrics stay.",
            schema: schema.number().int().min(1).max(3650),
            default: 30,
            scope: "space",
            overrides: [],
            apply: "immediate",
        },
        {
            package: Package.parse({
                id: PRICING,
                name: "@example/telemetry",
                version: "2026.9.0",
            }),
        },
    );
    expect([
        Entitlement.retention([custom], 400, definition),
        Entitlement.retention([custom], 7, definition),
        Entitlement.retention([metered("database", "within")], 400, definition),
        Entitlement.retention([metered("database", "within")], 7, definition),
    ]).toEqual([400, 7, 30, 7]);
});
