import { errorCode } from "../error/error.ts";
import { present } from "@destack/schema";
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

/** The bytes of one notification's text, below PostgreSQL's 8000 with room for its party header. */
const PAYLOAD_BYTES = 7_800;

/** How long a message's fragments wait for the rest before dropping: a minute, past a burst. */
const FRAGMENT_MILLISECONDS = 60_000;

/** The client codes of a connection that closed, which leaves nothing to notify or unlisten. */
const CLOSED_CODES: ReadonlySet<string> = new Set(["CONNECTION_DESTROYED", "CONNECTION_ENDED"]);

/** The prefix of a party's payload, one fragment of its message, which no JSON text starts with. */
const FRAGMENT = "#";

/** Who writes a PostgreSQL database: pools announcing commits via LISTEN and NOTIFY, or one. */
export type PostgresWriteMode = "shared" | "sole";

/** A PostgreSQL database with a separate pool. */
export class PostgresDatabase<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> extends DatabaseConnection<Models> {
    /** The PostgreSQL connection pool. */
    readonly $client: postgres.Sql;

    /** Bind tables to a pool, listening to other writers' commits unless it is the sole writer. */
    constructor(
        client: postgres.Sql,
        tables: declaration.Database<Models> | readonly Table[],
        writers: PostgresWriteMode = "shared",
    ) {
        // declare the tables with their tree tables
        const declared = "tables" in tables ? tables.tables : expandTrees(tables);
        requireDistinct(declared, "postgresql");
        passJsonText(client);

        // name each channel within the pool's schema, as the log's notifications do
        let schema: Promise<string> | undefined;
        const channel = (name: string) => {
            schema ??= client<{ hash: string }[]>`SELECT md5(current_schema()) AS hash`.then(
                ([row]) => present(row, "the schema's hash").hash,
            );

            return schema.then((hash) => `${CHANNEL_PREFIX}${name}_${hash}`);
        };
        super(
            new DatabaseDriver(
                new PostgresSession(client),
                new ConnectionState({
                    locality: "networked",
                    openChannel:
                        writers === "sole"
                            ? undefined
                            : (name) => postgresChannel(client, channel(name)),
                    announcer: "database",
                    tables: declared,
                    dialect: "postgresql",
                    replicated: "tables" in tables ? tables.replicated : new Set(),
                }),
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

/** Connect every party of a PostgreSQL notification channel with JSON messages. */
export function postgresChannel(client: postgres.Sql, name: Promise<string>): Channel<unknown> {
    // identify this party to its listeners, and collect the listeners' failures
    const party = crypto.randomUUID();
    const failures = new Set<(error: unknown) => void>();
    const send = (payload: string) =>
        void name
            .then((resolved) => client.notify(resolved, payload))
            .catch((error: unknown) => {
                // drop a notification a closed connection leaves nobody to send to
                const code = errorCode(error);
                if (code !== undefined && CLOSED_CODES.has(code)) {
                    return;
                }

                // fail every listener, or fail loudly without one
                if (failures.size === 0) {
                    throw error;
                }
                for (const fail of failures) {
                    fail(error);
                }
            });

    return {
        notify: (message) => notifyInFragments(send, party, message),
        listen: (receive, resume, fail) =>
            listenToChannel(client, name, party, failures, receive, resume, fail),
    };
}

/** Send a party's message in numbered fragments, one for a small message. */
function notifyInFragments(send: (payload: string) => void, party: string, message: unknown): void {
    const id = crypto.randomUUID();
    const chunks = fragments(JSON.stringify(message), PAYLOAD_BYTES);
    for (const [index, chunk] of chunks.entries()) {
        send(`${FRAGMENT}${party}:${id}:${index}:${chunks.length}:${chunk}`);
    }
}

/** Listen to a channel's notifications until stopped. */
function listenToChannel(
    client: postgres.Sql,
    name: Promise<string>,
    party: string,
    failures: Set<(error: unknown) => void>,
    receive: (message: unknown) => void,
    resume: (() => void) | undefined,
    fail: ((error: unknown) => void) | undefined,
): () => void {
    // deliver each notification once listening, resuming on each reconnect
    if (fail !== undefined) {
        failures.add(fail);
    }
    const listening = name.then((resolved) =>
        client.listen(resolved, reassembler(resolved, party, receive), () => resume?.()),
    );
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
}

/** Deliver the database's messages and reassemble the other parties', skipping a party's echo. */
function reassembler(
    name: string,
    party: string,
    receive: (message: unknown) => void,
): (payload: string) => void {
    const pending = new Map<string, { chunks: string[]; received: number; at: number }>();

    return (payload) => {
        // deliver a message the database sends whole, such as a commit's
        if (!payload.startsWith(FRAGMENT)) {
            const message: unknown = JSON.parse(payload);
            receive(message);

            return;
        }

        // skip this party's echo, and drop the messages whose fragments stopped arriving
        const { sender, id, position, total, chunk } = parseFragment(name, payload);
        if (sender === party) {
            return;
        }
        const now = Date.now();
        for (const [key, entry] of pending) {
            if (now - entry.at > FRAGMENT_MILLISECONDS) {
                pending.delete(key);
            }
        }

        // store the fragment, and deliver its message once every fragment arrived
        const entry = pending.get(id) ?? {
            chunks: Array.from<string>({ length: total }),
            received: 0,
            at: now,
        };
        pending.set(id, entry);
        entry.chunks[position] = chunk;
        entry.received += 1;
        if (entry.received === entry.chunks.length) {
            pending.delete(id);
            const message: unknown = JSON.parse(entry.chunks.join(""));
            receive(message);
        }
    };
}

/** Read a fragment's sending party, message id, position, count and text. */
function parseFragment(
    name: string,
    payload: string,
): {
    readonly sender: string;
    readonly id: string;
    readonly position: number;
    readonly total: number;
    readonly chunk: string;
} {
    // read the header's party, id, position and count
    const header = payload.slice(FRAGMENT.length);
    const [sender, id, index, count] = header.split(":", 4);
    const position = Number(index);
    const total = Number(count);
    if (
        sender === undefined ||
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
    const chunk = header.slice(sender.length + id.length + index.length + count.length + 4);

    return { sender, id, position, total, chunk };
}

/** Split text into chunks of at most some UTF-8 bytes each, never inside a character. */
function fragments(text: string, bytes: number): string[] {
    // fill each chunk up to the limit, measuring each character's UTF-8 width by its code point
    const chunks: string[] = [];
    let chunk = "";
    let size = 0;
    for (const character of text) {
        // start a new chunk once the character would not fit
        const width = UTF8.encode(character).length;
        if (size + width > bytes) {
            chunks.push(chunk);
            chunk = "";
            size = 0;
        }
        chunk += character;
        size += width;
    }
    chunks.push(chunk);

    return chunks;
}
