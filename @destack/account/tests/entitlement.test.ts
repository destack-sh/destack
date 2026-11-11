import { expect, test } from "@destack/test";
import { Snapshot } from "@destack/db";
import { TestAccountStore } from "../src/test/account.ts";
import {
    Entitlement,
    entitlement,
    type Allowance,
    type EntitlementUse,
} from "../src/object/index.ts";
import { Package, PackageId } from "@destack/package";
import { present, schema } from "@destack/schema";
import { AccountError } from "../src/error/error.ts";
import { defineSetting } from "@destack/setting/declare";

/** The package declaring the platform's features. */
const PRICING = PackageId.parse("package-019f5530-8000-7000-8000-000000000503");

/** The uses a machine decides for a space: its four resources and two tenancies. */
const USES: readonly EntitlementUse[] = [
    "database",
    "bucket",
    "compute",
    "network",
    { tenancy: "shared" },
    { tenancy: "dedicated" },
];

/** A metered entitlement to a resource with a limit of 1000, blocked at it or within it. */
function metered(
    resource: "database" | "bucket" | "network",
    state: "blocked" | "within",
): Allowance {
    return {
        packageId: PRICING,
        feature: resource,
        kind: "metered",
        resource,
        value: null,
        limit: 1000,
        resetAt: null,
        state,
    };
}

test("decide a use from an account's entitlements: unknown until replicated, blocked at its limit, unentitled without a grant, and by tenancy", () => {
    // entitle blocked database and network, bucket within its limit, and the shared and local tenancies
    const rows: Allowance[] = [
        metered("database", "blocked"),
        metered("bucket", "within"),
        metered("network", "blocked"),
        {
            packageId: PRICING,
            feature: "tenancy",
            kind: "static",
            resource: "tenancy",
            value: ["shared", "local"],
            limit: null,
            resetAt: null,
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
            { status: "true", reason: "Within", message: "bucket within the plan" },
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
    const custom: Allowance = {
        packageId: PRICING,
        feature: "customRetention",
        kind: "boolean",
        resource: "retention",
        value: null,
        limit: null,
        resetAt: null,
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

test("entitle an account to an allowance once, leaving it unchanged when entitled again, changing its kind in place, and refusing a repeated feature", async () => {
    // entitle an account to custom domains and a database limit, as finance does
    await using accounts = await TestAccountStore.open();
    const owner = await accounts.signIn("owner@example.com");
    const { id: accountId } = await accounts.createAccount(owner);
    const stored = () =>
        Snapshot.live(accounts.database)
            .read(entitlement.table)
            .findMany({ where: { scope: accountId }, orderBy: { feature: "asc" } });
    await accounts.entitle(accountId, "domain", { kind: "boolean" });
    await accounts.entitle(accountId, "database", {
        kind: "metered",
        limit: 1000,
        state: "within",
    });
    const first = await stored();

    // entitle it to both again, which writes nothing, then raise the limit, which updates the row in place
    await accounts.entitle(accountId, "domain", { kind: "boolean" });
    const again = await stored();
    await accounts.entitle(accountId, "database", {
        kind: "metered",
        limit: 2000,
        state: "within",
    });
    const raised = await stored();

    // make the database feature unlimited, which changes its kind in place
    await accounts.entitle(accountId, "database", { kind: "boolean" });
    const unlimited = await stored();

    // refuse allowances repeating a feature
    const domain: Allowance = {
        packageId: present(first[1], "the domain entitlement").packageId,
        feature: "domain",
        kind: "boolean",
        value: null,
        limit: null,
        resetAt: null,
        state: null,
        resource: "domain",
    };
    const repeated = await accounts
        .allow(accountId, [domain, domain])
        .catch((error: unknown) =>
            AccountError.is(error) ? [error.reason, error.message] : error,
        );
    expect({
        features: first.map((row) => row.feature),
        again,
        limits: raised.map((row) => [row.id, row.limit]),
        kinds: unlimited.map((row) => [row.id, row.kind, row.limit, row.state]),
        repeated,
    }).toEqual({
        features: ["database", "domain"],
        again: first,
        limits: [
            [first[0]?.id, 2000],
            [first[1]?.id, null],
        ],
        kinds: [
            [first[0]?.id, "boolean", null, null],
            [first[1]?.id, "boolean", null, null],
        ],
        repeated: [
            "FEATURE_INVALID",
            "allowance is invalid: the account's allowances repeat a feature",
        ],
    });
});
