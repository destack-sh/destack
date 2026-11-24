import type * as sync from "@destack/sync";
import type { Caller } from "@destack/access";
import {
    type JsonObject,
    type JsonValue,
    nth,
    present,
    schema,
    type UuidEnvironment,
} from "@destack/schema";
import type { ObjectType } from "../object/object.ts";
import { Failure } from "@destack/error";
import { waitAborted } from "@destack/environment";
import type { PushResult } from "./replica.ts";

/** The results of a mutation's calls, in call order. */
const CALL_RESULTS = schema.array(schema.json());

/** Whom a forwarding node signs as: a principal it holds keys for, or one acting under grants. */
export type Signer = Pick<Caller, "subject" | "actors" | "grants">;

/** One home's mutations a follower pushes through its remote. */
export interface Push {
    /** The mutations, in the order the follower committed them. */
    readonly mutations: readonly sync.Mutation[];
    /** The follower instance writing ephemeral objects, absent for durable mutations. */
    readonly writerId?: string;
    /** The scopes the follower's replica reads, nearest first. */
    readonly chain: readonly string[];
    /** The principal a forwarding node signs the mutations as, absent for the caller's own. */
    readonly signer?: Signer;
    /** The pushing node's log, under which a queueing receiver publishes the outcomes. */
    readonly origin?: string;
}

/** A node another node follows and writes through: the machine serving a scope, or a device. */
export interface ObjectRemote extends sync.Publisher {
    /** Push one home's mutations, answering their outcomes and watermark. */
    readonly push: (home: string, push: Push, signal?: AbortSignal) => Promise<PushResult>;
    /** Answer a reading call in the scope it reads, as a signing principal or the caller. */
    readonly call: (scope: string, call: sync.Call, signer?: Signer) => Promise<JsonValue>;
    /** Send an unstored event to an object's readers, as a signing principal or the caller. */
    readonly broadcast: (
        scope: string,
        object: Omit<sync.ObjectReference, "scope">,
        event: JsonValue,
        signer?: Signer,
    ) => Promise<void>;
}

/** A node another node follows and writes through. */
export const ObjectRemote = {
    /** Report whether a follow source also takes writes, as a remote does. */
    is(publisher: sync.Publisher): publisher is ObjectRemote {
        return "push" in publisher;
    },

    /** A remote of local objects alone, refusing every call to a server with a refusal. */
    local(refusal: () => Error): ObjectRemote {
        const refuse = () => Promise.reject(refusal());

        return {
            stream: () => ({ [Symbol.asyncIterator]: () => ({ next: refuse }) }),
            request: refuse,
            push: refuse,
            call: refuse,
            broadcast: refuse,
        };
    },

    /** A remote that never answers, as an offline machine: its follows wait until stopped. */
    unanswered(): ObjectRemote {
        return {
            stream: (_subscription, signal) => ({
                [Symbol.asyncIterator]: () => ({
                    next: async () => {
                        // end once stopped
                        await waitAborted(signal);

                        return { done: true, value: undefined };
                    },
                }),
            }),
            request: never,
            push: never,
            call: never,
            broadcast: never,
        };
    },

    /** Run a routed method by name with JSON input in a scope: read a query, push a change. */
    async run(
        remote: ObjectRemote,
        object: ObjectType,
        name: string,
        input: JsonObject,
        scope: string,
        environment: UuidEnvironment,
    ): Promise<JsonValue> {
        // answer a query in the scope it reads
        const built = object.callFrom(name, input, scope, environment);
        if (built.kind === "query") {
            return remote.call(object.readScope(name, built.call.input), built.call);
        }

        // push a change to the home of the scope it acts in, failing with its refusal
        const { mutation } = built;
        const home = object.readScope(name, nth(mutation.calls, 0).input);
        const [result = null] = await ObjectRemote.mutate(remote, home, mutation);

        return result;
    },

    /** Push one mutation to its home, answering its results or refusal. */
    async mutate(
        remote: ObjectRemote,
        home: string,
        mutation: sync.Mutation,
        signer?: Signer,
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

/** Settle never, as a call to a machine that never answers. */
function never(): Promise<never> {
    return new Promise(() => {});
}
