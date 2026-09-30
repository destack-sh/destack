import { accessProposal, decisionTables } from "@destack/access";
import {
    and,
    asc,
    encodeRow,
    eq,
    isNull,
    TABLE,
    type DatabaseConnection,
    type Table,
} from "@destack/db";
import { type LogPosition, Snapshot } from "@destack/db/log";
import { Condition } from "@destack/db/query";
import type { Directory, Zone } from "@destack/directory";
import { serverTables, type ObjectType } from "@destack/object";
import { Recipient, type ResourceKind, Provider } from "@destack/resource";
import { canonicalize } from "@destack/schema/json";
import { ServiceError } from "@destack/service/error";
import { implement, type ServiceContext } from "@destack/service/server";
import { Feed, Replica, replica, type QueryPage, Scope } from "@destack/sync";
import { SpaceError } from "../error/index.ts";
import { resource, space, transfer, type Transfer } from "../object/index.ts";
import { type Fence, relay } from "../service/relay.ts";
import { appliedStates } from "./resource.ts";

/** The rows one digest page reads: 5000 rows of about 500 B keep a page near 2.5 MB. */
const DIGEST_PAGE_ROWS = 5000;

/** Typed implementations of the zone procedures. */
const implementation = implement(relay).$context<ServiceContext>();

/** The name of the copy a transfer's target follows its source's rows through. */
const TRANSFER_COPY = "transfer";

/** The object server's tables a cell keeps for itself: its copies' progress, settlements, leases and outbox. */
const CELL_TABLES: readonly Table[] = serverTables.filter(
    (table) => !decisionTables.includes(table) && table !== accessProposal,
);

/** Rows a transfer carries by key outside the digest, such as the zone's own row. */
export interface KeyedRows {
    /** The scope the rows live in, the zone's own when absent. */
    readonly scope?: string;
    /** The rows carried. */
    readonly where: Condition;
}

/** The target's repeatable steps taking over a zone's database: follow, fence, reach, verify, promote, activate. */
export class ZoneTransfer {
    /** The zone as its source serves it. */
    readonly zone: Zone;
    /** The host or region receiving the zone. */
    readonly target: string;
    /** The tables with the zone's rows, parents before the tables referencing them. */
    readonly tables: readonly Table[];
    /** The tables whose rows travel by key, and the rows they carry. */
    readonly keyed: ReadonlyMap<Table, KeyedRows>;
    /** The copy the target follows the source's rows through. */
    readonly replica: Replica;

    /** Plan a zone's move over every table of its database that keeps rows in spaces, with its own row and its transfers by key. */
    static of(database: DatabaseConnection, zone: Zone, target: string): ZoneTransfer {
        const tables = database.tables.filter(
            (table) =>
                table[TABLE].retention !== "none" &&
                table[TABLE].columns.scope !== undefined &&
                !database.copies(table) &&
                !CELL_TABLES.includes(table) &&
                table !== space.table &&
                table !== transfer.table,
        );

        return new ZoneTransfer({
            zone,
            target,
            tables: [space.table, transfer.table, ...tables],
            keyed: new Map<Table, KeyedRows>([
                [space.table, { scope: zone.scope, where: Condition.eq("id", zone.id) }],
                [transfer.table, { where: Condition.all() }],
            ]),
        });
    }

    /** Plan a zone's transfer to a target over its tables, some of which travel by key. */
    constructor(options: {
        readonly zone: Zone;
        readonly target: string;
        readonly tables: readonly Table[];
        readonly keyed?: ReadonlyMap<Table, KeyedRows>;
    }) {
        // refuse handing a zone to its own cell
        if (options.target === options.zone.cell) {
            throw new ServiceError("CONFLICT", {
                message: `${options.zone.id} is served by ${options.target} already`,
            });
        }
        this.zone = options.zone;
        this.target = options.target;
        this.tables = options.tables;
        this.keyed = options.keyed ?? new Map();
        this.replica = new Replica({
            name: TRANSFER_COPY,
            scope: options.zone.id,
            tables: options.tables,
            where: new Map([...this.keyed].map(([table, rows]) => [table, rows.where])),
            scopes: new Map(
                [...this.keyed].flatMap(([table, rows]) =>
                    rows.scope === undefined ? [] : [[table, rows.scope] as const],
                ),
            ),
        });
    }

    /** Fence the source against writes into the zone unless fenced already, then read its position and digest. */
    async fence(source: DatabaseConnection, now: number): Promise<Fence> {
        return source.transaction(async (transaction) => {
            // fence the zone once
            const chain = await Scope.chain(Snapshot.live(transaction), this.zone.id);
            if (chain.every((link) => link.movedTo === undefined)) {
                await Scope.fence(transaction, this.zone.id, this.target, now);
            }

            // read the position and the digest of the rows at it
            return {
                position: await transaction.log.position(),
                digest: await ZoneTransfer.digest(transaction, this.#digested(), this.zone.id),
            };
        });
    }

    /** Follow the source's rows into the target until the signal aborts. */
    async follow(
        target: DatabaseConnection,
        pages: (after: LogPosition | undefined, signal: AbortSignal) => AsyncIterable<QueryPage>,
        signal: AbortSignal,
    ): Promise<void> {
        await this.replica.follow(target, pages, signal).catch((error: unknown) => {
            if (!signal.aborted) {
                throw error;
            }
        });
    }

    /** Wait until the target's copy reaches the fenced position, false once the signal aborts first. */
    reach(target: DatabaseConnection, fenced: Fence, signal: AbortSignal): Promise<boolean> {
        return Replica.reach(target, this.zone.id, fenced.position, signal);
    }

    /** Require the target's copy to have exactly the rows the source had at the fence. */
    async verify(target: DatabaseConnection, fenced: Fence): Promise<void> {
        const digest = await ZoneTransfer.digest(target, this.#digested(), this.zone.id);
        if (digest !== fenced.digest) {
            throw new ServiceError("CONFLICT", {
                message: `the copy of ${this.zone.id} differs from its fenced source`,
            });
        }
    }

    /** Make the target own its copy and lift the copied fence. */
    async promote(target: DatabaseConnection): Promise<void> {
        if (await Replica.isCopied(target, this.zone.id)) {
            await this.replica.promote(target);
        }
        await Scope.unfence(target, this.zone.id);
    }

    /** Delegate the zone to the target at the next epoch. */
    async activate(directory: Directory): Promise<Zone> {
        const moved = { ...this.zone, cell: this.target, epoch: this.zone.epoch + 1 };
        await directory.place(moved);

        return moved;
    }

    /** The tables the digest covers: those keeping the zone's rows whole. */
    #digested(): Table[] {
        return this.tables.filter((table) => !this.keyed.has(table));
    }

    /** Digest a zone's logged rows in some tables page by page in key order, keeping only each page's digest in memory. */
    static async digest(
        database: DatabaseConnection,
        tables: readonly Table[],
        zone: string,
    ): Promise<string> {
        // digest each table's rows of the zone a page at a time
        const pages: string[] = [];
        for (const table of tables) {
            const columns = table[TABLE].columns;
            const logged = Object.keys(table[TABLE].logged);
            for (let offset = 0; ; offset += DIGEST_PAGE_ROWS) {
                // read the next page of the zone's rows in key order
                const rows = (await database
                    .select()
                    .from(table)
                    .where(eq(columns.scope!, zone))
                    .orderBy(...table[TABLE].key.map((property) => asc(columns[property]!)))
                    .limit(DIGEST_PAGE_ROWS)
                    .offset(offset)) as Record<string, unknown>[];
                if (rows.length === 0) {
                    break;
                }

                // refuse moving rows with columns their log leaves out, which only a resource moves
                if (logged.length !== table[TABLE].entries.length) {
                    throw new SpaceError(
                        "UNSUPPORTED_DEFINITION",
                        `${table[TABLE].name} has columns its log leaves out, which only a resource moves`,
                    );
                }

                // keep the page's digest of its logged columns
                const values = rows.map((row) => {
                    const encoded = encodeRow(table, row) as Record<string, unknown>;

                    return logged.map((property) => encoded[property] ?? null);
                });
                pages.push(await sha256(canonicalize([table[TABLE].sqlName, offset, values])));
                if (rows.length < DIGEST_PAGE_ROWS) {
                    break;
                }
            }
        }

        return sha256(pages.join(""));
    }
}

/** What a source relays its spaces' zones with: their rows, the directory locating them, and the providers keeping their resources. */
export interface ZoneOptions {
    /** The space database. */
    readonly database: DatabaseConnection;
    /** The universe's directory placing the spaces' zones. */
    readonly directory: Directory;
    /** The providers keeping the spaces' resources. */
    readonly providers: readonly Provider<ResourceKind, ObjectType>[];
}

/** Relay each space's zone to the target of its transfer: its rows, the fence, and its resources' content. */
export function zoneRouter(options: ZoneOptions) {
    const { database } = options;

    return implementation.router({
        watch: implementation.watch.handler(async function* ({ input, context }) {
            // stream the space's rows to the target of its transfer, ending at a completed page once drained
            const steps = await transferring(options, input.spaceId, context);
            const feed = new Feed(database, [...steps.replica.tables, replica]);
            yield* feed.subscribe(steps.replica.queries, input.after, context.request.signal, {
                drain: context.signal,
            });
        }),
        fence: implementation.fence.handler(async ({ input, context }) => {
            // fence the space once, then answer where it stands
            const steps = await transferring(options, input.spaceId, context);

            return steps.fence(database, Date.now());
        }),
        export: implementation.export.handler(async function* ({ input, context }) {
            // copy a fenced space's content after it stops serving
            await active(database, input.spaceId, context);
            const chain = await Scope.chain(Snapshot.live(database), input.spaceId);
            const isFenced = chain.some((link) => link.movedTo !== undefined);
            if (input.stage === "fenced" && !isFenced) {
                throw new ServiceError("PRECONDITION_FAILED", {
                    message: `${input.spaceId} is not fenced yet`,
                });
            }

            // read the resource through the provider keeping it
            const [row] = await database
                .select()
                .from(resource.table)
                .where(
                    and(
                        eq(resource.table.scope, input.spaceId),
                        eq(resource.table.id, input.resourceId),
                    ),
                );
            if (row === undefined) {
                throw new ServiceError("NOT_FOUND", {
                    message: `${input.spaceId} has no resource ${input.resourceId}`,
                });
            }
            const provider = options.providers.find(
                (entry) => entry.kind.name === row.kind && entry.code === row.providerCode,
            );
            if (provider === undefined) {
                throw new ServiceError("PRECONDITION_FAILED", {
                    message: `no provider ${row.providerCode} of ${row.kind} on this host`,
                });
            } else if (!Provider.copies(provider)) {
                throw new ServiceError("PRECONDITION_FAILED", {
                    message: `provider ${provider.code} of ${row.kind} exports no content`,
                });
            }
            const copy = {
                record: provider.kind.record(row),
                desired: provider.kind.states(await appliedStates(database, row)),
                recipient: Recipient.of(input.recipient),
                stage: input.stage,
            };
            for await (const chunk of provider.export(copy, input.after, context.request.signal)) {
                yield { chunk };
            }

            // mark the end of an export the provider finished
            context.request.signal.throwIfAborted();
            yield { end: true as const };
        }),
    });
}

/** Plan the steps of a space's active transfer to the caller, from the zone the directory delegates to its source. */
async function transferring(options: ZoneOptions, spaceId: string, context: ServiceContext) {
    // require the space's source to serve its zone still
    const record = await active(options.database, spaceId, context);
    const located = await options.directory.locate(spaceId);
    if (located === undefined || located.cell !== record.source) {
        throw new ServiceError("CONFLICT", {
            message: `${spaceId} is no longer placed in ${record.source}`,
        });
    }

    return ZoneTransfer.of(options.database, located, record.target);
}

/** Read a space's active transfer to the caller's cell and refuse every other caller. */
async function active(
    database: DatabaseConnection,
    spaceId: string,
    context: ServiceContext,
): Promise<Transfer> {
    // read the active transfer and the cells the caller acts for
    const cells = new Set(
        context.requireCaller().authentication.subjects.map((subject) => subject.id),
    );
    const [record] = await database
        .select()
        .from(transfer.table)
        .where(
            and(
                eq(transfer.table.scope, spaceId as Transfer["scope"]),
                isNull(transfer.table.completedAt),
            ),
        );
    if (record === undefined || !cells.has(record.target)) {
        throw new ServiceError("NOT_FOUND", {
            message: `${spaceId} is transferring to no cell the caller acts for`,
        });
    }

    return record;
}

/** Hash a text with SHA-256, as hexadecimal. */
async function sha256(text: string): Promise<string> {
    const bytes = new TextEncoder().encode(text);

    return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex();
}
