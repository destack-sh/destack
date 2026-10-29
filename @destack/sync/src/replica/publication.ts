import {
    and,
    type DatabaseConnection,
    defineTable,
    eq,
    integer,
    json,
    lte,
    ne,
    LogPosition,
    or,
    type Select,
    sql,
    text,
} from "@destack/db";
import { canonicalize, Duration, schema } from "@destack/schema";
import { SyncError } from "../error/index.ts";
import type { Call, Mutation } from "../call/call.ts";
import { Subject } from "../call/subject.ts";
import type { Page } from "../query/index.ts";
import type { Replica } from "./replica.ts";

/** A publication's parameters in JSON form. */
const PARAMETERS = schema.jsonObject();

/** A follower's subscription to a publication over a scope, from the position it arrived at. */
const subscriptionSchema = schema.object({
    /** The replica's name, shared with a relaying source's replica. */
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
    /** The follower's log position when it chose the parameters, the latest of a log kept. */
    revision: LogPosition,
    /** The log position the replica arrived at, absent before its first snapshot. */
    after: LogPosition.exactOptional(),
    /** The parameters the replica reflects at its position, if other than the subscribed. */
    previous: PARAMETERS.exactOptional(),
    /** How often merged pages arrive, absent for one page per change. */
    refresh: schema.object({ every: Duration.schema }).exactOptional(),
    /** The follower's log as a replication origin, which the source never publishes back to. */
    origin: schema.string().min(1).exactOptional(),
});
/** A follower's subscription to a publication over one scope, from the position it arrived at. */
export type Subscription = schema.Infer<typeof subscriptionSchema>;

/** A subscription as a listing decides it and a replica records it, without resumption. */
export const SubscriptionRecord = subscriptionSchema.omit({
    revision: true,
    after: true,
    previous: true,
    refresh: true,
    origin: true,
});
/** A subscription as a listing decides it and a replica records it. */
export type SubscriptionRecord = schema.Infer<typeof SubscriptionRecord>;

/** The subscriptions a source's followers request. */
export const subscriptionTable = defineTable("subscription", {
    /** The replica's name. */
    name: text("name").primaryKey(),
    /** The replicated scope. */
    scope: text("scope").primaryKey(),
    /** The scope the replica is for. */
    target: text("target").primaryKey(),
    /** The publication's parameters the follower requests. */
    parameters: json("parameters", schema.jsonObject()).notNull(),
    /** The follower's log when it chose the parameters. */
    revisionLog: text("revision_log").notNull(),
    /** The follower's log epoch when it chose the parameters. */
    revisionEpoch: text("revision_epoch").notNull(),
    /** The follower's log sequence when it chose the parameters. */
    revisionSequence: integer("revision_sequence").notNull(),
    /** The key of the principal requesting the subscription. */
    caller: text("caller").primaryKey(),
});

/** A follower's request as its source records it. */
export type SubscriptionRequest = Select<typeof subscriptionTable>;

/** A follower's subscription and the requests it records at its source. */
export const Subscription = Object.assign(subscriptionSchema, {
    /** Record a caller's request at its source, unless it holds a later revision of the log. */
    async request(
        database: DatabaseConnection,
        subscription: Subscription,
        caller: string,
    ): Promise<void> {
        // take the request at its revision, a follower's other log or epoch replacing what it recorded
        const { name, scope, target, parameters, revision } = subscription;
        const revised = {
            revisionLog: revision.log,
            revisionEpoch: revision.epoch,
            revisionSequence: revision.sequence,
        };
        const row = { name, scope, target, caller, parameters, ...revised };
        await database
            .insert(subscriptionTable)
            .values(row)
            .onConflictDoUpdate({
                target: [
                    subscriptionTable.name,
                    subscriptionTable.scope,
                    subscriptionTable.target,
                    subscriptionTable.caller,
                ],
                set: { parameters, ...revised },
                setWhere: or(
                    ne(subscriptionTable.revisionLog, revision.log),
                    ne(subscriptionTable.revisionEpoch, revision.epoch),
                    lte(subscriptionTable.revisionSequence, revision.sequence),
                ),
            });
    },

    /** Read the recorded requests of some keys in one statement. */
    async requests(
        database: DatabaseConnection,
        keys: readonly RequestKey[],
    ): Promise<SubscriptionRequest[]> {
        const matching = keys.map((key) =>
            and(
                eq(subscriptionTable.name, key.name),
                eq(subscriptionTable.scope, key.scope),
                eq(subscriptionTable.target, key.target),
                eq(subscriptionTable.caller, key.caller),
            ),
        );

        return await database
            .select()
            .from(subscriptionTable)
            .where(or(sql`FALSE`, ...matching));
    },

    /** Name a request by its key, canonically. */
    key(key: RequestKey): string {
        return canonicalize([key.name, key.scope, key.target, key.caller]);
    },
});

/** A request's key: the replica, the scope it replicates, the scope it is for and its caller. */
export type RequestKey = Pick<SubscriptionRequest, "name" | "scope" | "target" | "caller">;

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
    /** Build the replica a subscription follows. */
    readonly replica: (subscription: SubscriptionRecord) => Replica;
    /** Read a subscription's parameters by the publication's schema, refusing invalid ones. */
    readonly read: <Requested extends SubscriptionRecord>(subscription: Requested) => Requested;
}

/** A declared publication, with the typed subscriptions of its parameters. */
export interface PublicationOf<Parameters> extends Publication {
    /** Build a subscription to the publication for typed parameters. */
    subscription(
        subscription: Omit<SubscriptionRecord, "publication" | "parameters"> & {
            readonly parameters: Parameters;
        },
    ): SubscriptionRecord;
}

/** Declare a publication: its name, parameter schema, audience and the replica they build. */
export function definePublication<Parameters extends object>(definition: {
    /** The publication's name. */
    readonly name: string;
    /** The schema of its parameters. */
    readonly parameters: schema.Schema<Parameters>;
    /** Whose access decides the rows. */
    readonly audience: PublicationAudience;
    /** Build the replica of a scope for parsed parameters. */
    readonly replica: (request: {
        readonly name: string;
        readonly scope: string;
        readonly target: string;
        readonly parameters: Parameters;
    }) => Replica;
}): PublicationOf<Parameters> {
    // read a subscription's parameters into their canonical form, refusing invalid ones
    const read = <Requested extends SubscriptionRecord>(subscription: Requested): Requested => ({
        ...subscription,
        parameters: PARAMETERS.parse(parseParameters(definition, subscription.parameters)),
    });

    return {
        name: definition.name,
        audience: definition.audience,
        replica: (request) =>
            definition.replica({
                name: request.name,
                scope: request.scope,
                target: request.target,
                parameters: parseParameters(definition, request.parameters),
            }),
        subscription: (subscription) => read({ ...subscription, publication: definition.name }),
        read,
    };
}

/** Parse a subscription's parameters by its publication, refusing invalid ones as the caller's. */
function parseParameters<Parameters>(
    definition: { readonly name: string; readonly parameters: schema.Schema<Parameters> },
    parameters: unknown,
): Parameters {
    const parsed = definition.parameters.safeParse(parameters);
    if (!parsed.success) {
        throw SyncError.of(
            "PARAMETERS_INVALID",
            `invalid parameters of publication ${definition.name}: ${schema.describeIssues(parsed.error.issues)}`,
        );
    }

    return parsed.data;
}

/** Serves a replica's subscriptions one step closer to its rows' home. */
export interface Publisher {
    /** Stream a subscription's pages from its position, or a snapshot without one. */
    readonly stream: (subscription: Subscription, signal: AbortSignal) => AsyncIterable<Page>;
    /** Record a subscription's request at its source: its parameters and its ask. */
    readonly request: (subscription: Subscription) => Promise<void>;
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
    /** Replicate and send through a client's replica calls, as follower or change signer. */
    of(
        replica: (signer?: Subject) => {
            readonly stream: (
                subscription: Subscription,
                options: { readonly signal: AbortSignal },
            ) => Promise<AsyncIterable<Page>>;
            readonly request: (input: { readonly subscription: Subscription }) => Promise<unknown>;
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
            request: async (subscription) => {
                await replica().request({ subscription });
            },
            receive: async (mutation, signer) =>
                (await replica(signer).receive({ mutation })).results,
            call: (call, scope, signer) => replica(signer).call({ scope, call }),
        };
    },
};
