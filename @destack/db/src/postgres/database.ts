import { type Clock, Random } from "@destack/environment";
import type { DatabaseEnvironment } from "../database/connection.ts";
import { errorCode } from "../error/error.ts";
import { present, Uuid } from "@destack/schema";
import { ConnectionState, DatabaseConnection, requireDistinct } from "../database/connection.ts";
import type { Channel, ChannelListener, Delivery } from "../channel/channel.ts";
import { CHANNEL_PREFIX } from "../log/schema.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { PostgresSession } from "../database/session.ts";
import type { PostgresPool } from "./pool.ts";
import { expandTrees } from "../tree/tree.ts";
import type * as declaration from "../declare/database.ts";
import type { Table } from "../table/table.ts";
import { Relations } from "../query/relation.ts";
import type { Model } from "../query/model.ts";

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

/** How a PostgreSQL connection writes: alone or beside other writers, and in which environment. */
export interface PostgresConnectOptions {
    /** Whether the connection is the database's sole writer, shared by default. */
    readonly writers?: PostgresWriteMode;
    /** The database's schema, in which each statement runs. */
    readonly schema: string;
    /** The clock, randomness, crypto and telemetry the connection runs on. */
    readonly environment: DatabaseEnvironment;
}

/** A PostgreSQL database: a schema, on a pool of its own or one shared across schemas. */
export class PostgresDatabase<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> extends DatabaseConnection<Models> {
    /** The pool the database runs on. */
    readonly pool: PostgresPool;

    /** Who closes the database's pool: the database for a pool of its own, or its caller. */
    readonly #poolCloser: "database" | "caller";

    /** Bind tables to a pool, listening to other writers' commits unless it is the sole writer. */
    constructor(
        pool: PostgresPool,
        tables: declaration.Database<Models> | readonly Table[],
        options: PostgresConnectOptions & { readonly poolCloser: "database" | "caller" },
    ) {
        // read how the connection writes, and declare the tables with their tree tables
        const { writers = "shared", schema, environment } = options;
        const declared = "tables" in tables ? tables.tables : expandTrees(tables);
        requireDistinct(declared, "postgresql");

        // key each channel by the database's schema, as the log's notifications do
        let hashed: Promise<string> | undefined;
        const channel = (name: string) => {
            hashed ??= environment.scheduler.track(
                pool
                    .run(
                        environment,
                        (sql) => sql<{ hash: string }[]>`SELECT md5(${schema}) AS hash`,
                    )
                    .then(([row]) => present(row, "the schema's hash").hash),
                "peer",
            );

            return hashed.then((hash) => `${CHANNEL_PREFIX}${name}_${hash}`);
        };
        super(
            new DatabaseDriver(
                new PostgresSession({ pool }, schema, environment),
                new ConnectionState({
                    locality: "networked",
                    openChannel:
                        writers === "sole"
                            ? undefined
                            : (name) => postgresChannel(pool, channel(name), environment),
                    announcer: "database",
                    environment,
                    tables: declared,
                    dialect: "postgresql",
                    replicated: "tables" in tables ? tables.replicated : new Set(),
                }),
            ),
            declared,
            "tables" in tables ? tables.relations : new Relations<Models>(),
        );
        this.pool = pool;
        this.#poolCloser = options.poolCloser;
    }

    /** Close the pool when its owner's scope ends. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.close();
    }

    /** Stop listening after pending queries, closing a pool of its own. */
    async close(): Promise<void> {
        await this.state.close(async () => {
            // end a pool of its own, which the server answers
            if (this.#poolCloser === "database") {
                await this.state.environment.scheduler.track(this.pool.end(), "peer");
            }
        });
    }
}

/** Connect every party of a PostgreSQL notification channel with JSON messages. */
export function postgresChannel(
    pool: PostgresPool,
    name: Promise<string>,
    environment: DatabaseEnvironment,
): Channel<unknown> {
    // identify this party to its listeners, and collect the listeners' failures
    const { random } = environment;
    const party = Uuid.v4(random);
    const failures = new Set<(error: unknown) => void>();

    // track the sends and unlistens in flight, which closing waits for
    const pending = new Set<Promise<void>>();
    const track = (work: Promise<unknown>) => {
        const settled = work.then(
            () => undefined,
            () => undefined,
        );
        pending.add(settled);
        void settled.then(() => pending.delete(settled));
    };

    // send each payload on a reserved connection, failing the listeners as it fails
    const send = (payload: string) => {
        const sending = name.then((resolved) =>
            environment.scheduler.track(
                pool.run(environment, (sql) => sql`SELECT pg_notify(${resolved}, ${payload})`),
                "peer",
            ),
        );
        track(sending);
        void sending.catch((error: unknown) => {
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
    };

    return {
        notify: (message) => notifyInFragments(send, party, message, random),
        listen: (listener) => {
            const stop = listenToChannel(pool, name, party, failures, listener, environment);

            return () => track(stop());
        },
        delivery: (token) => deliverToEveryParty(pool, name, token),
        close: async () => {
            await Promise.all(pending);
        },
    };
}

/** Wait until a commit's token reaches every party listening on a channel through a pool. */
function deliverToEveryParty(pool: PostgresPool, name: Promise<string>, token: string): Delivery {
    // expect the token at each party listening once the channel's name resolves
    const settlers = new Set<{ readonly parties: Map<string, () => void>; resolve: () => void }>();
    let isForgotten = false;
    const delivered = name.then(async (resolved) => {
        const waits = [...(pool.parties.get(resolved) ?? [])].map(
            (expected) =>
                new Promise<void>((resolve) => {
                    if (isForgotten) {
                        resolve();
                    } else {
                        expected.set(token, resolve);
                        settlers.add({ parties: expected, resolve });
                    }
                }),
        );
        await Promise.all(waits);
    });

    return {
        delivered,
        forget: () => {
            // settle and drop every expectation of a token no commit sends
            isForgotten = true;
            for (const { parties, resolve } of settlers) {
                parties.delete(token);
                resolve();
            }
        },
    };
}

/** Send a party's message in numbered fragments, one for a small message. */
function notifyInFragments(
    send: (payload: string) => void,
    party: string,
    message: unknown,
    random: Random,
): void {
    const id = Uuid.v4(random);
    const chunks = fragments(JSON.stringify(message), PAYLOAD_BYTES);
    for (const [index, chunk] of chunks.entries()) {
        send(`${FRAGMENT}${party}:${id}:${index}:${chunks.length}:${chunk}`);
    }
}

/** Listen to a channel's notifications until stopped. */
function listenToChannel(
    pool: PostgresPool,
    name: Promise<string>,
    party: string,
    failures: Set<(error: unknown) => void>,
    listener: ChannelListener<unknown>,
    environment: DatabaseEnvironment,
): () => Promise<void> {
    // pause while the listening connection is closed, which the next listen resumes
    const { receive, resume, fail, pause } = listener;
    const unwatch = pool.onListenerClose(() => pause?.());

    // deliver each notification once listening, resuming on each reconnect
    const expected = new Map<string, () => void>();
    const deliverAll = () => {
        for (const resolve of expected.values()) {
            resolve();
        }
        expected.clear();
    };
    const failing = (error: unknown) => {
        // settle the token waits of a party that stopped listening, then fail
        deliverAll();
        fail?.(error);
    };
    if (fail !== undefined) {
        failures.add(failing);
    }
    const deliver = (message: unknown) => {
        // deliver the message, then settle the wait for the commit's token
        receive(message);
        const token: unknown =
            typeof message === "object" && message !== null
                ? Reflect.get(message, "token")
                : undefined;
        if (typeof token === "string") {
            expected.get(token)?.();
            expected.delete(token);
        }
    };
    const subscription = name.then(async (resolved) => {
        // listen, then receive tokens as a party of the channel
        const subscribed = await environment.scheduler.track(
            pool.listener.listen(
                resolved,
                reassembler(resolved, party, deliver, environment.clock),
                () => {
                    // settle every token wait on a reconnect, which may have lost notifications
                    deliverAll();
                    resume?.();
                },
            ),
            "peer",
        );
        const parties = pool.parties.get(resolved) ?? new Set();
        parties.add(expected);
        pool.parties.set(resolved, parties);

        return { subscribed, resolved };
    });
    subscription.catch((error: unknown) => failing(error));

    // stop listening, settling once unlistened
    return () => {
        failures.delete(failing);
        unwatch();

        return subscription.then(
            ({ subscribed, resolved }) => {
                // stop receiving tokens, settling the ones this party waited for
                pool.parties.get(resolved)?.delete(expected);
                deliverAll();

                return environment.scheduler
                    .track(subscribed.unlisten(), "peer")
                    .catch((error: unknown) => {
                        // count a closed connection as stopped, and fail on anything else
                        const code = errorCode(error);
                        if (code === undefined || !CLOSED_CODES.has(code)) {
                            fail?.(error);
                        }
                    });
            },
            () => undefined,
        );
    };
}

/** Deliver the database's messages and reassemble the other parties', skipping a party's echo. */
function reassembler(
    name: string,
    party: string,
    receive: (message: unknown) => void,
    clock: Clock,
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
        const now = clock.now();
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
