import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type * as declaration from "../declare/database.ts";
import { channelNotifier } from "../log/notifier.ts";
import type { Channel } from "../channel/channel.ts";
import type { Table } from "../table/table.ts";
import { SqliteDatabase } from "../sqlite/database.ts";
import { DatabaseError } from "../error/error.ts";
import type { ConnectionClient, QueryClient, Statement } from "../sqlite/client.ts";

/**
 * How long an owner keeps a party's transaction open without a statement, in milliseconds.
 *
 * A transaction holds the owner's only connection, so this bounds how long a frozen or closed tab blocks the others.
 */
const IDLE_TRANSACTION_MILLISECONDS = 30_000;

/** A message between the parties of a shared database. */
export type Message =
    | {
          /** A statement or transaction step a party asks the owner to run. */
          readonly kind: "request";
          /** The asking party. */
          readonly from: string;
          /** The owner asked to run it, which alone answers. */
          readonly to: string;
          /** The request, answered once. */
          readonly id: number;
          /** What to run. */
          readonly step: Step;
      }
    | {
          /** The owner's answer to one request. */
          readonly kind: "answer";
          /** The asking party. */
          readonly to: string;
          /** The answered request. */
          readonly id: number;
          /** The result, absent on failure. */
          readonly value?: unknown;
          /** The failure, absent on success. */
          readonly error?: {
              readonly name: string;
              readonly message: string;
              readonly code?: string;
          };
      }
    | {
          /** A party committed changes every party may read. */
          readonly kind: "commit";
      }
    | {
          /** An owner serves, answering the requests addressed to it from now on. */
          readonly kind: "serving";
          /** The owner, new each time a party starts serving. */
          readonly owner: string;
      }
    | {
          /** A party joined and asks which owner serves. */
          readonly kind: "join";
      };

/** One statement or transaction step. */
export type Step =
    | {
          /** Run a statement, within a transaction when named. */
          readonly type: "statement";
          /** How to run it. */
          readonly method: "run" | "all" | "get" | "exec";
          /** The SQL. */
          readonly sql: string;
          /** The bound parameters. */
          readonly parameters: readonly unknown[];
          /** Whether rows come back as arrays. */
          readonly isRaw: boolean;
          /** Whether integers come back exactly. */
          readonly isSafe: boolean;
          /** The transaction, absent outside one. */
          readonly transaction?: number;
      }
    | {
          /** Begin a transaction. */
          readonly type: "begin";
          /** How to begin it. */
          readonly mode: "deferred" | "immediate" | "exclusive";
      }
    | {
          /** End a transaction. */
          readonly type: "commit" | "rollback";
          /** The transaction. */
          readonly transaction: number;
      };

/** Serve a connection to the other parties of a channel until the returned stop runs, each party announcing its own commits. */
export function serveDatabase<Result>(
    client: ConnectionClient<Result>,
    channel: Channel<Message>,
): () => void {
    // name this owner, and hold each party transaction open until its party ends it or falls silent
    const owner = crypto.randomUUID();
    const transactions = new Map<number, HeldTransaction>();
    let next = 0;

    // run one step, answering its party
    const run = async (step: Step): Promise<unknown> => {
        // run a statement on the connection or in a transaction
        if (step.type === "statement") {
            const open =
                step.transaction === undefined ? undefined : transactions.get(step.transaction);
            if (step.transaction !== undefined && open === undefined) {
                throw new DatabaseError("TRANSACTION_CLOSED", "the shared transaction has ended");
            }
            open?.touch();
            const target = open?.client ?? client;
            if (step.method === "exec") {
                return target.exec(step.sql);
            }
            const statement = (await target.prepare(step.sql))
                .safeIntegers(step.isSafe)
                .raw(step.isRaw);

            return await statement[step.method](...step.parameters);
        }
        // begin a transaction whose statements arrive as later steps
        else if (step.type === "begin") {
            const id = next++;
            const opened = await HeldTransaction.begin(client, step.mode, () =>
                transactions.delete(id),
            );
            transactions.set(id, opened);

            return id;
        }
        // end a transaction, answering once it committed or rolled back
        else {
            const open = transactions.get(step.transaction);
            if (open === undefined) {
                throw new DatabaseError("TRANSACTION_CLOSED", "the shared transaction has ended");
            }
            await open.end(step.type === "commit");

            return undefined;
        }
    };

    // name this owner to a joining party
    const stop = channel.listen((message) => {
        if (message.kind === "join") {
            channel.post({ kind: "serving", owner });
        }
        // answer each request addressed to this owner
        else if (message.kind === "request" && message.to === owner) {
            run(message.step).then(
                (value) =>
                    channel.post({ kind: "answer", to: message.from, id: message.id, value }),
                (error: unknown) =>
                    channel.post({
                        kind: "answer",
                        to: message.from,
                        id: message.id,
                        error: describeError(error),
                    }),
            );
        }
    });

    // tell the parties this owner serves, so they send what they held
    channel.post({ kind: "serving", owner });

    // roll back every open transaction once serving stops
    return () => {
        stop();
        for (const open of transactions.values()) {
            void open.end(false).catch(() => undefined);
        }
    };
}

/** A transaction the owner holds open for a party, rolled back once the party falls silent. */
class HeldTransaction {
    /** The transaction's client. */
    readonly client: QueryClient<unknown>;
    /** Settles once the transaction committed, rejecting once it rolled back. */
    readonly done: Promise<void>;
    /** End the callback holding the transaction open. */
    readonly #finish: (commit: boolean) => void;
    /** Forget the transaction once it ended. */
    readonly #forget: () => void;
    /** Roll back once the party stays silent. */
    #idle?: ReturnType<typeof setTimeout>;

    /** Retain an open transaction. */
    private constructor(
        client: QueryClient<unknown>,
        done: Promise<void>,
        finish: (commit: boolean) => void,
        forget: () => void,
    ) {
        // keep the client and how the transaction ends
        this.client = client;
        this.done = done;
        this.#finish = finish;
        this.#forget = forget;
        this.touch();
    }

    /** Begin a transaction on a connection and hold it open until it ends. */
    static begin(
        connection: ConnectionClient<unknown>,
        mode: "deferred" | "immediate" | "exclusive",
        forget: () => void,
    ): Promise<HeldTransaction> {
        return new Promise((resolve, reject) => {
            // hold the transaction's callback open until the party ends it
            let opened: HeldTransaction | undefined;
            const done = connection
                .transactionAsync(
                    (transaction) =>
                        new Promise<void>((finish, abort) => {
                            const end = (commit: boolean) =>
                                commit
                                    ? finish()
                                    : abort(new Error("the shared transaction rolled back"));
                            opened = new HeldTransaction(transaction, done, end, forget);
                            resolve(opened);
                        }),
                )
                [mode]();

            // report a failure to begin, and forget the transaction once it ends
            done.catch((error: unknown) => {
                if (opened === undefined) {
                    reject(error);
                }
            });
            done.finally(forget).catch(() => undefined);
        });
    }

    /** Keep the transaction open while its party keeps using it. */
    touch(): void {
        clearTimeout(this.#idle);
        this.#idle = setTimeout(
            () => void this.end(false).catch(() => undefined),
            IDLE_TRANSACTION_MILLISECONDS,
        );
    }

    /** Commit or roll back, settling once the connection finished. */
    async end(commit: boolean): Promise<void> {
        // stop the idle timer and end the callback holding the transaction
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

/** Open a database whose statements the channel's owner runs, notified of the owner's commits. */
export function connectShared(
    channel: Channel<Message>,
    party: string,
    tables: declaration.Database | readonly Table[] = [],
    options: Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> = {},
): SqliteDatabase<SharedClient> {
    const client = new SharedClient(channel, party);

    return new SqliteDatabase(client, tables, "embedded", channelNotifier(channel), options);
}

/** Statements a channel's owner runs, on its connection or within one of its transactions. */
export class SharedQuery implements QueryClient<unknown> {
    /** The party asking the owner. */
    readonly party: Party;
    /** The owner's transaction, absent outside one. */
    readonly transaction: number | undefined;

    /** Ask the owner through a party, within a transaction when named. */
    constructor(party: Party, transaction?: number) {
        this.party = party;
        this.transaction = transaction;
    }

    /** Prepare a statement the owner runs in the statement's current row and integer modes. */
    async prepare(sql: string): Promise<Statement<unknown>> {
        // track the modes the statement runs in
        let isRaw = false;
        let isSafe = false;
        const run = (method: "run" | "all" | "get", parameters: readonly unknown[]) =>
            this.party.request({
                type: "statement",
                method,
                sql,
                parameters,
                isRaw,
                isSafe,
                ...(this.transaction === undefined ? {} : { transaction: this.transaction }),
            });

        // change modes in place, as local statements do
        const statement: Statement<unknown> = {
            safeIntegers: (enabled) => {
                isSafe = enabled;

                return statement;
            },
            raw: (enabled) => {
                isRaw = enabled;

                return statement;
            },
            run: (...parameters) => run("run", parameters),
            all: (...parameters) => run("all", parameters) as Promise<unknown[]>,
            get: (...parameters) => run("get", parameters),
        };

        return statement;
    }

    /** Run a statement. */
    async run(sql: string, ...parameters: unknown[]): Promise<unknown> {
        return (await this.prepare(sql)).run(...parameters);
    }

    /** Read rows as named objects. */
    async all(sql: string, ...parameters: unknown[]): Promise<unknown[]> {
        return (await this.prepare(sql)).all(...parameters);
    }

    /** Read the first row as a named object. */
    async get(sql: string, ...parameters: unknown[]): Promise<unknown> {
        return (await this.prepare(sql)).get(...parameters);
    }

    /** Run a script. */
    exec(script: string): Promise<unknown> {
        return this.party.request({
            type: "statement",
            method: "exec",
            sql: script,
            parameters: [],
            isRaw: false,
            isSafe: false,
            ...(this.transaction === undefined ? {} : { transaction: this.transaction }),
        });
    }
}

/** A connection client whose statements and transactions a channel's owner runs. */
export class SharedClient extends SharedQuery implements ConnectionClient<unknown> {
    /** Reach a channel's owner as one party. */
    constructor(channel: Channel<Message>, name: string) {
        super(new Party(channel, name));
    }

    /** Stop reaching the owner. */
    async close(): Promise<void> {
        this.party.close();
    }

    /** Run a callback in a transaction the owner holds open. */
    transactionAsync<Value>(operation: (client: QueryClient<unknown>) => Promise<Value>) {
        const begin = async (mode: "deferred" | "immediate" | "exclusive") => {
            // begin at the owner and run the callback within it
            const transaction = (await this.party.request({ type: "begin", mode })) as number;
            const scoped = new SharedQuery(this.party, transaction);

            // end the transaction by the callback's outcome
            try {
                const value = await operation(scoped);
                await this.party.request({ type: "commit", transaction });

                return value;
            } catch (error) {
                await this.party.request({ type: "rollback", transaction }).catch(() => undefined);
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

/** One party of a channel, asking its owner to run steps and settling each on the owner's answer. */
export class Party {
    /** The channel reaching the owner. */
    readonly #channel: Channel<Message>;
    /** This party's name on the channel. */
    readonly #name: string;
    /** The unanswered requests by number, with the owner each went to, absent while held. */
    readonly #pending = new Map<number, Request>();
    /** The next request number. */
    #next = 0;
    /** The owner serving the channel, absent until one announces itself. */
    #owner: string | undefined;
    /** Stop receiving answers. */
    readonly #stop: () => void;

    /** Join a channel under a name, and ask which owner serves it. */
    constructor(channel: Channel<Message>, name: string) {
        // listen for the owner's answers, then ask which owner serves
        this.#channel = channel;
        this.#name = name;
        this.#stop = channel.listen((message) => this.#receive(message));
        channel.post({ kind: "join" });
    }

    /** Send one step to the owner, held until an owner serves, and wait for its answer. */
    request(step: Step): Promise<unknown> {
        const id = this.#next++;

        return new Promise((resolve, reject) => {
            const request: Request = { step, owner: undefined, resolve, reject };
            this.#pending.set(id, request);
            if (this.#owner !== undefined) {
                this.#send(id, request, this.#owner);
            }
        });
    }

    /** Leave the channel, failing every unanswered request. */
    close(): void {
        this.#stop();
        for (const pending of this.#pending.values()) {
            pending.reject(new DatabaseError("CONNECTION_CLOSED", "the shared connection closed"));
        }
        this.#pending.clear();
    }

    /** Follow the serving owner, and settle the requests it answers. */
    #receive(message: Message): void {
        // send held requests to a new owner, and fail those the previous owner may have run
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
        // settle a request once
        else if (message.kind === "answer" && message.to === this.#name) {
            const pending = this.#pending.get(message.id);
            this.#pending.delete(message.id);
            if (message.error === undefined) {
                pending?.resolve(message.value);
            } else {
                pending?.reject(Object.assign(new Error(message.error.message), message.error));
            }
        }
    }

    /** Address a request to an owner. */
    #send(id: number, request: Request, owner: string): void {
        request.owner = owner;
        this.#channel.post({
            kind: "request",
            from: this.#name,
            to: owner,
            id,
            step: request.step,
        });
    }
}

/** A request a party awaits an answer to. */
interface Request {
    /** What to run. */
    readonly step: Step;
    /** The owner the request went to, absent while held. */
    owner: string | undefined;
    /** Settle with the owner's result. */
    readonly resolve: (value: unknown) => void;
    /** Settle with the owner's failure. */
    readonly reject: (error: unknown) => void;
}

/** Describe a failure so it crosses the channel. */
function describeError(error: unknown): { name: string; message: string; code?: string } {
    const failure = error instanceof Error ? error : new Error(String(error));
    const code = (failure as { code?: unknown }).code;

    return {
        name: failure.name,
        message: failure.message,
        ...(typeof code === "string" ? { code } : {}),
    };
}
