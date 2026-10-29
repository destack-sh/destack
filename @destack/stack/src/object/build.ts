import { PackageDirectory } from "@destack/object";
import {
    and,
    desc,
    eq,
    isNull,
    type DatabaseConnection,
    type Select,
    type Snapshot,
} from "@destack/db";
import { none, relation, through, union } from "@destack/access";
import { defineObject, defineShape, field } from "@destack/object";
import { PackageFile } from "@destack/package/file";
import { PackageManifest } from "@destack/package/manifest";
import { PackageId } from "@destack/package";
import type { ResultsOf } from "@destack/object";
import type { Call } from "@destack/sync";
import { Commit, Digest, type Identifier, present, schema, Version } from "@destack/schema";
import { machine, space } from "@destack/account/object";
import { ObjectReference, Subject } from "@destack/sync";
import { Failure } from "@destack/error";
import { installationRevision } from "./rollout.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";

/** Where a build stands: waiting for its runner, running, or finished one of three ways. */
export const BUILD_STATES = ["pending", "running", "succeeded", "failed", "cancelled"] as const;

/** The states of a build its runner runs or will run. */
export const ACTIVE_BUILD_STATES = ["pending", "running"] as const;

/** The states of a build that ended. */
export const FINISHED_BUILD_STATES = ["succeeded", "failed", "cancelled"] as const;

/** The steps of a running build: resolving its source, compiling it, and storing the result. */
export const BUILD_PHASES = ["preparing", "building", "storing"] as const;

/** The source a build has: a checkout with unpublished changes, a commit, or a release. */
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
    schema.object({
        /** A published release, pulled from its registry rather than compiled here. */
        kind: schema.literal("release"),
        /** The released package. */
        package: PackageId,
        /** The released version. */
        version: Version,
        /** The digest of the release's manifest, which identifies its exact build. */
        manifest: Digest,
    }),
]);
/** The source a build has. */
export type BuildSource = schema.Infer<typeof BuildSource>;

/** The progress a runner reports: compiling the outputs, or storing the compiled build. */
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
        /** Storing the compiled build through the space's builds adapter. */
        phase: schema.literal("storing"),
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
        /** The digest and size of its code index, which the runner stored as a file. */
        code: PackageFile.pick({ digest: true, size: true }),
    }),
    FailedBuild,
]);

/** How a build ended, as its space records it. */
export const BuildOutcome = schema.discriminatedUnion("kind", [
    schema.object({
        /** A build whose manifest the space stores. */
        kind: schema.literal("stored"),
        /** The digest of the stored manifest. */
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
            /** The outputs, by the names the exports imply; every one while absent. */
            outputs: field.json(schema.array(schema.string().min(1))).optional(),

            // how it runs
            /** Where the build stands. */
            state: field.enum(BUILD_STATES).default("pending"),
            /** The step of a running build. */
            phase: field.enum(BUILD_PHASES).optional(),
            /** The commit a clean checkout was at when built. */
            commit: field.string(Commit).optional(),
            /** The repository, directory and commit of a commit build, absent for a release. */
            checkout: field.string().optional(),
            /** The stored build's manifest once succeeded, kept until retention expires it. */
            manifest: field.blob({ while: { expiredAt: { isNull: true } } }).optional(),
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
        // let the space's readers read its builds and their index, and a build's runner run it
        permissions: {
            read: union(relation("runner"), through("scope", "read")),
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
            // build each checkout of a space once while its build lives
            liveCheckout: {
                kind: "index",
                on: ["scope", "checkout"],
                unique: true,
                where: {
                    checkout: { isNotNull: true },
                    state: { ne: "cancelled" },
                    expiredAt: { isNull: true },
                },
            },
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
        update: method.update("record", { visibility: "package" }),
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
            output: schema.object({}),
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
    | { readonly state: "succeeded"; readonly manifest: Digest; readonly failure: null }
    | { readonly state: "failed"; readonly manifest: null; readonly failure: Failure }
    | {
          readonly state: "pending" | "running" | "cancelled";
          readonly manifest: null;
          readonly failure: null;
      };

/** One recorded build: a succeeded build has its manifest, a failed build its failure. */
export type Build = Omit<Select<typeof build.table>, keyof BuildState> & BuildState;

/** What build retention retains and drops. */
export interface BuildRetention {
    /** The manifests of the retained builds, releases among them. */
    readonly retained: ReadonlySet<Digest>;
    /** The unexpired builds retention drops. */
    readonly expired: readonly {
        readonly id: Identifier<"build">;
        readonly scope: Identifier<"space">;
    }[];
}

/** The builds a space has: compiled from a checkout or a commit, or pulled as a release. */
export const Build = {
    /** Narrow a build row by its state, failing on a row the table's checks refuse. */
    of<Row extends Pick<Select<typeof build.table>, "id" | keyof BuildState>>(
        row: Row,
    ): Omit<Row, keyof BuildState> & BuildState {
        const { state, manifest, failure } = row;

        // accept a succeeded build with its manifest
        if (state === "succeeded" && manifest !== null && failure === null) {
            return { ...row, state, manifest, failure };
        }
        // accept a failed build with its failure
        else if (state === "failed" && manifest === null && failure !== null) {
            return { ...row, state, manifest, failure };
        }
        // accept an unfinished or cancelled build without either
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
    ): Digest {
        // read a succeeded build's manifest until retention removes its files
        const narrowed = Build.of(entry);
        if (narrowed.state === "succeeded" && narrowed.expiredAt === null) {
            return narrowed.manifest;
        }
        // refuse an expired build
        else if (narrowed.state === "succeeded") {
            throw StackError.of("BUILD_EXPIRED", `build ${narrowed.id} expired`);
        }
        // refuse a failed build with its failure
        else if (narrowed.state === "failed") {
            throw Failure.error(narrowed.failure);
        }
        // refuse a build without a manifest
        else {
            throw StackError.of("BUILD_NOT_READY", `build ${narrowed.id} is ${narrowed.state}`);
        }
    },

    /** Find the one live build of a repository commit's package directory in a space. */
    async find(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        source: Extract<BuildSource, { readonly kind: "commit" }>,
    ): Promise<Build | undefined> {
        // read the space's live build of the checkout
        const found = await snapshot.read(build.table).findFirst({
            where: {
                scope: spaceId,
                checkout: Build.checkout(source),
                state: { ne: "cancelled" },
                expiredAt: { isNull: true },
            },
        });

        return found === undefined ? undefined : Build.of(found);
    },

    /** Key a commit build's checkout: its repository, directory and commit. */
    checkout(source: Extract<BuildSource, { readonly kind: "commit" }>): string {
        return JSON.stringify([source.repository, source.directory, source.commit]);
    },

    /** Require a space's build with its manifest, as a revision evaluates it. */
    async require(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        buildId: Identifier<"build">,
    ): Promise<Select<typeof build.table> & { readonly manifest: Digest }> {
        // require the space's build, refusing one without its files
        const [found] = await snapshot
            .read(build.table)
            .findMany({ where: { scope: spaceId, id: buildId } });
        if (found === undefined) {
            throw StackError.of("BUILD_NOT_FOUND", `build ${buildId} not found`);
        }

        return { ...found, manifest: Build.manifest(found) };
    },

    /** Read a space's build of a release, creating it pulled and born succeeded the first time. */
    async release(
        reader: Pick<Snapshot, "read">,
        mutate: <const Calls extends readonly Call[]>(calls: Calls) => Promise<ResultsOf<Calls>>,
        spaceId: Identifier<"space">,
        release: {
            readonly package: PackageId;
            readonly version: string;
            readonly manifest: Digest;
        },
    ): Promise<{ readonly id: Identifier<"build">; readonly manifest: Digest }> {
        // take the space's live build of the release's manifest
        const [found] = await reader.read(build.table).findMany({
            where: { scope: spaceId, manifest: release.manifest, expiredAt: { isNull: true } },
        });

        // create it pulled from the registry the first time
        const { package: packageId, version, manifest } = release;
        const source = { kind: "release" as const, package: packageId, version, manifest };
        const id =
            found?.id ??
            (await mutate([build.call("create", { ...build.in(spaceId), source })]))[0].id;

        return Build.require(reader, spaceId, id);
    },

    /** Read what retention retains. */
    async retention(
        database: DatabaseConnection,
        retention: (space: string) => Promise<number>,
    ): Promise<BuildRetention> {
        // read each build revisions apply once
        const revisions = await database
            .selectDistinct({ build: installationRevision.table.buildId })
            .from(installationRevision.table);
        const appliedBuilds = new Set(revisions.map((revision) => revision.build));
        const retained = new Set<Digest>();

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

        // retain each group's latest builds by its space's retention, read once per space
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

/** Read the package a build source has: a checkout or repository directory, or a release. */
function sourcePackage(source: BuildSource): readonly string[] {
    // group a checkout's builds by its directory
    if (source.kind === "checkout") {
        return [source.kind, source.machine, source.checkout, source.directory];
    }
    // group a repository's builds by its directory
    else if (source.kind === "commit") {
        return [source.kind, source.repository, source.directory];
    }
    // group a release's builds by its package
    else {
        return [source.kind, source.package];
    }
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
