import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type * as declaration from "../../declare/database.ts";
import { relayNotifier } from "../../log/notifier.ts";
import type { Relay } from "../../relay/relay.ts";
import type { Table } from "../../table/table.ts";
import { SqliteDatabase } from "../database.ts";
import { DatabaseError } from "../../error/error.ts";
import type { ConnectionClient, QueryClient, Statement } from "../client.ts";

/**
 * The idle timeout of a party's transaction at the owner, in milliseconds.
 *
 * A transaction holds the owner's only connection, and this bounds how long a frozen tab blocks the others.
 */
const IDLE_TRANSACTION_MILLISECONDS = 30_000;

/** A message between the parties of a shared database. */
export type Message =
    | {
          /** A statement or transaction step for the owner. */
          readonly kind: "request";
          /** The asking party. */
          readonly from: string;
          /** The owner that answers. */
          readonly to: string;
          /** The request number. */
          readonly id: number;
          /** What to run. */
          readonly step: Step;
      }
    | {
          /** The owner's answer. */
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
          /** A commit every party may read. */
          readonly kind: "commit";
      }
    | {
          /** An owner serving from now on. */
          readonly kind: "serving";
          /** The owner, new each time a party starts serving. */
          readonly owner: string;
      }
    | {
          /** A party asking which owner serves. */
          readonly kind: "join";
      };

/** One statement or transaction step. */
export type Step =
    | {
          /** Run a statement, in a transaction when named. */
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

/** Serve a connection to a relay's other parties until stopped. */
export function serveDatabase<Result>(
    client: ConnectionClient<Result>,
    relay: Relay<Message>,
): () => void {
    // name this owner and hold party transactions open
    const owner = crypto.randomUUID();
    const transactions = new Map<number, HeldTransaction>();
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
            if (step.method === "exec") {
                return target.exec(step.sql);
            }
            const statement = (await target.prepare(step.sql))
                .safeIntegers(step.isSafe)
                .raw(step.isRaw);

            return await statement[step.method](...step.parameters);
        }
        // begin a transaction
        else if (step.type === "begin") {
            const id = next++;
            const opened = await HeldTransaction.begin(client, step.mode, () =>
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

    // name this owner to a joining party
    const stop = relay.listen((message) => {
        if (message.kind === "join") {
            relay.post({ kind: "serving", owner });
        }
        // answer each request to this owner
        else if (message.kind === "request" && message.to === owner) {
            run(message.step).then(
                (value) => relay.post({ kind: "answer", to: message.from, id: message.id, value }),
                (error: unknown) =>
                    relay.post({
                        kind: "answer",
                        to: message.from,
                        id: message.id,
                        error: describeError(error),
                    }),
            );
        }
    });

    // announce this owner
    relay.post({ kind: "serving", owner });

    // roll back open transactions on stop
    return () => {
        stop();
        for (const open of transactions.values()) {
            void open.end(false).catch(() => undefined);
        }
    };
}

/** A transaction the owner holds for a party. */
class HeldTransaction {
    /** The transaction's client. */
    readonly client: QueryClient<unknown>;
    /** Resolve on commit and reject on rollback. */
    readonly done: Promise<void>;
    /** End the holding callback. */
    readonly #finish: (commit: boolean) => void;
    /** Forget the transaction. */
    readonly #forget: () => void;
    /** The idle rollback timer. */
    #idle?: ReturnType<typeof setTimeout>;

    /** Create the transaction. */
    private constructor(
        client: QueryClient<unknown>,
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

    /** Begin and hold a transaction. */
    static begin(
        connection: ConnectionClient<unknown>,
        mode: "deferred" | "immediate" | "exclusive",
        forget: () => void,
    ): Promise<HeldTransaction> {
        return new Promise((resolve, reject) => {
            // hold the callback open until the party ends it
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
        this.#idle = setTimeout(
            () => void this.end(false).catch(() => undefined),
            IDLE_TRANSACTION_MILLISECONDS,
        );
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

/** Open a database with statements that the relay's owner runs. */
export function connectShared(
    relay: Relay<Message>,
    party: string,
    tables: declaration.Database | readonly Table[] = [],
    options: Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> = {},
): SqliteDatabase<SharedClient> {
    const client = new SharedClient(relay, party);

    return new SqliteDatabase(client, tables, "embedded", relayNotifier(relay), options);
}

/** Statements the relay's owner runs. */
export class SharedQuery implements QueryClient<unknown> {
    /** The party asking the owner. */
    readonly party: Party;
    /** The owner's transaction. */
    readonly transaction: SharedTransaction | undefined;

    /** Create the client. */
    constructor(party: Party, transaction?: SharedTransaction) {
        this.party = party;
        this.transaction = transaction;
    }

    /** Prepare a statement the owner runs. */
    async prepare(sql: string): Promise<Statement<unknown>> {
        // track the statement's modes
        let isRaw = false;
        let isSafe = false;
        const run = (method: "run" | "all" | "get", parameters: readonly unknown[]) =>
            this.#statement({ method, sql, parameters, isRaw, isSafe });

        // set the modes on each run
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
        return this.#statement({
            method: "exec",
            sql: script,
            parameters: [],
            isRaw: false,
            isSafe: false,
        });
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

/** A connection client with work that the relay's owner runs. */
export class SharedClient extends SharedQuery implements ConnectionClient<unknown> {
    /** Join the relay as one party. */
    constructor(relay: Relay<Message>, name: string) {
        super(new Party(relay, name));
    }

    /** Stop reaching the owner. */
    async close(): Promise<void> {
        this.party.close();
    }

    /** Run a callback in a transaction the owner holds. */
    transactionAsync<Value>(operation: (client: QueryClient<unknown>) => Promise<Value>) {
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
                await this.party
                    .request({ type: "rollback", transaction: id }, owner)
                    .catch(() => undefined);
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

/** One party of a relay, asking its owner to run steps. */
export class Party {
    /** The relay reaching the owner. */
    readonly #relay: Relay<Message>;
    /** This party's name on the relay. */
    readonly #name: string;
    /** The unanswered requests by number. */
    readonly #pending = new Map<number, Request>();
    /** The next request number. */
    #next = 0;
    /** The serving owner. */
    #owner: string | undefined;
    /** Stop receiving answers. */
    readonly #stop: () => void;

    /** Join a relay and ask which owner serves. */
    constructor(relay: Relay<Message>, name: string) {
        // listen for answers and ask for the owner
        this.#relay = relay;
        this.#name = name;
        this.#stop = relay.listen((message) => this.#receive(message));
        relay.post({ kind: "join" });
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

        return { owner, id: value as number };
    }

    /** Leave the relay, failing every unanswered request. */
    close(): void {
        this.#stop();
        for (const pending of this.#pending.values()) {
            pending.reject(new DatabaseError("CONNECTION_CLOSED", "the shared connection closed"));
        }
        this.#pending.clear();
    }

    /** Follow the serving owner and settle its answers. */
    #receive(message: Message): void {
        // resend held requests and fail those the previous owner may have run
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
                pending?.resolve({ value: message.value, owner: pending.owner! });
            } else {
                pending?.reject(Object.assign(new Error(message.error.message), message.error));
            }
        }
    }

    /** Send one step, held until an owner serves. */
    #ask(
        step: Step,
        pinned: string | undefined,
    ): Promise<{ readonly value: unknown; readonly owner: string }> {
        // refuse a step of a stopped owner
        if (pinned !== undefined && pinned !== this.#owner) {
            return Promise.reject(
                new DatabaseError("OWNER_CHANGED", "the owner holding the transaction changed"),
            );
        }

        // send to the owner or hold
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
        this.#relay.post({
            kind: "request",
            from: this.#name,
            to: owner,
            id,
            step: request.step,
        });
    }
}

/** A transaction an owner holds for a party. */
export interface SharedTransaction {
    /** The owner holding the transaction. */
    readonly owner: string;
    /** The owner's number for the transaction. */
    readonly id: number;
}

/** A pending request. */
interface Request {
    /** What to run. */
    readonly step: Step;
    /** The owner the request went to, absent while held. */
    owner: string | undefined;
    /** Settle with the owner's result. */
    readonly resolve: (answer: { readonly value: unknown; readonly owner: string }) => void;
    /** Settle with the owner's failure. */
    readonly reject: (error: unknown) => void;
}

/** Describe a failure for the relay. */
function describeError(error: unknown): { name: string; message: string; code?: string } {
    const failure = error instanceof Error ? error : new Error(String(error));
    const code = (failure as { code?: unknown }).code;

    return {
        name: failure.name,
        message: failure.message,
        ...(typeof code === "string" ? { code } : {}),
    };
}
