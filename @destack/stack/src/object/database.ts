import { DatabaseKind, type Select, type Snapshot, type Transaction } from "@destack/db";
import { defineObject } from "@destack/object";
import { space } from "@destack/account/object";
import { sudo } from "@destack/account/authentication";
import { aligned, Digest, type Identifier, present } from "@destack/schema";
import { binding } from "./binding.ts";
import { type Declaration, declaration } from "./declaration.ts";
import { type Installation, installation } from "./installation.ts";

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
}).methods();

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

    /** Read the installations whose tables a space's main database applied. */
    async installed(
        snapshot: Snapshot,
        spaceId: Identifier<"space">,
    ): Promise<{ readonly installation: Installation; readonly declarations: Declaration[] }[]> {
        // read no installations of a space before its birth gives it main
        const [main] = await snapshot
            .read(database.table)
            .findMany({ where: { scope: spaceId, name: MAIN_DATABASE } });
        if (main === undefined) {
            return [];
        }

        // read the digests of the states main applied, none while a plan waits
        const desired = [...main.states, ...main.drainingStates];
        const isApplied = main.appliedState === (await Digest.json(desired));
        const applied = new Set(isApplied ? desired : []);

        // read the installations whose main binding asks for an applied state
        const bindings = await snapshot.read(binding.table).findMany({
            where: {
                scope: spaceId,
                name: MAIN_DATABASE,
                target: main.id,
            },
        });
        const digests = await Promise.all(bindings.map((entry) => Digest.json(entry.state)));
        const served = bindings.filter((_entry, index) => applied.has(aligned(digests, index)));
        const installations = await snapshot.read(installation.table).findMany({
            where: {
                scope: spaceId,
                id: { in: served.map((entry) => entry.installationId) },
            },
        });

        // read each installation's revision's declarations
        const revisions = installations.flatMap((entry) =>
            entry.revisionId === null ? [] : [entry.revisionId],
        );
        const declared = await snapshot.read(declaration.table).findMany({
            where: {
                scope: spaceId,
                revisionId: { in: revisions },
            },
        });

        return installations.map((entry) => ({
            installation: entry,
            declarations: declared.filter((each) => each.revisionId === entry.revisionId),
        }));
    },
};
