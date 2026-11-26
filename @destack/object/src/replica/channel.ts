import { Caller } from "@destack/access";
import { type Channel, Condition, Table, typedChannel } from "@destack/db";
import { type Environment, untilAborted } from "@destack/environment";
import { Failure } from "@destack/error";
import {
    Duration,
    type JsonObject,
    type JsonValue,
    mapValues,
    present,
    schema,
} from "@destack/schema";
import * as sync from "@destack/sync";
import type { Subject } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { traceCalls } from "../procedure/telemetry.ts";
import type {
    LiveQuery,
    MutationContext,
    SearchOptions,
    SearchResult,
    Submission,
    Writer,
} from "../client/access.ts";
import { undoEntry } from "../client/undo.ts";
import { Method } from "../method/method.ts";
import type { ObjectNode } from "../client/node.ts";
import {
    type LiveItems,
    RelationalQuery,
    type SubscribeOptions,
    type Subscriber,
} from "../client/query.ts";
import { ObjectType } from "../object/object.ts";
import { RelationGraph } from "../query/query.ts";
import type { ObjectRemote, Push } from "./remote.ts";
import { ObjectQuery, PushResult } from "./replica.ts";

/** The fields of a read a party follows at the serving node. */
const READ = {
    scope: schema.string(),
    query: ObjectQuery,
    ttl: schema.union([schema.literal("forever"), Duration]).exactOptional(),
};

/** The fields of a call of an object type's method. */
/** An object type as the channel refers to it. */
const TYPE = schema.object({ package: schema.string(), object: schema.string() });

/** A search's hits as the channel sends them, each type by reference. */
const SEARCH_RESULT = schema.object({
    text: schema.string(),
    isCorrected: schema.boolean(),
    hits: schema.array(
        schema.object({
            type: TYPE,
            reference: sync.ObjectReference,
            row: schema.jsonObject(),
            relevance: schema.number(),
            excerpt: schema.string().exactOptional(),
        }),
    ),
});

/** A call of an object type's method as the channel sends it. */
const CALL = {
    package: schema.string(),
    object: schema.string(),
    method: schema.string(),
    input: schema.jsonObject(),
};

/** The stages a write session tells its party: open for invokes, then predicted once committed. */
const SESSION_STAGE = schema.enum(["open", "predicted"]);

/** The verbs a channel's parties send its serving node, one per remote verb. */
const Send = schema.discriminatedUnion("verb", [
    schema.object({ verb: schema.literal("stream"), subscription: sync.Subscription }),
    schema.object({ verb: schema.literal("request"), subscription: sync.Subscription }),
    schema.object({
        verb: schema.literal("push"),
        home: schema.string(),
        mutations: schema.array(sync.Mutation),
        writerId: schema.string().exactOptional(),
        chain: schema.array(schema.string()),
        origin: schema.string().exactOptional(),
    }),
    schema.object({ verb: schema.literal("call"), scope: schema.string(), call: sync.Call }),
    schema.object({
        verb: schema.literal("broadcast"),
        scope: schema.string(),
        object: sync.ObjectReference.omit({ scope: true }),
        event: schema.json(),
    }),

    // a node's reads and writes, answered by the serving node's client of the scope
    schema.object({ verb: schema.literal("rows"), ...READ }),
    schema.object({ verb: schema.literal("results"), ...READ }),
    schema.object({
        verb: schema.literal("search"),
        scope: schema.string(),
        text: schema.string(),
        types: schema.array(TYPE),
        limit: schema.number(),
        where: Condition.schema.exactOptional(),
        typos: schema.enum(["correct", "keep"]).exactOptional(),
    }),
    schema.object({ verb: schema.literal("mutate"), scope: schema.string(), ...CALL }),
    schema.object({
        verb: schema.literal("mutation"),
        scope: schema.string(),
        session: schema.string(),
    }),
    schema.object({ verb: schema.literal("invoke"), session: schema.string(), ...CALL }),
    schema.object({ verb: schema.literal("commit"), session: schema.string() }),
    schema.object({ verb: schema.literal("cancel"), session: schema.string() }),
    schema.object({
        verb: schema.literal("find"),
        scope: schema.string(),
        package: schema.string(),
        object: schema.string(),
        reference: sync.ObjectReference,
    }),
    schema.object({
        verb: schema.literal("check"),
        scope: schema.string(),
        package: schema.string(),
        object: schema.string(),
        id: schema.string(),
        permission: schema.string(),
    }),
    schema.object({ verb: schema.literal("undo"), scope: schema.string() }),
    schema.object({ verb: schema.literal("redo"), scope: schema.string() }),
    schema.object({ verb: schema.literal("close"), scope: schema.string() }),
    schema.object({
        verb: schema.literal("latest"),
        scope: schema.string(),
        state: schema.enum(["done", "undone"]),
    }),
    schema.object({ verb: schema.literal("history"), scope: schema.string() }),
    schema.object({ verb: schema.literal("ready"), scope: schema.string() }),
    schema.object({ verb: schema.literal("chain"), scope: schema.string(), of: schema.string() }),
]);
/** A verb a party sends the serving node. */
type Send = schema.Infer<typeof Send>;

/** A frame between a channel's parties and the node serving them, as structured clones. */
const Frame = schema.discriminatedUnion("kind", [
    // presence
    schema.object({ kind: schema.literal("probe") }),
    schema.object({ kind: schema.literal("serving") }),
    schema.object({ kind: schema.literal("stopped") }),

    // a party's sends
    schema.object({
        kind: schema.literal("send"),
        from: schema.string(),
        id: schema.number().int(),
        send: Send,
    }),
    schema.object({
        kind: schema.literal("abort"),
        from: schema.string(),
        id: schema.number().int(),
    }),

    // the node's answers
    schema.object({
        kind: schema.literal("page"),
        to: schema.string(),
        id: schema.number().int(),
        page: sync.Page,
    }),
    schema.object({
        kind: schema.literal("value"),
        to: schema.string(),
        id: schema.number().int(),
        value: schema.unknown(),
    }),
    schema.object({
        kind: schema.literal("answer"),
        to: schema.string(),
        id: schema.number().int(),
        value: schema.json(),
    }),
    schema.object({
        kind: schema.literal("fail"),
        to: schema.string(),
        id: schema.number().int(),
        failure: Failure,
    }),
]);
/** A frame between a channel's parties and the node serving them. */
type Frame = schema.Infer<typeof Frame>;

/** An open send of a party: its pages, then its answer or failure. */
interface OpenSend {
    /** Take a page of a stream. */
    readonly page: (page: sync.Page) => void;
    /** Take a value before the answer: a read's items, or a write's prediction. */
    readonly value: (value: unknown) => void;
    /** Take the answer, a stream's end. */
    readonly answer: (value: JsonValue) => void;
    /** Take the failure. */
    readonly fail: (error: Error) => void;
}

/** The RPC system of a channel's calls, as OpenTelemetry's `rpc.system.name` reports it. */
const CHANNEL_SYSTEM = "destack.channel";

/** What a channel's serving side answers with. */
interface ChannelServed {
    /** The node's remote, for the replica verbs. */
    readonly remote: ObjectRemote;
    /** The principal the node answers as. */
    readonly signer: Subject;
    /** The node of each scope. */
    readonly nodes: (scope: string) => Promise<ObjectNode>;
    /** Each party's followed queries, read again before its write's prediction. */
    readonly follows: Map<string, Set<() => Promise<void>>>;
    /** The parties' open write sessions, by session. */
    readonly sessions: Map<string, ChannelSession>;
}

/** A party's open write session at the serving node, until it commits or cancels. */
interface ChannelSession {
    /** The node the session writes to. */
    readonly node: ObjectNode;
    /** The mutation the invokes predict in. */
    readonly mutation: MutationContext;
    /** Resolves to queue the mutation, rejects to drop it. */
    readonly closed: PromiseWithResolvers<void>;
}

/** A node reached over a channel of one origin, such as a device from its tabs. */
export class ChannelRemote implements ObjectRemote {
    /** The channel the frames travel on. */
    readonly #channel: Channel<Frame>;
    /** This party. */
    readonly #party: string;
    /** The next send's number. */
    #next = 0;
    /** The open sends, by number. */
    readonly #open = new Map<number, OpenSend>();
    /** Settles once a node serves the channel, renewed each time it stops. */
    #serving = Promise.withResolvers<void>();
    /** Trace this party's calls: a span and a duration each. */
    readonly #traced: ReturnType<typeof traceCalls>;
    /** The environment the party runs in, whose randomness names its write sessions. */
    readonly #environment: Environment;
    /** Never aborts: a channel's node does not move. */
    readonly moves = new AbortController().signal;

    /** Join a channel as one party in an environment, probing for its node. */
    constructor(channel: Channel<unknown>, party: string, environment: Environment) {
        // listen to the node's frames, then ask whether one serves
        this.#channel = typedChannel(channel, Frame);
        this.#party = party;
        this.#traced = traceCalls("client", CHANNEL_SYSTEM, environment);
        this.#environment = environment;
        this.#channel.listen({ receive: (frame) => this.#receive(frame) });
        this.#channel.notify({ kind: "probe" });
    }

    /** Serve a node's remote to every party of a channel as a principal, until stopped. */
    static serve(
        channel: Channel<unknown>,
        remote: ObjectRemote,
        signer: Subject,
        environment: Environment,
        nodes: (scope: string) => Promise<ObjectNode>,
    ): () => void {
        // answer each send and probe, tracing each answer, and abort a send its party abandons
        const typed = typedChannel(channel, Frame);
        const traced = traceCalls("server", CHANNEL_SYSTEM, environment);
        const running = new Map<string, AbortController>();
        const follows = new Map<string, Set<() => Promise<void>>>();
        const sessions = new Map<string, ChannelSession>();
        const stop = typed.listen({
            receive: (frame) => {
                // answer a probe
                if (frame.kind === "probe") {
                    typed.notify({ kind: "serving" });
                }
                // answer a send
                else if (frame.kind === "send") {
                    const key = `${frame.from} ${frame.id}`;
                    const controller = new AbortController();
                    running.set(key, controller);
                    void traced([frame.send.verb], () =>
                        ChannelRemote.#answer(
                            typed,
                            frame,
                            { remote, signer, nodes, follows, sessions },
                            controller.signal,
                        ),
                    ).finally(() => running.delete(key));
                }
                // abort an abandoned one
                else if (frame.kind === "abort") {
                    running.get(`${frame.from} ${frame.id}`)?.abort();
                }
            },
        });
        typed.notify({ kind: "serving" });

        // tell the parties the node stopped before aborting its answers
        return () => {
            typed.notify({ kind: "stopped" });
            stop();
            for (const controller of running.values()) {
                controller.abort();
            }
        };
    }

    /** Stream a replica's pages from the node. */
    async *stream(subscription: sync.Subscription, signal: AbortSignal): AsyncGenerator<sync.Page> {
        yield* this.#iterate({ verb: "stream", subscription }, signal, (take) => ({ page: take }));
    }

    /** Record a subscription's moved parameters at the node streaming it. */
    async request(subscription: sync.Subscription): Promise<void> {
        await this.#send({ verb: "request", subscription });
    }

    /** Push one home's mutations through the node, which forwards them to their home. */
    async push(home: string, push: Push, signal?: AbortSignal): Promise<PushResult> {
        const { mutations, writerId, chain, origin } = push;
        const answer = await this.#send(
            {
                verb: "push",
                home,
                mutations: [...mutations],
                ...(writerId === undefined ? {} : { writerId }),
                chain: [...chain],
                ...(origin === undefined ? {} : { origin }),
            },
            signal === undefined ? {} : { signal },
        );

        return PushResult.parse(answer);
    }

    /** Answer a reading call at the node. */
    call(scope: string, call: sync.Call): Promise<JsonValue> {
        return this.#send({ verb: "call", scope, call });
    }

    /** Follow queries of one scope at the node, compiled here to shape their items. */
    subscriber(scope: string, objects: readonly ObjectType[]): Subscriber {
        // compile each query as the node does, for the includes that nest its items
        const relations = new RelationGraph(ObjectType.served([...objects]));
        const compile = (object: ObjectType, query: Omit<ObjectQuery, "object" | "package">) => {
            const whole = ObjectQuery.parse({
                object: object.name,
                package: object.package.id,
                ...query,
            });
            const [compiled] = Object.values(relations.queries({ query: whole }, [scope]));

            return present(compiled, `the compiled query of ${object.name}`);
        };
        const items = schema.array(sync.Item);
        const groups = schema.array(sync.AggregateGroup);

        return {
            rows: (object, query, options): LiveItems => ({
                query: compile(object, query),
                ...this.#live(ChannelRemote.#read("rows", scope, object, query, options), (value) =>
                    items.parse(value),
                ),
            }),
            results: (object, query, options) =>
                this.#live(ChannelRemote.#read("results", scope, object, query, options), (value) =>
                    groups.parse(value),
                ),
        };
    }

    /** Call an object type's mutating method at the node, predicted there first. */
    mutate(
        scope: string,
        object: ObjectType,
        method: string,
        input: JsonObject,
    ): Submission<unknown> {
        // take the node's prediction, then its confirmation
        const predicted = Promise.withResolvers<unknown>();
        const confirmed = this.#send(
            {
                verb: "mutate",
                scope,
                package: object.package.id,
                object: object.name,
                method,
                input,
            },
            { value: (value) => predicted.resolve(value) },
        ).then(() => {});

        // fail the prediction with a failure before it, and mark the confirmation handled
        confirmed.catch((error: unknown) => predicted.reject(error));
        confirmed.catch(() => {});

        return { predicted: predicted.promise, confirmed };
    }

    /** Predict several calls at the node as one mutation, the run itself staying here. */
    mutation<Value>(
        scope: string,
        run: (mutation: MutationContext) => Promise<Value>,
    ): Submission<Value> {
        // open the session, taking its opening and its prediction as the node's values
        const session = sync.RequestId.create(this.#environment);
        const opened = Promise.withResolvers<void>();
        const predictedThere = Promise.withResolvers<void>();
        const confirmed = this.#send(
            { verb: "mutation", scope, session },
            {
                value: (stage) => {
                    // take the session's stage: open for invokes, then predicted once committed
                    if (SESSION_STAGE.parse(stage) === "open") {
                        opened.resolve();
                    } else {
                        predictedThere.resolve();
                    }
                },
            },
        ).then(() => {});
        confirmed.catch((error: unknown) => {
            opened.reject(error);
            predictedThere.reject(error);
        });
        confirmed.catch(() => {});

        // invoke each call at the node, then commit, or cancel once the run fails
        const invoke = (object: ObjectType, method: string, input: JsonObject) =>
            this.#send({
                verb: "invoke",
                session,
                package: object.package.id,
                object: object.name,
                method,
                input,
            });
        const call = <Object extends ObjectType>(object: Object): Writer<Object> =>
            mapValues<Readonly<Record<string, Method>>, Writer<Object>>(
                ChannelRemote.#mutating(object),
                (_method, name) => (input: JsonObject) => invoke(object, name, input),
            );
        const predicted = (async () => {
            // run once the session opens, keeping the run's result
            await opened.promise;
            let result: Value;
            try {
                result = await run({ call, invoke });
            } catch (error) {
                await this.#send({ verb: "cancel", session });
                throw error;
            }
            await this.#send({ verb: "commit", session });
            await predictedThere.promise;

            return result;
        })();

        return { predicted, confirmed };
    }

    /** Read an object's row at the node, absent where neither it nor the server has it. */
    async find(
        scope: string,
        object: ObjectType,
        reference: sync.ObjectReference,
    ): Promise<JsonObject | undefined> {
        const value = await this.#send({
            verb: "find",
            scope,
            package: object.package.id,
            object: object.name,
            reference,
        });

        return value === null ? undefined : schema.jsonObject().parse(value);
    }

    /** Read a scope's chain at a scope's node. */
    async chain(scope: string, of: string): Promise<readonly string[]> {
        return schema.array(schema.string()).parse(await this.#send({ verb: "chain", scope, of }));
    }

    /** Settle once the scope's node has the caller's access rows. */
    async ready(scope: string): Promise<void> {
        await this.#send({ verb: "ready", scope });
    }

    /** Check at the node whether the caller has a permission on an object. */
    async check(
        scope: string,
        object: ObjectType,
        id: string,
        permission: string,
    ): Promise<boolean> {
        const value = await this.#send({
            verb: "check",
            scope,
            package: object.package.id,
            object: object.name,
            id,
            permission,
        });

        return schema.boolean().parse(value);
    }

    /** The person's undo and redo at the node, for one scope. */
    undoStack(scope: string): ObjectNode["undoStack"] {
        // undo or redo, taking the prediction first
        const toggle = (verb: "undo" | "redo"): Submission<boolean> => {
            // take the prediction from the node's value, the confirmation from its answer
            const predicted = Promise.withResolvers<boolean>();
            const confirmed = this.#send(
                { verb, scope },
                { value: (value) => predicted.resolve(schema.boolean().parse(value)) },
            ).then(() => {});
            confirmed.catch((error: unknown) => predicted.reject(error));
            confirmed.catch(() => {});

            return { predicted: predicted.promise, confirmed };
        };
        const history = schema.object({ canUndo: schema.boolean(), canRedo: schema.boolean() });
        const iterate = this.#iterate.bind(this);

        return {
            undo: () => toggle("undo"),
            redo: () => toggle("redo"),
            close: async () => {
                await this.#send({ verb: "close", scope });
            },
            latest: async (state) => {
                const entry = await this.#send({ verb: "latest", scope, state });

                return entry === null ? undefined : Table.selectSchema(undoEntry).parse(entry);
            },
            watch: (signal) =>
                iterate({ verb: "history", scope }, signal, (take) => ({
                    value: (value) => take(history.parse(value)),
                })),
        };
    }

    /** Yield a send's pages or values in order until it ends, rethrowing its failure. */
    async *#iterate<Value>(
        send: Send,
        signal: AbortSignal,
        receive: (take: (value: Value) => void) => {
            readonly page?: (page: sync.Page) => void;
            readonly value?: (value: unknown) => void;
        },
    ): AsyncGenerator<Value> {
        // queue what arrives, ending at the answer
        const taken: Value[] = [];
        let ended: { readonly error?: Error } | undefined;
        let wake: (() => void) | undefined;
        const take = (value: Value) => {
            taken.push(value);
            wake?.();
        };
        const sent = this.#send(send, { signal, ...receive(take) });
        sent.then(
            () => {
                ended = {};
                wake?.();
            },
            (error: unknown) => {
                ended = { error: error instanceof Error ? error : new Error(String(error)) };
                wake?.();
            },
        );

        // yield in order, waiting while nothing is left
        while (true) {
            const next = taken.shift();
            if (next !== undefined) {
                yield next;
            } else if (ended?.error !== undefined) {
                throw ended.error;
            } else if (ended !== undefined) {
                return;
            } else {
                const { promise, resolve } = Promise.withResolvers<void>();
                wake = resolve;
                await promise;
            }
        }
    }

    /** List an object type's routed mutating methods. */
    static #mutating(object: ObjectType): Readonly<Record<string, Method>> {
        return Object.fromEntries(
            Object.entries(object.methods).filter(
                ([, method]) => method.mutates && Method.isRouted(method),
            ),
        );
    }

    /** Search some object types of one scope at the node, its hits' types read back here. */
    search(
        scope: string,
        objects: readonly ObjectType[],
        text: string,
        options: SearchOptions,
    ): LiveQuery<SearchResult> {
        // refer to each type by package and name
        const { types, limit, where, typos } = options;
        const send: Send = {
            verb: "search",
            scope,
            text,
            types: types.map((type) => ({ package: type.package.id, object: type.name })),
            limit,
            ...(where === undefined ? {} : { where }),
            ...(typos === undefined ? {} : { typos }),
        };

        // read each hit's type back among the node's types
        return this.#live(send, (value) => {
            const result = SEARCH_RESULT.parse(value);
            const hits = result.hits.map((hit) => ({
                ...hit,
                type: ChannelRemote.#type(objects, hit.type.package, hit.type.object),
            }));

            return { ...result, hits };
        });
    }

    /** Describe a read of one object type at the node. */
    static #read(
        verb: "rows" | "results",
        scope: string,
        object: ObjectType,
        query: Omit<ObjectQuery, "object" | "package">,
        options: SubscribeOptions,
    ): Send {
        const whole = ObjectQuery.parse({
            object: object.name,
            package: object.package.id,
            ...query,
        });

        return {
            verb,
            scope,
            query: whole,
            ...(options.ttl === undefined ? {} : { ttl: options.ttl }),
        };
    }

    /** Follow a send at the node, keeping its latest value and waking each watch on a new one. */
    #live<Value>(send: Send, parse: (value: unknown) => Value): LiveQuery<Value> {
        // take each value the node sends, the first making the query ready
        const closing = new AbortController();
        const first = Promise.withResolvers<void>();
        let latest: { readonly value: Value } | undefined;
        let changed = Promise.withResolvers<void>();
        const sent = this.#send(send, {
            signal: closing.signal,
            value: (value) => {
                // keep the value and wake the watches
                latest = { value: parse(value) };
                first.resolve();
                const woken = changed;
                changed = Promise.withResolvers<void>();
                woken.resolve();
            },
        });

        // fail readiness when the node refuses the query, unless closed
        sent.catch((error: unknown) => {
            if (!closing.signal.aborted) {
                first.reject(error);
            }
        });
        first.promise.catch(() => {});
        const current = async () => {
            await first.promise;

            return present(latest, "the query's value").value;
        };

        return {
            ready: first.promise,
            read: current,
            watch: async function* (signal) {
                // yield the current value, then each new one until aborted
                yield await current();
                while (!signal.aborted) {
                    const isChanged = await untilAborted(changed.promise, signal).then(
                        () => true,
                        () => false,
                    );
                    if (!isChanged) {
                        return;
                    }
                    yield await current();
                }
            },
            close: async () => {
                closing.abort();
                await sent.catch(() => {});
            },
        };
    }

    /** Send an unstored event to an object's readers through the node. */
    async broadcast(
        scope: string,
        object: Omit<sync.ObjectReference, "scope">,
        event: JsonValue,
    ): Promise<void> {
        await this.#send({ verb: "broadcast", scope, object, event });
    }

    /** Follow no moves: a channel's node does not move. */
    redirect(): boolean {
        return false;
    }

    /** Leave the channel, abandoning the open sends at the node and failing them here. */
    async close(): Promise<void> {
        for (const [id, open] of this.#open) {
            this.#channel.notify({ kind: "abort", from: this.#party, id });
            open.fail(
                ChannelRemote.#unavailable(
                    "the channel's node is unavailable: the party left the channel",
                ),
            );
        }
        this.#open.clear();
        await this.#channel.close();
    }

    /** Send a verb once the node serves, answering its answer until a signal aborts. */
    async #send(
        send: Send,
        options: {
            /** Abandons the send. */
            readonly signal?: AbortSignal;
            /** Take a page of a stream. */
            readonly page?: (page: sync.Page) => void;
            /** Take a value before the answer. */
            readonly value?: (value: unknown) => void;
        } = {},
    ): Promise<JsonValue> {
        // wait for the node unless abandoned first, then number the send
        const { signal, page = () => {}, value = () => {} } = options;
        await (signal === undefined
            ? this.#serving.promise
            : untilAborted(this.#serving.promise, signal));
        signal?.throwIfAborted();
        const id = this.#next;
        this.#next += 1;
        const answered = Promise.withResolvers<JsonValue>();
        this.#open.set(id, { page, value, answer: answered.resolve, fail: answered.reject });

        // abandon it once its signal aborts
        const abandon = () => {
            this.#open.delete(id);
            this.#channel.notify({ kind: "abort", from: this.#party, id });
            answered.reject(signal?.reason);
        };
        signal?.addEventListener("abort", abandon, { once: true });

        // send it and wait for its answer, in the call's span
        try {
            return await this.#traced([send.verb], () => {
                this.#channel.notify({ kind: "send", from: this.#party, id, send });

                return answered.promise;
            });
        } finally {
            signal?.removeEventListener("abort", abandon);
        }
    }

    /** Take the node's presence, or an answer frame of this party's open sends. */
    #receive(frame: Frame): void {
        // send once a node serves
        if (frame.kind === "serving") {
            this.#serving.resolve();
        }
        // fail the open sends once it stops, holding later ones until the next node serves
        else if (frame.kind === "stopped") {
            this.#serving = Promise.withResolvers<void>();
            for (const open of this.#open.values()) {
                open.fail(
                    ChannelRemote.#unavailable("the channel's node is unavailable: it stopped"),
                );
            }
            this.#open.clear();
        }
        // take a page, an answer or a failure of this party's open send
        else if (
            (frame.kind === "page" ||
                frame.kind === "value" ||
                frame.kind === "answer" ||
                frame.kind === "fail") &&
            frame.to === this.#party
        ) {
            const open = this.#open.get(frame.id);
            if (frame.kind === "page") {
                open?.page(frame.page);
            } else if (frame.kind === "value") {
                open?.value(frame.value);
            } else if (frame.kind === "answer") {
                this.#open.delete(frame.id);
                open?.answer(frame.value);
            } else {
                this.#open.delete(frame.id);
                open?.fail(Failure.error(frame.failure));
            }
        }
    }

    /** Answer one send with the node's remote as the principal. */
    static async #answer(
        channel: Channel<Frame>,
        frame: Extract<Frame, { readonly kind: "send" }>,
        served: ChannelServed,
        signal: AbortSignal,
    ): Promise<void> {
        const { from: to, id, send } = frame;
        const { remote, signer, nodes, follows, sessions } = served;
        try {
            // stream the pages, then end
            if (send.verb === "stream") {
                for await (const page of remote.stream(send.subscription, signal)) {
                    channel.notify({ kind: "page", to, id, page });
                }
                channel.notify({ kind: "answer", to, id, value: null });
            }
            // stream a read's values from the scope's node until abandoned
            else if (send.verb === "rows" || send.verb === "results" || send.verb === "search") {
                const party = follows.get(to) ?? new Set();
                follows.set(to, party);
                await ChannelRemote.#follow(
                    channel,
                    frame,
                    ChannelRemote.#followed(send, await nodes(send.scope)),
                    party,
                    signal,
                ).finally(() => {
                    // forget a party once it follows nothing
                    if (party.size === 0) {
                        follows.delete(to);
                    }
                });
            }
            // answer a write's prediction, then its confirmation
            else if (send.verb === "mutate") {
                const node = await nodes(send.scope);
                const object = ChannelRemote.#object(node, send.package, send.object);
                const submission = node.call(object, send.method, send.input);
                const predicted = await submission.predicted;

                // send the party's followed queries with the prediction before the prediction itself
                await Promise.all([...(follows.get(to) ?? [])].map((refresh) => refresh()));
                channel.notify({ kind: "value", to, id, value: predicted });
                await submission.confirmed;
                channel.notify({ kind: "answer", to, id, value: null });
            }
            // open a write session, predicting the party's invokes as one mutation until it commits
            else if (send.verb === "mutation") {
                const node = await nodes(send.scope);
                const closed = Promise.withResolvers<void>();
                const submission = node.mutation(async (mutation) => {
                    sessions.set(send.session, { node, mutation, closed });
                    channel.notify({ kind: "value", to, id, value: "open" });
                    await closed.promise;
                });
                await submission.predicted.finally(() => sessions.delete(send.session));

                // send the party's followed queries with the prediction, then the prediction and its confirmation
                await Promise.all([...(follows.get(to) ?? [])].map((refresh) => refresh()));
                channel.notify({ kind: "value", to, id, value: "predicted" });
                await submission.confirmed;
                channel.notify({ kind: "answer", to, id, value: null });
            }
            // predict one call of a party's open session
            else if (send.verb === "invoke") {
                const open = present(sessions.get(send.session), `session '${send.session}'`);
                const object = ChannelRemote.#object(open.node, send.package, send.object);
                const value = await open.mutation.invoke(object, send.method, send.input);
                channel.notify({
                    kind: "answer",
                    to,
                    id,
                    value: schema.json().parse(value ?? null),
                });
            }
            // end a party's session, queueing its mutation or dropping it
            else if (send.verb === "commit" || send.verb === "cancel") {
                const open = present(sessions.get(send.session), `session '${send.session}'`);
                if (send.verb === "commit") {
                    open.closed.resolve();
                } else {
                    open.closed.reject(new Error("the party cancelled the mutation"));
                }
                channel.notify({ kind: "answer", to, id, value: null });
            }
            // read an object's row, or whether the caller has a permission on it
            else if (send.verb === "find" || send.verb === "check") {
                const node = await nodes(send.scope);
                const object = ChannelRemote.#object(node, send.package, send.object);
                const value =
                    send.verb === "find"
                        ? ((await node.find(object, send.reference)) ?? null)
                        : await node.check(object, send.id, send.permission);
                channel.notify({ kind: "answer", to, id, value });
            }
            // undo or redo the person's latest group, answering its prediction, then its confirmation
            else if (send.verb === "undo" || send.verb === "redo") {
                const { undoStack } = await nodes(send.scope);
                const submission = send.verb === "undo" ? undoStack.undo() : undoStack.redo();
                channel.notify({ kind: "value", to, id, value: await submission.predicted });
                await submission.confirmed;
                channel.notify({ kind: "answer", to, id, value: null });
            }
            // close the open group, or read the latest entry in a state
            else if (send.verb === "close" || send.verb === "latest") {
                const { undoStack } = await nodes(send.scope);
                const value =
                    send.verb === "close"
                        ? await undoStack.close().then(() => null)
                        : schema.json().parse((await undoStack.latest(send.state)) ?? null);
                channel.notify({ kind: "answer", to, id, value });
            }
            // answer a scope's chain at the scope's node
            else if (send.verb === "chain") {
                const chain = await (await nodes(send.scope)).chain(send.of);
                channel.notify({ kind: "answer", to, id, value: [...chain] });
            }
            // answer once the scope's node has the caller's access rows
            else if (send.verb === "ready") {
                await (
                    await nodes(send.scope)
                ).ready;
                channel.notify({ kind: "answer", to, id, value: null });
            }
            // stream what the person can undo and redo until abandoned
            else if (send.verb === "history") {
                const { undoStack } = await nodes(send.scope);
                for await (const value of undoStack.watch(signal)) {
                    channel.notify({ kind: "value", to, id, value });
                }
                channel.notify({ kind: "answer", to, id, value: null });
            }
            // answer any other verb
            else {
                const value = await ChannelRemote.#run(send, remote, signer);
                channel.notify({ kind: "answer", to, id, value });
            }
        } catch (error) {
            // answer the failure, unless its party abandoned it
            if (!signal.aborted) {
                channel.notify({ kind: "fail", to, id, failure: Failure.of(error) });
            }
        }
    }

    /** Stream a read's values from a node: the first once ready, then one per change. */
    static async #follow(
        channel: Channel<Frame>,
        frame: Extract<Frame, { readonly kind: "send" }>,
        live: LiveQuery<unknown>,
        party: Set<() => Promise<void>>,
        signal: AbortSignal,
    ): Promise<void> {
        // follow the read on the node, as long as its party keeps it
        const { from: to, id } = frame;
        const refresh = async () => {
            channel.notify({ kind: "value", to, id, value: await live.read() });
        };
        try {
            // send each read of the query, and a read whenever the party's write predicts
            await live.ready;
            party.add(refresh);
            for await (const value of live.watch(signal)) {
                channel.notify({ kind: "value", to, id, value });
            }
        } finally {
            party.delete(refresh);
            await live.close();
        }
    }

    /** Follow a read on a node, its values as the channel sends them. */
    static #followed(
        send: Extract<Send, { readonly verb: "rows" | "results" | "search" }>,
        node: ObjectNode,
    ): LiveQuery<unknown> {
        // search the named types, sending each hit's type by reference
        if (send.verb === "search") {
            const { scope: _scope, verb: _verb, text, types, ...options } = send;
            const searched = node.search(text, {
                ...options,
                types: types.map((type) => ChannelRemote.#object(node, type.package, type.object)),
            });

            return RelationalQuery.presented(searched, (result) =>
                SEARCH_RESULT.parse({
                    ...result,
                    hits: result.hits.map((hit) => ({
                        ...hit,
                        type: { package: hit.type.package.id, object: hit.type.name },
                    })),
                }),
            );
        }

        // follow a read of one object type
        const { object: name, package: packageId, ...query } = send.query;
        const object = ChannelRemote.#object(node, packageId, name);
        const options = send.ttl === undefined ? {} : { ttl: send.ttl };

        return send.verb === "rows"
            ? node.subscriber.rows(object, query, options)
            : node.subscriber.results(object, query, options);
    }

    /** Find a node's object type by package and name, refusing one it lacks. */
    static #object(node: ObjectNode, packageId: string, name: string): ObjectType {
        return ChannelRemote.#type(node.objects, packageId, name);
    }

    /** Find an object type among some by package and name, refusing one they lack. */
    static #type(objects: readonly ObjectType[], packageId: string, name: string): ObjectType {
        const object = objects.find((type) => type.package.id === packageId && type.name === name);
        if (object === undefined) {
            throw new TypeError(`the node has no object '${name}'`);
        }

        return object;
    }

    /** Run a verb other than a stream with the node's remote as the principal. */
    static async #run(
        send: Extract<Send, { readonly verb: "request" | "push" | "call" | "broadcast" }>,
        remote: ObjectRemote,
        signer: Subject,
    ): Promise<JsonValue> {
        // record a stream's moved parameters
        if (send.verb === "request") {
            await remote.request(send.subscription);

            return null;
        }
        // push mutations as the principal
        else if (send.verb === "push") {
            const { home, verb: _verb, ...push } = send;

            return schema
                .json()
                .parse(await remote.push(home, { ...push, signer: Caller.of(signer) }));
        }
        // answer a reading call as the principal
        else if (send.verb === "call") {
            return remote.call(send.scope, send.call, Caller.of(signer));
        }
        // send an event as the principal
        else {
            await remote.broadcast(send.scope, send.object, send.event, Caller.of(signer));

            return null;
        }
    }

    /** Build the failure of a send the node could not answer, which the party retries. */
    static #unavailable(message: string): Error {
        return ObjectError.of("SERVER_UNAVAILABLE", message, {
            data: {},
        });
    }
}
