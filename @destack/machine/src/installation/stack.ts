import { Caller } from "@destack/access";
import { Account, space } from "@destack/account/object";
import { KNOWN_TYPES } from "@destack/stack/stack";
import { type JsonObject, present, schema } from "@destack/schema";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import {
    ObjectController,
    ObjectDescription,
    type ObservationReport,
    ObjectType,
    ObjectTypePool,
    SYNCED,
} from "@destack/object";
import { Failure } from "@destack/error";
import { Stack, StackPlan, ObjectStore } from "@destack/object/server";
import { StackError } from "@destack/stack/error";
import { installation, type MethodContextOf, Observation } from "@destack/object";
import { ApplicationInstallation, type ApplicationOptions } from "./application.ts";
import {
    installationRevision,
    Rollout,
    rollout,
    type StackReadiness,
    type InstallationRollout,
} from "@destack/stack/object";
import { Approval } from "./approval.ts";
import { implementRelationship, implementRole } from "../access/role.ts";
import { RolloutTarget, type RevisionOptions } from "./target.ts";

/** The space's stack installation: it applies its document and targets what it manages. */
export class StackInstallation {
    /** The stack's rollout, with its installation's fields. */
    readonly #target: InstallationRollout;
    /** What the machine applies stacks and describes revisions with. */
    readonly #options: ApplicationOptions;

    /** Reconcile a space's stack installation with what the machine applies it with. */
    constructor(target: InstallationRollout, options: ApplicationOptions) {
        this.#target = target;
        this.#options = options;
    }

    /** Target the stack's revision, apply its document, then deploy what it manages. */
    async deploy(call: MethodContextOf<typeof rollout, "deploy">): Promise<void> {
        // target the build the stack runs, then read the rollout as it stands
        await this.#revise(call);
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };
        const rows = await call
            .read(rollout.table)
            .findMany({ where: { installationId: this.#target.installationId } });
        const [stack] = await Rollout.read(reader, rows);
        if (stack === undefined) {
            return;
        }

        // apply its document, then deploy the installations its plan targets while it lives
        const written = await this.#apply(call, stack);
        if (stack.installation.deletedAt === null && written.length > 0) {
            await this.#deployManaged(call, stack, written);
        }
    }

    /** Target the build the stack runs, as a revision with its evaluated document. */
    async #revise(call: MethodContextOf<typeof rollout, "deploy">): Promise<void> {
        // choose the build the stack runs: the requested one, else what it follows
        const target = this.#target;
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };
        const chosen = await RolloutTarget.choose(
            reader,
            target,
            call.input.build,
            call.objects,
            this.#options,
        );
        if (chosen === undefined) {
            return;
        }

        // refuse a requested build the stack cannot run
        const application = new ApplicationInstallation(target, this.#options);
        if (call.input.build !== undefined) {
            await application.requireDeployable(call, chosen.image);
        }

        // record a chosen release as the space's build, and evaluate its export at its inputs
        const built = chosen.build ?? (await application.record(call, chosen.source)).id;
        const image = await this.#options.space.builds.image(chosen.image);
        const inputs = await RolloutTarget.inputs(reader, target.installation);
        const evaluation = await RolloutTarget.evaluate(inputs, image, this.#options);

        // target the revision of the evaluation
        const revision = await application.revise(call, {
            build: built,
            dependencies: chosen.dependencies,
            states: chosen.states,
            evaluation,
        });
        if (revision.id !== target.targetRevisionId) {
            await call.mutate(call.self, [
                rollout.call("update", {
                    ...rollout.target({ scope: target.scope, id: target.installationId }),
                    targetRevisionId: revision.id,
                }),
            ]);

            // report it applying until its document is applied
            await call.observe(
                installation,
                target.installation,
                ready("Applying", "applying the targeted revision", {}),
            );
        }
    }

    /** Apply the stack's document in the call, answering the installations its plan targets. */
    async #apply(
        call: MethodContextOf<typeof rollout, "deploy">,
        stack: InstallationRollout,
    ): Promise<readonly string[]> {
        // read the revision it targets, the document it applied last, and whether its space is its account's home
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };
        const { store, transaction } = call.execution;
        const revision =
            stack.targetRevisionId === null
                ? undefined
                : await reader
                      .read(installationRevision.table)
                      .findFirst({ where: { id: stack.targetRevisionId } });
        const objects = declaredTypes(store.objects);
        const applied = await appliedDefinition(reader, stack);
        const placed = await reader
            .read(space.table)
            .findFirst({ where: { id: space.identifier(stack.scope) } });
        const isHome = placed !== undefined && Account.home(placed.scope) === stack.scope;
        const options = { ...this.#options, store, isHome };

        // tear down a deleted stack: retire every declaration, then finalize it once nothing it manages remains
        if (stack.installation.deletedAt !== null) {
            const retiring = stackApplication(store.database, stack, {}, applied, objects, options);
            await StackPlan.open(retiring, call.now, transaction);
            const managed = await reader
                .read(installation.table)
                .findMany({ where: { managerInstallationId: stack.installationId } });
            if (managed.length === 0) {
                await call.mutate(call.self, [
                    installation.call("finalize", installation.target(stack.installation)),
                ]);
            }

            return [];
        }
        // leave a stack without a revision
        else if (revision === undefined) {
            return [];
        }
        // observe an applied revision at the installation's latest generation
        else if (stack.appliedRevisionId === revision.id) {
            if (!Observation.isCurrent(stack.installation, "Ready")) {
                await call.observe(
                    installation,
                    stack.installation,
                    ready("Applied", "every declaration applied", {}),
                );
            }

            return [];
        }

        // require an evaluated revision of this stack
        const { definition } = revision;
        if (revision.installationId !== stack.installationId || !definition) {
            throw StackError.of(
                "DEFINITION_INVALID",
                "revision is invalid: it is not an evaluated revision of this stack",
            );
        }

        // plan the declarations into the call, dropping them while the plan waits for its approval
        const applying = stackApplication(
            store.database,
            stack,
            definition,
            applied,
            objects,
            options,
        );
        const outcome = await transaction
            .savepoint(async () => {
                // plan and apply, stopping at a plan the space must approve
                const planned = await StackPlan.open(applying, call.now, transaction);
                const waiting = await Approval.waiting(
                    reader,
                    stack,
                    planned.plan,
                    call.environment.crypto,
                );
                if (waiting !== undefined) {
                    throw new ApprovalWait(waiting);
                }

                return {
                    waiting: undefined,
                    deferred: planned.plan.deferred,
                    steps: planned.plan.steps,
                };
            })
            .catch((error: unknown) => {
                if (error instanceof ApprovalWait) {
                    return { waiting: error.report, deferred: undefined, steps: [] };
                }
                throw error;
            });

        // observe a plan waiting for its approval
        if (outcome.waiting !== undefined) {
            await call.observe(installation, stack.installation, outcome.waiting);
        }
        // observe it applied and forget its plan
        else if (outcome.deferred === undefined) {
            await call.observe(
                installation,
                stack.installation,
                ready("Applied", "every declaration applied", { plan: null, approvedPlan: null }),
            );
            await call.mutate(call.self, [
                rollout.call("update", {
                    ...rollout.target({ scope: stack.scope, id: stack.installationId }),
                    appliedRevisionId: revision.id,
                }),
            ]);
        }
        // observe it waiting for what the rest of its document needs
        else {
            await call.observe(
                installation,
                stack.installation,
                ready("WaitingForResources", outcome.deferred, {}),
            );
        }

        // answer the installations the plan's steps target
        return outcome.steps
            .map((step) => step.target.split("/"))
            .filter(([collection, name]) => collection === installation.name && name !== undefined)
            .map(([, name]) => String(name));
    }

    /** Deploy the managed installations of these names, each in its own savepoint. */
    async #deployManaged(
        call: MethodContextOf<typeof rollout, "deploy">,
        stack: InstallationRollout,
        names: readonly string[],
    ): Promise<void> {
        // read the live installations the stack manages, past detached ones
        const { transaction } = call.execution;
        const managed = await call.read(installation.table).findMany({
            where: {
                scope: stack.scope,
                managerInstallationId: stack.installationId,
                name: { in: [...names] },
                detachedAt: { isNull: true },
                deletedAt: { isNull: true },
            },
        });

        // deploy each one, recording a declared failure on it and leaving the others
        for (const row of managed) {
            const deploying = rollout.call(
                "deploy",
                rollout.target({ scope: row.scope, id: row.id }),
            );
            await transaction
                .savepoint(() => call.mutate(call.self, [deploying]))
                .catch(async (error: unknown) => {
                    // pass on an undeclared failure and a lost race, which the call runs again
                    if (!Failure.isDeclared(error) || Failure.isTransient(Failure.of(error))) {
                        throw error;
                    }
                    await call.observe(installation, row, {
                        conditions: {
                            [SYNCED]: ObjectController.failedCondition(Failure.of(error)),
                        },
                    });
                });
        }
    }
}

/** A plan waiting for its approval, with what its installation reports. */
class ApprovalWait extends Error {
    /** What the installation reports while it waits. */
    readonly report: ObservationReport;

    /** Wait with a report. */
    constructor(report: ObservationReport) {
        super("the plan waits for its approval");
        this.report = report;
    }
}

/** Read the definition of the revision a stack applied last, empty before its first apply. */
async function appliedDefinition(
    reader: Pick<Snapshot, "read">,
    stack: InstallationRollout,
): Promise<JsonObject> {
    // read the applied revision, which a stack's evaluation always defines
    if (stack.appliedRevisionId === null) {
        return {};
    }
    const revision = await reader
        .read(installationRevision.table)
        .findFirst({ where: { id: stack.appliedRevisionId } });
    const definition = revision?.definition;
    if (definition === undefined || definition === null) {
        throw StackError.of(
            "DEFINITION_INVALID",
            "stack is invalid: it has no applied evaluated revision",
        );
    }

    return definition;
}

/** Where a stack revision applies. */
export interface StackApplicationOptions extends Pick<
    RevisionOptions,
    "space" | "linked" | "shipped"
> {
    /** The object store running the handlers' calls as its principal. */
    readonly store: Pick<
        ObjectStore,
        | "principal"
        | "mutate"
        | "query"
        | "authorizeHere"
        | "clock"
        | "claims"
        | "isReplica"
        | "environment"
        | "release"
    >;
    /** Whether the stack's space is its account's home, which may declare into the account. */
    readonly isHome: boolean;
}

/** Create the stack of a document's declarations in its space. */
function stackApplication(
    database: DatabaseConnection,
    stack: InstallationRollout,
    document: JsonObject,
    applied: JsonObject,
    objects: readonly ObjectType[],
    options: StackApplicationOptions,
): Stack {
    // apply every declaration the machine supports in dependency order, those of installed types once the store serves them
    return new Stack({
        installing: (snapshot) => installedTypes(snapshot, stack, document, objects, options),
        database,
        objects,
        manager: { installationId: stack.installationId, packageId: stack.installation.packageId },
        as: options.store.principal,
        declarer: () => Caller.of(options.store.principal),
        scope: stack.scope,
        document,
        applied,
        policies: [space.policy],
        ...(options.store.claims === undefined ? {} : { claims: options.store.claims }),
        ...options,
    });
}

/** Read the types the builds of a stack's installations declare, and which follow a build. */
async function installedTypes(
    snapshot: Snapshot,
    stack: InstallationRollout,
    document: JsonObject,
    objects: readonly ObjectType[],
    options: StackApplicationOptions,
): Promise<{ readonly types: readonly ObjectType[]; readonly isPending: boolean }> {
    // read the installations the stack manages that follow a revision or select a release
    const installed = await snapshot.read(installation.table).findMany({
        where: { scope: stack.scope, managerInstallationId: stack.installationId },
    });
    const managed = await Rollout.read(
        snapshot,
        await snapshot
            .read(rollout.table)
            .findMany({ where: { installationId: { in: installed.map((entry) => entry.id) } } }),
    );
    const following = managed.filter(
        (entry) => entry.targetRevisionId !== null || RolloutTarget.isPinned(entry.installation),
    );

    // read the object types each one's build declares
    const declared = await Promise.all(
        following.map(async (entry) => {
            const { packageId } = entry.installation;
            const reader = present(
                await RolloutTarget.reader(
                    snapshot,
                    stack.scope,
                    packageId,
                    entry.installationId,
                    options,
                ),
                `the release ${entry.installationId} follows`,
            );

            return await ObjectDescription.read(reader);
        }),
    );

    // declare them against the space's types, the account's and the kernel's, which they refer to
    const pool = ObjectTypePool.of(ObjectType.known(objects, KNOWN_TYPES));

    // report an installation the document declares that follows no build yet
    const followed = new Set(following.map((entry) => entry.installation.name));
    const names = Object.keys(
        schema.record(schema.string(), schema.unknown()).parse(document["installations"] ?? {}),
    );

    return {
        types: pool.declare(declared.flat()),
        isPending: names.some((name) => !followed.has(name)),
    };
}

/** List the types stacks declare, once per table: the managed ones, roles and relationships. */
function declaredTypes(types: readonly ObjectType[]): readonly ObjectType[] {
    // take the managed types and the stack's roles and relationships, once per table
    const role = implementRole();
    const relationship = implementRelationship();
    const handled = [...types.filter((type) => type.isDeclarable()), role, relationship];

    return [...new Map(handled.map((type) => [type.table, type])).values()];
}

/** Describe a stack's ready observation: its reason, message and observed fields. */
function ready(reason: StackReadiness, message: string, fields: Readonly<Record<string, unknown>>) {
    const status = reason === "Applied" ? "true" : "false";

    return { conditions: { Ready: { status, reason, message } }, fields } as const;
}
