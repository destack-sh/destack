import { DatabaseKind, type Select, type Snapshot } from "@destack/db";
import { MAIN_DATABASE } from "../declare/index.ts";
import { defineObject, type ObjectType, provisioned } from "@destack/object";
import { space } from "@destack/account/object";
import { sudo } from "@destack/account/authentication";
import { type Identifier, present } from "@destack/schema";
import { argument, plural, t } from "@destack/locale";

/** The provider that stores a space's main database in the space's own storage. */
export const SPACE_PROVIDER = "space";

/** A space's database resource. */
export const database = defineObject({
    name: "database",
    plural: "databases",
    title: t`${plural(argument("count"), { one: "Database", other: "Databases" })}`,
    scope: space,
    fields: {},
    access: {
        suspended: ["read", "manage"],
        elevated: { manage: sudo },
    },
    traits: [provisioned({ kind: DatabaseKind })],
});

/** A persisted database resource. */
export type Database = Select<typeof database.table>;

/** The databases of spaces. */
export const Database = {
    /** Report whether a resource of some type is a space's main database. */
    isMain(object: ObjectType, row: Readonly<Record<string, unknown>>): boolean {
        return object.same(database) && row["name"] === MAIN_DATABASE;
    },

    /** Read a space's main database, which the space has from its birth on. */
    async main(snapshot: Pick<Snapshot, "read">, spaceId: Identifier<"space">): Promise<Database> {
        return present(
            await Database.mainMaybe(snapshot, spaceId),
            `the main database of ${spaceId}`,
        );
    },

    /** Read a space's main database, absent where its row has not arrived yet. */
    async mainMaybe(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
    ): Promise<Database | undefined> {
        return snapshot
            .read(database.table)
            .findFirst({ where: { scope: spaceId, name: MAIN_DATABASE } });
    },
};
