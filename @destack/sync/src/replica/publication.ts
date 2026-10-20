import { LogPosition } from "@destack/db";
import { Duration, schema } from "@destack/schema";
import type { Call, Mutation } from "../call/call.ts";
import { Subject } from "../call/subject.ts";
import type { Page } from "../query/index.ts";
import type { Watermark } from "../scope/watermark.ts";
import type { Replica } from "./replica.ts";

/** A publication's parameters in JSON form. */
const PARAMETERS = schema.jsonObject();

/** A follower's subscription to a publication over a scope, from the position it reached. */
export const Subscription = schema.object({
    /** The replica's name, shared with a relaying source's own replica. */
    name: schema.string().min(1),
    /** The publication the replica follows. */
    publication: schema.string().min(1),
    /** The replicated scope. */
    scope: schema.string().min(1),
    /** The scope the replica is for, whose principal the source decides the rows for. */
    target: schema.string().min(1),
    /** The principal the source decides the rows for on the follower's behalf. */
    subject: Subject.exactOptional(),
    /** The publication's parameters, which its declaration parses. */
    parameters: PARAMETERS,
    /** The log position the replica reached, absent before its first snapshot. */
    after: LogPosition.exactOptional(),
    /** The parameters the replica reflects at its position, if other than the subscribed. */
    previous: PARAMETERS.exactOptional(),
    /** How often merged pages arrive, absent for one page per change. */
    refresh: schema.object({ every: Duration.schema }).exactOptional(),
    /** The follower's log as a replication origin, which the source never publishes back to. */
    origin: schema.string().min(1).exactOptional(),
    /** The position the follower waits for its source to confirm. */
    confirming: LogPosition.exactOptional(),
});
/** A follower's subscription to a publication over one scope, from the position it reached. */
export type Subscription = schema.Infer<typeof Subscription>;

/** Where a replica resumes its subscription, and the origin it follows as. */
export type Resumption = Pick<Subscription, "after" | "previous" | "origin">;

/** Who a publication's replica is for, which authorizes its followers and decides its rows. */
export const PublicationAudience = schema.enum(["contained", "reader", "caller"]);
/** Who a publication's replica is for, which authorizes its followers and decides its rows. */
export type PublicationAudience = schema.Infer<typeof PublicationAudience>;

/** A publication a server streams and follows: its audience and the replicas it builds. */
export interface Publication {
    /** The publication's name, as subscriptions give it. */
    readonly name: string;
    /** Whose access decides the rows. */
    readonly audience: PublicationAudience;
    /** Whether the replica must have a complete snapshot before its follower serves. */
    readonly isReadinessGate: boolean;
    /** Build the replica a subscription follows. */
    readonly replica: (subscription: Subscription) => Replica;
}

/** A declared publication, with the typed subscriptions of its parameters. */
export interface PublicationOf<Parameters> extends Publication {
    /** Build a subscription to the publication for typed parameters. */
    subscription(
        subscription: Omit<Subscription, "publication" | "parameters"> & {
            readonly parameters: Parameters;
        },
    ): Subscription;
}

/** Declare a publication: its name, parameter schema, audience and the replica they build. */
export function definePublication<Parameters extends object>(definition: {
    /** The publication's name. */
    readonly name: string;
    /** The schema of its parameters. */
    readonly parameters: schema.Schema<Parameters>;
    /** Whose access decides the rows. */
    readonly audience: PublicationAudience;
    /** Whether the replica must have a complete snapshot before its follower serves. */
    readonly isReadinessGate: boolean;
    /** Build the replica of a scope for parsed parameters. */
    readonly replica: (request: {
        readonly name: string;
        readonly scope: string;
        readonly target: string;
        readonly parameters: Parameters;
    }) => Replica;
}): PublicationOf<Parameters> {
    return {
        name: definition.name,
        audience: definition.audience,
        isReadinessGate: definition.isReadinessGate,
        replica: (request) =>
            definition.replica({
                name: request.name,
                scope: request.scope,
                target: request.target,
                parameters: definition.parameters.parse(request.parameters),
            }),
        subscription: (subscription) => ({
            ...subscription,
            publication: definition.name,
            parameters: PARAMETERS.parse(definition.parameters.parse(subscription.parameters)),
        }),
    };
}

/** Serves a replica's subscriptions one step closer to its rows' home. */
export interface Publisher {
    /** Stream a subscription's pages from its position, or a snapshot without one. */
    readonly stream: (subscription: Subscription, signal: AbortSignal) => AsyncIterable<Page>;
    /** Ask the subscription's stream to confirm a position at or past a watermark. */
    readonly confirm: (subscription: Subscription, watermark: Watermark) => Promise<void>;
}

/** A publisher that also runs the calls its follower's server sends to the rows' home. */
export interface Uplink extends Publisher {
    /** Run a mutation of the rows at their home once however often sent, answering its results. */
    readonly receive: (mutation: Mutation, signer?: Subject) => Promise<readonly unknown[]>;
    /** Answer a query of the rows at their home, in the scope it reads. */
    readonly call: (call: Call, scope: string, signer?: Subject) => Promise<unknown>;
}

/** A publisher that also runs the calls its follower's server sends to the rows' home. */
export const Uplink = {
    /** Replicate and send through a client's replica procedures, as follower or change signer. */
    of(
        replica: (signer?: Subject) => {
            readonly stream: (
                subscription: Subscription,
                options: { readonly signal: AbortSignal },
            ) => Promise<AsyncIterable<Page>>;
            readonly confirm: (input: {
                readonly subscription: Subscription;
                readonly watermark: Watermark;
            }) => Promise<unknown>;
            readonly receive: (input: {
                readonly mutation: Mutation;
            }) => Promise<{ readonly results: readonly unknown[] }>;
            readonly call: (input: {
                readonly scope: string;
                readonly call: Call;
            }) => Promise<unknown>;
        },
    ): Uplink {
        return {
            async *stream(subscription, signal) {
                yield* await replica().stream(subscription, { signal });
            },
            confirm: async (subscription, watermark) => {
                await replica().confirm({ subscription, watermark });
            },
            receive: async (mutation, signer) =>
                (await replica(signer).receive({ mutation })).results,
            call: (call, scope, signer) => replica(signer).call({ scope, call }),
        };
    },
};
