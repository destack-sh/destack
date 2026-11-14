import {
    type Condition,
    type DatabaseConnection,
    eq,
    Expression,
    type LogPosition,
    type Table,
} from "@destack/db";
import type { Given, SetStep } from "@destack/package/declare";
import { Simulation, type SimulatedTelemetry } from "@destack/environment/simulation";
import { type JsonValue, present } from "@destack/schema";
import type { Driver } from "@destack/test";
import { testWorldOptions } from "@destack/test/world";
import { Replica } from "../replica/replica.ts";
import type { Query } from "../query/index.ts";
import { comment, open, openReplica, project, relations, START, task } from "./fixture.ts";
import { Feed } from "../feed/feed.ts";
import { ConditionAudience } from "../feed/test/audience.ts";
import {
    type FeedObservation,
    type FeedStep,
    feedInteraction,
    type QuerySet,
} from "../scenario/interaction.ts";
import {
    AUDIENCE,
    EVERYTHING,
    FILTERS,
    LOOKUPS,
    PATHS,
    TALLIES,
    TREE,
    WINDOWS,
} from "../feed/test/query.ts";
import { Random } from "../feed/test/random.ts";
import { Follower } from "../feed/test/replica.ts";
import { Workload } from "../feed/test/workload.ts";

/** The most subscribers a feed world's feed serves, its largest scenario's. */
const SUBSCRIBERS = 1000;

/** The seed of a feed world's simulation and of its random writes. */
const SEED = 1;

/** The rows one filing insert writes, well within every parameter budget. */
const FILING_ROWS = 500;

/** The query sets by the name a scenario follows them by. */
const QUERY_SETS: Readonly<Record<QuerySet, Readonly<Record<string, Query>>>> = {
    "filters-windows": { ...FILTERS, ...WINDOWS },
    snapshot: { ...FILTERS, ...WINDOWS, ...TREE, ...TALLIES, ...PATHS },
    lookups: LOOKUPS,
    everything: EVERYTHING,
    discussed: {
        discussed: {
            table: task,
            scopes: ["inbox"],
            relations,
            extras: { comments: Expression.rollup("count", "comments") },
            orderBy: { comments: "desc" },
            limit: 10,
        },
    },
};

/** A driver that plays feed scenarios over a database, its feed and subscribers in a world. */
export class FeedDriver implements Driver<FeedStep, FeedObservation> {
    /** The interaction the driver plays. */
    static readonly interaction = feedInteraction;
    /** The settings its worlds run with: the test database's. */
    static readonly configuration = {};

    /** The world. */
    readonly #simulation: Simulation;
    /** The written database. */
    readonly #source: DatabaseConnection;
    /** The feed of its log. */
    readonly #feed: Feed;
    /** The random writes. */
    readonly #workload: Workload;
    /** The random picks of commented tasks. */
    readonly #random = new Random(SEED);
    /** The followers, in starting order. */
    readonly #followers: Follower[] = [];
    /** The log sequence each direct subscriber completed last. */
    readonly #subscribed: (number | undefined)[] = [];
    /** Stops the direct subscribers. */
    readonly #subscribing = new AbortController();
    /** The direct subscribers' loops. */
    readonly #subscriptions: Promise<void>[] = [];
    /** Resolves when a direct subscriber completes a page. */
    #progress = Promise.withResolvers<void>();
    /** The failure that ended a direct subscriber, none while all run. */
    #failure: unknown = undefined;
    /** The rows the last snapshot held. */
    #snapshot = 0;
    /** The writes taken so far, numbering each. */
    #writes = 0;

    /** Take a started world and its feed. */
    constructor(simulation: Simulation, source: DatabaseConnection) {
        // keep the world and database, and feed its log to every subscriber
        this.#simulation = simulation;
        this.#source = source;
        this.#feed = new Feed(source, Workload.tables, { subscribers: SUBSCRIBERS });
        this.#workload = new Workload(new Random(SEED));
    }

    /** The world's clock. */
    get clock(): Simulation["clock"] {
        return this.#simulation.clock;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.#simulation.telemetry;
    }

    /** Start a world with a database of the project's tables, given no examples. */
    static async start(_given: Given): Promise<FeedDriver> {
        const simulation = new Simulation({ world: testWorldOptions(), seed: SEED, start: START });

        return new FeedDriver(simulation, await open({ simulation }));
    }

    /** Take one step, until every follower and subscriber has it. */
    async act(step: SetStep | FeedStep): Promise<void> {
        // write random rows
        if (step.action === "seed") {
            await this.#workload.seed(this.#source, step.rows);
        }
        // file open tasks, in a project when named
        else if (step.action === "file") {
            await this.#file(step.tasks, step.project);
        }
        // start followers until each has the rows
        else if (step.action === "follow") {
            await this.#follow(step.queries, step.followers, step.audiences);
        }
        // subscribe directly until each subscriber completes
        else if (step.action === "subscribe") {
            await this.#subscribe(step.queries, step.subscribers);
        }
        // snapshot into a new follower's memory, or a new replica database
        else if (step.action === "snapshot" || step.action === "replicate") {
            const database =
                step.action === "replicate"
                    ? await openReplica({ simulation: this.#simulation })
                    : undefined;
            const replica = new Replica({
                name: "replica",
                scope: "inbox",
                tables: Workload.tables,
            });
            const follower = new Follower(this.#feed, QUERY_SETS[step.queries], Workload.tables, {
                audience: AUDIENCE,
                ...(database === undefined ? {} : { replica: { replica, database } }),
            });
            follower.start();
            await follower.wait(await this.#source.log.position());
            await follower.stop();
            this.#snapshot = follower.replica.rows.size;
        }
        // write a row, until everyone has it
        else if (step.action === "write") {
            await this.#workload.write(this.#source);
            await this.#reached();
        }
        // rank a task anew by the write's number
        else if (step.action === "rank") {
            const rank = this.#count() % 10;
            await this.#source.transaction(
                async (transaction) =>
                    await transaction.update(task).set({ rank }).where(eq(task.id, step.task)),
            );
            await this.#reached();
        }
        // rename a project back and forth
        else if (step.action === "rename") {
            const name = this.#count() % 2 === 0 ? "x" : "a";
            await this.#source.transaction(
                async (transaction) =>
                    await transaction
                        .update(project)
                        .set({ name })
                        .where(eq(project.id, step.project)),
            );
            await this.#reached();
        }
        // comment on one of the first tasks
        else if (step.action === "comment") {
            const index = this.#count();
            await this.#source.transaction(
                async (transaction) =>
                    await transaction.insert(comment).values({
                        id: `c${index}`,
                        scope: "inbox",
                        taskId: `t${this.#random.integer(step.among)}`,
                        position: index,
                    }),
            );
            await this.#reached();
        }
        // refuse the set step, as a feed world has no example properties
        else {
            throw new TypeError("a feed world takes no example properties");
        }
    }

    /** Count the rows the first follower holds, or the last snapshot held. */
    observe(observation: FeedObservation): JsonValue {
        return observation.count === "snapshot"
            ? this.#snapshot
            : present(this.#followers[0], "a follower").replica.rows.size;
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.#simulation.fingerprint;
    }

    /** Stop the followers and subscribers, then the world. */
    async [Symbol.asyncDispose](): Promise<void> {
        // stop every reader before the world they read
        await Promise.all(this.#followers.map((follower) => follower.stop()));
        this.#subscribing.abort();
        await Promise.all(this.#subscriptions);
        await this.#simulation[Symbol.asyncDispose]();
    }

    /** Number the next write. */
    #count(): number {
        this.#writes += 1;

        return this.#writes;
    }

    /** File open tasks in batches, in a project when named. */
    async #file(tasks: number, projectId: string | undefined): Promise<void> {
        // create the project first
        if (projectId !== undefined) {
            await this.#source.transaction(
                async (transaction) =>
                    await transaction
                        .insert(project)
                        .values({ id: projectId, scope: "inbox", name: "a" }),
            );
        }

        // insert the tasks a batch at a time
        for (let start = 0; start < tasks; start += FILING_ROWS) {
            const rows = Array.from(
                { length: Math.min(FILING_ROWS, tasks - start) },
                (_, index) => ({
                    id: `t${start + index}`,
                    scope: "inbox",
                    ...(projectId === undefined ? {} : { projectId }),
                    state: "open",
                    rank: (start + index) % 10,
                    points: 1,
                    title: "work",
                    isSecret: false,
                }),
            );
            await this.#source.transaction(
                async (transaction) => await transaction.insert(task).values(rows),
            );
        }
    }

    /** Start followers of a query set, in distinct audiences when asked, until each has it. */
    async #follow(
        queries: QuerySet,
        followers: number,
        audiences: number | undefined,
    ): Promise<void> {
        // give each follower the shared audience, or one of the distinct ones
        const distinct = Array.from(
            { length: audiences ?? 0 },
            (_, index) =>
                new ConditionAudience(
                    new Map<Table, Condition>([[task, { id: { ne: `s${index}` } }]]),
                ),
        );
        const started = Array.from({ length: followers }, (_, index) => {
            const audience =
                audiences === undefined
                    ? AUDIENCE
                    : present(distinct[index % audiences], "an audience");

            return new Follower(this.#feed, QUERY_SETS[queries], Workload.tables, { audience });
        });
        for (const follower of started) {
            follower.start();
            this.#followers.push(follower);
        }

        // wait until each has the rows
        await this.#reached();
    }

    /** Subscribe to a query set's pages directly, recording each one's completed sequence. */
    async #subscribe(queries: QuerySet, subscribers: number): Promise<void> {
        // read each subscriber's pages until the world ends
        for (let index = 0; index < subscribers; index += 1) {
            const at = this.#subscribed.push(undefined) - 1;
            const pages = this.#feed.subscribe(
                QUERY_SETS[queries],
                undefined,
                this.#subscribing.signal,
                {
                    audience: AUDIENCE,
                },
            );
            this.#subscriptions.push(
                (async () => {
                    for await (const sent of pages) {
                        if (sent.complete) {
                            this.#subscribed[at] = sent.position.sequence;
                        }
                        this.#progress.resolve();
                    }
                })().catch((error: unknown) => {
                    // wake the waits with the failure, which the next wait throws
                    this.#failure = error;
                    this.#progress.resolve();
                }),
            );
        }

        // wait until each has the rows
        await this.#reached();
    }

    /** Wait until every follower and direct subscriber has the database's latest position. */
    async #reached(): Promise<void> {
        const position: LogPosition = await this.#source.log.position();
        await Promise.all(this.#followers.map((follower) => follower.wait(position)));
        while (
            this.#subscribed.some(
                (sequence) => sequence === undefined || sequence < position.sequence,
            )
        ) {
            await this.#progress.promise;
            this.#progress = Promise.withResolvers<void>();
            if (this.#failure !== undefined) {
                throw new Error("a subscriber failed", { cause: this.#failure });
            }
        }
    }
}
