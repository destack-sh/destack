import { LogPosition } from "@destack/db";
import { defineSchema, Duration, schema } from "@destack/schema";
import type { Mutation } from "../call/call.ts";
import type { Subject } from "../call/subject.ts";
import type { Page } from "../query/index.ts";
import type { Replica } from "./replica.ts";

/** A shape's parameters in JSON form. */
const PARAMETERS = schema.record(schema.string(), schema.json());

/** A follower's subscription to a declared shape over a scope, from the position it reached. */
export const Subscription = defineSchema(
    schema.object({
        /** The copy's name, shared with a relaying source's own copy. */
        name: schema.string().min(1),
        /** The declared shape the copy follows. */
        shape: schema.string().min(1),
        /** The copied scope. */
        scope: schema.string().min(1),
        /** The scope the follower serves, whose principal the source decides for. */
        below: schema.string().min(1),
        /** The shape's parameters, which its declaration parses. */
        parameters: PARAMETERS,
        /** The log position the copy reached, absent before its first snapshot. */
        after: LogPosition.exactOptional(),
        /** The parameters the copy reflects at its position, if they differ from the subscribed. */
        previous: PARAMETERS.exactOptional(),
        /** How often merged pages arrive, absent for one page per change. */
        refresh: schema.object({ every: Duration.schema }).exactOptional(),
        /** The follower's origin, its log epoch, whose writes the source never sends back. */
        origin: schema.string().min(1).exactOptional(),
    }),
);
/** A follower's subscription to a declared shape over one scope, from the position it reached. */
export type Subscription = schema.Infer<typeof Subscription>;

/** Where a copy resumes its subscription, and the origin it follows as. */
export type Resumption = Pick<Subscription, "after" | "previous" | "origin">;

/** Who a shape's copy is for, which admits its followers and decides its rows. */
export type ShapeAudience = "contained" | "reader" | "caller" | "recipient";

/** A shape a server serves and follows: the copy its parameters build and its audience. */
export interface Shape {
    /** The shape's name, which requests name. */
    readonly name: string;
    /** Whose access decides the rows. */
    readonly audience: ShapeAudience;
    /** Whether the copy must hold a complete snapshot before its follower serves. */
    readonly isReadinessGate: boolean;
    /** Build the copy a subscription follows. */
    replica(subscription: Subscription): Replica;
}

/** A declared shape, with the typed subscriptions of its parameters. */
export interface ShapeOf<Parameters> extends Shape {
    /** Build a subscription to the shape for typed parameters. */
    subscription(
        subscription: Omit<Subscription, "shape" | "parameters"> & {
            readonly parameters: Parameters;
        },
    ): Subscription;
}

/** Declare a shape: its name, parameter schema, audience and the copy they build. */
export function defineShape<Parameters extends object>(definition: {
    /** The shape's name. */
    readonly name: string;
    /** The schema of its parameters. */
    readonly parameters: schema.Schema<Parameters>;
    /** Whose access decides the rows. */
    readonly audience: ShapeAudience;
    /** Whether the copy must hold a complete snapshot before its follower serves. */
    readonly isReadinessGate: boolean;
    /** Build the copy of a scope for parsed parameters. */
    readonly replica: (request: {
        readonly name: string;
        readonly scope: string;
        readonly parameters: Parameters;
    }) => Replica;
}): ShapeOf<Parameters> {
    return {
        name: definition.name,
        audience: definition.audience,
        isReadinessGate: definition.isReadinessGate,
        replica: (request) =>
            definition.replica({
                name: request.name,
                scope: request.scope,
                parameters: definition.parameters.parse(request.parameters),
            }),
        subscription: (subscription) => ({
            ...subscription,
            shape: definition.name,
            parameters: jsonParameters(definition.parameters.parse(subscription.parameters)),
        }),
    };
}

/** Take a shape's parsed parameters as the JSON object a subscription sends. */
function jsonParameters(parameters: object): Subscription["parameters"];
/**
 * Take the parsed parameters as they are.
 *
 * @construct a shape's parameters parse the JSON its subscriptions send, so they are JSON.
 */
function jsonParameters(parameters: object): object {
    return parameters;
}

/** Serves a copy's subscriptions one step closer to its rows' home. */
export interface Publisher {
    /** Stream a subscription's pages from its position, or a snapshot without one. */
    stream(subscription: Subscription, signal: AbortSignal): AsyncIterable<Page>;
}

/** A publisher that also receives the changes its follower's server sends to the rows' home. */
export interface Uplink extends Publisher {
    /** Run a mutation of the rows at their home, once however often it is delivered. */
    receive(mutation: Mutation, signer?: Subject): Promise<void>;
}

/** A publisher that also receives the changes its follower's server sends to the rows' home. */
export const Uplink = {
    /** Copy and send through a client's replica procedures, as follower or a change's signer. */
    of(
        replica: (signer?: Subject) => {
            stream(
                subscription: Subscription,
                options: { readonly signal: AbortSignal },
            ): Promise<AsyncIterable<Page>>;
            receive(input: { readonly mutation: Mutation }): Promise<unknown>;
        },
    ): Uplink {
        return {
            async *stream(subscription, signal) {
                yield* await replica().stream(subscription, { signal });
            },
            receive: async (mutation, signer) => {
                await replica(signer).receive({ mutation });
            },
        };
    },
};
