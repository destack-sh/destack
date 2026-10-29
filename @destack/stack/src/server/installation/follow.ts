import { Manager } from "@destack/access";
import { type RepositorySelection } from "@destack/object";
import { declaredStack } from "../../declare/index.ts";
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
import {
    installationRevision,
    type InstallationRollout,
    Rollout,
    rollout,
} from "../../object/rollout.ts";
import type { StackEvaluation } from "../../object/stack.ts";
import type { BuildStore, CommitSource, Registry } from "./build.ts";
import { SelectedRepository } from "./selection.ts";
import { space } from "@destack/account/object";

/** The forge a machine follows: the references and tags its spaces' installations select. */
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
    /** The table of the distribution tags the space replicates. */
    readonly tags: Table;
    /** Read the package and name of a changed tag row. */
    readonly tag: (row: RowImage<Table>) => { readonly package: string; readonly name: string };
    /** Read the version a tag points at in the space's replica, absent while none. */
    readonly version: (
        database: DatabaseConnection,
        packageId: string,
        name: string,
    ) => Promise<string | undefined>;
}

/** What building an installation's selected commit builds and evaluates with. */
export interface CommitBuildOptions {
    /** The repositories the installations select. */
    readonly forge: Forge;
    /** The space's builds, read through to the machine's cache. */
    readonly builds: BuildStore;
    /** The registry of the releases a stack's build depends on. */
    readonly registry: Pick<Registry, "locate" | "referrer">;
}

/** The build of the commit an installation's selected reference resolves to. */
export class CommitBuild {
    /** The installation's rollout, with its installation's fields. */
    readonly #target: InstallationRollout;
    /** The repository reference the installation selects. */
    readonly #selection: RepositorySelection;
    /** What the machine builds and evaluates with. */
    readonly #options: CommitBuildOptions;

    /** Build the commit an installation's selected reference resolves to. */
    constructor(
        target: InstallationRollout,
        selection: RepositorySelection,
        options: CommitBuildOptions,
    ) {
        this.#target = target;
        this.#selection = selection;
        this.#options = options;
    }

    /** Build the reference's commit once and submit it to an unmanaged installation. */
    async reconcile(reconciliation: ObjectReconciliation): Promise<void> {
        // refuse a repository of another account before reading or building it
        const target = this.#target;
        const selection = this.#selection;
        const { forge } = this.#options;
        const { database } = reconciliation;
        const owner = await SelectedRepository.owner(forge, selection);
        const [served] = await Snapshot.live(database)
            .read(space.table)
            .findMany({ where: { id: target.scope } });
        if (served?.scope !== owner) {
            throw SelectedRepository.missing(selection.repository);
        }

        // cancel the directory's unfinished builds at commits no selection follows, and wait while the reference resolves to none
        await this.#cancelSuperseded(reconciliation);
        const commit = await forge.commit(database, selection.repository, selection.reference);
        if (commit === undefined) {
            return;
        }

        // create the commit's build once, and wait until it succeeded, leaving a managed installation's to its manager
        const source: CommitSource = {
            kind: "commit",
            repository: selection.repository,
            commit,
            directory: selection.directory,
        };
        const built = await this.#create(reconciliation, source);
        if (
            built.state !== "succeeded" ||
            Manager.isManaged(target.installation) ||
            (await this.#isTargeted(built.id, database))
        ) {
            return;
        }

        // submit the succeeded build, a stack's with its evaluation
        await reconciliation.mutate([
            rollout.call("submit", {
                ...rollout.target(target),
                selection,
                build: { kind: "build", build: built.id },
                ...(target.installation.export !== null
                    ? { evaluation: await this.#evaluate(built) }
                    : {}),
            }),
        ]);
    }

    /** Cancel the controller's unfinished builds of a repository directory at other commits. */
    async #cancelSuperseded(reconciliation: ObjectReconciliation): Promise<void> {
        // read the commits the space's selections of the repository directory follow
        const { scope } = this.#target;
        const selected = this.#selection;
        const { forge } = this.#options;
        const { database } = reconciliation;
        const isSame = (source: { readonly repository: string; readonly directory: string }) =>
            source.repository === selected.repository && source.directory === selected.directory;
        const selecting = await Rollout.selecting(
            database,
            scope,
            (selection) => selection.kind === "repository" && isSame(selection),
        );
        const followed = new Set(
            await Promise.all(
                selecting.flatMap(({ selection }) =>
                    selection.kind === "repository"
                        ? [forge.commit(database, selection.repository, selection.reference)]
                        : [],
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

    /** Read the live build of a commit source in the space, creating it when there is none. */
    async #create(reconciliation: ObjectReconciliation, source: CommitSource): Promise<Build> {
        // reuse the live build of the source
        const { scope } = this.#target;
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

    /** Report whether the installation targets a revision of a build. */
    async #isTargeted(
        buildId: Identifier<"build">,
        database: DatabaseConnection,
    ): Promise<boolean> {
        // target nothing before the first submission
        const row = this.#target;
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

    /** Evaluate the stack installation's export of a succeeded build with its arguments. */
    async #evaluate(built: Build): Promise<StackEvaluation> {
        // read the stack's export and arguments
        const row = this.#target;
        const options = this.#options;
        const { export: exported, parameters } = row.installation;
        if (exported === null || parameters === null) {
            throw new TypeError(`stack ${row.id} has no export`);
        }

        // read the stack the build declares at the export
        const reader = await options.builds.contents(Build.manifest(built));
        const definition = await declaredStack(reader, exported, {}, (packageId, manifest) =>
            options.builds.pull(options.registry, packageId, manifest),
        );

        return { export: exported, parameters, definition };
    }
}
