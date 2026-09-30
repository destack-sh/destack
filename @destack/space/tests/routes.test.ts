import { expect, test } from "@destack/test";
import type { Destination } from "@destack/host/router";
import type { InstanceSpec, WorkloadResource } from "@destack/host/runtime";
import type { BuildReader } from "@destack/package/manifest";
import { PackageId } from "@destack/package";
import { identifier } from "@destack/schema";
import * as object from "../src/object/index.ts";
import { SpaceRoutes } from "../src/server/index.ts";
import { ids, openSpace } from "./fixture/space.ts";

/** The package of a universe service. */
const ACCOUNT_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f1");

/** A space another cell serves. */
const acme = identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f3");

/** Another package than the notes installation's. */
const OTHER_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f2");

/** A service binding of the calling workload, with an address. */
const binding = (name: string, address: string, packageId: PackageId): WorkloadResource => ({
    id: identifier("resource").parse(`resource-01996ab0-0000-7000-8000-0000000000${name}`),
    scope: ids.space,
    kind: "service",
    spec: { service: { packageId, name: "service" } },
    reference: address,
    packageId: OTHER_PACKAGE,
    name,
    providerCode: "http",
});

test("resolve a workload's addresses: its cell's services, and only the installations and services it binds", async () => {
    // keep the notes installation in the space
    const database = await openSpace();
    const now = Date.now();
    await database.insert(object.installation.table).values({
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        id: ids.notes,
        packageId: ids.package,
        role: "application",
        selection: { kind: "release", version: "2026.9.0" },
        alias: "notes",
    });

    // route with the cell's audit service, serving board.alice here and nothing else
    const audit: Extract<Destination, { kind: "service" }> = {
        kind: "service",
        audience: OTHER_PACKAGE,
        fetch: async () => Response.json({}),
    };
    const routes = new SpaceRoutes(database, {
        services: new Map([["@destack/audit", audit]]),
        universe: "https://destack.cloud",
        locate: async (space) =>
            space === "board.alice"
                ? { id: ids.space, isServed: true }
                : space === "work.acme"
                  ? { id: acme, isServed: false }
                  : undefined,
        origin: (address) => `https://${address}.destack.space/.destack/service`,
    });
    const run: InstanceSpec = {
        instanceId: identifier("instance").parse("instance-01996ab0-0000-7000-8000-0000000000e1"),
        installationId: identifier("installation").parse(
            "installation-01996ab0-0000-7000-8000-0000000000e2",
        ),
        scope: ids.space,
        deploymentId: identifier("deployment").parse(
            "deployment-01996ab0-0000-7000-8000-0000000000e3",
        ),
        build: {} as BuildReader,
        output: "bun",
        workload: "main",
        resources: [
            binding("a1", "notes", ids.package),
            binding("a2", "notes.board.alice", ids.package),
            binding("a3", "notes.work.acme", ids.package),
            binding("a4", "@destack/account", ACCOUNT_PACKAGE),
            binding("a6", "notes.gone.nobody", ids.package),
        ],
    };
    const resolve = (address: string, resources = run.resources) =>
        routes.resolve({ ...run, resources }, address).then(
            (destination) => destination,
            (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
        );

    // reach the cell's service, installations of served spaces, other spaces' origins and the universe, and refuse the rest
    expect([
        await resolve("@destack/audit"),
        await resolve("notes"),
        await resolve("notes.board.alice"),
        await resolve("notes.work.acme"),
        await resolve("@destack/account"),
        await resolve("notes.gone.nobody"),
        await resolve("tasks"),
        await resolve("notes", [binding("a5", "notes", OTHER_PACKAGE)]),
    ]).toEqual([
        audit,
        {
            kind: "installation",
            scope: ids.space,
            installationId: ids.notes,
            audience: ids.package,
        },
        {
            kind: "installation",
            scope: ids.space,
            installationId: ids.notes,
            audience: ids.package,
        },
        {
            kind: "remote",
            scope: acme,
            url: "https://notes.work.acme.destack.space/.destack/service",
            audience: ids.package,
        },
        {
            kind: "remote",
            scope: ids.space,
            url: `https://destack.cloud/service/${ACCOUNT_PACKAGE}`,
            audience: ACCOUNT_PACKAGE,
        },
        "NOT_FOUND: no space answers at notes.gone.nobody",
        `NOT_FOUND: installation ${run.installationId} binds no tasks`,
        `NOT_FOUND: no installation notes of ${OTHER_PACKAGE} is served here`,
    ]);
});
