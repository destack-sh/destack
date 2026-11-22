import { TABLE, defineTable, identifier, index, text } from "@destack/db";
import type { RowChange, Snapshot, Transaction } from "@destack/db/database";
import { schema } from "@destack/schema";
import type { ObjectType } from "./object.ts";
import type { ObjectStore } from "../server/store.ts";

/** The keys objects held before a rename, each resolving to its object until another takes it. */
export const formerKey = defineTable(
    "former_key",
    {
        /** The scope the key was unique within. */
        scope: text("scope").primaryKey(),
        /** The package declaring the keyed type. */
        packageId: identifier("package_id", "package").primaryKey(),
        /** The keyed type. */
        type: text("type").primaryKey(),
        /** The former key. */
        key: text("key").primaryKey(),
        /** The object that held it. */
        objectId: text("object_id").notNull(),
    },
    {
        constraints: (columns) => [index("former_key_object").on(columns.objectId)],
        log: {},
    },
);

/** The keys objects held before a rename, as the store records and resolves them. */
export const FormerKey = {
    /** Enlist a store's written rows in their transaction's commit, to keep their former keys. */
    enlist(transaction: Transaction, store: Pick<ObjectStore, "objects" | "principal">): void {
        transaction.enlist(`destack.object.former:${store.principal.id}`, {
            prepare: () => FormerKey.written(transaction, store.objects),
            confirm: async () => {},
            cancel: async () => {},
        });
    },

    /** Follow every row of a keyed type a transaction wrote, before it commits. */
    async written(transaction: Transaction, types: readonly ObjectType[]): Promise<void> {
        // take each written row of a type keeping former keys, once all calls wrote it
        const keyed = new Map(
            types
                .filter((type) => type.keyRenamed === "former")
                .map((type) => [type.table[TABLE].sqlName, type]),
        );
        const written = transaction.changes.changes.flatMap((change) => {
            const type = keyed.get(change.table[TABLE].sqlName);

            return type === undefined ? [] : [{ type, change }];
        });

        // follow each of them
        for (const { type, change } of written) {
            await FormerKey.follow(transaction, type, change);
        }
    },

    /** Follow a written row of a keyed type: keep a renamed key, release a taken or removed one. */
    async follow(transaction: Transaction, type: ObjectType, change: RowChange): Promise<void> {
        // follow only keyed types
        const column = type.keyed;
        if (column === undefined) {
            return;
        }
        const { before, after } = change;
        const row = after ?? before;
        if (row === null) {
            return;
        }
        const owner = { scope: schema.string().parse(row["scope"]), ...FormerKey.of(type) };
        const objectId = schema.string().parse(row["id"]);
        const taken = after?.[column];
        const held = before?.[column];

        // free the key a written object holds now, from any object holding it before
        if (typeof taken === "string") {
            await FormerKey.release(transaction, { ...owner, key: taken });
        }

        // keep a renamed object's old key, and drop every key a removed object held before
        if (after !== null && typeof held === "string" && held !== taken) {
            await transaction.write({
                table: formerKey,
                before: null,
                after: { ...owner, key: held, objectId },
            });
        } else if (after === null) {
            await FormerKey.release(transaction, { ...owner, objectId });
        }
    },

    /** Find the object that held a key before a rename, none for a key nobody held. */
    async holder(
        view: Pick<Snapshot, "read">,
        type: ObjectType,
        scope: string,
        key: string,
    ): Promise<string | undefined> {
        const found = await view
            .read(formerKey)
            .findFirst({ where: { scope, ...FormerKey.of(type), key } });

        return found?.objectId;
    },

    /** Read a type's columns in the table. */
    of(type: ObjectType): {
        readonly packageId: ObjectType["package"]["id"];
        readonly type: string;
    } {
        return { packageId: type.package.id, type: type.name };
    },

    /** Delete the former keys a condition selects, at the rows the transaction reads. */
    async release(
        transaction: Transaction,
        where: Readonly<Record<string, string>>,
    ): Promise<void> {
        const rows = await transaction.view().read(formerKey).findMany({ where });
        if (rows.length > 0) {
            await transaction.write(
                ...rows.map((before) => ({ table: formerKey, before, after: null })),
            );
        }
    },
};
