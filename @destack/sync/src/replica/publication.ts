import {
    and,
    type DatabaseConnection,
    defineTable,
    eq,
    json,
    LogPosition,
    or,
    type Select,
    sql,
    text,
} from "@destack/db";
import { canonicalize, Duration, schema } from "@destack/schema";
import type { Call, Mutation } from "../call/call.ts";
import { Subject } from "../call/subject.ts";
import type { Page } from "../query/index.ts";
import { Watermark } from "../scope/watermark.ts";
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
    /** The log position the replica arrived at, absent before its first snapshot. */
    after: LogPosition.exactOptional(),
    /** The parameters the replica reflects at its position, if other than the subscribed. */
    previous: PARAMETERS.exactOptional(),
    /** How often merged pages arrive, absent for one page per change. */
    refresh: schema.object({ every: Duration.schema }).exactOptional(),
    /** The follower's log as a replication origin, which the source never publishes back to. */
    origin: schema.string().min(1).exactOptional(),
    /** The watermark the follower waits for its source to confirm. */
    confirming: Watermark.exactOptional(),
});
/** A follower's subscription to a publication over one scope, from the position it arrived at. */
export type Subscription = schema.Infer<typeof subscriptionSchema>;

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
    /** The watermark the follower last asked its stream to confirm, absent before any ask. */
    confirming: json("confirming", Watermark),
    /** The key of the principal requesting the subscription. */
    caller: text("caller").primaryKey(),
});

/** A follower's request as its source records it. */
export type SubscriptionRequest = Select<typeof subscriptionTable>;

/** A follower's subscription and the requests it records at its source. */
export const Subscription = Object.assign(subscriptionSchema, {
    /** Record a caller's request at its source: its parameters and its latest ask. */
    async request(
        database: DatabaseConnection,
        subscription: Subscription,
        caller: string,
    ): Promise<SubscriptionRequest> {
        // keep the latest ask
        const { name, scope, target, parameters, confirming } = subscription;
        const [recorded] = await Subscription.requests(database, [{ name, scope, target, caller }]);
        const ask = Watermark.later(recorded?.confirming ?? null, confirming ?? null);
        const row = { name, scope, target, caller, parameters, confirming: ask };
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
                set: row,
            });

        return row;
    },

    /** Ask every stream a principal requested of a scope to confirm a watermark. */
    async confirm(
        database: DatabaseConnection,
        caller: string,
        watermark: Watermark,
    ): Promise<void> {
        // move each of the caller's asks of the scope to the watermark when later
        const rows = await database
            .select()
            .from(subscriptionTable)
            .where(
                and(
                    eq(subscriptionTable.scope, watermark.scope),
                    eq(subscriptionTable.caller, caller),
                ),
            );
        for (const row of rows) {
            const ask = Watermark.later(row.confirming, watermark);
            if (ask !== row.confirming) {
                await database
                    .update(subscriptionTable)
                    .set({ confirming: ask })
                    .where(
                        and(
                            eq(subscriptionTable.name, row.name),
                            eq(subscriptionTable.scope, row.scope),
                            eq(subscriptionTable.target, row.target),
                            eq(subscriptionTable.caller, row.caller),
                        ),
                    );
            }
        }
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
