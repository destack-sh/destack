import { Space, space } from "@destack/account/object";
import { type Identifier, type JsonObject, type JsonValue } from "@destack/schema";
import { and, type DatabaseConnection, eq, Snapshot, Transaction } from "@destack/db";
import {
    ObjectDescription,
    ObjectError,
    type ObjectReconciliation,
    type ObjectType,
} from "@destack/object";
import { type ApplyOptions, Stack, type ObjectStore } from "@destack/object/server";
import type { PackageRelease, PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { PackageDescription } from "@destack/runtime";
import { CallError } from "@destack/runtime/error";
import { install, type InstallationDefinition } from "../../declare/installation.ts";
import { StackDefinition } from "../../declare/stack.ts";
import { StackError } from "../../error/index.ts";
import stackPackage from "../../package.ts";
import {
    Installation,
    installation,
    InstallationRevision,
    installationRevision,
    MAIN_DATABASE,
    type StackReason,
} from "../../object/index.ts";
import { Approval } from "./approval.ts";
import { implementRelationship, implementRole } from "../access/role.ts";
import { type BuildStore, installableReleases } from "./build.ts";
import { Failure } from "@destack/sync";

/** What a machine applies stacks with: its build store. */
export interface StackOptions {
    /** The machine's store of every build. */
    readonly builds: BuildStore;
}

/** Apply the revision a served space's stack follows and observe it applied or waiting. */
export async function reconcileStack(
    spaceId: Identifier<"space">,
    reconciliation: ObjectReconciliation,
    options: StackOptions,
): Promise<undefined> {
    // read the stack of a served space and the revision it follows
    const database = reconciliation.database;
    const served = await readServedStack(spaceId, database);
    if (served === undefined || served.stack.appliedRevisionId === served.revision.id) {
        return undefined;
    }
    const { stack, revision } = served;

    // plan the declarations once and apply the plan unchanged unless it waits for an approval, again while it loses a race
    let deferred: string | undefined;
    try {
        const release: StackApplicationOptions["release"] = (snapshot, packageId, installationId) =>
            options.builds.installed(snapshot, spaceId, packageId, installationId);
        const objects = declaredTypes(reconciliation.store.objects);
        const store = reconciliation.store;
        const applying = stackApplication(database, stack, revision, objects, { store, release });
        const outcome = await Transaction.retry(async () => {
            // plan, and apply the plan unless it waits for an approval
            const planned = await Stack.plan(applying);
            const waiting = await Approval.waiting(Snapshot.live(database), stack, planned.plan);
            if (waiting === undefined) {
                await planned.apply();
            }

            return { waiting, deferred: planned.plan.deferred };
        });

        // observe a plan waiting for its approval
        if (outcome.waiting !== undefined) {
            await reconciliation.observe(installation, stack, outcome.waiting);

            return undefined;
        }
        deferred = outcome.deferred;
    } catch (error) {
        // block on a definition the next revision must fix and retry another failure
        const isBlocked =
            (error instanceof StackError && error.code === "INVALID_DEFINITION") ||
            (error instanceof ObjectError && error.code === "INVALID_DECLARATION");
        const { message } = Failure.of(error);
        await reconciliation.observe(
            installation,
            stack,
            ready(isBlocked ? "Blocked" : "ApplyFailed", message, {}),
        );
        if (!isBlocked) {
            throw error;
        }

        return undefined;
    }

    // observe it applied, forgetting the plan it applied, or waiting, keeping the approved plan the rest resumes
    if (deferred === undefined) {
        const fields = { plan: null, approvedPlan: null };
        const applied = { ...fields, ...Installation.applied(stack, revision.id) };
        await reconciliation.observe(
            installation,
            stack,
            ready("Applied", "every declaration applied", applied),
        );
    } else {
        await reconciliation.observe(
            installation,
            stack,
            ready("WaitingForResources", deferred, {}),
        );
    }

    return undefined;
}

/** Read the stack of a served space and the revision it follows. */
async function readServedStack(spaceId: Identifier<"space">, database: DatabaseConnection) {
    const [served] = await database
        .select({ stack: installation.table, revision: installationRevision.table })
        .from(installation.table)
        .innerJoin(space.table, eq(space.table.id, installation.table.scope))
        .innerJoin(
            installationRevision.table,
            eq(installationRevision.table.id, installation.table.revisionId),
        )
        .where(
            and(
                eq(installation.table.scope, spaceId),
                eq(installation.table.role, "stack"),
                Space.served(),
            ),
        );

    return served;
}

/** Read the stack a stack package declares at an export, with its installations' dependencies. */
export async function declaredStack(
    reader: BuildReader,
    name: string,
    settings: Readonly<Record<string, JsonValue>>,
    read: (packageId: PackageId, manifest: string) => Promise<BuildReader>,
): Promise<JsonObject> {
    // find the symbols the package exports under the name
    const modules = await reader.modules();
    const exported = new Set(
        modules.flatMap((module) =>
            module.exports.filter((entry) => entry.name === name).map((entry) => entry.symbol),
        ),
    );

    // require the package's stack declared at one of them
    const found = (await reader.declared(stackPackage.id, "stack", StackDefinition)).find((entry) =>
        exported.has(entry.symbol),
    );
    if (found === undefined) {
        throw new CallError("BAD_REQUEST", {
            message: `stack package declares no stack ${name}`,
        });
    }
    const declared = found.description;

    // read the one release the build resolved each Destack package at
    const resolved = oneReleaseEach(await reader.releases(), name);

    // refuse a stack declaring the main database every space has
    const document = StackDefinition.collections(declared);
    if (document.resources[MAIN_DATABASE] !== undefined) {
        throw new StackError(
            "INVALID_DEFINITION",
            `stack ${name} declares ${MAIN_DATABASE}, the database every space has`,
        );
    }

    // pin each installation to the manifest of the release its package resolved to
    const pinned = Object.fromEntries(
        Object.entries(document.installations).map(([key, installed]) => {
            const manifest = resolved.get(installed.package.id)?.manifest;
            if (manifest === undefined) {
                throw new StackError(
                    "INVALID_DEFINITION",
                    `installation ${key} of ${name} installs ${installed.package.id}, which the stack's build resolved to no release`,
                );
            }

            return [key, { ...installed, package: { ...installed.package, manifest } }];
        }),
    );

    // read the installable packages each declared installation's build depends on, but those its optional capabilities need
    const declaredPackages = new Set(
        Object.values(pinned).map((installed) => installed.package.id),
    );
    const closures = await Promise.all(
        Object.values(pinned).map(async (installed) => {
            // read the installation's package and the packages only its optional capabilities need
            const build = await read(installed.package.id, installed.package.manifest);
            const description = await PackageDescription.of(build);
            const optional = new Set(Installation.optionalPackages(description));

            // keep its installable dependencies the stack leaves out
            return (await installableReleases(build, read)).filter(
                (release) =>
                    !declaredPackages.has(release.package.id) && !optional.has(release.package.id),
            );
        }),
    );

    // install each once as any package, bound to nothing, at the release its dependents resolved
    const transitive: Record<string, InstallationDefinition> = {};
    for (const release of oneReleaseEach(closures.flat(), name).values()) {
        const installed = install({ ...release.package, resources: {}, secrets: {} }, {});
        if (Object.hasOwn(document.installations, installed.name)) {
            throw new StackError(
                "INVALID_DEFINITION",
                `stack ${name} declares installation ${installed.name}, the key of its dependency ${release.package.name}`,
            );
        }
        transitive[installed.name] = {
            ...installed,
            package: { ...installed.package, manifest: release.manifest },
        };
    }

    return StackDefinition.parse({
        ...declared,
        installations: { ...transitive, ...pinned },
        settings: { ...document.settings, ...StackDefinition.placed(settings) },
    });
}

/** Key releases by package, refusing two releases of one package in a stack. */
function oneReleaseEach(
    releases: readonly PackageRelease[],
    stack: string,
): Map<PackageId, PackageRelease> {
    const byPackage = new Map<PackageId, PackageRelease>();
    for (const release of releases) {
        // refuse a second release of a package
        const known = byPackage.get(release.package.id);
        if (known !== undefined && known.manifest !== release.manifest) {
            throw new StackError(
                "INVALID_DEFINITION",
                `stack ${stack} resolves two releases of ${release.package.name}`,
            );
        }
        byPackage.set(release.package.id, release);
    }

    return byPackage;
}

/** Where a stack revision applies. */
export interface StackApplicationOptions {
    /** The object store running the handlers' calls as its principal. */
    readonly store: Pick<
        ObjectStore,
        "principal" | "mutate" | "authorizeHere" | "clock" | "claims" | "isReplica"
    >;
    /** Open the build of a package's release in the stack's space, through a step's database. */
    readonly release: NonNullable<ApplyOptions["release"]>;
}

/** Describe how a stack revision's declarations apply to its space. */
function stackApplication(
    database: DatabaseConnection,
    stack: Installation,
    revision: InstallationRevision,
    objects: readonly ObjectType[],
    options: StackApplicationOptions,
): ApplyOptions {
    // require an evaluated revision of this stack
    if (stack.role !== "stack" || revision.installationId !== stack.id || !revision.definition) {
        throw new StackError("INVALID_DEFINITION", `not a revision of stack ${stack.id}`);
    }

    // apply every declaration the machine supports in dependency order, those of installed types once the store serves them
    return {
        installing: (snapshot) => installedTypes(snapshot, stack, options.release),
        database,
        objects,
        manager: { installationId: stack.id, packageId: stack.packageId },
        as: options.store.principal,
        scope: stack.scope,
        now: options.store.clock.now(),
        document: revision.definition,
        policies: [space.policy],
        ...(options.store.claims === undefined ? {} : { claims: options.store.claims }),
        ...options,
    };
}

/** Read the object types the builds of the installations a stack manages declare. */
async function installedTypes(
    snapshot: Snapshot,
    stack: Installation,
    release: StackApplicationOptions["release"],
): Promise<ObjectDescription[]> {
    // read the installations the stack manages that follow a revision
    const managed = await snapshot.read(installation.table).findMany({
        where: { scope: stack.scope, managerInstallationId: stack.id },
    });
    const following = managed.filter((entry) => entry.revisionId !== null);

    // read the object types each one's build declares
    const declared = await Promise.all(
        following.map(async (entry) => {
            const reader = await release(snapshot, entry.packageId, entry.id);

            return ObjectDescription.installed(await reader.declarations());
        }),
    );

    return declared.flat();
}

/** List the types stacks declare, once per table: the declarable ones, roles and relationships. */
function declaredTypes(types: readonly ObjectType[]): readonly ObjectType[] {
    // take the declarable types and the stack's roles and relationships, once per table
    const role = implementRole();
    const relationship = implementRelationship();
    const handled = [...types.filter((type) => type.declaration !== undefined), role, relationship];

    return [...new Map(handled.map((type) => [type.table, type])).values()];
}

/** Describe a stack's ready observation: its reason, message and observed fields. */
function ready(reason: StackReason, message: string, fields: Readonly<Record<string, unknown>>) {
    const status = reason === "Applied" ? "true" : "false";

    return { conditions: { ready: { status, reason, message } }, fields } as const;
}
