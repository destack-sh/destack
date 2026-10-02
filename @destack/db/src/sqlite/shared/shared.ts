import { DriverValue } from "../../table/column.ts";
import { schema } from "@destack/schema";
import type * as declaration from "../../declare/database.ts";
import { typedChannel, type Channel } from "../../channel/channel.ts";
import type { Table } from "../../table/table.ts";
import { SqliteDatabase } from "../database.ts";
import { DatabaseError, errorCode } from "../../error/error.ts";
import { Savepoint, type ConnectionClient, type QueryClient } from "../client.ts";
import { LOG_TOPIC } from "../../log/schema.ts";

/**
 * The idle timeout of a party's transaction at the owner, in milliseconds.
 *
 * A transaction keeps the owner's only connection, and this bounds how long a frozen tab blocks the others.
 */
const IDLE_TRANSACTION_MILLISECONDS = 30_000;

/** One statement or transaction step. */
export const Step = schema.discriminatedUnion("type", [
    schema.object({
        /** Run a statement, in a transaction when named. */
        type: schema.literal("statement"),
        /** How to run it, by the client method. */
        method: schema.enum(["values", "all", "run", "exec"]),
        /** The SQL. */
        sql: schema.string(),
        /** The bound parameters. */
        parameters: schema.array(DriverValue),
        /** The transaction, absent outside one. */
        transaction: schema.number().exactOptional(),
    }),
    schema.object({
        /** Begin a transaction. */
        type: schema.literal("begin"),
        /** How to begin it. */
        mode: schema.enum(["deferred", "immediate", "exclusive"]),
    }),
    schema.object({
        /** End a transaction. */
        type: schema.enum(["commit", "rollback"]),
        /** The transaction. */
        transaction: schema.number(),
    }),
]);
/** One statement or transaction step. */
export type Step = schema.Infer<typeof Step>;

/** A message between the parties of a shared database. */
export const Message = schema.discriminatedUnion("kind", [
    schema.object({
        /** A statement or transaction step for the owner. */
        kind: schema.literal("request"),
        /** The asking party. */
        from: schema.string(),
        /** The owner that answers. */
        to: schema.string(),
        /** The request number. */
        id: schema.number(),
        /** What to run. */
        step: Step,
    }),
    schema.object({
        /** The owner's answer. */
        kind: schema.literal("answer"),
        /** The asking party. */
        to: schema.string(),
        /** The answered request. */
        id: schema.number(),
        /** The result, absent on failure. */
        value: schema.unknown().exactOptional(),
        /** The failure, absent on success. */
        error: schema
            .object({
                /** The failure's name. */
                name: schema.string(),
                /** The failure's message. */
                message: schema.string(),
                /** The driver code, absent without one. */
                code: schema.string().exactOptional(),
            })
            .exactOptional(),
    }),
    schema.object({
        /** A commit every party may read. */
        kind: schema.literal("commit"),
    }),
    schema.object({
        /** An owner serving from now on. */
        kind: schema.literal("serving"),
        /** The owner, new each time a party starts serving. */
        owner: schema.string(),
    }),
    schema.object({
        /** A party asking which owner serves. */
        kind: schema.literal("join"),
    }),
]);
/** A message between the parties of a shared database. */
export type Message = schema.Infer<typeof Message>;

/** Serve a connection to a channel's other parties until stopped. */
export function serveDatabase(client: ConnectionClient, raw: Channel<unknown>): () => void {
    // type the channel's messages, name this owner and keep party transactions open
    const channel = typedChannel(raw, Message);
    const owner = crypto.randomUUID();
    const transactions = new Map<number, OpenTransaction>();
    let next = 0;

    // run one step
    const run = async (step: Step): Promise<unknown> => {
        // run a statement
        if (step.type === "statement") {
            const open =
                step.transaction === undefined ? undefined : transactions.get(step.transaction);
            if (step.transaction !== undefined && open === undefined) {
                throw new DatabaseError("TRANSACTION_CLOSED", "the shared transaction has ended");
            }
            open?.touch();
            const target = open?.client ?? client;
            return step.method === "exec"
                ? await target.exec(step.sql)
                : await target[step.method](step.sql, step.parameters);
        }
        // begin a transaction
        else if (step.type === "begin") {
            const id = next++;
            const opened = await OpenTransaction.begin(client, step.mode, () =>
                transactions.delete(id),
            );
            transactions.set(id, opened);

            return id;
        }
        // end a transaction
        else {
            const open = transactions.get(step.transaction);
            if (open === undefined) {
                throw new DatabaseError("TRANSACTION_CLOSED", "the shared transaction has ended");
            }
            await open.end(step.type === "commit");

            return undefined;
        }
    };

    // announce this owner to a joining party
    const stop = channel.listen((message) => {
        if (message.kind === "join") {
            channel.notify({ kind: "serving", owner });
        }
        // answer each request to this owner
        else if (message.kind === "request" && message.to === owner) {
            run(message.step).then(
                (value) =>
                    channel.notify({ kind: "answer", to: message.from, id: message.id, value }),
                (error: unknown) =>
                    channel.notify({
                        kind: "answer",
                        to: message.from,
                        id: message.id,
                        error: describeError(error),
                    }),
            );
        }
    });

    // announce this owner
    channel.notify({ kind: "serving", owner });

    // roll back open transactions on stop
    return () => {
        stop();
        for (const open of transactions.values()) {
            void open.end(false);
        }
    };
}

/** A transaction the owner keeps open for a party. */
class OpenTransaction {
    /** The transaction's client. */
    readonly client: QueryClient;
    /** Resolve on commit and reject on rollback. */
    readonly done: Promise<void>;
    /** End the callback that keeps the transaction open. */
    readonly #finish: (commit: boolean) => void;
    /** Forget the transaction. */
    readonly #forget: () => void;
    /** The idle rollback timer. */
    #idle?: ReturnType<typeof setTimeout>;

    /** Create the transaction. */
    private constructor(
        client: QueryClient,
        done: Promise<void>,
        finish: (commit: boolean) => void,
        forget: () => void,
    ) {
        // keep the client and its end
        this.client = client;
        this.done = done;
        this.#finish = finish;
        this.#forget = forget;
        this.touch();
    }

    /** Begin a transaction and keep it open. */
    static begin(
        connection: ConnectionClient,
        mode: "deferred" | "immediate" | "exclusive",
        forget: () => void,
    ): Promise<OpenTransaction> {
        return new Promise((resolve, reject) => {
            // keep the callback open until the party ends it
            let opened: OpenTransaction | undefined;
            const begin = connection.transactionAsync(
                (transaction) =>
                    new Promise<void>((finish, abort) => {
                        const end = (commit: boolean) =>
                            commit
                                ? finish()
                                : abort(new Error("the shared transaction rolled back"));
                        opened = new OpenTransaction(transaction, done, end, forget);
                        resolve(opened);
                    }),
            );
            const done = begin[mode]();

            // report a failed begin and forget the transaction
            done.catch((error: unknown) => {
                if (opened === undefined) {
                    reject(error);
                }
            });
            done.finally(forget).catch(() => undefined);
        });
    }

    /** Reset the idle timer. */
    touch(): void {
        clearTimeout(this.#idle);
        this.#idle = setTimeout(() => void this.end(false), IDLE_TRANSACTION_MILLISECONDS);
    }

    /** Commit or roll back. */
    async end(commit: boolean): Promise<void> {
        // stop the timer and end the callback
        clearTimeout(this.#idle);
        this.#forget();
        this.#finish(commit);
        if (commit) {
            await this.done;
        } else {
            await this.done.catch(() => undefined);
        }
    }
}

/** Open a database with statements that the channel's owner runs. */
export function connectShared(
    channel: Channel<unknown>,
    party: string,
    tables: declaration.Database | readonly Table[] = [],
): SqliteDatabase<SharedClient> {
    const client = new SharedClient(channel, party);

    return new SqliteDatabase(client, tables, "embedded", (name: string) =>
        SharedClient.channel(channel, name),
    );
}

/** Statements the channel's owner runs. */
export class SharedQuery implements QueryClient {
    /** The party asking the owner. */
    readonly party: Party;
    /** The owner's transaction. */
    readonly transaction: SharedTransaction | undefined;

    /** Create the client. */
    constructor(party: Party, transaction?: SharedTransaction) {
        this.party = party;
        this.transaction = transaction;
    }

    /** Read every row as an array of values, integers exact. */
    async values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]> {
        const rows = await this.#statement({
            method: "values",
            sql,
            parameters: parameters.map(sharedValue),
        });
        if (!Array.isArray(rows) || !rows.every((row) => Array.isArray(row))) {
            throw new TypeError("the shared owner answered rows as arrays with another value");
        }

        return rows;
    }

    /** Read every row by column name. */
    async all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]> {
        // ask the owner, and require a list of rows
        const rows = await this.#statement({
            method: "all",
            sql,
            parameters: parameters.map(sharedValue),
        });
        if (!Array.isArray(rows)) {
            throw new TypeError("the shared owner answered rows with another value");
        }
        const answered: unknown[] = rows;

        return answered;
    }

    /** Run a statement for its effect. */
    async run(sql: string, parameters: readonly DriverValue[]): Promise<void> {
        await this.#statement({ method: "run", sql, parameters: parameters.map(sharedValue) });
    }

    /** Run work in a savepoint the owner keeps, at a depth. */
    nest<Value>(depth: number, operation: (client: QueryClient) => Promise<Value>): Promise<Value> {
        return Savepoint.run(this, depth, operation);
    }

    /** Run a script. */
    async exec(script: string): Promise<void> {
        await this.#statement({ method: "exec", sql: script, parameters: [] });
    }

    /** Ask the owner to run a statement. */
    #statement(
        statement: Omit<Extract<Step, { readonly type: "statement" }>, "type" | "transaction">,
    ): Promise<unknown> {
        const transaction = this.transaction;

        return transaction === undefined
            ? this.party.request({ type: "statement", ...statement })
            : this.party.request(
                  { type: "statement", ...statement, transaction: transaction.id },
                  transaction.owner,
              );
    }
}

/** A connection client with work that the channel's owner runs. */
export class SharedClient extends SharedQuery implements ConnectionClient {
    /** Join the channel as one party. */
    constructor(channel: Channel<unknown>, name: string) {
        super(new Party(channel, name));
    }

    /** Open the log channel the parties announce commits on, delivering its commits alone, refusing every other name. */
    static channel(channel: Channel<unknown>, name: string): Channel<unknown> {
        if (name !== LOG_TOPIC) {
            throw new DatabaseError(
                "NO_CHANNEL",
                `a shared connection opens only its ${LOG_TOPIC} channel, not ${name}`,
            );
        }

        // deliver the commits among the shared messages
        const shared = typedChannel(channel, Message);

        return {
            notify: (message) => channel.notify(message),
            listen: (receive, resume, fail) =>
                shared.listen(
                    (message) => {
                        if (message.kind === "commit") {
                            receive(message);
                        }
                    },
                    resume,
                    fail,
                ),
        };
    }

    /** Stop reaching the owner. */
    async close(): Promise<void> {
        this.party.close();
    }

    /** Run a callback in a transaction the owner keeps open. */
    transactionAsync<Value>(operation: (client: QueryClient) => Promise<Value>) {
        const begin = async (mode: "deferred" | "immediate" | "exclusive") => {
            // begin at the owner and run the callback
            const transaction = await this.party.begin(mode);
            const scoped = new SharedQuery(this.party, transaction);

            // end the transaction by the callback's outcome
            const { owner, id } = transaction;
            try {
                const value = await operation(scoped);
                await this.party.request({ type: "commit", transaction: id }, owner);

                return value;
            } catch (error) {
                // roll back, reporting a failed rollback beside the failure
                await this.party
                    .request({ type: "rollback", transaction: id }, owner)
                    .catch((rollback: unknown) => {
                        throw new AggregateError(
                            [error, rollback],
                            "transaction and rollback failed",
                            {
                                cause: error,
                            },
                        );
                    });
                throw error;
            }
        };

        return {
            deferred: () => begin("deferred"),
            immediate: () => begin("immediate"),
            exclusive: () => begin("exclusive"),
        };
    }
}

/** One party of a channel, asking its owner to run steps. */
export class Party {
    /** The channel reaching the owner. */
    readonly #channel: Channel<Message>;
    /** This party's name on the channel. */
    readonly #name: string;
    /** The unanswered requests by number. */
    readonly #pending = new Map<number, Request>();
    /** The next request number. */
    #next = 0;
    /** The serving owner. */
    #owner: string | undefined;
    /** Stop receiving answers. */
    readonly #stop: () => void;

    /** Join a channel and ask which owner serves. */
    constructor(raw: Channel<unknown>, name: string) {
        // listen for answers and ask for the owner
        const channel = typedChannel(raw, Message);
        this.#channel = channel;
        this.#name = name;
        this.#stop = channel.listen((message) => this.#receive(message));
        channel.notify({ kind: "join" });
    }

    /**
     * Send one step to the owner and wait for its answer.
     *
     * A transaction step goes only to its owner and fails once another owner serves.
     */
    async request(step: Step, owner?: string): Promise<unknown> {
        return (await this.#ask(step, owner)).value;
    }

    /** Begin a transaction at the serving owner. */
    async begin(mode: "deferred" | "immediate" | "exclusive"): Promise<SharedTransaction> {
        const { value, owner } = await this.#ask({ type: "begin", mode }, undefined);
        if (typeof value !== "number") {
            throw new TypeError("the shared owner answered a transaction with another value");
        }

        return { owner, id: value };
    }

    /** Leave the channel, failing every unanswered request. */
    close(): void {
        this.#stop();
        for (const pending of this.#pending.values()) {
            pending.reject(new DatabaseError("CONNECTION_CLOSED", "the shared connection closed"));
        }
        this.#pending.clear();
    }

    /** Follow the serving owner and settle its answers. */
    #receive(message: Message): void {
        // resend queued requests and fail those the previous owner may have run
        if (message.kind === "serving" && message.owner !== this.#owner) {
            this.#owner = message.owner;
            for (const [id, pending] of this.#pending) {
                if (pending.owner === undefined) {
                    this.#send(id, pending, message.owner);
                } else {
                    this.#pending.delete(id);
                    pending.reject(
                        new DatabaseError("OWNER_CHANGED", "the owner changed before answering"),
                    );
                }
            }
        }
        // settle a sent request once, ignoring answers to settled ones
        else if (message.kind === "answer" && message.to === this.#name) {
            const pending = this.#pending.get(message.id);
            this.#pending.delete(message.id);
            if (pending?.owner === undefined) {
                return;
            } else if (message.error === undefined) {
                pending.resolve({ value: message.value, owner: pending.owner });
            } else {
                pending.reject(Object.assign(new Error(message.error.message), message.error));
            }
        }
    }

    /** Send one step, queued until an owner serves. */
    #ask(
        step: Step,
        pinned: string | undefined,
    ): Promise<{ readonly value: unknown; readonly owner: string }> {
        // refuse a step of a stopped owner
        if (pinned !== undefined && pinned !== this.#owner) {
            return Promise.reject(
                new DatabaseError("OWNER_CHANGED", "the owner of the transaction changed"),
            );
        }

        // send to the owner or queue
        const id = this.#next++;

        return new Promise((resolve, reject) => {
            const request: Request = { step, owner: undefined, resolve, reject };
            this.#pending.set(id, request);
            if (this.#owner !== undefined) {
                this.#send(id, request, this.#owner);
            }
        });
    }

    /** Address a request to an owner. */
    #send(id: number, request: Request, owner: string): void {
        request.owner = owner;
        this.#channel.notify({
            kind: "request",
            from: this.#name,
            to: owner,
            id,
            step: request.step,
        });
    }
}

/** A transaction an owner keeps open for a party. */
export interface SharedTransaction {
    /** The owner of the transaction. */
    readonly owner: string;
    /** The owner's number for the transaction. */
    readonly id: number;
}

/** A pending request. */
interface Request {
    /** What to run. */
    readonly step: Step;
    /** The owner the request went to, absent while queued. */
    owner: string | undefined;
    /** Settle with the owner's result. */
    readonly resolve: (answer: { readonly value: unknown; readonly owner: string }) => void;
    /** Settle with the owner's failure. */
    readonly reject: (error: unknown) => void;
}

/** Carry a value to the owner, bytes in a buffer of their own as structured cloning copies them. */
function sharedValue(value: DriverValue): DriverValue {
    return value instanceof Uint8Array ? new Uint8Array(value) : value;
}

/** Describe a failure for the channel. */
function describeError(error: unknown): { name: string; message: string; code?: string } {
    const failure = error instanceof Error ? error : new Error(String(error));
    const code = errorCode(failure);

    return {
        name: failure.name,
        message: failure.message,
        ...(code === undefined ? {} : { code }),
    };
}
