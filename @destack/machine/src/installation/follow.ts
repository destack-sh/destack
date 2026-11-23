import {
    installation,
    Installation,
    ACTIVE_BUILD_STATES,
    build,
    Build,
    installationRevision,
} from "@destack/stack/object";
import type { RevisionOptions } from "./target.ts";
import { Manager } from "@destack/access";
import { Space } from "@destack/account/object";
import { reference, referenceShape, tag, tagShape } from "@destack/registry/object";
import { and, eq, inArray, DatabaseError } from "@destack/db";
import type { DatabaseConnection, Snapshot } from "@destack/db/database";
import type { ObjectReconciliation } from "@destack/object";
import { Commit, type Identifier, present, schema } from "@destack/schema";
import { StackError } from "@destack/stack/error";
import type { CommitSource } from "./build.ts";

/** Read a followed reference in a space's replica of its registry, absent once deleted. */
export async function followedReference(
    reader: Pick<Snapshot, "read">,
    id: string,
): Promise<
    | {
          readonly repository: Identifier<"repository">;
          readonly name: string;
          readonly commit: Commit | undefined;
      }
    | undefined
> {
    const followed = await reader
        .read(reference.table)
        .findFirst({ where: { id: schema.identifier("reference").parse(id) } });

    return followed === undefined || followed.deletedAt !== null
        ? undefined
        : {
              repository: followed.parentId,
              name: followed.name,
              commit: followed.commit === null ? undefined : Commit.parse(followed.commit),
          };
}

/** Read the version a followed tag points at in a space's registry replica, absent while none. */
export async function followedVersion(
    reader: Pick<Snapshot, "read">,
    id: string,
): Promise<string | undefined> {
    const followed = await reader
        .read(tag.table)
        .findFirst({ where: { id: schema.identifier("tag").parse(id) } });

    return followed?.version;
}

/** Follow the reference each served installation follows, from its home. */
export const referenceFollow = referenceShape.per(installation, {
    where: { deletedAt: { isNull: true }, followsType: reference.name },
    key: (row, view) => followedKey(row, view, "reference"),
});

/** Follow the tag each served installation follows, from its home. */
export const tagFollow = tagShape.per(installation, {
    where: { deletedAt: { isNull: true }, followsType: tag.name },
    key: (row, view) => followedKey(row, view, "tag"),
});

/** Key the row an installation follows at its home, none outside a served space. */
async function followedKey<Type extends "reference" | "tag">(
    row: Pick<Installation, "scope" | "followsScope" | "followsId">,
    view: Pick<Snapshot, "read">,
    type: Type,
) {
    // follow nothing outside a space served here
    const spaceId = schema.identifier("space").safeParse(row.scope);
    const isServed = spaceId.success && (await Space.isServing(view, spaceId.data));
    if (!isServed || row.followsScope === null || row.followsId === null) {
        return undefined;
    }

    return { key: { scope: row.followsScope, id: schema.identifier(type).parse(row.followsId) } };
}

/** What building an installation's selected commit builds and evaluates with. */
export interface CommitBuildOptions {
    /** The space whose builds are read, through to its machine's builds and registries. */
    readonly space: RevisionOptions["space"];
}

/** The build of the commit an installation's followed reference points at. */
export class CommitBuild {
    /** The installation following the commit. */
    readonly #target: Installation;
    /** The followed reference's identifier, and the package directory within its repository. */
    readonly #followed: { readonly id: string; readonly directory: string };
    /** What the machine builds and evaluates with. */
    readonly #options: CommitBuildOptions;

    /** Build the commit an installation's followed reference points at. */
    constructor(
        target: Installation,
        followed: { readonly id: string; readonly directory: string },
        options: CommitBuildOptions,
    ) {
        this.#target = target;
        this.#followed = followed;
        this.#options = options;
    }

    /** Build the reference's commit once and submit it to an unmanaged installation. */
    async reconcile(reconciliation: ObjectReconciliation): Promise<void> {
        // read the followed reference once replicated, refusing a repository of another account
        const target = this.#target;
        const { id, directory } = this.#followed;
        const { database } = reconciliation;
        const snapshot = database.snapshot();
        const followed = await followedReference(snapshot, id);
        if (followed === undefined) {
            return;
        }
        await this.#options.space.requireRepository(followed.repository);

        // cancel the directory's unfinished builds at commits nothing follows, and wait while the reference points at none
        await this.#cancelSuperseded(reconciliation, followed.repository);
        const { commit } = followed;
        if (commit === undefined) {
            return;
        }

        // create the commit's build once, and wait until it succeeded, leaving a managed installation's to its manager
        const source: CommitSource = {
            kind: "commit",
            repository: followed.repository,
            commit,
            directory,
        };
        const built = await this.#create(reconciliation, source);

        // fail the pass with the failed build's failure
        if (built.state === "failed") {
            const { message } = present(built.failure, `the failure of build '${built.id}'`);
            throw StackError.of(
                "BUILD_BLOCKED",
                `cannot deploy commit '${commit}' of '${target.name}': its build failed: ${message}`,
            );
        }
        // leave a build still running, an installation its manager deploys, and one deployed already
        else if (
            built.state !== "succeeded" ||
            Manager.isManaged(target) ||
            (await this.#isTargeted(built.id, database))
        ) {
            return;
        }

        // apply the installation at the succeeded build
        await reconciliation.mutate([
            installation.call("reconcile", {
                ...installation.target({ scope: target.scope, id: target.id }),
                build: built.id,
            }),
        ]);
    }

    /** Cancel the controller's unfinished builds of a repository directory at other commits. */
    async #cancelSuperseded(
        reconciliation: ObjectReconciliation,
        repository: Identifier<"repository">,
    ): Promise<void> {
        // read the commits the space's installations following references of the directory point at
        const { scope } = this.#target;
        const { directory } = this.#followed;
        const { database } = reconciliation;
        const snapshot = database.snapshot();
        const isSame = (source: { readonly repository: string; readonly directory: string }) =>
            source.repository === repository && source.directory === directory;
        const following = await snapshot.read(installation.table).findMany({
            where: { scope, followsType: reference.name, directory, deletedAt: { isNull: true } },
        });
        const references = await Promise.all(
            following.map((each) =>
                followedReference(snapshot, present(each.followsId, "a followed reference")),
            ),
        );
        const followed = new Set(
            references.flatMap((each) =>
                each !== undefined && each.repository === repository ? [each.commit] : [],
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
                if (!StackError.is(error, "BUILD_BLOCKED")) {
                    throw error;
                }
            }
        }
    }

    /** Read the live build of a commit source in the space, creating it when there is none. */
    async #create(reconciliation: ObjectReconciliation, source: CommitSource): Promise<Build> {
        // reuse the live build of the source
        const scope = Installation.space(this.#target);
        const live = await Build.find(reconciliation.database.snapshot(), scope, source);
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
            if (!StackError.is(error, "BUILD_EXISTS") && !DatabaseError.is(error, "ROW_EXISTS")) {
                throw error;
            }
            return present(
                await Build.find(reconciliation.database.snapshot(), scope, source),
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
        if (row.targetRevisionId === null) {
            return false;
        }

        // compare the build of the revision it follows
        const [revision] = await database
            .select({ buildId: installationRevision.table.buildId })
            .from(installationRevision.table)
            .where(eq(installationRevision.table.id, row.targetRevisionId));

        return revision?.buildId === buildId;
    }
}
