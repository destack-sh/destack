import { declaredStack, type RepositorySelection } from "../../declare/index.ts";
import {
    and,
    type DatabaseConnection,
    eq,
    inArray,
    type RowImage,
    Snapshot,
    type Table,
} from "@destack/db";
import type { ObjectReconciliation } from "@destack/object";
import { type Commit, type Identifier, present } from "@destack/schema";
import { DatabaseError } from "@destack/db";
import { StackError } from "../../error/index.ts";
import { ACTIVE_BUILD_STATES, build, Build } from "../../object/build.ts";
import { installationRevision, Rollout, rollout } from "../../object/rollout.ts";
import type { StackEvaluation } from "../../object/stack.ts";
import type { BuildStore, CommitSource, Registry } from "./build.ts";
import { SelectedRepository } from "./selection.ts";
import { space } from "@destack/account/object";

/** The forge a machine follows: the repositories its spaces' installations select. */
export interface Forge {
    /** Read the account of a repository, absent for a repository without an account. */
    readonly owner: (id: string) => Promise<Identifier<"account"> | undefined>;
    /** The table of the references the space replicates. */
    readonly references: Table;
    /** Read the repository and name of a changed reference row. */
    readonly reference: (row: RowImage<Table>) => {
        readonly repository: string;
        readonly name: string;
    };
    /** Read the commit a reference resolves to in the space's replica, absent while none. */
    readonly commit: (
        database: DatabaseConnection,
        repository: string,
        name: string,
    ) => Promise<Commit | undefined>;
}

/** What following a repository selection builds and evaluates with. */
export interface RepositoryFollowOptions {
    /** The repositories the installations select. */
    readonly forge: Forge;
    /** The machine's store of every build. */
    readonly builds: BuildStore;
    /** The registry of the releases a stack's build depends on. */
    readonly registry: Pick<Registry, "locate" | "referrer">;
}

/** Build the commit an installation's repository reference resolves to, then submit it. */
export async function followRepository(
    row: Rollout & { readonly selection: RepositorySelection },
    reconciliation: ObjectReconciliation,
    options: RepositoryFollowOptions,
): Promise<void> {
    // refuse a repository of another account before reading or building it
    const { selection } = row;
    const { database } = reconciliation;
    const owner = await SelectedRepository.owner(options.forge, selection);
    const [served] = await Snapshot.live(database)
        .read(space.table)
        .findMany({ where: { id: row.scope } });
    if (served?.scope !== owner) {
        throw SelectedRepository.missing(selection.repository);
    }

    // cancel the directory's unfinished builds at commits no selection follows, and wait while the reference resolves to none
    await cancelSuperseded(row.scope, selection, reconciliation, options.forge);
    const commit = await options.forge.commit(database, selection.repository, selection.reference);
    if (commit === undefined) {
        return;
    }

    // create the commit's build once, and wait until it succeeded
    const source: CommitSource = {
        kind: "commit",
        repository: selection.repository,
        commit,
        directory: selection.directory,
    };
    const built = await createBuild(row.scope, source, reconciliation);
    if (built.state !== "succeeded" || (await isFollowing(row, built.id, database))) {
        return;
    }

    // submit the succeeded build, a stack's with its evaluation
    await reconciliation.mutate([
        rollout.call("submit", {
            ...rollout.target(row),
            selection,
            build: { kind: "build", build: built.id },
            ...(row.kind === "stack" ? { evaluation: await evaluate(row, built, options) } : {}),
        }),
    ]);
}

/** Cancel the controller's unfinished builds of a repository directory at other commits. */
async function cancelSuperseded(
    scope: Identifier<"space">,
    selected: RepositorySelection,
    reconciliation: ObjectReconciliation,
    forge: Forge,
): Promise<void> {
    // read the commits the space's selections of the repository directory follow
    const { database } = reconciliation;
    const isSame = (source: { readonly repository: string; readonly directory: string }) =>
        source.repository === selected.repository && source.directory === selected.directory;
    const selecting = await Rollout.selecting(database, scope, isSame);
    const followed = new Set(
        await Promise.all(
            selecting.map(({ selection }) =>
                forge.commit(database, selection.repository, selection.reference),
            ),
        ),
    );

    // read the principal's unfinished builds of the directory at other commits
    const active = await database
        .select()
        .from(build.table)
        .where(
            and(
                build.inScope(scope),
                inArray(build.table.state, [...ACTIVE_BUILD_STATES]),
                eq(build.table.createdById, reconciliation.caller.subject.id),
            ),
        );
    const superseded = active.filter(
        (each) =>
            each.source.kind === "commit" &&
            isSame(each.source) &&
            !followed.has(each.source.commit),
    );

    // cancel each build that has not ended or changed since
    for (const each of superseded) {
        try {
            await reconciliation.mutate([build.call("cancel", build.target(each))]);
        } catch (error) {
            // leave a build that ended meanwhile
            if (!StackError.is(error, "BUILD_STATE_CONFLICT")) {
                throw error;
            }
        }
    }
}

/** Read the live build of a commit source in a space, creating it when the space has none. */
async function createBuild(
    scope: Identifier<"space">,
    source: CommitSource,
    reconciliation: ObjectReconciliation,
): Promise<Build> {
    // reuse the live build of the source
    const live = await Build.find(Snapshot.live(reconciliation.database), scope, source);
    if (live !== undefined) {
        return live;
    }

    // create one for the space's build runner to run
    try {
        const [created] = await reconciliation.mutate([
            build.call("create", { ...build.in(scope), source }),
        ]);

        return Build.of(present(created, "the created build"));
    } catch (error) {
        // adopt the live build of the same checkout, which the creation refuses to repeat
        if (!StackError.is(error, "BUILD_DUPLICATE") && !DatabaseError.is(error, "DUPLICATE")) {
            throw error;
        }
        return present(
            await Build.find(Snapshot.live(reconciliation.database), scope, source),
            "the live build of the source",
        );
    }
}

/** Report whether an installation follows a revision of a build. */
async function isFollowing(
    row: Rollout,
    buildId: Identifier<"build">,
    database: DatabaseConnection,
): Promise<boolean> {
    // follow nothing before the first submission
    if (row.revisionId === null) {
        return false;
    }

    // compare the build of the revision it follows
    const [revision] = await database
        .select({ buildId: installationRevision.table.buildId })
        .from(installationRevision.table)
        .where(eq(installationRevision.table.id, row.revisionId));

    return revision?.buildId === buildId;
}

/** Evaluate a stack installation's export of a succeeded build with its arguments. */
async function evaluate(
    row: Rollout,
    built: Build,
    options: RepositoryFollowOptions,
): Promise<StackEvaluation> {
    // read the stack's export and arguments
    const { export: exported, parameters } = row;
    if (exported === null || parameters === null) {
        throw new TypeError(`stack ${row.id} has no export`);
    }

    // read the stack the build declares at the export
    const reader = await options.builds.packages.contents(Build.manifest(built));
    const definition = await declaredStack(reader, exported, {}, (packageId, manifest) =>
        options.builds.pull(options.registry, packageId, manifest),
    );

    return { export: exported, parameters, definition };
}
