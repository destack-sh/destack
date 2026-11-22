import {
    installation,
    installationRevision,
    Installation,
    InstallationWait,
    type StackReadiness,
} from "@destack/stack/object";
import { Caller } from "@destack/access";
import { Account, space } from "@destack/account/object";
import { KNOWN_TYPES } from "@destack/stack/stack";
import { type Identifier, type JsonObject, present, schema } from "@destack/schema";
import { type DatabaseConnection, Snapshot } from "@destack/db/database";
import {
    ObjectController,
    ObjectDescription,
    type ObservationReport,
    ObjectType,
    ObjectTypePool,
    SYNCED,
    SYNCED_CONDITION,
    type MethodContextOf,
    Observation,
} from "@destack/object";
import { Failure } from "@destack/error";
import { Stack, StackPlan, ObjectStore } from "@destack/object/server";
import { StackError } from "@destack/stack/error";
import { ApplicationInstallation, type ApplicationOptions } from "./application.ts";
import { Approval } from "./approval.ts";
import { planWait } from "./wait.ts";
import { implementRelationship, implementRole } from "../access/role.ts";
import { InstallationTarget, type RevisionOptions } from "./target.ts";

/** The space's stack installation: it applies its document and targets what it manages. */
export class StackInstallation {
    /** The stack's installation. */
    readonly #target: Installation;
    /** What the machine applies stacks and describes revisions with. */
    readonly #options: ApplicationOptions;

    /** Reconcile a space's stack installation with what the machine applies it with. */
    constructor(target: Installation, options: ApplicationOptions) {
        this.#target = target;
        this.#options = options;
    }

    /** Run a step of a reconcile in a span of the space and the installation it works on. */
    #span<Result>(step: string, row: Installation, work: () => Promise<Result>): Promise<Result> {
        return this.#options.space.machine.scope.span(
            `destack.installation.${step}`,
            InstallationTarget.attributes(row),
            work,
        );
    }

    /** Target the stack's revision, apply its document, then reconcile what the plan wrote. */
    async reconcile(
        call: MethodContextOf<typeof installation, "reconcile">,
        requested: Identifier<"build"> | undefined,
    ): Promise<void> {
        // target the build the stack runs, then read it as it stands
        await this.#span("revise", this.#target, () => this.#revise(call, requested));
        const stack = await call
            .read(installation.table)
            .findFirst({ where: { id: this.#target.id } });
        if (stack === undefined) {
            return;
        }

        // apply its document, answering the installations its plan wrote
        const written = await this.#span("plan", stack, () => this.#apply(call, stack));
        if (stack.deletedAt !== null || written.length === 0) {
            return;
        }
        const managed = await call.read(installation.table).findMany({
            where: {
                scope: stack.scope,
                managerInstallationId: stack.id,
                name: { in: [...written] },
                detachedAt: { isNull: true },
                deletedAt: { isNull: true },
            },
        });

        // target each one's revision, a stack it declares whole
        const targeted: Installation[] = [];
        let isMoved = false;
        for (const row of managed) {
            const moved = await this.#step(call, row, async () => {
                if (row.export !== null) {
                    await new StackInstallation(row, this.#options).reconcile(call, undefined);

                    return "reconciled";
                }

                return await this.#span("target", row, () =>
                    new ApplicationInstallation(row, this.#options).target(call, undefined),
                );
            });
            if (typeof moved === "boolean") {
                targeted.push(row);
                isMoved ||= moved;
            }
        }

        // establish the types their revisions declare, before a server of the space serves them
        const ids = targeted.map((row) => row.id);
        await this.#span("establish", stack, () =>
            ApplicationInstallation.establish(call, ids, this.#options),
        );

        // write the states the revisions require once they moved, and reconcile the space's own storage
        const scope = space.identifier(stack.scope);
        if (isMoved) {
            await this.#span("specify", stack, () => this.#options.binder.specify(call, scope));
        }
        await this.#span("storage", stack, () => this.#options.binder.reconcilePure(call, scope));

        // deploy each one at its revision
        for (const row of targeted) {
            await this.#step(call, row, () =>
                this.#span("deploy", row, () =>
                    new ApplicationInstallation(row, this.#options).deployTargeted(call),
                ),
            );
        }
    }

    /** Run one managed installation's step in a savepoint, recording its declared failure on it. */
    async #step<Result>(
        call: MethodContextOf<typeof installation, "reconcile">,
        row: Installation,
        work: () => Promise<Result>,
    ): Promise<Result | undefined> {
        try {
            return await call.execution.transaction.savepoint(work);
        } catch (error) {
            // pass on an undeclared failure and a lost race, which the call runs again
            if (!Failure.isDeclared(error) || Failure.isTransient(Failure.of(error))) {
                throw error;
            }
            await call.observe(installation, row, {
                conditions: { [SYNCED]: ObjectController.failedCondition(Failure.of(error)) },
            });

            return undefined;
        }
    }

    /** Target the build the stack runs, as a revision with its evaluated document. */
    async #revise(
        call: MethodContextOf<typeof installation, "reconcile">,
        requested: Identifier<"build"> | undefined,
    ): Promise<void> {
        // choose the build the stack runs: the requested one, else what it follows
        const target = this.#target;
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };
        const chosen = await InstallationTarget.choose(reader, target, requested, this.#options);
        if (chosen === undefined) {
            return;
        }

        // refuse a requested build the stack cannot run
        const application = new ApplicationInstallation(target, this.#options);
        if (requested !== undefined) {
            await application.requireDeployable(call, chosen.image);
        }

        // record a chosen release as the space's build, and evaluate its export at its inputs
        const built = chosen.build ?? (await application.record(call, chosen.source)).id;
        const image = await this.#options.space.builds.image(chosen.image);
        const inputs = await InstallationTarget.inputs(reader, target);
        const evaluation = await InstallationTarget.evaluate(inputs, image, this.#options);

        // target the revision of the evaluation
        const revision = await application.revise(call, {
            build: built,
            dependencies: chosen.dependencies,
            states: chosen.states,
            release: chosen.release,
            declarations: chosen.declarations,
            evaluation,
        });
        if (revision.id !== target.targetRevisionId) {
            await call.mutate(call.self, [
                Observation.call(installation, target, {
                    conditions: {},
                    fields: { targetRevisionId: revision.id },
                }),
            ]);

            // report it applying until its document is applied
            await call.observe(
                installation,
                target,
                ready("Applying", "applying the targeted revision", {}),
            );
        }
    }

    /** Apply the stack's document in the call, answering the installations its plan targets. */
    async #apply(
        call: MethodContextOf<typeof installation, "reconcile">,
        stack: Installation,
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
        if (stack.deletedAt !== null) {
            const retiring = stackApplication(store.database, stack, {}, applied, objects, options);
            await StackPlan.open(retiring, call.now, transaction);
            const managed = await reader
                .read(installation.table)
                .findMany({ where: { managerInstallationId: stack.id } });
            if (managed.length === 0) {
                await call.mutate(call.self, [
                    installation.call("finalize", installation.target(stack)),
                ]);
            }
            // wait for the installations it manages to go
            else {
                await call.observe(installation, stack, {
                    conditions: {},
                    fields: InstallationWait.of(installation, { managerInstallationId: stack.id }),
                });
            }

            return [];
        }
        // leave a stack without a revision
        else if (revision === undefined) {
            return [];
        }
        // observe an applied revision at the installation's latest generation
        else if (stack.appliedRevisionId === revision.id) {
            if (!Observation.isCurrent(stack, "Ready")) {
                const isFollowed = await InstallationTarget.isFollowed(
                    reader,
                    stack,
                    this.#options,
                );
                await call.observe(installation, stack, readyApplied(isFollowed, {}));
            }

            return [];
        }

        // require an evaluated revision of this stack
        const { definition } = revision;
        if (revision.installationId !== stack.id || !definition) {
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
                    wait: planned.waiting,
                    steps: planned.plan.steps,
                };
            })
            .catch((error: unknown) => {
                if (error instanceof ApprovalWait) {
                    return {
                        waiting: error.report,
                        deferred: undefined,
                        wait: undefined,
                        steps: [],
                    };
                }
                throw error;
            });

        // observe a plan waiting for its approval
        if (outcome.waiting !== undefined) {
            await call.observe(installation, stack, outcome.waiting);
        }
        // observe it applied and forget its plan
        else if (outcome.deferred === undefined) {
            await call.observe(
                installation,
                stack,
                readyApplied(await InstallationTarget.isFollowed(reader, stack, this.#options), {
                    plan: null,
                    approvedPlan: null,
                    ...InstallationWait.none,
                }),
            );
            await call.mutate(call.self, [
                Observation.call(installation, stack, {
                    conditions: {},
                    fields: { appliedRevisionId: revision.id },
                }),
            ]);
        }
        // observe it waiting for what the rest of its document needs
        else {
            await call.observe(
                installation,
                stack,
                ready(
                    "WaitingForResources",
                    outcome.deferred,
                    planWait(present(outcome.wait, "the row the deferred plan waits for"), options),
                ),
            );
        }

        // answer the installations the plan's steps target
        return outcome.steps
            .map((step) => step.target.split("/"))
            .filter(([collection, name]) => collection === installation.name && name !== undefined)
            .map(([, name]) => String(name));
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
    stack: Installation,
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
    stack: Installation,
    document: JsonObject,
    applied: JsonObject,
    objects: readonly ObjectType[],
    options: StackApplicationOptions,
): Stack {
    // apply every declaration the machine supports in dependency order, those of installed types once the store serves them
    return new Stack({
        installing: (snapshot) => installedTypes(snapshot, stack, document, objects, options),
        // wait for a package's installation in the space, or for the ones the stack declares
        installed: (packageId) => ({
            object: installation,
            condition:
                packageId === undefined
                    ? { managerInstallationId: stack.id }
                    : { scope: stack.scope, packageId },
        }),
        database,
        objects,
        manager: { installationId: stack.id, packageId: stack.packageId },
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

/** List the installations a stack manages that follow a revision or select a release. */
async function followingInstallations(
    snapshot: Pick<Snapshot, "read">,
    stack: Installation,
): Promise<Installation[]> {
    const managed = await snapshot.read(installation.table).findMany({
        where: { scope: stack.scope, managerInstallationId: stack.id },
    });

    return managed.filter(
        (entry) => entry.targetRevisionId !== null || InstallationTarget.isPinned(entry),
    );
}

/** Read the object descriptions the build of each of some of a stack's installations declares. */
async function readDescriptions(
    snapshot: Pick<Snapshot, "read">,
    stack: Installation,
    following: readonly Installation[],
    options: Pick<StackApplicationOptions, "space" | "linked" | "shipped">,
): Promise<ObjectDescription[][]> {
    return await Promise.all(
        following.map(async (entry) => {
            const { packageId } = entry;
            const reader = present(
                await InstallationTarget.reader(
                    snapshot,
                    Installation.space(stack),
                    packageId,
                    entry.id,
                    options,
                ),
                `the release ${entry.id} follows`,
            );

            return await ObjectDescription.read(reader);
        }),
    );
}

/** Read the types the builds of a stack's installations declare, and which follow a build. */
async function installedTypes(
    snapshot: Snapshot,
    stack: Installation,
    document: JsonObject,
    objects: readonly ObjectType[],
    options: StackApplicationOptions,
): Promise<{ readonly types: readonly ObjectType[]; readonly isPending: boolean }> {
    // declare the types of the installations following a build against the types they refer to
    const following = await followingInstallations(snapshot, stack);
    const described = await readDescriptions(snapshot, stack, following, options);
    const pool = ObjectTypePool.of(ObjectType.known(objects, KNOWN_TYPES));
    const types = pool.declare(described.flat());

    // report an installation the document declares that follows no build yet
    const followed = new Set(following.map((entry) => entry.name));
    const names = Object.keys(
        schema.record(schema.string(), schema.unknown()).parse(document["installations"] ?? {}),
    );

    return { types, isPending: names.some((name) => !followed.has(name)) };
}

/** List the types stacks declare, once per table: the managed ones, roles and relationships. */
function declaredTypes(types: readonly ObjectType[]): readonly ObjectType[] {
    // take the managed types and the stack's roles and relationships, once per table
    const role = implementRole();
    const relationship = implementRelationship();
    const handled = [...types.filter((type) => type.isDeclarable()), role, relationship];

    return [...new Map(handled.map((type) => [type.table, type])).values()];
}

/** Describe an applied stack, synced once it runs what it follows. */
function readyApplied(isFollowed: boolean, fields: Readonly<Record<string, unknown>>) {
    const described = ready("Applied", "every declaration applied", fields);
    const synced = isFollowed ? { [SYNCED]: SYNCED_CONDITION } : {};

    return { ...described, conditions: { ...described.conditions, ...synced } };
}

/** Describe a stack's ready observation: its reason, message and observed fields. */
function ready(reason: StackReadiness, message: string, fields: Readonly<Record<string, unknown>>) {
    const status = reason === "Applied" ? "true" : "false";

    return {
        conditions: { Ready: { status, reason, message } },
        fields,
    } as const;
}
