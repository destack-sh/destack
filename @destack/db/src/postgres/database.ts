import { errorCode } from "../error/error.ts";
import type postgres from "postgres";
import { ConnectionState, DatabaseConnection, requireDistinct } from "../database/connection.ts";
import type { Channel } from "../channel/channel.ts";
import { CHANNEL_PREFIX } from "../log/schema.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { PostgresSession } from "../database/session.ts";
import { expandTrees } from "../tree/tree.ts";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { Relations } from "../query/relation.ts";
import type { Model } from "../query/model.ts";

/** The PostgreSQL type identifiers of json and jsonb. */
const JSON_TYPES = [114, 3802] as const;

/** The encoder measuring characters' UTF-8 widths. */
const UTF8 = new TextEncoder();

/** The bytes one notification payload carries: below PostgreSQL's limit of 8000, leaving room for the fragment header. */
const PAYLOAD_BYTES = 7_800;

/** How long a message's fragments wait for the rest before they are dropped: a minute, far beyond one sender's burst. */
const FRAGMENT_MILLISECONDS = 60_000;

/** The client codes of a connection that closed, which leaves nothing to unlisten. */
const CLOSED_CODES: ReadonlySet<string> = new Set(["CONNECTION_DESTROYED", "CONNECTION_ENDED"]);

/** The prefix of a payload carrying one fragment of a message, which no JSON text starts with. */
const FRAGMENT = "#";

/** A PostgreSQL database with its own pool. */
export class PostgresDatabase<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> extends DatabaseConnection<Models> {
    /** The PostgreSQL connection pool. */
    readonly $client: postgres.Sql;

    /** Bind tables to a connection pool. */
    constructor(client: postgres.Sql, tables: declaration.Database<Models> | readonly Table[]) {
        // declare the tables with their tree tables
        const declared = "tables" in tables ? tables.tables : expandTrees(tables);
        requireDistinct(declared, "postgresql");
        passJsonText(client);
        super(
            new DatabaseDriver(
                new PostgresSession(client),
                new ConnectionState(
                    "networked",
                    (name) => postgresChannel(client, `${CHANNEL_PREFIX}${name}`),
                    "database",
                    "tables" in tables ? tables.spec.tier : undefined,
                ),
            ),
            declared,
            "tables" in tables ? tables.relations : new Relations<Models>(),
        );
        this.$client = client;
    }

    /** Close the pool when its owner's scope ends. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.close();
    }

    /** Close the pool after pending queries. */
    async close(): Promise<void> {
        await this.state.close(() => this.$client.end());
    }
}

/** Pass JSON columns' text to the server as it is, since columns encode JSON themselves. */
function passJsonText(client: postgres.Sql): void {
    for (const type of JSON_TYPES) {
        client.options.serializers[type] = (value: unknown) => value;
    }
}

/** Reach every party of a PostgreSQL notification channel, carrying messages as JSON text, fragmented above one payload's limit. */
export function postgresChannel(client: postgres.Sql, name: string): Channel<unknown> {
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
            if (UTF8.encode(text).byteLength <= PAYLOAD_BYTES) {
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
                    const message: unknown = JSON.parse(payload);
                    receive(message);

                    return;
                }

                // keep a fragment, and deliver its message once every fragment arrived
                const header = payload.slice(FRAGMENT.length);
                const [id, index, count] = header.split(":", 3);
                const position = Number(index);
                const total = Number(count);
                if (
                    id === undefined ||
                    index === undefined ||
                    count === undefined ||
                    !Number.isSafeInteger(position) ||
                    !Number.isSafeInteger(total) ||
                    position < 0 ||
                    position >= total
                ) {
                    throw new TypeError(`malformed notification fragment on ${name}`);
                }
                const part = header.slice(id.length + index.length + count.length + 3);
                const now = Date.now();
                for (const [key, entry] of pending) {
                    if (now - entry.at > FRAGMENT_MILLISECONDS) {
                        pending.delete(key);
                    }
                }
                const entry = pending.get(id) ?? {
                    parts: Array.from<string>({ length: total }),
                    received: 0,
                    at: now,
                };
                pending.set(id, entry);
                entry.parts[position] = part;
                entry.received += 1;
                if (entry.received === entry.parts.length) {
                    pending.delete(id);
                    const message: unknown = JSON.parse(entry.parts.join(""));
                    receive(message);
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
                            const code = errorCode(error);
                            if (code === undefined || !CLOSED_CODES.has(code)) {
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
        const width = UTF8.encode(character).length;
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
