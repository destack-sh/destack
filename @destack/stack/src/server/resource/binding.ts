import { placement, Space, space } from "@destack/account/object";
import { MAIN_DATABASE } from "../../declare/index.ts";
import {
    type Bindable,
    type BindingSelection,
    type BindingTarget,
    type InstallationBindings,
    installation,
    type MethodContext,
    type ObjectType,
    Provisioned,
    type Resource,
    type ResourceType,
} from "@destack/object";
import { AccessContext, Caller, principal } from "@destack/access";
import {
    and,
    Change,
    Condition,
    type DatabaseConnection,
    eq,
    inArray,
    Projection,
    type Row,
    type Snapshot,
    type Table,
    type TableState,
} from "@destack/db";
import { PackageId, type PackageRelease } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import { RequestId } from "@destack/sync";
import type { SpaceOptions } from "../server.ts";
import {
    DRAINING,
    type ObjectStore,
    type PackageImplementation,
    Stack,
    StackPlan,
} from "@destack/object/server";
import { RESOURCE_KIND, type ResourceKind } from "@destack/resource";
import { Digest, type Identifier, type JsonObject, present, schema } from "@destack/schema";
import { StackError } from "../../error/index.ts";
import {
    Binding,
    database,
    Deployment,
    deployment,
    Rollout,
    rollout,
    type InstallationRollout,
    installationRevision,
    type InstallationRevision,
    type ResourceNeed,
} from "../../object/index.ts";

/** A resource kind bindings target, whose providers provision its objects. */
export interface ResourceBindable extends Bindable<ResourceType<typeof space>> {
    /** The resource kind the objects are. */
    readonly kind: ResourceKind;
    /** Read the record an installation's own need declares, as its type's stack declaration. */
    readonly declare: (need: ResourceNeed, installation: string) => JsonObject;
}

/** A kind of object a package serves to workloads, such as secrets. */
export interface ObjectBindable<
    Object extends ObjectType = ObjectType,
    Client = unknown,
> extends Bindable<Object> {
    /** The provider workloads read the bound objects through, such as vault. */
    readonly provider: string;
    /** The address workloads call the package serving the bound objects at. */
    readonly reference: string;
    /** Read the record an installation's own need declares, none while a binding it needs lacks. */
    readonly declare: (need: ResourceNeed, bindings: readonly Binding[]) => JsonObject | undefined;
    /** Report whether a bound target is ready to run with, reading it alone. */
    readonly ready: (
        context: MethodContext,
        bound: Binding,
        need: ResourceNeed,
    ) => Promise<boolean>;
    /** Start making a bound target ready to capture once its parent is provisioned. */
    readonly prepare: (
        context: MethodContext,
        declarer: AccessContext,
        bound: Binding,
        need: ResourceNeed,
    ) => Promise<void>;
    /** Connect an in-process installation to a binding's target at its bound version. */
    readonly connect: (
        store: Promise<ObjectStore>,
        row: InstallationRollout,
        bound: Binding,
    ) => Promise<Client>;
}

/** The kinds of objects the servers a machine runs serve to workloads. */
export const ObjectBindable = {
    /** Read the object kinds some servers declare, refusing one without a connection. */
    of(servers: readonly PackageImplementation[]): ObjectBindable[] {
        return servers
            .flatMap((server) => server.bindables)
            .map((bindable) => {
                // refuse a kind without a connection
                if (!ObjectBindable.is(bindable)) {
                    throw StackError.of(
                        "BINDABLE_UNSUPPORTED",
                        `bindable ${bindable.object.name} without a connection is not supported`,
                    );
                }

                return bindable;
            });
    },

    /** Report whether a bindable kind connects and provisions through the store. */
    is(bindable: Bindable): bindable is ObjectBindable {
        return "connect" in bindable && "declare" in bindable;
    },
};

/** Resource kinds as bindings target them. */
export const ResourceBindable = {
    /** Bind resources of one kind, at their generation or a binding's pinned version. */
    of(kind: ResourceKind, object: ResourceType<typeof space>): ResourceBindable {
        return {
            object,
            kind,
            declaration: RESOURCE_KIND,
            used: [object],
            version: (target, pin) => pin ?? target.generation,
            uses: async (bound) => [object.reference(bound.scope, bound.target)],
            declare: (need, installationName) => ({
                name: `${installationName}.${need.name}`,
                definitionPackageId: need.package.id,
                definitionVersion: need.package.version,
                definitionName: need.name,
                spec: need.spec,
            }),
        };
    },
};

/** Bind declarations to objects of each bindable kind, and capture what deployments run with. */
export class Binder {
    /** The bindable kinds. */
    readonly #kinds: readonly Bindable[];
    /** The bindable resource kinds, by their kind's name. */
    readonly #resources: ReadonlyMap<string, ResourceBindable>;
    /** The kinds packages serve, by their declaration kind. */
    readonly #objects: ReadonlyMap<string, ObjectBindable>;
    /** The state of the platform's tables by its digest, per set of replicated tables left out. */
    readonly #platform = new Map<string, Promise<ReadonlyMap<Digest, ResourceState>>>();

    /** Index the resource kinds and the other bindable kinds a machine serves. */
    constructor(resources: readonly ResourceBindable[], objects: readonly ObjectBindable[]) {
        // index the kinds by resource kind and by declaration kind
        this.#kinds = [...resources, ...objects];
        this.#resources = new Map(resources.map((kind) => [kind.kind.name, kind]));
        this.#objects = new Map(objects.map((kind) => [kind.declaration.name, kind]));
    }

    /** List the kinds packages serve to workloads, such as secrets. */
    objects(): readonly ObjectBindable[] {
        return [...this.#objects.values()];
    }

    /** List the object types bindings target. */
    targets(): readonly ObjectType[] {
        return this.#kinds.map((kind) => kind.object);
    }

    /** Report whether a target is an object of a kind the binder binds on this machine. */
    isBindable(target: string): boolean {
        return this.#kinds.some((kind) => kind.object.identifies(target));
    }

    /** Report whether a bindable kind binds a declaration, by its kind's identity. */
    binds(declaration: { readonly kind: string; readonly kindPackageId: string }): boolean {
        return this.#kinds.some((kind) => kind.declaration.has(declaration));
    }

    /** Resolve the targets an installation's declared bindings name to the records they select. */
    async resolve(bindings: InstallationBindings, plan: StackPlan): Promise<InstallationBindings> {
        // resolve each binding's target with its version and state
        const entries = await Promise.all(
            Object.entries(bindings).map(async ([packageId, names]) => {
                const resolved = await Promise.all(
                    Object.entries(names).map(async ([name, declared]) => {
                        const target = await this.#resolveTarget(declared, plan);

                        return [name, { ...declared, target }] as const;
                    }),
                );

                return [packageId, Object.fromEntries(resolved)] as const;
            }),
        );

        return Object.fromEntries(entries);
    }

    /** Resolve a binding's target to another space's installation or a record of the space. */
    async #resolveTarget(declared: BindingSelection, plan: StackPlan): Promise<BindingTarget> {
        // take an installation of another space by its name as it stands there, unpinned
        const { target } = declared;
        if ("spaceId" in target) {
            if (target.spaceId === plan.stack.scope || declared.version !== undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `binding of '${target.name}' is invalid: it must select an installation of another space by name, without a version`,
                );
            }

            return target;
        }
        // resolve a record the stack declares by its name, a resource among the resource types
        else if ("name" in target) {
            const object = this.#object(target.type, plan);
            const id = schema.anyIdentifier().parse(await plan.require(object, target.name));

            return { type: target.type, id };
        }
        // require an existing target within the destination space
        else {
            const object = this.#object(target.type, plan);
            const [existing] = await plan.transaction
                .read(object.table)
                .findMany({ where: { id: target.id, scope: plan.stack.scope } });
            if (existing !== undefined) {
                return target;
            }
            throw StackError.of(
                "DEFINITION_INVALID",
                `${target.type} '${target.id}' is invalid: the destination space has no such object`,
            );
        }
    }

    /** List the kinds of some needs no provider here serves yet, which wait until one does. */
    unprovided(needs: readonly ResourceNeed[]): string[] {
        const kinds = needs
            .map((need) => need.kind)
            .filter((kind) => !this.#resources.has(kind) && !this.#objects.has(kind));

        return [...new Set(kinds)];
    }

    /** Bind each need to the target its installation selects, else to the record it declares. */
    async bind(
        context: MethodContext,
        target: InstallationRollout,
        needs: readonly ResourceNeed[],
        running: readonly ResourceNeed[],
    ): Promise<Binding[]> {
        // bind each need the installation selects a target for, at the target's version unless pinned
        const bound: Binding[] = [];
        const set = (need: ResourceNeed, selection: BindingSelection) => {
            const selected = selection.target;
            bound.push({
                packageId: need.package.id,
                name: need.name,
                target: "id" in selected ? selected.id : selected.name,
                targetSpaceId: "spaceId" in selected ? selected.spaceId : null,
                version: selection.version ?? null,
            });
        };
        for (const need of needs) {
            const declared = target.installation.bindings[need.package.id]?.[need.name];
            if (declared !== undefined) {
                set(need, await this.#select(context, target, need, declared));
            }
        }

        // refuse an external kind's need nothing binds, as only the stack or the installing person supplies one
        const isUnselected = (need: ResourceNeed) =>
            target.installation.bindings[need.package.id]?.[need.name] === undefined;
        const unbound = needs.find(
            (each) =>
                this.#resources.get(each.kind)?.kind.management === "external" &&
                isUnselected(each),
        );
        if (unbound !== undefined) {
            throw StackError.of(
                "RESOURCE_BLOCKED",
                `cannot install '${unbound.package.name}': it needs ${unbound.kind} '${unbound.name}', so bind one at install`,
            );
        }

        // declare the installation's own targets, then bind each to its managed record at its version
        await this.declare(context, target, { needs, running }, bound);
        for (const need of needs.filter((each) => this.#isOwned(target, each))) {
            const row = await this.#managed(context, target, need);
            if (row !== undefined) {
                const selection = {
                    target: { type: need.kind, id: schema.anyIdentifier().parse(row.id) },
                };
                set(need, await this.#select(context, target, need, selection));
            }
        }

        // order them by package and name, as the revision's digest covers them
        return bound.toSorted((left, right) =>
            `${left.packageId} ${left.name}`.localeCompare(`${right.packageId} ${right.name}`),
        );
    }

    /** Resolve a need to the target its installation selects, at its version unless pinned. */
    async #select(
        context: MethodContext,
        target: InstallationRollout,
        need: ResourceNeed,
        declared: BindingSelection,
    ): Promise<BindingSelection> {
        // bind another space's installation by its name, unpinned
        const selected = declared.target;
        if ("spaceId" in selected) {
            return { target: selected };
        }
        // require the record here, and take its version unless the selection pins one
        else if ("id" in selected) {
            const kind = Binder.#kind(selected.id, this.#kinds);
            const row = await context
                .read(kind.object.table)
                .findFirst({ where: { id: selected.id, scope: target.scope } });
            if (row === undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `binding '${need.name}' of '${target.installation.name}' is invalid: it targets unknown ${kind.object.name} '${selected.id}'`,
                );
            }
            const version = kind.version(row, declared.version ?? null);

            return { target: selected, ...(version === null ? {} : { version }) };
        }
        throw StackError.of(
            "DEFINITION_INVALID",
            `binding '${need.name}' of '${target.installation.name}' is invalid: it selects an unresolved target`,
        );
    }

    /** Report whether a bound target here moved past the version a revision runs it at. */
    async isOutdated(
        reader: Pick<Snapshot, "read">,
        target: InstallationRollout,
        revision: InstallationRevision,
    ): Promise<boolean> {
        // read each local target, and compare its version at the selection's pin
        for (const entry of revision.bindings.filter((each) => each.targetSpaceId === null)) {
            const kind = Binder.#kind(entry.target, this.#kinds);
            const row = await reader
                .read(kind.object.table)
                .findFirst({ where: { id: schema.anyIdentifier().parse(entry.target) } });
            const pinned = target.installation.bindings[entry.packageId]?.[entry.name]?.version;
            if (row === undefined || kind.version(row, pinned ?? null) !== entry.version) {
                return true;
            }
        }

        return false;
    }

    /** Declare the installation's own targets as their manager, with what live deployments run. */
    async declare(
        context: MethodContext,
        target: InstallationRollout,
        declared: {
            readonly needs: readonly ResourceNeed[];
            readonly running: readonly ResourceNeed[];
        },
        bindings: readonly Binding[],
    ): Promise<void> {
        // keep what a live deployment still runs beside the build's own needs
        const owned = declared.needs.filter((need) => this.#isOwned(target, need));
        const kept = declared.running.filter(
            (need) =>
                this.#isOwned(target, need) &&
                !owned.some(
                    (each) => each.package.id === need.package.id && each.name === need.name,
                ),
        );
        await this.#declare(context, target, [...owned, ...kept], bindings);
    }

    /** Report whether a need is the installation's own: a declared kind no selection binds. */
    #isOwned(target: InstallationRollout, need: ResourceNeed): boolean {
        const isSelected = target.installation.bindings[need.package.id]?.[need.name] !== undefined;

        return (this.#resources.has(need.kind) || this.#objects.has(need.kind)) && !isSelected;
    }

    /** Retire a removed installation's records: its own kinds' while it runs, then the others. */
    async retire(
        context: MethodContext,
        target: InstallationRollout,
        kept: readonly ResourceNeed[],
        bindings: readonly Binding[],
    ): Promise<void> {
        // keep declaring the needs of other packages' kinds, which their consumers stop using first
        const others = kept.filter(
            (need) =>
                this.#isOwned(target, need) &&
                this.#bindable(need.kind).object.package.id !== target.installation.packageId,
        );
        await this.#declare(context, target, others, bindings);
    }

    /** Read the bindable kind of a need's kind. */
    #bindable(kind: string): Bindable {
        return present(
            this.#resources.get(kind) ?? this.#objects.get(kind),
            `the bindable kind ${kind}`,
        );
    }

    /** Report whether a removed installation still manages records its own code finalizes. */
    async isFinalizing(context: MethodContext, target: InstallationRollout): Promise<boolean> {
        const own = this.#managedTypes().filter(
            (object) => object.package.id === target.installation.packageId,
        );
        for (const object of own) {
            const [row] = await context.read(Binder.#declarable(object).table).findMany({
                where: { scope: target.scope, managerInstallationId: target.installationId },
            });
            if (row !== undefined) {
                return true;
            }
        }

        return false;
    }

    /** Let go of the retired records a removed installation still manages, which types keep. */
    async disown(context: MethodContext, target: InstallationRollout): Promise<void> {
        // release each managed record by its declared name, one document per declaring package
        for (const packageId of await this.#managers(context, target)) {
            const released = [];
            for (const type of this.#managedTypes()) {
                const object = Binder.#declarable(type);
                const rows = await context.read(object.table).findMany({
                    where: Binder.#managedBy(target, packageId),
                });
                for (const row of rows) {
                    released.push(`${object.collection}/${String(row.managerName)}`);
                }
            }
            await this.#apply(context, target, packageId, { released });
        }
    }

    /** Declare an installation's own targets as their manager, one document per package. */
    async #declare(
        context: MethodContext,
        target: InstallationRollout,
        needs: readonly ResourceNeed[],
        bindings: readonly Binding[],
    ): Promise<void> {
        // apply a document for each package declaring a need or managing a record
        const packages = new Set([
            ...needs.map((need) => need.package.id),
            ...(await this.#managers(context, target)),
        ]);
        for (const packageId of packages) {
            // declare each need's record in its type's collection, adopting a retained one of the same declaration
            const document: Record<string, Record<string, JsonObject>> = {};
            const adopted: Record<string, string> = {};
            for (const need of needs.filter((each) => each.package.id === packageId)) {
                const declared = this.#declaration(need, target, bindings);
                if (declared !== undefined) {
                    const { collection } = declared.object;
                    document[collection] = { ...document[collection], [need.name]: declared.input };
                    const retained = await this.#retained(context, target, need);
                    if (retained !== undefined) {
                        adopted[`${collection}/${need.name}`] = retained;
                    }
                }
            }
            await this.#apply(context, target, packageId, { ...document, adopted });
        }
    }

    /** Read the record a need declares and its type, none while a binding it needs is missing. */
    #declaration(
        need: ResourceNeed,
        target: InstallationRollout,
        bindings: readonly Binding[],
    ): { readonly object: ObjectType; readonly input: JsonObject } | undefined {
        // declare a resource under the installation's name
        const resource = this.#resources.get(need.kind);
        if (resource !== undefined) {
            return {
                object: resource.object,
                input: resource.declare(need, target.installation.name),
            };
        }

        // declare another kind's record once the bindings it needs exist
        const bindable = present(this.#objects.get(need.kind), `the object kind ${need.kind}`);
        const input = bindable.declare(need, bindings);

        return input === undefined ? undefined : { object: bindable.object, input };
    }

    /** Apply an installation's document of one package's declarations in the call's transaction. */
    async #apply(
        context: MethodContext,
        target: InstallationRollout,
        packageId: PackageId,
        document: JsonObject,
    ): Promise<void> {
        const { store, transaction } = context.execution;
        const stack = new Stack({
            database: store.database,
            objects: this.#managedTypes(),
            manager: { installationId: target.installationId, packageId },
            as: store.principal,
            declarer: (object) => Binder.#caller(context, target, object),
            scope: target.scope,
            document,
            applied: {},
            store,
            isHome: false,
            ...(store.claims === undefined ? {} : { claims: store.claims }),
        });
        await StackPlan.open(stack, context.now, transaction);
    }

    /** List the packages whose declarations an installation's records are managed under. */
    async #managers(
        context: MethodContext,
        target: InstallationRollout,
    ): Promise<readonly PackageId[]> {
        const packages = new Set<PackageId>();
        for (const type of this.#managedTypes()) {
            const rows = await context.read(Binder.#declarable(type).table).findMany({
                where: { scope: target.scope, managerInstallationId: target.installationId },
            });
            for (const row of rows) {
                packages.add(PackageId.parse(row.managerPackageId));
            }
        }

        return [...packages];
    }

    /** Find the record an installation manages for a need, absent while it waits. */
    async #managed(
        context: MethodContext,
        target: InstallationRollout,
        need: ResourceNeed,
    ): Promise<{ readonly id: string } | undefined> {
        const kind = this.#bindable(need.kind);
        const [row] = await context.read(Binder.#declarable(kind.object).table).findMany({
            where: { ...Binder.#managedBy(target, need.package.id), managerName: need.name },
        });

        return row;
    }

    /** Find a retained, unmanaged resource of a need's declaration, to adopt again. */
    async #retained(
        context: MethodContext,
        target: InstallationRollout,
        need: ResourceNeed,
    ): Promise<string | undefined> {
        // look for the retained resource of the installation's name only
        const kind = this.#resources.get(need.kind);
        if (kind === undefined) {
            return undefined;
        }
        const [named] = await context.read(kind.object.table).findMany({
            where: { scope: target.scope, name: `${target.installation.name}.${need.name}` },
        });

        // adopt one of the same package's declaration, deleted with its content retained and unmanaged
        const isAdoptable =
            named !== undefined &&
            named.deletedAt !== null &&
            named.retention !== "delete" &&
            named.managerInstallationId === null &&
            named.definitionPackageId === need.package.id &&
            named.definitionName === need.name;

        return isAdoptable ? named.id : undefined;
    }

    /** List the types of an installation's records: the bindable kinds'. */
    #managedTypes(): readonly ObjectType[] {
        return this.#kinds.map((kind) => kind.object);
    }

    /** Require a bindable kind's type to be declarable, as installations declare its records. */
    static #declarable(object: ObjectType) {
        if (!object.isDeclarable()) {
            throw new TypeError(`${object.name} records are bound, so they must be declarable`);
        }

        return object;
    }

    /** Match the records an installation manages under one package's declarations. */
    static #managedBy(target: InstallationRollout, packageId: PackageId) {
        return {
            scope: target.scope,
            managerInstallationId: target.installationId,
            managerPackageId: packageId,
        };
    }

    /** Read the object type of a bindable kind: a resource kind's, or an object kind's own. */
    #object(type: string, plan: StackPlan): ObjectType {
        return this.#resources.get(type)?.object ?? plan.object(type);
    }

    /** Make every bound target of an installation's needs ready to capture where it can. */
    async ready(
        context: MethodContext,
        target: InstallationRollout,
        bindings: readonly Binding[],
        needs: readonly ResourceNeed[],
    ): Promise<boolean> {
        // ready each bound need whose target is in the space and whose kind prepares its targets
        let isReady = true;
        for (const need of needs) {
            const bound = bindings.find(
                (entry) =>
                    entry.packageId === need.package.id &&
                    entry.name === need.name &&
                    entry.targetSpaceId === null,
            );
            // pass over an unbound need
            if (bound === undefined) {
                continue;
            }

            // ready a target its object kind prepares once its row and its provisioned parent arrived
            const bindable = this.#objects.get(need.kind);
            if (bindable !== undefined) {
                const declarer = Binder.#declarer(context, target, bindable.object);
                isReady =
                    (await Binder.#readyObject(context, bindable, declarer, bound, need)) &&
                    isReady;
            }
            // ready a resource once its provider provisioned it
            else {
                const kind = Binder.#kind(bound.target, this.#resources.values());
                const id = kind.object.identifier(bound.target);
                const row = await context.read(kind.object.table).findFirst({ where: { id } });
                isReady = row !== undefined && row.reference !== null && isReady;
            }
        }

        return isReady;
    }

    /** Report whether an object target is ready, preparing one whose provisioned parent is. */
    static async #readyObject(
        context: MethodContext,
        bindable: ObjectBindable,
        declarer: AccessContext,
        bound: Binding,
        need: ResourceNeed,
    ): Promise<boolean> {
        // wait for the target's row, and for a nested target's provisioned parent
        const row: Row | undefined = await context
            .read(bindable.object.table)
            .findFirst({ where: { id: bound.target } });
        if (
            row === undefined ||
            !(await Binder.#isParentProvisioned(context, bindable.object, row))
        ) {
            return false;
        }

        // take a ready target, else prepare it and wait
        if (await bindable.ready(context, bound, need)) {
            return true;
        }
        await bindable.prepare(context, declarer, bound, need);

        return false;
    }

    /** Report whether a nested target's parent is provisioned, true for a target without one. */
    static async #isParentProvisioned(
        context: MethodContext,
        object: ObjectType,
        row: Row,
    ): Promise<boolean> {
        // pass a target whose parent type provisions nothing
        const declared = object.parent;
        const parentId = row["parentId"];
        if (
            declared === undefined ||
            declared.object === "any" ||
            !Provisioned.is(declared.object) ||
            typeof parentId !== "string"
        ) {
            return true;
        }

        // read whether its provider provisioned the parent
        const parent = await context
            .read<Table>(declared.object.table)
            .findFirst({ where: { id: parentId } });
        const reference = schema
            .string()
            .nullable()
            .parse(parent?.["reference"] ?? null);

        return parent !== undefined && Provisioned.isProvisioned({ reference });
    }

    /** Relate an installation as consumer of the objects its live deployments' revisions bind. */
    async relate(
        context: MethodContext,
        target: InstallationRollout,
        related: Digest | null,
    ): Promise<Digest> {
        // read the bindings of its live deployments' revisions to targets in its own space
        const live = await context.read(deployment.table).findMany({
            where: {
                ...Deployment.live(),
                scope: target.scope,
                installationId: target.installationId,
            },
        });
        const revisions = await context.read(installationRevision.table).findMany({
            where: { id: { in: [...new Set(live.map((entry) => entry.revisionId))] } },
        });
        const bound = revisions
            .flatMap((revision) => revision.bindings)
            .filter((entry) => entry.targetSpaceId === null && this.isBindable(entry.target));

        // read the objects each binding of a kind served here lets the installation use
        const wanted = (
            await Promise.all(
                bound.map((entry) =>
                    Binder.#kind(entry.target, this.#kinds).uses(
                        { ...entry, scope: target.scope },
                        context,
                    ),
                ),
            )
        ).flat();

        // list the objects of each used type the space serves, since only its installed packages store objects
        const used = [...new Set(this.#kinds.flatMap((kind) => kind.used))].filter((object) =>
            context.objects.some((served) => served.same(object)),
        );
        const consumed = used.map((object) => {
            const { packageId } = object.policy.definition;
            const ids = wanted
                .filter((entry) => entry.packageId === packageId && entry.type === object.name)
                .map((entry) => entry.id)
                .toSorted();

            return { object, ids };
        });

        // relate the installation to them unless it last related the same, as its rows may live elsewhere
        const digest = await Digest.json(
            consumed.map(({ object, ids }) => [
                object.policy.definition.packageId,
                object.name,
                ids,
            ]),
            context.environment.crypto,
        );
        if (digest !== related) {
            // relate it to each used type's objects as their declarer, skipping types it never related
            const consumer = principal.installation.reference(target.scope, target.installationId);
            const relating = consumed.filter(({ ids }) => related !== null || ids.length > 0);
            for (const { object, ids } of relating) {
                const declarer = Binder.#declarer(context, target, object);
                await context.mutate(declarer, [object.call("bind", { consumer, ids })]);
            }
        }

        return digest;
    }

    /** Find the kind whose objects have a target's identifier, refusing a target of no kind. */
    static #kind<Kind extends Bindable>(target: string, kinds: Iterable<Kind>): Kind {
        const kind = [...kinds].find((each) => each.object.identifies(target));
        if (kind === undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `target '${target}' is invalid: it is not an identifier of any bindable kind`,
            );
        }

        return kind;
    }

    /** Read who declares an installation's records: itself for its kind, else its consenter. */
    static #declarer(
        context: MethodContext,
        target: InstallationRollout,
        object: ObjectType,
    ): AccessContext {
        return AccessContext.of(Binder.#caller(context, target, object), context.now);
    }

    /** Read the caller declaring an installation's records of a type, acting through the stack. */
    static #caller(
        context: MethodContext,
        target: InstallationRollout,
        object: ObjectType,
    ): Caller {
        const owner = principal.installation.reference(target.scope, target.installationId);
        const declarer =
            object.package.id === target.installation.packageId ? owner : Rollout.consenter(target);

        return Caller.acting(declarer, context.self.subject);
    }

    /** Declare the projection writing each resource's states from its consumers. */
    specification(
        store: ObjectStore,
        main: SpaceOptions["main"],
        linked: ReadonlyMap<PackageId, PackageRelease>,
    ): Projection {
        return new Projection(store.database, {
            name: "specification",
            tables: [
                rollout.table,
                installation.table,
                deployment.table,
                space.table,
                placement.table,
                ...this.#resourceObjects().map((object) => object.table),
            ],
            keys: async (change) => this.#specificationKeys(change, store),
            all: async () => this.#servedResources(store),
            project: async (key) => this.#specify(key, store, main, linked),
        });
    }

    /** List a space's main database and the resources a revision binds, none for no revision. */
    async #revisionKeys(
        connection: DatabaseConnection,
        scope: Identifier<"space">,
        revisionId: Identifier<"installation-revision"> | null,
    ): Promise<readonly string[]> {
        const main = await connection
            .select({ id: database.table.id })
            .from(database.table)
            .where(and(eq(database.table.scope, scope), eq(database.table.name, MAIN_DATABASE)));
        const bound = revisionId === null ? [] : await this.#bound(connection, [revisionId]);

        return [...main.map((entry) => entry.id), ...bound];
    }

    /** List the objects of the bindable resource kinds. */
    #resourceObjects(): readonly ResourceType<typeof space>[] {
        return [...this.#resources.values()].map((kind) => kind.object);
    }

    /** List the resources a change of a consumer, a space, its placement or a resource touches. */
    async #specificationKeys(change: Change, store: ObjectStore): Promise<readonly string[]> {
        const connection = store.database;

        // select the space's main database and the resources an applied revision binds
        if (Change.of(change, rollout.table)) {
            const { scope, appliedRevisionId } = Change.image(change);

            return this.#revisionKeys(connection, scope, appliedRevisionId);
        }
        // select the space's main database and the resources a targeted revision binds, none outside spaces
        else if (Change.of(change, installation.table)) {
            const { scope, revisionId } = Change.image(change);
            const spaced = schema.identifier("space").safeParse(scope);

            return spaced.success ? this.#revisionKeys(connection, spaced.data, revisionId) : [];
        }
        // select the resources a changed deployment's revision binds
        else if (Change.of(change, deployment.table)) {
            return this.#bound(connection, [Change.image(change).revisionId]);
        }
        // select the resources of a space whose approval threshold changed
        else if (Change.of(change, space.table)) {
            return this.#resourcesIn(connection, [Change.image(change).id]);
        }
        // select the resources of a space whose placement changed
        else if (Change.of(change, placement.table)) {
            return this.#resourcesIn(connection, [Change.image(change).spaceId]);
        }
        // select a changed resource
        else {
            const changed = Change.image(change)["id"];

            return typeof changed === "string" ? [changed] : [];
        }
    }

    /** List the resources of a served kind some revisions bind. */
    async #bound(
        connection: DatabaseConnection,
        ids: readonly Identifier<"installation-revision">[],
    ): Promise<string[]> {
        const revisions = await connection
            .select()
            .from(installationRevision.table)
            .where(inArray(installationRevision.table.id, [...ids]));

        return this.#targets(
            revisions.flatMap((revision) => revision.bindings.map((entry) => entry.target)),
        );
    }

    /** List the resources of the spaces the machine serves. */
    async #servedResources(store: ObjectStore): Promise<readonly string[]> {
        const connection = store.database;
        const served = await connection
            .select({ id: space.table.id })
            .from(space.table)
            .where(Space.served());

        return this.#resourcesIn(
            connection,
            served.map((entry) => entry.id),
        );
    }

    /** Write a resource's desired states and approval threshold. */
    async #specify(
        key: string,
        store: ObjectStore,
        main: SpaceOptions["main"],
        linked: ReadonlyMap<PackageId, PackageRelease>,
    ): Promise<void> {
        // read the resource and the served space it lives in, none for a resource of an account
        const connection = store.database;
        const object = this.#resourceObjects().find((each) => each.identifies(key));
        const [row] =
            object === undefined
                ? []
                : await connection
                      .select()
                      .from(object.table)
                      .where(object.withIds([key]));
        const [served] =
            row === undefined || !space.identifies(row.scope)
                ? []
                : await connection
                      .select({ approval: space.table.approval })
                      .from(space.table)
                      .where(and(eq(space.table.id, row.scope), Space.served()));
        if (object === undefined || row === undefined || served === undefined) {
            return;
        }

        // read the states its consumers need by digest, the platform's beside them for the main database
        const isMain = object === database && row.name === MAIN_DATABASE;
        const consumed = await consumedStates(connection, object, row, isMain);
        const platform = isMain
            ? await this.#platformState(
                  new Set(consumed.targeted.map((entry) => entry.packageId)),
                  store,
                  main,
                  linked,
              )
            : new Map<Digest, ResourceState>();
        const { active, draining } = desiredStates(consumed, platform);
        const input = {
            id: row.id,
            states: active,
            drainingStates: draining,
            approval: served.approval,
        };

        // record the states, and write their digests and its space's approval threshold once they changed
        const isChanged =
            input.states.join() !== row.states.join() ||
            input.drainingStates.join() !== row.drainingStates.join() ||
            input.approval !== row.approval;
        if (isChanged) {
            // write the platform's state as a blob once the space lacks it, else lease the one it has
            const missing = new Set(await store.blobs.missing([...platform.keys()]));
            for (const [digest, state] of platform) {
                if (missing.has(digest)) {
                    await store.blobs.writeJson(state);
                }
            }
            await store.blobs.lease([...platform.keys()].filter((digest) => !missing.has(digest)));
            await store.mutate(
                store.as(object),
                {
                    id: RequestId.create(store.environment),
                    calls: [object.call("specify", input)],
                },
                { scope: row.scope, component: "specification" },
            );
        }

        // stop the draining deployments whose revision binds a draining resource
        if (row.conditions[DRAINING]?.status === "true") {
            await this.#stopDraining(row, isMain, store);
        }
    }

    /** Read the platform's tables' state, without the replicated tables of a bound package. */
    #platformState(
        owning: ReadonlySet<string>,
        store: ObjectStore,
        main: SpaceOptions["main"],
        linked: ReadonlyMap<PackageId, PackageRelease>,
    ): Promise<ReadonlyMap<Digest, ResourceState>> {
        // find the replicated tables of a bound package, which its own state describes
        const { tables } = main.state();
        const isOwned = (state: TableState) =>
            main.replicated.has(state.table.name) && owning.has(state.package.id);
        const key = tables.sqlite
            .filter(isOwned)
            .map((state) => state.table.name)
            .join(" ");

        // describe the other tables once per set left out, each at the release the machine links of its package
        let read = this.#platform.get(key);
        if (read === undefined) {
            const kept = (states: readonly TableState[]) =>
                states
                    .filter((state) => !isOwned(state))
                    .map((state) => {
                        const release = linked.get(state.package.id);

                        return release === undefined
                            ? state
                            : { ...state, package: release.package };
                    });
            const state = {
                tables: { sqlite: kept(tables.sqlite), postgresql: kept(tables.postgresql) },
            };
            read = Digest.json(state, store.environment.crypto).then(
                (digest) => new Map([[digest, state]]),
            );
            this.#platform.set(key, read);
        }

        return read;
    }

    /** Stop the draining deployments whose revision binds a resource. */
    async #stopDraining(
        row: Pick<Resource, "id" | "scope">,
        isMain: boolean,
        store: ObjectStore,
    ): Promise<void> {
        // find the deployments whose revision binds the resource on their way out: draining, or active on a revision their installation no longer targets
        const scope = schema.identifier("space").parse(row.scope);
        const { running } = await consumersOf(store.database, scope, row.id, isMain);
        const draining = running.filter(
            (entry) => !entry.isCurrent && entry.deployment.status !== "retired",
        );
        await store.mutate(
            store.as(deployment),
            {
                id: RequestId.create(store.environment),
                calls: draining
                    .filter((entry) => ["draining", "active"].includes(entry.deployment.status))
                    .map((entry) =>
                        deployment.call("update", {
                            ...deployment.target(entry.deployment),
                            status: "stopping",
                        }),
                    ),
            },
            { component: "specification" },
        );
    }

    /** Filter the targets to the resources of a served kind. */
    #targets(targets: readonly string[]): string[] {
        return targets.filter((target) =>
            this.#resourceObjects().some((object) => object.identifies(target)),
        );
    }

    /** List the resources of every kind in some spaces. */
    async #resourcesIn(
        connection: DatabaseConnection,
        spaces: readonly string[],
    ): Promise<string[]> {
        const resources: string[] = [];
        for (const object of this.#resourceObjects()) {
            const rows = await connection
                .select({ id: object.table.id })
                .from(object.table)
                .where(inArray(object.table.scope, [...spaces]));
            resources.push(...rows.map((entry) => entry.id));
        }

        return resources;
    }
}

/** The states a resource's consumers require: targeted revisions' and deployed revisions'. */
interface ConsumedStates {
    /** The state each installation's targeted revision requires, with its package. */
    readonly targeted: readonly { readonly packageId: string; readonly state: Digest }[];
    /** The state each deployment's revision requires, current while its revision is targeted. */
    readonly running: readonly { readonly isCurrent: boolean; readonly state: Digest }[];
}

/** Read the states consumers require of a resource: every installation's for main. */
async function consumedStates(
    connection: DatabaseConnection,
    object: ResourceType<typeof space>,
    row: Resource,
    isMain: boolean,
): Promise<ConsumedStates> {
    // require no states of a kind without a state
    const { kind } = present(object.provisioned, `the kind of ${object.name}`);
    if (!kind.isStateful) {
        return { targeted: [], running: [] };
    }

    // take the state each consumer's declaration requires of it
    const scope = schema.identifier("space").parse(row.scope);
    const { targeted, running } = await consumersOf(connection, scope, row.id, isMain);

    return {
        targeted: targeted.flatMap((entry) => {
            const state = entry.revision.states[entry.name];

            return state === undefined ? [] : [{ packageId: entry.packageId, state }];
        }),
        running: running.flatMap((entry) => {
            const state = entry.revision.states[entry.name];

            return state === undefined ? [] : [{ isCurrent: entry.isCurrent, state }];
        }),
    };
}

/** Read the revisions binding a resource that rollouts target and available deployments run. */
async function consumersOf(
    connection: DatabaseConnection,
    scope: Identifier<"space">,
    id: string,
    isMain: boolean,
): Promise<{
    readonly targeted: readonly {
        readonly packageId: string;
        readonly name: string;
        readonly revision: InstallationRevision;
    }[];
    readonly running: readonly {
        readonly name: string;
        readonly revision: InstallationRevision;
        readonly deployment: Deployment;
        readonly isCurrent: boolean;
    }[];
}> {
    // read the space's rollouts with their installations, and its available deployments
    const rollouts = await connection
        .select({
            installationId: rollout.table.installationId,
            revisionId: installation.table.revisionId,
            packageId: installation.table.packageId,
            deletedAt: installation.table.deletedAt,
        })
        .from(rollout.table)
        .innerJoin(
            installation.table,
            and(
                eq(installation.table.id, rollout.table.installationId),
                eq(installation.table.scope, rollout.table.scope),
            ),
        )
        .where(eq(rollout.table.scope, scope));
    const deployed = await connection
        .select()
        .from(deployment.table)
        .where(
            and(
                eq(deployment.table.scope, scope),
                Condition.render(Deployment.available(), deployment.table),
            ),
        );

    // read their revisions once
    const ids = [
        ...rollouts.flatMap((entry) => (entry.revisionId === null ? [] : [entry.revisionId])),
        ...deployed.map((entry) => entry.revisionId),
    ];
    const revisions = new Map(
        (
            await connection
                .select()
                .from(installationRevision.table)
                .where(inArray(installationRevision.table.id, [...new Set(ids)]))
        ).map((revision) => [revision.id, revision]),
    );

    // take the declarations binding the resource: main for a revision keeping tables there, else its bindings
    const binders = (revision: InstallationRevision): string[] =>
        isMain
            ? revision.states[MAIN_DATABASE] === undefined
                ? []
                : [MAIN_DATABASE]
            : revision.bindings
                  .filter((entry) => entry.targetSpaceId === null && entry.target === id)
                  .map((entry) => entry.name);

    // list the targeted revisions of live installations, and the deployments running a revision
    const targeted = rollouts.flatMap((entry) => {
        const revision = entry.revisionId === null ? undefined : revisions.get(entry.revisionId);
        const isLive = !isMain || entry.deletedAt === null;

        return revision === undefined || !isLive
            ? []
            : binders(revision).map((name) => ({ packageId: entry.packageId, name, revision }));
    });
    const running = deployed.flatMap((entry) => {
        // count a deployment current while active on the revision its live installation targets
        const revision = revisions.get(entry.revisionId);
        const owner = rollouts.find((each) => each.installationId === entry.installationId);
        const isCurrent =
            entry.status === "active" &&
            owner !== undefined &&
            owner.deletedAt === null &&
            owner.revisionId === entry.revisionId;

        return revision === undefined
            ? []
            : binders(revision).map((name) => ({ name, revision, deployment: entry, isCurrent }));
    });

    return { targeted, running };
}

/** List the distinct desired states of a resource, active and draining, by digest in order. */
function desiredStates(
    consumed: ConsumedStates,
    platform: ReadonlyMap<Digest, ResourceState>,
): { readonly active: Digest[]; readonly draining: Digest[] } {
    // list each targeted or current state once, then the states of deployments on their way out as draining
    const active = new Set([
        ...platform.keys(),
        ...consumed.targeted.map((entry) => entry.state),
        ...consumed.running.filter((entry) => entry.isCurrent).map((entry) => entry.state),
    ]);
    const draining = new Set(
        consumed.running
            .filter((entry) => !entry.isCurrent && !active.has(entry.state))
            .map((entry) => entry.state),
    );

    return { active: [...active].toSorted(), draining: [...draining].toSorted() };
}
