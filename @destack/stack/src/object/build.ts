import {
    and,
    desc,
    eq,
    isNull,
    type DatabaseConnection,
    type Select,
    type Snapshot,
} from "@destack/db";
import { none, relation } from "@destack/access";
import { defineObject, defineShape, field } from "@destack/object";
import { PackageManifest } from "@destack/package/manifest";
import { Lease } from "@destack/resource";
import { Commit, Digest, type Identifier, present, schema } from "@destack/schema";
import { errorOf, CallError } from "@destack/runtime/error";
import { type InstallationBuild, PackageDirectory } from "../declare/installation.ts";
import { machine, space } from "@destack/account/object";
import { Failure, ObjectReference, Subject } from "@destack/sync";
import { installationRevision } from "./installation.ts";

/** Where a build stands: waiting for its runner, running, or finished one of three ways. */
export const BUILD_STATES = ["pending", "running", "succeeded", "failed", "cancelled"] as const;

/** The states of a build its runner runs or will run. */
export const ACTIVE_BUILD_STATES = ["pending", "running"] as const;

/** The states of a build that ended. */
export const FINISHED_BUILD_STATES = ["succeeded", "failed", "cancelled"] as const;

/** The steps of a running build: resolving its source, compiling it, and storing the result. */
export const BUILD_PHASES = ["preparing", "building", "storing"] as const;

/** The source a build compiles: a checkout with its unpublished changes, or a commit. */
export const BuildSource = schema.discriminatedUnion("kind", [
    schema.object({
        /** A working directory registered on a machine. */
        kind: schema.literal("checkout"),
        /** The machine administering the checkout, which runs the build. */
        machine: schema.identifier("machine"),
        /** The checkout registered on that machine. */
        checkout: schema.identifier("checkout"),
        /** The package directory within the checkout. */
        directory: PackageDirectory,
    }),
    schema.object({
        /** Committed repository history. */
        kind: schema.literal("commit"),
        /** The registered repository retaining the commit. */
        repository: schema.identifier("repository"),
        /** The complete Git commit object identifier. */
        commit: Commit,
        /** The package directory within the repository. */
        directory: PackageDirectory,
    }),
]);
/** The source a build compiles. */
export type BuildSource = schema.Infer<typeof BuildSource>;

/** A file a runner stores among a space's builds: its content's digest and size. */
export const BuildFile = schema.object({
    /** The digest of the file's content, which keys it. */
    digest: Digest,
    /** The file's size in bytes. */
    size: schema.number().int().nonnegative(),
    /** The file's media type. */
    mediaType: schema.string().min(1),
});
/** A file a runner stores among a space's builds. */
export type BuildFile = schema.Infer<typeof BuildFile>;

/** A file a runner uploads to a space's builds: its digest, written through a lease. */
export const BuildUpload = schema.object({
    /** The digest of the file's content. */
    digest: Digest,
    /** The lease writing it. */
    lease: Lease,
});
/** A file a runner uploads to a space's builds. */
export type BuildUpload = schema.Infer<typeof BuildUpload>;

/** The progress a runner reports: compiling the outputs, or storing files through write leases. */
export const BuildProgress = schema.discriminatedUnion("phase", [
    schema.object({
        /** Compiling the source's outputs. */
        phase: schema.literal("building"),
        /** The outputs compiled. */
        outputs: schema.array(schema.string().min(1)).min(1),
        /** The commit a clean checkout is at, absent for a dirty one. */
        commit: Commit.exactOptional(),
    }),
    schema.object({
        /** Storing the compiled build. */
        phase: schema.literal("storing"),
        /** The files the build stores, each leased for writing unless the space keeps it. */
        files: schema.array(BuildFile),
    }),
]);

/** A build that failed, with why. */
const FailedBuild = schema.object({
    /** A build that failed. */
    kind: schema.literal("failed"),
    /** Why the build failed. */
    failure: Failure,
});

/** How a runner reports a build's end: stored under its manifest, or failed. */
export const BuildEnd = schema.discriminatedUnion("kind", [
    schema.object({
        /** A build whose files the runner stored. */
        kind: schema.literal("stored"),
        /** The stored build's manifest. */
        manifest: PackageManifest,
    }),
    FailedBuild,
]);

/** How a build ended as its space keeps it. */
export const BuildOutcome = schema.discriminatedUnion("kind", [
    schema.object({
        /** A build whose manifest the space keeps. */
        kind: schema.literal("stored"),
        /** The digest of the kept manifest. */
        digest: Digest,
    }),
    FailedBuild,
]);

/** One compilation of a package's source into a stored build, run by its runner. */
export const build = defineObject(
    {
        name: "build",
        plural: "builds",
        scope: space,
        isScope: true,
        fields: {
            // what to build
            /** The source compiled. */
            source: field.json(BuildSource),
            /** The outputs compiled by the names the exports imply, every one while absent. */
            outputs: field.json(schema.array(schema.string().min(1)).min(1)).optional(),

            // how it runs
            /** Where the build stands. */
            state: field.enum(BUILD_STATES).default("pending"),
            /** The step of a running build. */
            phase: field.enum(BUILD_PHASES).optional(),
            /** The commit a clean checkout was at when built. */
            commit: field.string(Commit).optional(),
            /** The digest of the stored build's manifest, once succeeded. */
            manifest: field.string(Digest).optional(),
            /** Why the build failed, once failed. */
            failure: field.json(Failure).optional(),
            /** The start of the build's run, in UTC epoch milliseconds. */
            startedAt: field.time().optional(),
            /** The finish, in UTC epoch milliseconds. */
            finishedAt: field.time().optional(),
            /** The machine running the build, absent on a machine running none. */
            runner: field.subject(machine).optional(),
            /** When retention removed the succeeded build's files, in UTC epoch milliseconds. */
            expiredAt: field.time().optional(),
        },
        relations: {
            runner: { subjects: [machine] },
        },
        // let a build's runner alone read and run it
        permissions: {
            read: relation("runner"),
            create: none(),
            cancel: none(),
            run: relation("runner"),
            record: none(),
            push: none(),
        },
        reserved: ["run"],
        constraints: {
            state: { kind: "index", on: ["state"] },
            live: { kind: "index", on: ["scope", "expiredAt", "state"] },
            phase: {
                kind: "check",
                condition: {
                    OR: [
                        { state: "running", phase: { isNotNull: true } },
                        { state: { ne: "running" }, phase: { isNull: true } },
                    ],
                },
            },
            manifest: {
                kind: "check",
                condition: {
                    OR: [
                        { state: "succeeded", manifest: { isNotNull: true } },
                        { state: { ne: "succeeded" }, manifest: { isNull: true } },
                    ],
                },
            },
            failure: {
                kind: "check",
                condition: {
                    OR: [
                        { state: "failed", failure: { isNotNull: true } },
                        { state: { ne: "failed" }, failure: { isNull: true } },
                    ],
                },
            },
            expiry: {
                kind: "check",
                condition: { OR: [{ expiredAt: { isNull: true } }, { state: "succeeded" }] },
            },
            outputs: {
                kind: "check",
                condition: {
                    OR: [
                        { outputs: { isNotNull: true } },
                        { state: { in: ["pending", "failed", "cancelled"] } },
                        { phase: "preparing" },
                    ],
                },
            },
            start: {
                kind: "check",
                condition: {
                    OR: [
                        { state: "cancelled" },
                        { state: "pending", startedAt: { isNull: true } },
                        { state: { ne: "pending" }, startedAt: { isNotNull: true } },
                    ],
                },
            },
            finish: {
                kind: "check",
                condition: {
                    OR: [
                        {
                            state: { in: ["succeeded", "failed", "cancelled"] },
                            finishedAt: { isNotNull: true },
                        },
                        {
                            state: { notIn: ["succeeded", "failed", "cancelled"] },
                            finishedAt: { isNull: true },
                        },
                    ],
                },
            },
        },
    },
    (method) => ({
        create: method.create("create", {
            fields: [
                "source",
                "outputs",
                "runnerPackageId",
                "runnerType",
                "runnerScope",
                "runnerId",
            ],
        }),
        cancel: method.update("cancel", { fields: [] }),
        update: method.update("record", { isInternal: true }),
        start: method.mutation({ permission: "run", output: schema.object({}) }),
        /** Push the succeeded build's files to the forge of its package's account. */
        push: method.mutation({
            permission: "push",
            output: schema.object({ manifest: Digest }),
            prepared: schema.object({ manifest: Digest }),
        }),
        progress: method.mutation({
            permission: "run",
            input: schema.object({ progress: BuildProgress }),
            output: schema.object({ uploads: schema.array(BuildUpload) }),
            prepared: schema.object({ uploads: schema.array(BuildUpload) }),
        }),
        finish: method.mutation({
            permission: "run",
            input: schema.object({ end: BuildEnd }),
            output: schema.object({}),
            prepared: BuildOutcome,
        }),
    }),
);

/** A build's state with the manifest and failure the table's checks require of it. */
export type BuildState =
    | { readonly state: "succeeded"; readonly manifest: string; readonly failure: null }
    | { readonly state: "failed"; readonly manifest: null; readonly failure: Failure }
    | {
          readonly state: "pending" | "running" | "cancelled";
          readonly manifest: null;
          readonly failure: null;
      };

/** One recorded build: a succeeded build has its manifest, a failed build its failure. */
export type Build = Omit<Select<typeof build.table>, keyof BuildState> & BuildState;

/** What build retention keeps and drops. */
export interface BuildRetention {
    /** The manifests of the builds and releases kept. */
    readonly retained: ReadonlySet<string>;
    /** The unexpired builds retention drops. */
    readonly expired: readonly {
        readonly id: Identifier<"build">;
        readonly scope: Identifier<"space">;
    }[];
}

/** Reads of builds that installations resolve their submissions by. */
export const Build = {
    /** Narrow a build row by its state, failing on a row the table's checks refuse. */
    of<Row extends Pick<Select<typeof build.table>, "id" | keyof BuildState>>(
        row: Row,
    ): Omit<Row, keyof BuildState> & BuildState {
        const { state, manifest, failure } = row;

        // keep a succeeded build's manifest
        if (state === "succeeded" && manifest !== null && failure === null) {
            return { ...row, state, manifest, failure };
        }
        // keep a failed build's failure
        else if (state === "failed" && manifest === null && failure !== null) {
            return { ...row, state, manifest, failure };
        }
        // keep an unfinished or cancelled build without either
        else if (
            state !== "succeeded" &&
            state !== "failed" &&
            manifest === null &&
            failure === null
        ) {
            return { ...row, state, manifest, failure };
        }
        // refuse a row the table's checks refuse
        else {
            throw new TypeError(
                `build ${row.id} is ${state} with a mismatched manifest or failure`,
            );
        }
    },

    /** Report whether a build ended. */
    isFinished(entry: Pick<Select<typeof build.table>, "state">): boolean {
        return FINISHED_BUILD_STATES.some((state) => state === entry.state);
    },

    /** Read a build's manifest. */
    manifest(
        entry: Pick<Select<typeof build.table>, "id" | "expiredAt" | keyof BuildState>,
    ): string {
        // read a succeeded build's manifest until retention removes its files
        const narrowed = Build.of(entry);
        if (narrowed.state === "succeeded" && narrowed.expiredAt === null) {
            return narrowed.manifest;
        }
        // refuse an expired build
        else if (narrowed.state === "succeeded") {
            throw new CallError("CONFLICT", { message: `build ${narrowed.id} expired` });
        }
        // refuse a failed build with its failure
        else if (narrowed.state === "failed") {
            throw errorOf(narrowed.failure);
        }
        // refuse a build without a manifest
        else {
            throw new CallError("CONFLICT", {
                message: `build ${narrowed.id} is ${narrowed.state}`,
            });
        }
    },

    /** Find the one live build of a repository commit's package directory in a space. */
    async find(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        source: Extract<BuildSource, { readonly kind: "commit" }>,
    ): Promise<Build | undefined> {
        // read the space's live builds and take the one of the commit's package directory
        const live = await snapshot.read(build.table).findMany({
            where: {
                scope: spaceId,
                state: { ne: "cancelled" },
                expiredAt: { isNull: true },
            },
        });
        const found = live.find(
            (entry) =>
                entry.source.kind === "commit" &&
                entry.source.repository === source.repository &&
                entry.source.commit === source.commit &&
                entry.source.directory === source.directory,
        );

        return found === undefined ? undefined : Build.of(found);
    },

    /** Resolve a succeeded build of a space to the build an installation's revision evaluates. */
    async resolve(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        buildId: Identifier<"build">,
    ): Promise<InstallationBuild> {
        // require the space's build
        const [found] = await snapshot
            .read(build.table)
            .findMany({ where: { scope: spaceId, id: buildId } });
        if (found === undefined) {
            throw new CallError("NOT_FOUND", { message: `no build ${buildId}` });
        }

        return { kind: "build", build: buildId, manifest: Build.manifest(found) };
    },

    /** Read what retention keeps. */
    async retention(
        database: DatabaseConnection,
        retention: (space: string) => Promise<number>,
    ): Promise<BuildRetention> {
        // read each build and release revisions apply once
        const revisions = await database
            .selectDistinct({ build: installationRevision.table.build })
            .from(installationRevision.table);
        const applied = revisions.map((revision) => revision.build);
        const retained = new Set(applied.map((applies) => applies.manifest));
        const appliedBuilds = new Set(
            applied.flatMap((applies) => (applies.kind === "build" ? [applies.build] : [])),
        );

        // group the unexpired builds by space and package source, newest first
        const builds = await database
            .select({
                id: build.table.id,
                scope: build.table.scope,
                source: build.table.source,
                manifest: build.table.manifest,
            })
            .from(build.table)
            .where(and(eq(build.table.state, "succeeded"), isNull(build.table.expiredAt)))
            .orderBy(desc(build.table.finishedAt), desc(build.table.id));
        const groups = Map.groupBy(builds, (row) =>
            JSON.stringify([row.scope, sourcePackage(row.source)]),
        );

        // keep each group's latest builds by its space's retention, read once per space
        const expired: { readonly id: Identifier<"build">; readonly scope: Identifier<"space"> }[] =
            [];
        const retentions = new Map<string, Promise<number>>();
        for (const rows of groups.values()) {
            const { scope } = present(rows[0], "a build of the group");
            let limit = retentions.get(scope);
            if (limit === undefined) {
                limit = retention(scope);
                retentions.set(scope, limit);
            }
            const count = await limit;
            for (const [position, row] of rows.entries()) {
                // retain the latest builds and every applied one, and expire the rest
                const id = schema.identifier("build").parse(row.id);
                if (position < count || appliedBuilds.has(id)) {
                    retained.add(present(row.manifest, "a succeeded build's manifest"));
                } else {
                    expired.push({ id, scope: schema.identifier("space").parse(row.scope) });
                }
            }
        }

        return { retained, expired };
    },
};

/** Read the package a build source compiles: a checkout's or a repository's package directory. */
function sourcePackage(source: BuildSource): readonly string[] {
    return source.kind === "checkout"
        ? [source.kind, source.machine, source.checkout, source.directory]
        : [source.kind, source.repository, source.directory];
}

/** The builds of one space assigned to one runner, replicated to the runner's machine. */
export const buildShape = defineShape({
    name: "build",
    parameters: schema.object({
        /** The runner's subject key. */
        runner: schema.string().min(1),
    }),
    objects: [build],
    queries: ({ runner }) => [
        build.query({
            scopes: "every",
            where: ObjectReference.columns("runner", Subject.read(runner)),
        }),
    ],
});
