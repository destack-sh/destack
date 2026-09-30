import { asc, eq, inArray, type DatabaseConnection } from "@destack/db";
import type { Dialect } from "@destack/db";
import { identifier, identifierUuid, type Identifier } from "@destack/schema";
import { v7 } from "uuid";
import { ControlLoop } from "@destack/service/control";
import { RequestId } from "@destack/service/request";
import type { RetryPolicy } from "@destack/service/timer";
import type * as sync from "@destack/sync";
import { principal } from "@destack/access";
import type { LentAuthority } from "@destack/service/authentication";
import * as object from "../../src/object/index.ts";
import { type PushOutcome, type RunOptions, serveObjects } from "../../src/server/index.ts";
import { ids, openSpace, reconcile, spaceOptions, TestRuntime } from "./space.ts";
import type { InstanceSpec } from "@destack/host/runtime";

/** The first occurrence of the fixture schedules, 09:00 UTC on 27 September 2026. */
export const T0 = Date.UTC(2026, 8, 27, 9);

/** The notes installation's one instance. */
const INSTANCE = identifier("instance").parse("instance-01996ab0-0000-7000-8000-0000000000e2");

/** Mark a note of the notes installation pinned. */
export const pin = {
    method: "note.update",
    input: { id: "note-1", pinned: true },
    release: "2026.9.0",
};

/** Answer a pushed mutation with its outcome, or throw a transient failure. */
export type Answer = (mutation: sync.Mutation, signal: AbortSignal) => Promise<PushOutcome>;

/** A space database keeping the notes installation, its schedules and runs, whose pushed calls an answer settles. */
export class RunFixture implements AsyncDisposable {
    /** The space database. */
    readonly database: DatabaseConnection;
    /** The object server of the space database, attempting runs and firing schedules. */
    readonly server: ReturnType<typeof serveObjects>;
    /** The mutations pushed, in order, with the installation each went to and the principal it acted for. */
    readonly pushed: {
        readonly installation: string;
        readonly mutation: sync.Mutation;
        readonly onBehalfOf?: LentAuthority;
    }[] = [];
    /** The failures the control loop reported before retrying them. */
    readonly reports: unknown[] = [];
    /** The time the controllers read. */
    now = T0;
    /** The runs being attempted. */
    readonly #attempting = new Set<string>();
    /** The control loops running, stopped before the fixture closes. */
    readonly #loops: { readonly stop: AbortController; readonly running: Promise<void> }[] = [];

    /** Keep a prepared database and its object server. */
    private constructor(database: DatabaseConnection, server: ReturnType<typeof serveObjects>) {
        this.database = database;
        this.server = server;
    }

    /** Keep the notes installation in a new space database of a dialect, answering its pushed calls. */
    static async open(
        dialect: Dialect,
        answer: Answer = async () => ({ value: [null] }),
        options: Pick<RunOptions, "maximumAttempts"> = {},
    ): Promise<RunFixture> {
        // keep the notes installation in the space
        const database = await openSpace([], dialect);
        const now = Date.now();
        await database.insert(object.installation.table).values({
            id: ids.notes,
            scope: ids.space,
            packageId: ids.package,
            role: "application",
            alias: "notes",
            selection: { kind: "release", version: "2026.9.0" },
            createdAt: now,
            updatedAt: now,
        } as never);

        // deploy the installation's release, and run one instance of it
        const record = { scope: ids.space, createdAt: now, updatedAt: now };
        const revisionId = identifier("installation-revision").parse(
            `installation-revision-${v7()}`,
        );
        await database.insert(object.installationRevision.table).values({
            ...record,
            id: revisionId,
            installationId: ids.notes,
            build: { kind: "release", version: "2026.9.0" },
            views: {},
            settings: [],
            digest: "e".repeat(64),
        } as never);
        const deploymentId = identifier("deployment").parse(`deployment-${v7()}`);
        await database.insert(object.deployment.table).values({
            ...record,
            id: deploymentId,
            installationId: ids.notes,
            packageId: ids.package,
            revisionId,
            output: "main",
            workload: "main",
            runtime: "bun",
            release: "2026.9.0",
            description: {
                entrypoint: ".",
                services: [],
                triggers: [],
                resources: [],
                secrets: [],
                connections: [],
                compute: {},
            },
            policies: { packages: [], network: [] },
        } as never);
        const hostId = identifier("host").parse("host-01996ab0-0000-7000-8000-0000000000e1");
        await database.insert(object.instance.table).values({
            ...record,
            id: INSTANCE,
            deploymentId,
            hostId,
            hostEpoch: 1,
            status: "running",
            observedAt: now,
        } as never);

        // record each push before answering it, at the fixture's time
        let fixture: RunFixture | undefined = undefined;
        const runs: RunOptions = {
            ...options,
            push: (target, mutation, signal, onBehalfOf) => {
                fixture!.pushed.push({
                    installation: target.id,
                    mutation,
                    ...(onBehalfOf === undefined ? {} : { onBehalfOf }),
                });

                return answer(mutation, signal);
            },
            // take a token "lent:<user>:<installation>" as that user's authority lent to that installation
            verify: async (token) => {
                const [kind, user, installation] = token.split(":");
                if (kind !== "lent" || user === undefined || installation === undefined) {
                    throw new TypeError("the token lends no authority");
                }

                const subject = principal.user.reference("universe", user);

                return {
                    subject,
                    subjects: [subject],
                    installation: principal.installation.reference(ids.space, installation),
                    scope: ids.space,
                    expiresAt: Date.now() + 60_000,
                };
            },
        };
        // run the instance on the cell's runtime already, as its host does
        const runtime = new TestRuntime();
        runtime.started.push({ instanceId: INSTANCE } as InstanceSpec);
        fixture = new RunFixture(
            database,
            serveObjects(
                await spaceOptions(database, {
                    runs,
                    runtimes: [runtime],
                    now: () => fixture!.now,
                }),
            ),
        );

        return fixture;
    }

    /** Stop the notes installation's one instance, or run it again. */
    async stop(isStopped: boolean): Promise<void> {
        await this.database
            .update(object.instance.table)
            .set({ status: isStopped ? "stopped" : "running" })
            .where(eq(object.instance.table.id, INSTANCE));
    }

    /** Declare a schedule of the notes installation. */
    async schedule(
        fields: Pick<object.Schedule, "name" | "timing" | "concurrency" | "deadline">,
    ): Promise<object.Schedule> {
        const [declared] = await this.server.executeAsSystem(
            object.schedule,
            "declare",
            [
                {
                    scope: ids.space,
                    input: {
                        installation: ids.notes,
                        packageId: ids.package,
                        call: pin,
                        ...fields,
                    },
                },
            ],
            this.now,
        );

        return declared as object.Schedule;
    }

    /** Record a call the notes installation sends. */
    async send(fields: Readonly<Record<string, unknown>> = {}): Promise<object.Run> {
        const [sent] = await this.server.executeAsSystem(
            object.run,
            "send",
            [
                {
                    scope: ids.space,
                    input: {
                        installation: ids.notes,
                        requestId: RequestId.create(),
                        call: pin,
                        cause: "send",
                        ...fields,
                    },
                },
            ],
            this.now,
        );

        return sent as object.Run;
    }

    /** Suspend the notes installation, or enable it again. */
    async suspend(isSuspended: boolean): Promise<void> {
        await this.database
            .update(object.installation.table)
            .set({ status: isSuspended ? "suspended" : "enabled" })
            .where(eq(object.installation.table.id, ids.notes));
    }

    /** Fire a schedule once, returning the wait until its next occurrence. */
    fire(schedule: object.Schedule): Promise<number | undefined> {
        return reconcile(this.server, "schedule", { id: schedule.id });
    }

    /** Attempt every active run not being attempted already, as the control loop attempts runs side by side. */
    async attempt(): Promise<void> {
        // take the active runs not being attempted
        const active = await this.database
            .select({ id: object.run.table.id })
            .from(object.run.table)
            .where(inArray(object.run.table.state, ["pending", "running"]));
        const ready = active.filter(({ id }) => !this.#attempting.has(id));

        // attempt each at once, keeping it while it runs
        await Promise.all(
            ready.map(async ({ id }) => {
                this.#attempting.add(id);
                try {
                    await reconcile(this.server, "run", { id });
                } finally {
                    this.#attempting.delete(id);
                }
            }),
        );
    }

    /** Run the server's controllers in a control loop until the fixture closes. */
    serve(retry: Partial<RetryPolicy> = { initialInterval: 1, maximumInterval: 1 }): void {
        const stop = new AbortController();
        const loop = new ControlLoop(this.database, this.server.controllers(), {
            report: (_controller, _key, error) => this.reports.push(error),
            retry,
        });
        this.#loops.push({ stop, running: loop.run(stop.signal) });
    }

    /** Wait until no run waits or runs. */
    settle(): Promise<boolean> {
        return this.database.log.until(async () => {
            const active = await this.database
                .select({ id: object.run.table.id })
                .from(object.run.table)
                .where(inArray(object.run.table.state, ["pending", "running"]));

            return active.length === 0;
        }, new AbortController().signal);
    }

    /** Read every run in creation order. */
    async runs(): Promise<object.Run[]> {
        return this.database
            .select()
            .from(object.run.table)
            .where(eq(object.run.table.installation, ids.notes))
            .orderBy(asc(object.run.table.createdAt), asc(object.run.table.id));
    }

    /** Read the request identifier a run's pushes carry. */
    static requestOf(run: Pick<object.Run, "id">): string {
        return identifierUuid(run.id as Identifier<"run">);
    }

    /** Stop the control loops. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const loop of this.#loops) {
            loop.stop.abort();
            await loop.running;
        }
    }
}
