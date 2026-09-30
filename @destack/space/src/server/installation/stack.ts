import { and, type DatabaseConnection, eq } from "@destack/db";
import { ObjectError, type ObjectReconciliation, type ObjectType } from "@destack/object";
import type { Directory } from "@destack/directory";
import { Stack, type ObjectServer, SystemCall } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import type { Plan } from "@destack/resource";
import { canonicalize, digest } from "@destack/schema/json";
import { ServiceError } from "@destack/service/error";
import { SpaceDefinition, type SpaceDeclaration } from "../../declare/space.ts";
import { SpaceError } from "../../error/index.ts";
import {
    type Installation,
    installation,
    type InstallationRevision,
    installationRevision,
    space,
    SpaceCell,
    type StackReason,
    type Submission,
} from "../../object/index.ts";
import { Approval } from "./approval.ts";
import { type OpenBuild, openRelease } from "./release.ts";

/** What a cell applies stacks with: the object types they declare, the directory and its builds. */
export interface StackOptions {
    /** List the object types stacks declare on this cell, each handling its declarations. */
    declared(): readonly ObjectType[];
    /** The directory keeping the claims of the declared objects' unique indexes. */
    readonly directory: Directory;
    /** Open a package's build through the cell's store of builds. */
    readonly openBuild: OpenBuild;
}

/** Apply the revision a served space's stack follows, then observe it applied or waiting. */
export async function reconcileStack(
    spaceId: Identifier<"space">,
    reconciliation: ObjectReconciliation,
    options: StackOptions,
): Promise<undefined> {
    // read the stack of a served space and the revision it follows
    const database = reconciliation.database;
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
                SpaceCell.served(),
            ),
        );
    if (served === undefined || served.stack.appliedRevisionId === served.revision.id) {
        return undefined;
    }
    const { stack, revision } = served;
    const apply = (transaction: DatabaseConnection, isDry: boolean) =>
        applyStack(transaction, stack, revision, options.declared(), {
            server: reconciliation.server,
            directory: options.directory,
            release: (packageId, installationId) =>
                openRelease(transaction, options.openBuild, spaceId, packageId, installationId),
            isDry,
        });

    // plan the declarations in a transaction rolled back, and wait for an approval the plan needs
    let deferred: string | undefined;
    try {
        let planned: Plan = { steps: [] };
        await database.transaction(async (transaction) => {
            planned = await apply(transaction, true);
            transaction.rollback();
        });
        if (await Approval.awaits(stack, planned, reconciliation)) {
            return undefined;
        }

        // apply it, including declarations waiting for resources
        deferred = (await apply(database, false)).deferred;
    } catch (error) {
        // block on a definition the next revision must fix, and retry another failure
        const isBlocked =
            (error instanceof SpaceError && error.code === "INVALID_DEFINITION") ||
            (error instanceof ObjectError && error.code === "INVALID_DECLARATION");
        const message = error instanceof Error ? error.message : String(error);
        await observe(stack, isBlocked ? "Blocked" : "ApplyFailed", message, {}, reconciliation);
        if (!isBlocked) {
            throw error;
        }

        return undefined;
    }

    // observe it applied or waiting for resources, forgetting the plan it applied
    const fields = { plan: null, approvedPlan: null };
    if (deferred === undefined) {
        const applied = { ...fields, appliedRevisionId: revision.id };
        await observe(stack, "Applied", "every declaration applied", applied, reconciliation);
    } else {
        await observe(stack, "WaitingForResources", deferred, fields, reconciliation);
    }

    return undefined;
}

/** Observe a stack's ready condition and fields unless it has them already. */
async function observe(
    stack: Installation,
    reason: StackReason,
    message: string,
    fields: Readonly<Record<string, unknown>>,
    reconciliation: ObjectReconciliation,
): Promise<void> {
    // skip an observation the stack has
    const status = reason === "Applied" ? "true" : "false";
    const ready = stack.conditions.ready;
    const isObserved =
        stack.observedGeneration === stack.generation &&
        ready?.status === status &&
        ready.reason === reason &&
        ready.message === message &&
        Object.entries(fields).every(
            ([name, value]) => stack[name as keyof Installation] === value,
        );
    if (!isObserved) {
        await reconciliation.server.executeAsSystem(
            installation,
            "observe",
            [
                SystemCall.of(stack, {
                    observedGeneration: stack.generation,
                    conditions: { ready: { status, reason, message } },
                    fields,
                }),
            ],
            reconciliation.now,
        );
    }
}

/** Run a system method in a call's or reconciliation's transaction and scope. */
export type Invoke = (
    object: ObjectType,
    name: string,
    input: Readonly<Record<string, unknown>>,
) => Promise<unknown>;

/** Retain a submitted build as an installation's revision and point the installation at it: a stack's to apply, an application's to deploy. */
export async function recordSubmission(
    database: DatabaseConnection,
    invoke: Invoke,
    target: Installation,
    submission: Submission,
): Promise<InstallationRevision> {
    // require a stack's evaluation for a stack and none for an application
    const { selection, build, evaluation } = submission;
    const isStack = target.role === "stack";
    if (isStack !== (evaluation !== undefined)) {
        throw new ServiceError("BAD_REQUEST", {
            message: isStack
                ? `${target.alias} is a stack, submitted with its evaluation`
                : `${target.alias} is an application, submitted without a stack's evaluation`,
        });
    }

    // retain the evaluation once
    const evaluated = await digest(
        evaluation === undefined
            ? { build }
            : { build, parameters: evaluation.parameters, definition: evaluation.definition },
    );
    const [existing] = await database
        .select()
        .from(installationRevision.table)
        .where(
            and(
                eq(installationRevision.table.installationId, target.id),
                eq(installationRevision.table.digest, evaluated),
            ),
        );
    const revision =
        existing ??
        ((await invoke(installationRevision, "create", {
            installationId: target.id,
            packageId: target.packageId,
            build,
            ...(evaluation === undefined ? {} : { definition: evaluation.definition }),
            digest: evaluated,
        })) as InstallationRevision);

    // point the installation at the selection, arguments and revision submitted
    const followed = {
        selection,
        ...(evaluation === undefined
            ? {}
            : { export: evaluation.export, parameters: evaluation.parameters }),
        revisionId: revision.id,
    };
    const current = {
        selection: target.selection,
        ...(evaluation === undefined
            ? {}
            : { export: target.export, parameters: target.parameters }),
        revisionId: target.revisionId,
    };
    if (canonicalize(followed) !== canonicalize(current)) {
        await invoke(installation, "follow", { id: target.id, ...followed });
    }

    return revision;
}

/** Evaluate a stack module's export into its space definition. */
export function evaluateStack(
    module: Readonly<Record<string, unknown>>,
    name: string,
    parameters: Readonly<Record<string, unknown>>,
): SpaceDefinition {
    // require the named export
    const exported = module[name];
    if (exported === undefined) {
        throw new SpaceError("INVALID_DEFINITION", `stack does not export ${name}`);
    }

    // call parameterised definitions with the installation's arguments
    const declared = typeof exported === "function" ? exported(parameters) : exported;

    return SpaceDefinition.parse((declared as SpaceDeclaration).definition);
}

/** Where a stack revision applies: its object server and directory, and whether only to report changes applied elsewhere. */
export interface StackApplicationOptions {
    /** The object server running the handlers' system calls. */
    readonly server: Pick<ObjectServer, "invoke">;
    /** Open the build of a package's release in the stack's space. */
    readonly release: Stack["release"];
    /** The directory keeping the claims of unique indexes. */
    readonly directory?: Directory;
    /** Report the changes to records kept in other databases without writing them. */
    readonly isDry?: boolean;
}

/** Apply a stack revision's declarations to its space, deferring those that wait for resources. */
export async function applyStack(
    database: DatabaseConnection,
    stack: Installation,
    revision: InstallationRevision,
    objects: readonly ObjectType[],
    options: StackApplicationOptions,
): Promise<Plan> {
    // require an evaluated revision of this stack
    if (stack.role !== "stack" || revision.installationId !== stack.id || !revision.definition) {
        throw new SpaceError("INVALID_DEFINITION", `not a revision of stack ${stack.id}`);
    }

    // apply every declaration the host supports in dependency order
    return Stack.apply({
        database,
        objects,
        manager: { installationId: stack.id, packageId: stack.packageId },
        scope: stack.scope,
        document: revision.definition,
        policies: [space.policy],
        ...options,
    });
}
