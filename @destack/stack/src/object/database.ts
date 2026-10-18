import { DatabaseKind, type Select, type Snapshot, type Transaction } from "@destack/db";
import { defineObject } from "@destack/object";
import { space } from "@destack/account/object";
import { sudo } from "@destack/account/authentication";
import { Digest, type Identifier, present } from "@destack/schema";
import { RESOURCE_DECLARATION } from "@destack/resource";
import { OBJECT_DECLARATION } from "@destack/object/inspect";
import { binding } from "./binding.ts";
import { Build, build } from "./build.ts";
import {
    type Installation,
    installation,
    installationRevision,
    type InstallationRevision,
} from "./installation.ts";
import type { graph } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";

/** The name of every space's main database, which keeps every installation's objects. */
export const MAIN_DATABASE = "main";

/** The provider keeping a space's main database in the space's own storage. */
export const SPACE_PROVIDER = "space";

/** A space's database resource. */
export const database = defineObject({
    name: "database",
    plural: "databases",
    scope: space,
    provisioned: { kind: DatabaseKind },
    fields: {},
    administration: ["read", "manage"],
    elevated: { manage: sudo },
});

/** A persisted database resource. */
export type Database = Select<typeof database.table>;

/** The databases of spaces. */
export const Database = {
    /** Read a space's main database, which the space has from its birth on. */
    async main(snapshot: Pick<Snapshot, "read">, spaceId: Identifier<"space">): Promise<Database> {
        const [row] = await snapshot
            .read(database.table)
            .findMany({ where: { scope: spaceId, name: MAIN_DATABASE } });

        return present(row, `the main database of ${spaceId}`);
    },

    /** Point a space's main database at where a machine keeps it. */
    async place(
        transaction: Transaction,
        main: Database,
        reference: string,
        machineId: Identifier<"machine">,
    ): Promise<void> {
        await transaction.write({
            table: database.table,
            before: main,
            after: { ...main, reference, machineId },
        });
    },

    /** Read the revision a space serves each installation at, with its declarations. */
    async installed(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        open: (manifest: string) => Promise<Pick<BuildReader, "declarations">>,
    ): Promise<
        {
            readonly installation: Installation;
            readonly revisionId: InstallationRevision["id"];
            readonly declarations: graph.Declaration[];
        }[]
    > {
        // read no installations of a space before its birth gives it main
        const [main] = await snapshot
            .read(database.table)
            .findMany({ where: { scope: spaceId, name: MAIN_DATABASE } });
        if (main === undefined) {
            return [];
        }

        // read the installations, their bindings, and their revisions' builds
        const installations = await snapshot
            .read(installation.table)
            .findMany({ where: { scope: spaceId } });
        const bindings = await snapshot.read(binding.table).findMany({ where: { scope: spaceId } });
        const revisionIds = installations.flatMap((entry) =>
            [entry.revisionId, entry.appliedRevisionId].filter((id) => id !== null),
        );
        const revisions = await snapshot
            .read(installationRevision.table)
            .findMany({ where: { scope: spaceId, id: { in: revisionIds } } });
        const builds = await snapshot.read(build.table).findMany({
            where: { scope: spaceId, id: { in: revisions.map((entry) => entry.buildId) } },
        });

        // read each revision's declarations from its build, once
        const declared = new Map(
            await Promise.all(
                revisions.map(async (revision) => {
                    const built = present(
                        builds.find((each) => each.id === revision.buildId),
                        `the build of revision ${revision.id}`,
                    );
                    const reader = await open(Build.manifest(built));

                    return [revision.id, await reader.declarations()] as const;
                }),
            ),
        );

        // serve each installation at its revision once main applied its tables, else at the revision it deployed last
        const applied = new Set(main.appliedStates);
        const served = await Promise.all(
            installations.map(async (entry) => {
                // serve a revision declaring no object types at once, and one declaring some once main applied them
                const latest =
                    entry.revisionId === null ? undefined : declared.get(entry.revisionId);
                const tables = bindings.find(
                    (each) =>
                        each.installationId === entry.id &&
                        each.name === MAIN_DATABASE &&
                        each.target === main.id,
                );
                const isApplied =
                    latest !== undefined &&
                    (!latest.some((each) => each.kind === OBJECT_DECLARATION) ||
                        (tables !== undefined && applied.has(await Digest.json(tables.state))));
                const revisionId = isApplied ? entry.revisionId : entry.appliedRevisionId;

                return revisionId === null ? [] : [{ installation: entry, revisionId }];
            }),
        );

        // keep each one whose own resources are each bound, as servers open them at start and read secrets on use
        const bound = new Set(
            bindings.map((entry) => `${entry.installationId}:${entry.packageId}:${entry.name}`),
        );

        return served.flat().flatMap(({ installation: entry, revisionId }) => {
            const declarations = present(declared.get(revisionId), `revision ${revisionId}`);
            const isBound = declarations
                .filter((each) => each.kind === RESOURCE_DECLARATION)
                .every((each) => bound.has(`${entry.id}:${entry.packageId}:${each.name}`));

            return isBound ? [{ installation: entry, revisionId, declarations }] : [];
        });
    },
};
