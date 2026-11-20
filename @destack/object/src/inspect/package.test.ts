import { testIdentity } from "../test/index.ts";
import { none, Policy } from "@destack/access";
import { defineErrorFamily } from "@destack/error";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { defineMount, definePackage, type ObjectDeclaration } from "../declare/package.ts";
import type { ProcedureRouter } from "../procedure/index.ts";
import { comparePackage, describePackage } from "./package.ts";
import { t } from "@destack/locale";

/** The release under comparison. */
const RELEASE = "2026.9.0";

/** The package whose releases the entries declare. */
const PACKAGE = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-00000000c001"),
    name: "@destack/compare-fixture",
};

/** A release's manifest entry of a declaration, its description read back from JSON. */
const entry = (
    handle: ReturnType<typeof definePackage>,
    version: string,
    capabilities: readonly string[] = [],
) => ({
    description: schema.jsonObject().parse(JSON.parse(JSON.stringify(describePackage(handle)))),
    symbol: {
        package: { ...PACKAGE, version },
        symbol: { module: "src/index.ts", name: "declared" },
    },
    capabilities,
});

/** The module declaring the package's releases. */
const MODULE = { package: { ...PACKAGE, version: RELEASE } };

/** A query type of the package routing a set of procedures. */
const query = (procedures: ProcedureRouter): ObjectDeclaration => ({
    package: PACKAGE,
    name: "query",
    typeReference: { packageId: PACKAGE.id, type: "query" },
    hooks: {},
    procedures,
    requireValid: () => {},
});

/** Define a release of the package with some mounts. */
function releaseMounts(mounts: Readonly<Record<string, `/${string}`>>) {
    return definePackage(
        {
            ...testIdentity,
            mounts: Object.entries(mounts).map(([name, path]) =>
                defineMount({ name, path }, MODULE),
            ),
        },
        MODULE,
    );
}

test("plan a package's mount changes: adding is safe, removing or moving breaks callers", () => {
    // define two releases of the package's mounts
    const before = releaseMounts({ feed: "/feed", npm: "/npm", retired: "/retired" });
    const after = releaseMounts({ feed: "/feed", npm: "/registry", archive: "/archive" });

    // plan the added, moved and removed mounts
    expect(comparePackage(entry(before, "2026.8.0"), entry(after, RELEASE)).steps).toEqual([
        {
            action: "create",
            target: "mount/archive",
            risk: "safe",
            detail: "add mount archive",
        },
        {
            action: "update",
            target: "mount/npm",
            risk: "backward-incompatible",
            detail: "move mount npm from /npm to /registry",
        },
        {
            action: "delete",
            target: "mount/retired",
            risk: "backward-incompatible",
            detail: "remove mount retired",
        },
    ]);
});

test("plan granting what a release asks of the space beyond the earlier one, with the capabilities the installation enables", () => {
    // ask to read notes always, and to write them with the optional export capability
    const note = new Policy(MODULE.package, {
        name: "note",
        permissions: { read: none(), write: none() },
    });
    const earlier = definePackage(
        {
            ...testIdentity,
            access: { permissions: { space: { space: [note.permission("read")] } } },
        },
        MODULE,
    );
    const later = definePackage(
        {
            ...testIdentity,
            access: { permissions: { space: { space: [note.permission("read")] } } },
            capabilities: [
                {
                    name: "export",
                    reason: t`Export notes`,
                    optional: true,
                    permissions: { space: { space: [note.permission("write")] } },
                },
            ],
        },
        MODULE,
    );

    // grant the write once the installation enables export, and nothing while it does not
    expect({
        enabled: comparePackage(entry(earlier, "2026.8.0"), entry(later, RELEASE, ["export"]))
            .steps,
        disabled: comparePackage(entry(earlier, "2026.8.0"), entry(later, RELEASE)).steps,
    }).toEqual({
        enabled: [
            {
                action: "grant",
                target: `access/space/${PACKAGE.id}/note/write`,
                risk: "privileged",
                detail: "request write on note",
            },
        ],
        disabled: [],
    });
});

test("describe a package's routes, mounts, views, requested permissions, capabilities and errors, with its types' hooks", () => {
    // route the package's query with hooks
    const custom = { ...query({}), hooks: { create: ["authorize"] } };
    const user = new Policy(MODULE.package, { name: "user", permissions: { recipient: none() } });
    const handle = definePackage(
        {
            ...testIdentity,
            objects: [custom],
            mounts: [defineMount({ name: "npm", path: "/npm" }, MODULE)],
            access: { permissions: { space: { universe: [user.permission("recipient")] } } },
            capabilities: [
                {
                    name: "push",
                    reason: t`Drop the browsers push services forgot`,
                    optional: true,
                    permissions: { user: { home: [user.permission("recipient")] } },
                    features: [{ packageId: MODULE.package.id, name: "push" }],
                },
            ],
            errors: defineErrorFamily(
                {
                    reasons: {
                        QUERY_DENIED: {},
                    },
                },
                { package: MODULE.package },
            ),
        },
        MODULE,
    );

    expect(describePackage(handle)).toEqual({
        ...testIdentity,
        objects: {
            query: { type: custom.typeReference, hooks: { create: ["authorize"] } },
        },
        mounts: { npm: { path: "/npm" } },
        protocols: [],
        views: [],
        permissions: {
            space: {
                universe: [{ packageId: MODULE.package.id, type: "user", name: "recipient" }],
            },
        },
        capabilities: {
            push: {
                reason: t`Drop the browsers push services forgot`,
                optional: true,
                permissions: {
                    user: {
                        home: [{ packageId: MODULE.package.id, type: "user", name: "recipient" }],
                    },
                },
                features: [{ packageId: MODULE.package.id, name: "push" }],
            },
        },
        errors: [{ domain: MODULE.package.name, reason: "QUERY_DENIED", code: "FORBIDDEN" }],
    });
});

test("refuse a package listing an object type another package defines, which that package alone serves", () => {
    const foreign: ObjectDeclaration = {
        ...query({}),
        package: { name: "@destack/board" },
        name: "card",
        typeReference: { packageId: "package-01996ab0-0000-7000-8000-00000000c002", type: "card" },
    };

    expect(() =>
        definePackage(
            {
                ...testIdentity,
                objects: [foreign],
            },
            MODULE,
        ),
    ).toThrow(
        `object 'card' is invalid: '${PACKAGE.name}' lists it, but only '@destack/board' can serve it`,
    );
});
