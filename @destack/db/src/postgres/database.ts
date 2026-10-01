import postgres from "postgres";
import { drizzle, type PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzlePgConfig } from "drizzle-orm/pg-core/utils";
import { ConnectionState, DatabaseConnection } from "../database/connection.ts";
import type { Channel } from "../channel/channel.ts";
import { CHANNEL_PREFIX } from "../log/schema.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { PostgresSchemaCompiler } from "./compiler.ts";
import { expandTrees } from "../tree/tree.ts";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";

/** A PostgreSQL database with its own pool. */
export class PostgresDatabase extends DatabaseConnection<"postgresql"> {
    /** The PostgreSQL connection pool. */
    readonly $client: postgres.Sql;
    /** The native SQL API. */
    readonly native: PostgresJsDatabase;

    /** Bind tables to a connection pool. */
    constructor(
        client: postgres.Sql,
        tables: declaration.Database | readonly Table[],
        options: Omit<DrizzlePgConfig<EmptyRelations>, "relations"> = {},
    ) {
        // compile the tables with their tree tables
        const compiler = new PostgresSchemaCompiler(
            "tables" in tables ? tables.tables : expandTrees(tables),
        );
        const native = drizzle({ ...options, client });
        super(
            new DatabaseDriver(
                { dialect: "postgresql", database: native },
                new ConnectionState(
                    "networked",
                    (name) => postgresChannel(client, `${CHANNEL_PREFIX}${name}`),
                    "database",
                    "tables" in tables ? tables.spec.tier : undefined,
                ),
            ),
            compiler,
        );
        this.$client = client;
        this.native = native;
    }

    /** Close the pool after pending queries. */
    async close(): Promise<void> {
        await this.state.close(() => this.$client.end());
    }
}

/** The bytes one notification payload carries: below PostgreSQL's limit of 8000, leaving room for the fragment header. */
const PAYLOAD_BYTES = 7_800;

/** How long a message's fragments wait for the rest before they are dropped: a minute, far beyond one sender's burst. */
const FRAGMENT_MILLISECONDS = 60_000;

/** The client codes of a connection that closed, which leaves nothing to unlisten. */
const CLOSED_CODES: ReadonlySet<string> = new Set(["CONNECTION_DESTROYED", "CONNECTION_ENDED"]);

/** The prefix of a payload carrying one fragment of a message, which no JSON text starts with. */
const FRAGMENT = "#";

/** Reach every party of a PostgreSQL notification channel, carrying messages as JSON text, fragmented above one payload's limit. */
export function postgresChannel<Message>(client: postgres.Sql, name: string): Channel<Message> {
    // collect the failures of notifications for the listeners
    const failures = new Set<(error: unknown) => void>();
    const send = (payload: string) =>
        void client.notify(name, payload).catch((error: unknown) => {
            // fail every listener, or fail loudly without one
            if (failures.size === 0) {
                throw error;
            }
            for (const fail of failures) {
                fail(error);
            }
        });

    return {
        notify: (message) => {
            // send a small message whole, and a large one in numbered fragments
            const text = JSON.stringify(message);
            if (new TextEncoder().encode(text).byteLength <= PAYLOAD_BYTES) {
                send(text);
            } else {
                const id = crypto.randomUUID();
                const parts = fragments(text, PAYLOAD_BYTES);
                for (const [index, part] of parts.entries()) {
                    send(`${FRAGMENT}${id}:${index}:${parts.length}:${part}`);
                }
            }
        },
        listen: (receive, resume, fail) => {
            // reassemble fragmented messages, dropping ones whose fragments stop arriving
            const pending = new Map<string, { parts: string[]; received: number; at: number }>();
            const deliver = (payload: string) => {
                // deliver a whole message at once, as the database's own notifications send them too
                if (!payload.startsWith(FRAGMENT)) {
                    receive(JSON.parse(payload) as Message);

                    return;
                }

                // keep a fragment, and deliver its message once every fragment arrived
                const header = payload.slice(FRAGMENT.length);
                const [id, index, count] = header.split(":", 3) as [string, string, string];
                const part = header.slice(id.length + index.length + count.length + 3);
                const now = Date.now();
                for (const [key, entry] of pending) {
                    if (now - entry.at > FRAGMENT_MILLISECONDS) {
                        pending.delete(key);
                    }
                }
                const entry = pending.get(id) ?? {
                    parts: Array.from<string>({ length: Number(count) }),
                    received: 0,
                    at: now,
                };
                pending.set(id, entry);
                entry.parts[Number(index)] = part;
                entry.received += 1;
                if (entry.received === entry.parts.length) {
                    pending.delete(id);
                    receive(JSON.parse(entry.parts.join("")) as Message);
                }
            };

            // deliver each notification once listening, resuming on each reconnect
            if (fail !== undefined) {
                failures.add(fail);
            }
            const listening = client.listen(name, deliver, () => resume?.());
            listening.catch((error: unknown) => fail?.(error));

            // stop listening
            return () => {
                if (fail !== undefined) {
                    failures.delete(fail);
                }
                void listening.then(
                    (listener) =>
                        listener.unlisten().catch((error: unknown) => {
                            // count a closed connection as stopped, and fail on anything else
                            if (!CLOSED_CODES.has((error as { code?: unknown }).code as string)) {
                                fail?.(error);
                            }
                        }),
                    () => undefined,
                );
            };
        },
    };
}

/** Split text into parts of at most some UTF-8 bytes each, never inside a character. */
function fragments(text: string, bytes: number): string[] {
    // fill each part up to the limit, measuring each character's UTF-8 width by its code point
    const parts: string[] = [];
    let part = "";
    let size = 0;
    for (const character of text) {
        // start a new part once the character would not fit
        const point = character.codePointAt(0)!;
        const width = point < 0x80 ? 1 : point < 0x800 ? 2 : point < 0x10000 ? 3 : 4;
        if (size + width > bytes) {
            parts.push(part);
            part = "";
            size = 0;
        }
        part += character;
        size += width;
    }
    parts.push(part);

    return parts;
}
