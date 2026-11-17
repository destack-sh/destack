import type * as sync from "@destack/sync";
import type { Subject } from "@destack/sync";
import {
    type Identifier,
    type JsonObject,
    type JsonValue,
    nth,
    present,
    schema,
    type UuidEnvironment,
} from "@destack/schema";
import { Failure } from "@destack/error";
import type { ObjectType } from "../object/object.ts";
import { ObjectDescription } from "../object/description.ts";
import { MAX_PAGE_LIMIT } from "../method/page.ts";
import { Method } from "../method/method.ts";
import type { Environment } from "@destack/environment";
import type { Client } from "../procedure/index.ts";
import { ObjectError } from "../error/error.ts";
import { CallError } from "../error/call.ts";
import { createClient, type ClientOptions } from "../client/package.ts";
import {
    type ObjectQuery,
    storeProcedures,
    type PushResult,
    type StoreProcedures,
} from "./replica.ts";

/** The results of a mutation's calls, in call order. */
const CALL_RESULTS = schema.array(schema.json());

/** A page of a list method: its rows and the cursor continuing it, null after the last. */
const ListPage = schema.looseObject({
    items: schema.array(schema.jsonObject()),
    cursor: schema.string().nullable(),
});

/** One home's mutations a follower pushes through its remote. */
export interface Push {
    /** The mutations, in the order the follower committed them. */
    readonly mutations: readonly sync.Mutation[];
    /** The follower instance writing ephemeral objects, absent for durable mutations. */
    readonly writerId?: string;
    /** The scopes the follower's replica reads, nearest first. */
    readonly chain: readonly string[];
    /** The principal a forwarding node signs the mutations as, absent for the caller's own. */
    readonly signer?: Subject;
    /** The pushing node's log, under which a queueing receiver publishes the outcomes. */
    readonly origin?: string;
}

/** A node another node follows and writes through: the machine serving a scope, or a device. */
export interface ObjectRemote extends sync.Publisher {
    /** Push one home's mutations, answering their outcomes and watermark. */
    readonly push: (home: string, push: Push, signal?: AbortSignal) => Promise<PushResult>;
    /** Answer a reading call in the scope it reads, as a signing principal or the caller. */
    readonly call: (scope: string, call: sync.Call, signer?: Subject) => Promise<JsonValue>;
    /** Send an unstored event to an object's readers, as a signing principal or the caller. */
    readonly broadcast: (
        scope: string,
        object: Omit<sync.ObjectReference, "scope">,
        event: JsonValue,
        signer?: Subject,
    ) => Promise<void>;
    /** Aborts once the remote moved, for streams to follow it anew. */
    readonly moves: AbortSignal;
    /** Follow a move a failure under a move signal reports, answering whether it was one. */
    readonly redirect: (error: unknown, moves: AbortSignal) => boolean;
}

/** A node another node follows and writes through. */
export const ObjectRemote = {
    /** Report whether a follow source also takes writes, as a remote does. */
    is(publisher: sync.Publisher): publisher is ObjectRemote {
        return "push" in publisher;
    },

    /** Push one mutation to its home, answering its results or refusal. */
    async run(
        remote: ObjectRemote,
        home: string,
        mutation: sync.Mutation,
        signer?: Subject,
    ): Promise<readonly JsonValue[]> {
        // push the mutation alone, signed as given
        const pushed = await remote.push(home, {
            mutations: [mutation],
            chain: [],
            ...(signer === undefined ? {} : { signer }),
        });

        // answer its results, or throw its refusal
        const { outcome } = present(pushed.outcomes[0], `the outcome of mutation '${mutation.id}'`);
        if (!("value" in outcome)) {
            throw Failure.error(outcome.error);
        }

        return CALL_RESULTS.parse(outcome.value);
    },
};

/** A remote store reached through its procedures, and other scopes' homes. */
export class ProcedureRemote implements ObjectRemote {
    /** The scope the follower replicates, absent for a remote of a whole home. */
    readonly scope: string | undefined;
    /** The store procedures of the machine serving the scope now. */
    #store: Client<StoreProcedures>;
    /** Open the store procedures as a signing principal, absent for none. */
    readonly #signing: ((signer: Subject) => Client<StoreProcedures>) | undefined;
    /** Open the procedures of a moved scope's machine, absent to follow no moves. */
    readonly #reconnect: ((machine: Identifier<"machine">) => Client<StoreProcedures>) | undefined;
    /** Open the store procedures of an enclosing scope's home, with the scope they enclose. */
    readonly #homes:
        | {
              readonly scope: string;
              readonly open: (scope: string) => Client<StoreProcedures>;
          }
        | undefined;
    /** The connected homes of enclosing scopes, by scope. */
    readonly #connected = new Map<string, Client<StoreProcedures>>();
    /** The clock and randomness of the requests the remote identifies. */
    readonly #environment: UuidEnvironment;
    /** Aborts the streams following the machine that served a moved scope before. */
    #moves = new AbortController();

    /** Reach the store serving a scope through its procedures. */
    constructor(options: {
        /** The scope the follower replicates, absent for a remote of a whole home. */
        readonly scope?: string;
        /** The store procedures of the machine serving the scope. */
        readonly store: Client<StoreProcedures>;
        /** Open them as a signing principal, absent for a follower that signs as none. */
        readonly signing?: (signer: Subject) => Client<StoreProcedures>;
        /** Open those of the machine serving a moved scope now, absent to follow no moves. */
        readonly reconnect?: (machine: Identifier<"machine">) => Client<StoreProcedures>;
        /** Open those of an enclosing scope's home whose rows the follower changes. */
        readonly homes?: (scope: string) => Client<StoreProcedures>;
        /** The clock and randomness of the requests the remote identifies. */
        readonly environment: UuidEnvironment;
    }) {
        // take the scope and the procedures, requiring the scope enclosing homes reach out of
        this.scope = options.scope;
        this.#environment = options.environment;
        this.#store = options.store;
        this.#signing = options.signing;
        this.#reconnect = options.reconnect;
        const { scope, homes } = options;
        this.#homes =
            homes === undefined
                ? undefined
                : { scope: present(scope, "the scope the enclosing homes enclose"), open: homes };
    }

    /** Reach the store serving a scope at an endpoint, with its moves and homes. */
    static at(options: {
        /** The scope the follower replicates. */
        readonly scope: string;
        /** Where and how to call the store serving the scope. */
        readonly endpoint: ClientOptions;
        /** Where to call a moved scope's machine, absent to follow no moves. */
        readonly reconnect?: (machine: Identifier<"machine">) => ClientOptions;
        /** Where and how to call the home of an enclosing scope whose rows the follower changes. */
        readonly homes?: (scope: string) => ClientOptions;
        /** The environment the calls run in. */
        readonly environment: Environment;
    }): ProcedureRemote {
        const { reconnect, homes, environment } = options;
        const open = (endpoint: ClientOptions) => ProcedureRemote.procedures(endpoint, environment);

        return new ProcedureRemote({
            scope: options.scope,
            store: open(options.endpoint),
            ...(reconnect === undefined
                ? {}
                : { reconnect: (machine) => open(reconnect(machine)) }),
            ...(homes === undefined ? {} : { homes: (scope) => open(homes(scope)) }),
            environment,
        });
    }

    /** Call a store's procedures, speaking the object package's release. */
    static procedures(endpoint: ClientOptions, environment: Environment): Client<StoreProcedures> {
        const router = { store: storeProcedures };

        return createClient({ router }, endpoint, environment).store;
    }

    /** Aborts once the scope moved, for streams to follow its new machine. */
    get moves(): AbortSignal {
        return this.#moves.signal;
    }

    /** Stream a replica's pages from the machine serving the scope. */
    async *stream(subscription: sync.Subscription, signal: AbortSignal): AsyncGenerator<sync.Page> {
        yield* await this.#store.stream(subscription, { signal });
    }

    /** Record a subscription's moved parameters at the machine streaming it. */
    async request(subscription: sync.Subscription): Promise<void> {
        await this.#store.request({ subscription });
    }

    /** Push one home's mutations, reading the replica's watermark after a home commits. */
    async push(home: string, push: Push, signal?: AbortSignal): Promise<PushResult> {
        // push signed mutations as their signer
        const { mutations, writerId, chain, signer, origin } = push;
        const options = signal === undefined ? {} : { signal };
        const input = {
            scope: home,
            mutations: [...mutations],
            ...(writerId === undefined ? {} : { writerId }),
            ...(origin === undefined ? {} : { origin }),
        };
        if (signer !== undefined) {
            return this.#signed(signer).push(input, options);
        }

        // push to the machine serving the scope, which forwards replicated rows to their home
        if (home === this.scope || this.#homes === undefined) {
            return this.#call((store) => store.push(input, options));
        }

        // push an enclosing scope's mutations to its home
        const pushed = await this.home(home).push(input, options);

        // read the replica's watermark, waiting for the home's commit when the chain has the home
        const homes = chain.includes(home) ? [pushed.watermark] : [];
        const scope = present(this.#homes, "the enclosed scope").scope;
        const watermark = await this.#call((store) => store.watermark({ scope, homes }, options));

        return { outcomes: pushed.outcomes, queued: pushed.queued, watermark };
    }

    /** Answer a reading call at the scope's machine, as a signer or the caller. */
    call(scope: string, call: sync.Call, signer?: Subject): Promise<JsonValue> {
        return signer === undefined
            ? this.#call((store) => store.call({ scope, call }))
            : this.#signed(signer).call({ scope, call });
    }

    /** Send an unstored event to an object's readers, as a signing principal or the caller. */
    async broadcast(
        scope: string,
        object: Omit<sync.ObjectReference, "scope">,
        event: JsonValue,
        signer?: Subject,
    ): Promise<void> {
        const input = { scope, object, event };
        await (signer === undefined
            ? this.#call((store) => store.broadcast(input))
            : this.#signed(signer).broadcast(input));
    }

    /** Describe object types the store knows, every served one when given none. */
    async describe(types?: readonly sync.ObjectTypeReference[]): Promise<ObjectDescription[]> {
        // ask nothing for an empty list
        if (types?.length === 0) {
            return [];
        }

        // describe them at the machine serving the scope
        const { objects } = await this.#call((store) =>
            store.describe(types === undefined ? {} : { types: [...types] }),
        );

        return objects.map((object) => ObjectDescription.parse(object));
    }

    /** Follow a move a failure under a move signal reports, answering whether it was one. */
    redirect(error: unknown, moves: AbortSignal): boolean {
        return this.#follow(error, moves === this.#moves.signal);
    }

    /** Run a routed method by name with JSON input in the follower's scope. */
    async run(object: ObjectType, name: string, input: JsonObject): Promise<JsonValue> {
        // answer a query in the scope it reads
        const scope = present(this.scope, `the scope '${object.name}.${name}' acts in`);
        const built = object.callFrom(name, input, scope, this.#environment);
        if (built.kind === "query") {
            return this.call(object.readScope(name, built.call.input), built.call);
        }

        // push a change to the home of the scope it acts in, failing with its refusal
        const { mutation } = built;
        const home = object.readScope(name, nth(mutation.calls, 0).input);
        const [result = null] = await ObjectRemote.run(this, home, mutation);

        return result;
    }

    /** List a type's rows through its list method, page by page up to the query's limit. */
    async list(
        object: ObjectType,
        query: Pick<ObjectQuery, "where" | "orderBy" | "limit" | "permission"> &
            Readonly<Record<string, unknown>> = {},
    ): Promise<readonly JsonObject[]> {
        // find the type's list method
        const method = Object.entries(object.methods).find(
            ([, declared]) => declared.kind === "list" && Method.isRouted(declared),
        )?.[0];
        if (method === undefined) {
            throw ObjectError.of(
                "LIST_METHOD_NOT_FOUND",
                `list method of ${object.name} not found`,
            );
        }

        // read pages until the last one or the limit, each continuing from the one before
        const input = schema.jsonObject().parse(query);
        const { limit } = query;
        const rows: JsonObject[] = [];
        let cursor: string | null = null;
        do {
            const size = Math.min(MAX_PAGE_LIMIT, (limit ?? Infinity) - rows.length);
            const page = ListPage.parse(
                await this.run(object, method, {
                    ...input,
                    limit: size,
                    ...(cursor === null ? {} : { cursor }),
                }),
            );
            rows.push(...page.items);
            cursor = page.cursor;
        } while (cursor !== null && rows.length < (limit ?? Infinity));

        return rows;
    }

    /** Call a scope's home: the machine serving the scope, else each enclosing home once. */
    home(home: string): Client<StoreProcedures> {
        // call the follower's scope at the machine serving it, refusing another scope without homes
        if (home === this.scope) {
            return this.#store;
        } else if (this.#homes === undefined) {
            throw ObjectError.of(
                "HOME_INVALID",
                `home '${home}' is invalid: the follower of '${String(this.scope)}' cannot reach it`,
            );
        }

        // connect to each other scope's home once
        const connected = this.#connected.get(home) ?? this.#homes.open(home);
        this.#connected.set(home, connected);

        return connected;
    }

    /** Open the store procedures as a signing principal, refusing a follower that signs as none. */
    #signed(signer: Subject): Client<StoreProcedures> {
        return present(this.#signing, "the remote's signing procedures")(signer);
    }

    /** Call the machine serving the scope now, retrying once after a move. */
    async #call<Value>(send: (store: Client<StoreProcedures>) => Promise<Value>): Promise<Value> {
        const store = this.#store;
        try {
            return await send(store);
        } catch (error) {
            // retry once after a move
            if (!this.#follow(error, store === this.#store)) {
                throw error;
            }

            return send(this.#store);
        }
    }

    /** Reconnect to the machine serving a moved scope, unless a move since was followed already. */
    #follow(error: unknown, isCurrent: boolean): boolean {
        // ignore other failures, and moves of a remote following none
        const moved = CallError.data(error, "MISDIRECTED_REQUEST");
        const reconnect = this.#reconnect;
        if (moved === undefined || !("machine" in moved) || reconnect === undefined) {
            return false;
        }

        // reconnect once per move
        if (isCurrent) {
            this.#store = reconnect(moved.machine);
            this.#moves.abort();
            this.#moves = new AbortController();
        }

        return true;
    }
}
