import { TestClock } from "@destack/test";
import type { TriggerOn } from "@destack/runtime/trigger";
import type { ExchangeAsk } from "@destack/runtime/authentication";
import { asc, eq, type DatabaseConnection, type Dialect } from "@destack/db";
import type { EventStore } from "@destack/event";
import { Identifier, schema } from "@destack/schema";
import { type Controller, ControlLoop } from "@destack/runtime/control";
import { subjectContext } from "@destack/runtime/test";
import { RequestId } from "@destack/runtime/request";
import type { RetryPolicy } from "@destack/runtime/timer";
import type { Mutation } from "@destack/sync";
import { principal } from "@destack/access";
import {
    type FinishedRun,
    Run,
    RunOutcome,
    type Trigger,
    deployment,
    installation,
    installationRevision,
    instance,
    run,
    runOutcome,
    trigger,
} from "../object/index.ts";
import {
    controlRuns,
    type PushOutcome,
    type RunAttemptOptions,
    serveSpace,
} from "../server/index.ts";
import { ids, openEvents, reconcile, StackFixture, WAIT_MILLISECONDS } from "./fixture.ts";
import type { InstallationBuild } from "../declare/installation.ts";
import { TestRuntime } from "./runtime.ts";
import { workload } from "./build.ts";

/** The first occurrence of the fixture schedules, 09:00 UTC on 27 September 2026. */
export const T0 = Date.UTC(2026, 8, 27, 9);

/** The notes installation's one instance. */
const INSTANCE = schema
    .identifier("instance")
    .parse("instance-01996ab0-0000-7000-8000-0000000000e2");

/** Mark a note of the notes installation pinned. */
export const pin = {
    type: { packageId: ids.package, type: "note" },
    method: "update",
    input: { id: "note-1", pinned: true },
    release: "2026.9.0",
};

/** Answer a pushed mutation with its outcome, or throw a transient failure. */
export type Answer = (mutation: Mutation, signal: AbortSignal) => Promise<PushOutcome>;

/** Read why a run's outcome ended it: its skip reason or failure code, null for a success. */
export function endingOf(outcome: RunOutcome): string | null {
    // describe a skip by its reason
    if (outcome.kind === "skipped") {
        return outcome.skip.reason;
    }
    // describe a failure by its code
    else if (outcome.kind === "failed") {
        return outcome.failure.code;
    }
    // describe a success by nothing
    else {
        return null;
    }
}

/** A space database keeping the notes installation, its schedules and runs. */
export class RunFixture implements AsyncDisposable {
    /** The controller attempting the space's runs. */
    readonly attempts: Controller;
    /** The space database. */
    readonly database: DatabaseConnection;
    /** The object store of the space database, attempting runs and firing schedules. */
    readonly store: ReturnType<typeof serveSpace>["store"];
    /** The space's events, keeping its finished runs. */
    readonly events: EventStore;
    /** The mutations pushed in order, with each one's installation and principal. */
    readonly pushed: {
        readonly installation: string;
        readonly mutation: Mutation;
        readonly exchange: ExchangeAsk | null;
    }[] = [];
    /** The failures the control loop reported before retrying them. */
    readonly reports: unknown[] = [];
    /** The time the controllers read, which the tests move. */
    readonly clock: TestClock;
    /** The runs being attempted. */
    readonly #attempting = new Set<string>();
    /** The control loops running, stopped before the fixture closes. */
    readonly #loops: { readonly stop: AbortController; readonly running: Promise<void> }[] = [];

    /** Keep a prepared database and its object store, attempting runs by answering each push. */
    private constructor(
        database: DatabaseConnection,
        store: ReturnType<typeof serveSpace>["store"],
        events: EventStore,
        clock: TestClock,
        answer: Answer,
        options: Pick<RunAttemptOptions, "maximumAttempts">,
    ) {
        // keep the database, its store, its events and the clock, and attempt the runs through the answer
        this.database = database;
        this.store = store;
        this.events = events;
        this.clock = clock;
        this.attempts = controlRuns(store, {
            ...options,
            machine: ids.cloud,
            // record each push at the fixture's time before answering it
            push: (target, mutation, _now, signal, exchange) => {
                this.pushed.push({ installation: target.id, mutation, exchange });

                return answer(mutation, signal);
            },
        });
    }

    /** Keep the notes installation in a new space database of a dialect. */
    static async open(
        dialect: Dialect,
        answer: Answer = async () => ({ value: [null] }),
        options: Pick<RunAttemptOptions, "maximumAttempts"> = {},
    ): Promise<RunFixture> {
        // keep the notes installation in the space following its release with one running instance
        const space = await StackFixture.open({ dialect });
        const { database } = space;
        const release = await space.release(ids.package, "2026.9.0");
        const revisionId = await installNotes(database, release, T0);
        await runNotes(database, revisionId, T0);

        // run the instance on the machine's runtime already
        const runtime = new TestRuntime("bun", [INSTANCE]);
        const clock = new TestClock(T0);
        const events = await openEvents(database);
        const store = serveSpace(
            await space.options({
                events,
                runtimes: [runtime],
                clock,
            }),
        ).store;

        return new RunFixture(database, store, events, clock, answer, options);
    }

    /** The controllers the fixture runs: the store's own and its run attempts. */
    controllers(): readonly Controller[] {
        return [...this.store.controllers(), this.attempts];
    }

    /** Stop the notes installation's one instance, or run it again. */
    async stop(isStopped: boolean): Promise<void> {
        await this.database
            .update(instance.table)
            .set(
                isStopped
                    ? { status: "stopped", stoppedAt: this.clock.now() }
                    : { status: "running", startedAt: this.clock.now(), stoppedAt: null },
            )
            .where(eq(instance.table.id, INSTANCE));
    }

    /** Declare a trigger of the notes installation whose schedule runs the pinning call. */
    async trigger(name: string, on: TriggerOn): Promise<Trigger> {
        const [declared] = await this.store.mutate(this.store.self, {
            id: RequestId.create(this.clock.now()),
            calls: [
                trigger.call("declare", {
                    ...trigger.in(ids.space),
                    installationId: ids.notes,
                    origin: "declared",
                    name,
                    on,
                }),
            ],
        });

        if (declared === undefined) {
            throw new TypeError(`declaring trigger ${name} answered no trigger`);
        }

        return declared;
    }

    /** Record a call the notes installation sends. */
    async send(fields: Readonly<Record<string, unknown>> = {}): Promise<Run> {
        // send as the notes installation under a fresh request
        const sent = await this.store
            .mutate(
                subjectContext(principal.installation.reference(ids.space, ids.notes), ids.space, {
                    clock: this.clock,
                }).access(),
                {
                    id: RequestId.create(this.clock.now()),
                    calls: [
                        run.call("send", {
                            spaceId: ids.space,
                            installationId: ids.notes,
                            call: pin,
                            ...fields,
                        }),
                    ],
                },
            )
            .then(([written]) => written);

        // read the recorded run
        const { id } = schema.looseObject({ id: schema.identifier("run") }).parse(sent);
        const [recorded] = await this.database.select().from(run.table).where(eq(run.table.id, id));
        if (recorded === undefined) {
            throw new TypeError(`sending a call recorded no run ${id}`);
        }

        return recorded;
    }

    /** Suspend the notes installation, or enable it again. */
    async suspend(isSuspended: boolean): Promise<void> {
        await this.database
            .update(installation.table)
            .set({ status: isSuspended ? "suspended" : "enabled" })
            .where(eq(installation.table.id, ids.notes));
    }

    /** Fire a schedule trigger once, returning the wait until its next occurrence. */
    fire(fired: Trigger): Promise<number | undefined> {
        return reconcile(this.store, "trigger", { id: fired.id });
    }

    /** Attempt every live run not being attempted already side by side. */
    async attempt(): Promise<void> {
        // take the live runs not being attempted
        const live = await this.database.select({ id: run.table.id }).from(run.table);
        const ready = live.filter(({ id }) => !this.#attempting.has(id));

        // attempt each at once and keep it while it runs
        await Promise.all(
            ready.map(async ({ id }) => {
                this.#attempting.add(id);
                try {
                    await reconcile(this, "run", { id });
                } finally {
                    this.#attempting.delete(id);
                }
            }),
        );
    }

    /** Run the store's controllers in a control loop until the fixture closes. */
    serve(retry: Partial<RetryPolicy> = { initialInterval: 1, maximumInterval: 1 }): void {
        const stop = new AbortController();
        const loop = new ControlLoop(this.database, this.controllers(), {
            report: (_controller, _key, error) => this.reports.push(error),
            retry,
            clock: this.clock,
        });
        this.#loops.push({ stop, running: loop.run(stop.signal) });
    }

    /** Wait until no run waits or runs. */
    settle(): Promise<boolean> {
        return this.database.log.until(async () => {
            const live = await this.database.select({ id: run.table.id }).from(run.table);

            return live.length === 0;
        }, AbortSignal.timeout(WAIT_MILLISECONDS));
    }

    /** Read every finished run's event in the order the runs were due. */
    async finished(): Promise<FinishedRun[]> {
        const page = await this.events.query(
            runOutcome.eventKind(),
            { scopes: [ids.space] },
            { limit: 1000 },
        );

        return page.events.toSorted(
            (left, right) => left.data.at - right.data.at || left.time - right.time,
        );
    }

    /** Read every live run in creation order. */
    async runs(): Promise<Run[]> {
        return this.database
            .select()
            .from(run.table)
            .where(eq(run.table.installationId, ids.notes))
            .orderBy(asc(run.table.createdAt), asc(run.table.id));
    }

    /** Read the request identifier a run's pushes carry. */
    static requestId(attempted: { readonly id: string }): string {
        return Identifier.uuid(schema.identifier("run").parse(attempted.id));
    }

    /** Stop the control loops. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const loop of this.#loops) {
            loop.stop.abort();
            await loop.running;
        }
    }
}

/** Keep the notes installation in the space following its release's revision. */
async function installNotes(
    database: DatabaseConnection,
    release: InstallationBuild,
    now: number,
): Promise<Identifier<"installation-revision">> {
    // keep the notes installation in the space
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });

    // follow the installation's release
    const revisionId = Identifier.create("installation-revision");
    await database.insert(installationRevision.table).values({
        id: revisionId,
        scope: ids.space,
        createdAt: now,
        updatedAt: now,
        installationId: ids.notes,
        build: release,
        views: {},
        settings: [],
        catalogs: [],
        digest: "e".repeat(64),
    });
    await database
        .update(installation.table)
        .set({ revisionId, appliedRevisionId: revisionId })
        .where(eq(installation.table.id, ids.notes));

    return revisionId;
}

/** Deploy the notes installation's revision and run one instance of it. */
async function runNotes(
    database: DatabaseConnection,
    revisionId: Identifier<"installation-revision">,
    now: number,
): Promise<void> {
    // deploy the revision
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const deploymentId = Identifier.create("deployment");
    await database.insert(deployment.table).values({
        ...record,
        id: deploymentId,
        installationId: ids.notes,
        packageId: ids.package,
        revisionId,
        output: "server",
        workload: "main",
        runtime: "bun",
        release: "2026.9.0",
        description: workload,
        activatedAt: now,
    });

    // run one instance of it
    const machineId = schema
        .identifier("machine")
        .parse("machine-01996ab0-0000-7000-8000-0000000000e1");
    await database.insert(instance.table).values({
        ...record,
        id: INSTANCE,
        deploymentId,
        machineId,
        status: "running",
        observedAt: now,
        startedAt: now,
    });
}
