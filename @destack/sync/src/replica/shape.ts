import { LogPosition } from "@destack/db";
import { defineSchema, Duration, schema } from "@destack/schema";
import type { Mutation } from "../call/call.ts";
import type { Page } from "../query/index.ts";
import type { Replica } from "./replica.ts";

/** A shape's parameters in JSON form. */
const PARAMETERS = schema.record(schema.string(), schema.json());

/** A follower's subscription to a declared shape over one scope, decided for the scope it serves, from the position it reached. */
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
        /** The parameters the copy at its position reflects, when they differ from the subscribed ones. */
        previous: PARAMETERS.exactOptional(),
        /** How often merged pages arrive, absent for one page per change. */
        refresh: schema.object({ every: Duration.schema }).exactOptional(),
        /** The follower's own origin, its log epoch, whose writes the source's copies replicate and never send back. */
        origin: schema.string().min(1).exactOptional(),
    }),
);
/** A follower's subscription to a declared shape over one scope, from the position it reached. */
export type Subscription = schema.Infer<typeof Subscription>;

/** Where a copy resumes its subscription, and the origin it follows as. */
export type Resumption = Pick<Subscription, "after" | "previous" | "origin">;

/** Who a shape's copy is for, which admits its followers and decides its rows: the scope below by containment, the reader where the rows live, the calling follower, or each row's recipient. */
export type ShapeAudience = "contained" | "reader" | "caller" | "recipient";

/** A shape a server serves and follows: the copy a request's parameters build, and the audience deciding its rows. */
export interface Shape {
    /** The shape's name, which requests name. */
    readonly name: string;
    /** Whose access decides the rows. */
    readonly audience: ShapeAudience;
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

/** Declare a shape: its name, the schema of its parameters, its audience and the copy they build. */
export function defineShape<Parameters extends object>(definition: {
    /** The shape's name. */
    readonly name: string;
    /** The schema of its parameters. */
    readonly parameters: schema.Schema<Parameters>;
    /** Whose access decides the rows. */
    readonly audience: ShapeAudience;
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
        replica: (request) =>
            definition.replica({
                name: request.name,
                scope: request.scope,
                parameters: definition.parameters.parse(request.parameters),
            }),
        subscription: (subscription) => ({
            ...subscription,
            shape: definition.name,
            parameters: PARAMETERS.parse(definition.parameters.parse(subscription.parameters)),
        }),
    };
}

/** Serves the subscriptions of a copy one step closer to its rows' home: its cell, or the installation keeping its rows. */
export interface Publisher {
    /** Stream a subscription's pages from its position, or a snapshot without one. */
    stream(subscription: Subscription, signal: AbortSignal): AsyncIterable<Page>;
}

/** A publisher that also receives the changes its followers send to the rows they copy from it. */
export interface Uplink extends Publisher {
    /** Run a mutation of copied rows at their home as the follower, once per mutation however often it is delivered. */
    receive(mutation: Mutation): Promise<void>;
}
