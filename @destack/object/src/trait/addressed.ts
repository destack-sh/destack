import { keySubject, type Subject } from "@destack/access";
import { accessRelationship, accessRole } from "@destack/access";
import {
    and,
    type Column,
    type ColumnBuilder,
    type DatabaseConnection,
    decodeRow,
    defineTable,
    encodeRow,
    eq,
    integer,
    isNull,
    primaryKey,
    type Table,
    text,
    uniqueIndex,
} from "@destack/db";
import { Snapshot } from "@destack/db/log";
import { defineSchema, schema } from "@destack/schema";
import type { Controller } from "@destack/service/control";
import { type Address, Outbox } from "@destack/service/outbox";
import type { ObjectType } from "../object/object.ts";
import type { ObjectServer } from "../server/server.ts";
import { Authorization } from "../server/authorization.ts";
import type { Trait } from "./trait.ts";
import { v7 } from "uuid";

/** The key under which access changes reconsider every row of an addressed type. */
const ACCESS_KEY = "*";

/** The options of addressed objects. */
export interface AddressedDefinition {
    /** The subject field holding the recipient. */
    readonly recipient: string;
}

/** The columns naming the source row on a copy. */
export type AddressedBuilderMap<Addressed> = Addressed extends AddressedDefinition
    ? { origin: ColumnBuilder<string, false, false>; originId: ColumnBuilder<string, false, false> }
    : {};

/** The revision of each addressed row its recipient's home holds. */
export const copy = defineTable(
    "copy",
    {
        /** The addressed object type, by name. */
        type: text("type").notNull(),
        /** The addressed row. */
        id: text("id").notNull(),
        /** The space holding the addressed row. */
        space: text("space").notNull(),
        /** The recipient's subject key. */
        recipient: text("recipient").notNull(),
        /** The revision sent to the recipient's home, null once withdrawn. */
        revision: integer("revision"),
    },
    { constraints: (held) => [primaryKey({ name: "copy_key", columns: [held.type, held.id] })] },
);

/** One change of an addressed row for its recipient's home. */
export const Copy = defineSchema(
    schema.object({
        /** The addressed object type, by name. */
        type: schema.string().min(1),
        /** The space holding the row. */
        space: schema.string().min(1),
        /** The row's identifier. */
        id: schema.string().min(1),
        /** The recipient's subject key. */
        recipient: schema.string().min(1),
        /** The row as its columns encode it, null to withdraw the copy. */
        row: schema.record(schema.string(), schema.json()).nullable(),
    }),
);
/** One change of an addressed row for its recipient's home. */
export type Copy = schema.Infer<typeof Copy>;

/** The outbox address of recipients' homes. */
export const INBOX: Address<Copy> = { name: "inbox", message: Copy };

/** The parts of an object server keeping addressed rows' copies. */
type Addressing = Pick<ObjectServer, "objects" | "database" | "authorizer">;

/** Objects copied into their recipient's home while the recipient may read them. */
export const addressed: Trait<AddressedDefinition> & {
    /** Send addressed rows' changes to their recipients' homes through the outbox. */
    controller(server: Addressing): Controller;
    /** Write copies a home receives, each into the home its recipient names. */
    accept(
        server: Pick<ObjectServer, "objects" | "database">,
        copies: readonly Copy[],
        home: (recipient: Subject) => Promise<string>,
    ): Promise<void>;
} = {
    key: "addressed",
    isDurable: true,
    options: (definition) => definition.addressed,
    columns: () => ({
        /** The space holding the source row, null on the source row. */
        origin: text("origin"),
        /** The source row's identifier, null on the source row. */
        originId: text("origin_id"),
    }),
    constraints: (_options, table, columns) => [
        uniqueIndex(`${table}_origin`).on(columns.origin!, columns.originId!),
    ],
    policy: () => ({ attributes: { origin: "string" } }),
    methods: () => ({}),
    validate: (options, object) => {
        // require a subject field naming the recipient
        if (object.fields[options.recipient]?.type !== "subject") {
            throw new TypeError(
                `object ${object.name} addresses no subject field ${options.recipient}`,
            );
        }
    },
    controller(server) {
        const types = server.objects.filter((object) => object.addressed !== undefined);
        const outbox = new Outbox(server.database);

        return {
            name: "addressed",
            watches: [
                ...types.map((object) => object.table as Table),
                accessRelationship,
                accessRole,
            ] as Table[],
            keys: (change) => {
                // key a changed source row, or every row after an access change
                const object = types.find((type) => type.table === change.table);
                const row = (change.after ?? change.before) as Record<string, unknown>;

                return object === undefined
                    ? types.map((type) => `${type.name} ${ACCESS_KEY}`)
                    : row.origin === null || row.origin === undefined
                      ? [`${object.name} ${String(row.id)}`]
                      : [];
            },
            list: async () => types.map((type) => `${type.name} ${ACCESS_KEY}`),
            reconcile: async (key) => {
                // reconsider one row, or every source row of the type
                const [name, id] = key.split(" ") as [string, string];
                const object = types.find((type) => type.name === name)!;
                const table = object.table as Table & Record<string, Column>;
                const ids =
                    id === ACCESS_KEY
                        ? (
                              await server.database
                                  .select({ id: table.id! })
                                  .from(table)
                                  .where(isNull(table.origin!))
                          ).map((row) => String(row.id))
                        : [id];
                const untils: number[] = [];
                for (const each of ids) {
                    const until = await send(server, outbox, object, each);
                    if (until !== undefined) {
                        untils.push(until);
                    }
                }

                // look again when time alone next changes a recipient's access
                const next = untils.length === 0 ? undefined : Math.min(...untils);

                return next === undefined ? undefined : Math.max(0, next - Date.now());
            },
        };
    },
    async accept(server, copies, home) {
        await server.database.transaction(async (transaction) => {
            for (const entry of copies) {
                // find the type and the recipient's home
                const object = server.objects.find((type) => type.name === entry.type);
                if (object?.addressed === undefined) {
                    throw new TypeError(`object ${entry.type} is not addressed here`);
                }
                const table = object.table as Table & Record<string, Column>;
                const scope = await home(keySubject(entry.recipient));

                // withdraw a copy
                const source = and(eq(table.origin!, entry.space), eq(table.originId!, entry.id));
                if (entry.row === null) {
                    await transaction.delete(table).where(source);
                    continue;
                }

                // upsert the copy in the home
                const [held] = await transaction
                    .select({ id: table.id! })
                    .from(table)
                    .where(source);
                const { id: _id, ...row } = decodeRow(table, entry.row);
                const values = { ...row, scope, origin: entry.space, originId: entry.id };
                if (held === undefined) {
                    await transaction
                        .insert(table)
                        .values({ ...values, id: `${object.identity}-${v7()}` } as never);
                } else {
                    await transaction
                        .update(table)
                        .set(values as never)
                        .where(eq(table.id!, held.id));
                }
            }
        });
    },
};

/** Send or withdraw one row's copy, returning when time next changes the recipient's access. */
async function send(
    server: Addressing,
    outbox: Outbox,
    object: ObjectType,
    id: string,
): Promise<number | undefined> {
    return server.database.transaction(async (transaction) => {
        // read the source row and its copy
        const table = object.table as Table & Record<string, Column>;
        const [row] = (await transaction
            .select()
            .from(table)
            .where(and(eq(table.id!, id), isNull(table.origin!)))) as Record<string, unknown>[];
        const [held] = await transaction
            .select()
            .from(copy)
            .where(and(eq(copy.type, object.name), eq(copy.id, id)));

        // withdraw the copy of a row gone
        if (row === undefined) {
            if (held !== undefined && held.revision !== null) {
                await withdraw(transaction, outbox, object, held, "gone");
            }

            return undefined;
        }

        // check the recipient's read permission
        const now = Date.now();
        const recipient = String(row[object.addressed!.recipient]);
        const snapshot = Snapshot.live(transaction);
        const scope = String(row.scope);
        const access = await server.authorizer.resolveAssured(
            snapshot,
            scope,
            keySubject(recipient),
            now,
        );
        const admission = await server.authorizer.checkRows(
            snapshot,
            object.permission("read"),
            access,
            [row],
        );
        const revision = Number(row.revision);

        // send a new revision as the recipient reads it
        if (admission.held.has(0) && held?.revision !== revision) {
            const authorization = new Authorization(
                server.authorizer,
                transaction,
                () => access.context,
                access,
            );
            const [read] = await authorization.redact(object, [row]);
            const message = {
                type: object.name,
                space: scope,
                id,
                recipient,
                row: encodeRow(table, read!),
            };
            await outbox.append(INBOX, `${object.name}/${id}/${revision}`, message, transaction);
            await transaction
                .insert(copy)
                .values({ type: object.name, id, space: scope, recipient, revision })
                .onConflictDoUpdate({
                    target: [copy.type, copy.id],
                    set: { space: scope, recipient, revision },
                });
        }
        // withdraw a copy its recipient may no longer read
        else if (!admission.held.has(0) && held !== undefined && held.revision !== null) {
            await withdraw(transaction, outbox, object, held, String(revision));
        }

        return admission.until;
    });
}

/** Withdraw a recipient's copy through the outbox. */
async function withdraw(
    transaction: DatabaseConnection,
    outbox: Outbox,
    object: ObjectType,
    held: typeof copy.$inferSelect,
    reason: string,
): Promise<void> {
    await outbox.append(
        INBOX,
        `${object.name}/${held.id}/withdrawn/${reason}`,
        {
            type: object.name,
            space: held.space,
            id: held.id,
            recipient: held.recipient,
            row: null,
        },
        transaction,
    );
    await transaction
        .update(copy)
        .set({ revision: null })
        .where(and(eq(copy.type, held.type), eq(copy.id, held.id)));
}
