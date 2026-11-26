import { PackageDirectory, defineObject, field, type ResultsOf } from "@destack/object";
import { and, desc, eq, isNull, type Select } from "@destack/db";
import type { DatabaseConnection, Snapshot } from "@destack/db/database";
import { none, principal, through } from "@destack/access";
import { PackageId, PackageRelease } from "@destack/package";
import type { Call } from "@destack/sync";
import {
    canonicalize,
    Commit,
    Digest,
    type Identifier,
    present,
    schema,
    Version,
} from "@destack/schema";
import { space } from "@destack/account/object";
import { Failure } from "@destack/error";
import { installationRevision } from "./revision.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";
import { argument, plural, t } from "@destack/locale";
import { defineWorkflow } from "@destack/workflow";

/** Where a build stands: compiling, or finished one of three ways. */
export const BUILD_STATES = ["running", "succeeded", "failed", "cancelled"] as const;

/** The states of a build that ended. */
export const FINISHED_BUILD_STATES = ["succeeded", "failed", "cancelled"] as const;

/** The source a build has: a commit or a release. */
export const BuildSource = schema.discriminatedUnion("kind", [
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
        /** The digest of the release's image: its build and every build it runs with. */
        image: Digest,
    }),
]);
/** The source a build has. */
export type BuildSource = schema.Infer<typeof BuildSource>;

/** What a build's compile answers: the image stored and the outputs it compiled. */
export const CompileOutput = schema.object({
    /** The digest of the stored image. */
    image: Digest,
    /** The outputs compiled. */
    outputs: schema.array(schema.string().min(1)).min(1),
});
/** What a build's compile answers. */
export type CompileOutput = schema.Infer<typeof CompileOutput>;

/** One compilation of a package's source into a stored build, which its compile run fills. */
export const build = defineObject(
    {
        name: "build",
        plural: "builds",
        title: t`${plural(argument("count"), { one: "Build", other: "Builds" })}`,
        scope: space,
        fields: {
            // what to build
            /** The source compiled. */
            source: field.json(BuildSource),
            /** The outputs, by the names the exports imply; every one while absent. */
            outputs: field.json(schema.array(schema.string().min(1))).optional(),

            // how it runs
            /** Where the build stands. */
            state: field.enum(BUILD_STATES),
            /** The version the build releases its package at, absent for a development build. */
            version: field.string(Version).optional(),
            /** The canonical form of the source, outputs and version, one live build's each. */
            key: field.string(),
            /** The commit of a commit build, which the followers of its references join on. */
            commit: field.string(Commit).optional(),
            /** The machine compiling a commit build. */
            runner: field.reference(principal.machine).optional(),
            /** The stored build's image once succeeded, kept until retention expires it. */
            image: field.blob({ while: { expiredAt: { isNull: true } } }).optional(),
            /** Why the build failed, once failed. */
            failure: field.json(Failure).optional(),
            /** The start of the build's run, in UTC epoch milliseconds. */
            startedAt: field.time(),
            /** The finish, in UTC epoch milliseconds. */
            finishedAt: field.time().optional(),
            /** When retention removed the succeeded build's files, in UTC epoch milliseconds. */
            expiredAt: field.time().optional(),
        },
        constraints: {
            state: { kind: "index", on: ["state"] },
            live: { kind: "index", on: ["scope", "expiredAt", "state"] },
            // build each source of a space once while its build lives
            source: {
                kind: "index",
                on: ["scope", "key"],
                unique: true,
                where: { state: { ne: "cancelled" }, expiredAt: { isNull: true } },
            },
            image: {
                kind: "check",
                condition: {
                    OR: [
                        { state: "succeeded", image: { isNotNull: true } },
                        { state: { ne: "succeeded" }, image: { isNull: true } },
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
            runner: {
                kind: "check",
                condition: {
                    OR: [
                        { commit: { isNotNull: true }, runnerId: { isNotNull: true } },
                        { commit: { isNull: true }, runnerId: { isNull: true } },
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
                        { state: { in: ["running", "failed", "cancelled"] } },
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
        container: "scope",
        access: {
            // let the space's readers read its builds and their index
            permissions: {
                read: through("scope", "read"),
                create: none(),
                cancel: none(),
                record: none(),
                push: none(),
            },
        },
    },
    (method) => ({
        create: method.create("create", {
            fields: ["source", "outputs", "version", "runnerId"],
            onConflict: { key: "source" },
        }),
        cancel: method.update("cancel", { fields: [] }),
        update: method.update("record", { visibility: "package" }),
        /** Push the succeeded build's files to the forge of its package's account. */
        push: method.mutation({
            title: t.context("action")`Push`,
            permission: "push",
            input: schema.object({
                /** The account publishing the build's package. */
                account: schema.identifier("account"),
            }),
            output: schema.object({ image: Digest }),
            prepared: schema.object({ image: Digest }),
        }),
        /** Publish the succeeded build and its unpublished dependencies for the caller. */
        publish: method.mutation({
            title: t.context("action")`Publish`,
            permission: "push",
            input: schema.object({
                /** The account publishing the build's package. */
                account: schema.identifier("account"),
                /** The tag the release takes, latest by default. */
                tag: schema.string().min(1).exactOptional(),
            }),
            output: schema.object({ releases: schema.array(PackageRelease) }),
            prepared: schema.object({ releases: schema.array(PackageRelease) }),
        }),
    }),
);

/** What a build is keyed by: its source, its outputs and its version. */
export interface BuildKey {
    /** The source compiled. */
    readonly source: BuildSource;
    /** The outputs compiled, null for every one. */
    readonly outputs: readonly string[] | null;
    /** The version released, null for a development build. */
    readonly version: string | null;
}

/** A build's state with the image and failure the table's checks require of it. */
export type BuildState =
    | { readonly state: "succeeded"; readonly image: Digest; readonly failure: null }
    | { readonly state: "failed"; readonly image: null; readonly failure: Failure }
    | {
          readonly state: "running" | "cancelled";
          readonly image: null;
          readonly failure: null;
      };

/** One recorded build: a succeeded build has its image, a failed build its failure. */
export type Build = Omit<Select<typeof build.table>, keyof BuildState> & BuildState;

/** What build retention retains and drops. */
export interface BuildRetention {
    /** The images of the retained builds, releases among them. */
    readonly retained: ReadonlySet<Digest>;
    /** The unexpired builds retention drops. */
    readonly expired: readonly {
        readonly id: Identifier<"build">;
        readonly scope: Identifier<"space">;
    }[];
}

/** The builds a space has: compiled from a commit, or pulled as a release. */
export const Build = {
    /** Narrow a build row by its state, failing on a row the table's checks refuse. */
    of<Row extends Pick<Select<typeof build.table>, "id" | keyof BuildState>>(
        row: Row,
    ): Omit<Row, keyof BuildState> & BuildState {
        const { state, image, failure } = row;

        // accept a succeeded build with its image
        if (state === "succeeded" && image !== null && failure === null) {
            return { ...row, state, image, failure };
        }
        // accept a failed build with its failure
        else if (state === "failed" && image === null && failure !== null) {
            return { ...row, state, image, failure };
        }
        // accept an unfinished or cancelled build without either
        else if (
            state !== "succeeded" &&
            state !== "failed" &&
            image === null &&
            failure === null
        ) {
            return { ...row, state, image, failure };
        }
        // refuse a row the table's checks refuse
        else {
            throw new TypeError(
                `build '${row.id}' is '${state}' with a mismatched image or failure`,
            );
        }
    },

    /** Report whether a build ended. */
    isFinished(entry: Pick<Select<typeof build.table>, "state">): boolean {
        return FINISHED_BUILD_STATES.some((state) => state === entry.state);
    },

    /** Read a build's image. */
    image(entry: Pick<Select<typeof build.table>, "id" | "expiredAt" | keyof BuildState>): Digest {
        // read a succeeded build's image until retention removes its files
        const narrowed = Build.of(entry);
        if (narrowed.state === "succeeded" && narrowed.expiredAt === null) {
            return narrowed.image;
        }
        // refuse an expired build
        else if (narrowed.state === "succeeded") {
            throw StackError.of("BUILD_NOT_FOUND", `build '${narrowed.id}' not found`);
        }
        // refuse a failed build with its failure
        else if (narrowed.state === "failed") {
            throw Failure.error(narrowed.failure);
        }
        // refuse a build without an image
        else {
            throw StackError.of(
                "BUILD_UNAVAILABLE",
                `build '${narrowed.id}' is unavailable: it is '${narrowed.state}'`,
            );
        }
    },

    /** Find the live build of a source, its outputs and version in a space. */
    async find(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        keyed: BuildKey,
    ): Promise<Build | undefined> {
        const found = await snapshot.read(build.table).findFirst({
            where: {
                scope: spaceId,
                key: Build.key(keyed),
                state: { ne: "cancelled" },
                expiredAt: { isNull: true },
            },
        });

        return found === undefined ? undefined : Build.of(found);
    },

    /** Write the canonical form of a source, its outputs and version, the key of its live build. */
    key(keyed: BuildKey): string {
        const { source, outputs, version } = keyed;

        return canonicalize({ source, outputs, version });
    },

    /** Require a space's build with its image, as a revision evaluates it. */
    async require(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        buildId: Identifier<"build">,
    ): Promise<Select<typeof build.table> & { readonly image: Digest }> {
        // require the space's build, refusing one without its files
        const found = await Build.get(snapshot, spaceId, buildId);

        return { ...found, image: Build.image(found) };
    },

    /** Read a space's build in any state, refusing one the space does not have. */
    async get(
        snapshot: Pick<Snapshot, "read">,
        spaceId: Identifier<"space">,
        buildId: Identifier<"build">,
    ): Promise<Build> {
        const [found] = await snapshot
            .read(build.table)
            .findMany({ where: { scope: spaceId, id: buildId } });
        if (found === undefined) {
            throw StackError.of("BUILD_NOT_FOUND", `build '${buildId}' not found`);
        }

        return Build.of(found);
    },

    /** Read a space's build of a release, created pulled and born succeeded the first time. */
    async release(
        reader: Pick<Snapshot, "read">,
        mutate: <const Calls extends readonly Call[]>(calls: Calls) => Promise<ResultsOf<Calls>>,
        spaceId: Identifier<"space">,
        release: {
            readonly package: PackageId;
            readonly version: string;
            readonly image: Digest;
        },
    ): Promise<{ readonly id: Identifier<"build">; readonly image: Digest }> {
        // create it pulled from the registry the first time, else take its live build
        const { package: packageId, version, image } = release;
        const source = { kind: "release" as const, package: packageId, version, image };
        const [created] = await mutate([build.call("create", { ...build.in(spaceId), source })]);
        const { id } = created;

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
                image: build.table.image,
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
                    retained.add(present(row.image, "a succeeded build's image"));
                } else {
                    expired.push({ id, scope: schema.identifier("space").parse(row.scope) });
                }
            }
        }

        return { retained, expired };
    },
};

/** Read the package a build source has: a repository's directory, or a release. */
function sourcePackage(source: BuildSource): readonly string[] {
    // group a repository's builds by its directory
    if (source.kind === "commit") {
        return [source.kind, source.repository, source.directory];
    }
    // group a release's builds by its package
    else {
        return [source.kind, source.package];
    }
}

/** Compile a build's source on a runner, storing its image and recording it on the build. */
export const compile = defineWorkflow({
    name: "compile",
    title: t.context("title")`Compile`,
    input: schema.object({
        /** The build compiled. */
        build: schema.identifier("build"),
    }),
    output: CompileOutput,
});
