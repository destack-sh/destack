import { type Channel, typedChannel } from "@destack/db";
import { type Environment, waitAborted } from "@destack/environment";
import { Failure } from "@destack/error";
import { type JsonValue, schema } from "@destack/schema";
import * as sync from "@destack/sync";
import type { Subject } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { traceCalls } from "../procedure/telemetry.ts";
import type { ObjectRemote, Push } from "./remote.ts";
import { PushResult } from "./replica.ts";

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
    /** Take the answer, a stream's end. */
    readonly answer: (value: JsonValue) => void;
    /** Take the failure. */
    readonly fail: (error: Error) => void;
}

/** The RPC system of a channel's calls, as OpenTelemetry's `rpc.system.name` reports it. */
const CHANNEL_SYSTEM = "destack.channel";

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
    /** Never aborts: a channel's node does not move. */
    readonly moves = new AbortController().signal;

    /** Join a channel as one party in an environment, probing for its node. */
    constructor(channel: Channel<unknown>, party: string, environment: Environment) {
        // listen to the node's frames, then ask whether one serves
        this.#channel = typedChannel(channel, Frame);
        this.#party = party;
        this.#traced = traceCalls("client", CHANNEL_SYSTEM, environment);
        this.#channel.listen({ receive: (frame) => this.#receive(frame) });
        this.#channel.notify({ kind: "probe" });
    }

    /** Serve a node's remote to every party of a channel as a principal, until stopped. */
    static serve(
        channel: Channel<unknown>,
        remote: ObjectRemote,
        signer: Subject,
        environment: Environment,
    ): () => void {
        // answer each send and probe, tracing each answer, and abort a send its party abandons
        const typed = typedChannel(channel, Frame);
        const traced = traceCalls("server", CHANNEL_SYSTEM, environment);
        const running = new Map<string, AbortController>();
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
                        ChannelRemote.#answer(typed, frame, remote, signer, controller.signal),
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
        // queue the pages as they arrive, ending at the answer
        const pages: sync.Page[] = [];
        let ended: { readonly error?: Error } | undefined;
        let wake: (() => void) | undefined;
        const sent = this.#send({ verb: "stream", subscription }, signal, (page) => {
            pages.push(page);
            wake?.();
        });
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

        // yield them in order, waiting while none is left
        while (true) {
            const page = pages.shift();
            if (page !== undefined) {
                yield page;
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
            signal,
        );

        return PushResult.parse(answer);
    }

    /** Answer a reading call at the node. */
    call(scope: string, call: sync.Call): Promise<JsonValue> {
        return this.#send({ verb: "call", scope, call });
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
        signal?: AbortSignal,
        page: (page: sync.Page) => void = () => {},
    ): Promise<JsonValue> {
        // wait for the node unless abandoned first, then number the send
        const abandoned = signal === undefined ? undefined : waitAborted(signal);
        await (abandoned === undefined
            ? this.#serving.promise
            : Promise.race([this.#serving.promise, abandoned]));
        signal?.throwIfAborted();
        const id = this.#next;
        this.#next += 1;
        const answered = Promise.withResolvers<JsonValue>();
        this.#open.set(id, { page, answer: answered.resolve, fail: answered.reject });

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
            (frame.kind === "page" || frame.kind === "answer" || frame.kind === "fail") &&
            frame.to === this.#party
        ) {
            const open = this.#open.get(frame.id);
            if (frame.kind === "page") {
                open?.page(frame.page);
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
        remote: ObjectRemote,
        signer: Subject,
        signal: AbortSignal,
    ): Promise<void> {
        const { from: to, id, send } = frame;
        try {
            // stream the pages, then end
            if (send.verb === "stream") {
                for await (const page of remote.stream(send.subscription, signal)) {
                    channel.notify({ kind: "page", to, id, page });
                }
                channel.notify({ kind: "answer", to, id, value: null });

                return;
            }

            // answer any other verb
            const value = await ChannelRemote.#run(send, remote, signer);
            channel.notify({ kind: "answer", to, id, value });
        } catch (error) {
            // answer the failure, unless its party abandoned it
            if (!signal.aborted) {
                channel.notify({ kind: "fail", to, id, failure: Failure.of(error) });
            }
        }
    }

    /** Run a verb other than a stream with the node's remote as the principal. */
    static async #run(
        send: Exclude<Send, { readonly verb: "stream" }>,
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

            return schema.json().parse(await remote.push(home, { ...push, signer }));
        }
        // answer a reading call as the principal
        else if (send.verb === "call") {
            return remote.call(send.scope, send.call, signer);
        }
        // send an event as the principal
        else {
            await remote.broadcast(send.scope, send.object, send.event, signer);

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
