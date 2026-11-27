import {
    installation,
    ObjectBindingKind,
    SPACE_PROVIDER,
    Binding,
    Database,
    database,
    Deployment,
    deployment,
    Installation,
    installationRevision,
    type InstallationRevision,
    type ResourceRequirement,
} from "@destack/stack/object";
import { placement, Space, space } from "@destack/account/object";
import { MAIN_DATABASE } from "@destack/stack/declare";
import {
    type BindingKind,
    type BindingSelection,
    type BindingTarget,
    type InstallationBindings,
    type MethodContext,
    type ObjectType,
    Provisioned,
    type Resource,
    type ResourceType,
    ObjectController,
} from "@destack/object";
import { AccessContext, Caller, principal } from "@destack/access";
import { and, eq, type Row, type Table, TABLE, DatabaseState } from "@destack/db";
import type { Projection, Snapshot } from "@destack/db/database";
import { PackageId, Kind, type Provisioning } from "@destack/package";
import { RequestId } from "@destack/sync";
import { DRAINING, type ObjectStore, Stack, StackPlan } from "@destack/object/server";
import type { BuildReader } from "@destack/package/manifest";
import { Digest, type Identifier, type JsonObject, present, schema } from "@destack/schema";
import { StackError } from "@destack/stack/error";

/** A resource kind bindings target, whose providers provision its objects. */
export interface ResourceBindingKind extends BindingKind<ResourceType<typeof space>> {
    /** The resource kind the objects are. */
    readonly kind: Kind<schema.Schema, Provisioning>;
    /** Read the record an installation's own requirement declares, as its type's declaration. */
    readonly declare: (requirement: ResourceRequirement, installation: string) => JsonObject;
}

/** Resource kinds as bindings target them. */
export const ResourceBindingKind = {
    /** Bind resources of one kind, at their generation or a binding's pinned version. */
    of(
        kind: Kind<schema.Schema, Provisioning>,
        object: ResourceType<typeof space>,
    ): ResourceBindingKind {
        return {
            object,
            kind,
            used: [object],
            version: (target, pin) => pin ?? target.generation,
            uses: async (bound) => [object.reference(bound.scope, bound.target)],
            declare: (requirement, installationName) => ({
                name: `${installationName}.${requirement.name}`,
                definitionPackageId: requirement.package.id,
                definitionVersion: requirement.package.version,
                definitionName: requirement.name,
                spec: requirement.spec,
            }),
        };
    },
};

/** A bound target: a row of an object type by its identifier. */
export interface BoundTarget {
    /** The target's object type. */
    readonly object: ObjectType;
    /** The target's identifier. */
    readonly id: string;
}

/** Bind declarations to objects of each bindable kind, and capture what deployments run with. */
export class Binder {
    /** The bindable kinds. */
    readonly #kinds: readonly BindingKind[];
    /** The bindable resource kinds, by their kind's name. */
    readonly #resources: ReadonlyMap<string, ResourceBindingKind>;
    /** The kinds packages serve, by their declaration kind. */
    readonly #objects: ReadonlyMap<string, ObjectBindingKind>;
    /** The state of the tables every space's main database replicates. */
    readonly #replicated: DatabaseState | undefined;
    /** The tables every space's main database replicates. */
    readonly #replicatedTables: readonly Table[];

    /** Index the kinds a machine serves, and describe the tables every main database replicates. */
    constructor(
        resources: readonly ResourceBindingKind[],
        objects: readonly ObjectBindingKind[],
        replicated: readonly Table[],
    ) {
        // describe the replicated tables as one state of the main database, none without any
        const names = new Set(replicated.map((table) => table[TABLE].sqlName));
        this.#replicatedTables = replicated;
        this.#replicated =
            replicated.length === 0
                ? undefined
                : DatabaseState.of(replicated, { replicated: names });

        // index the kinds by resource kind and by declaration kind
        this.#kinds = [...resources, ...objects];
        this.#resources = new Map(
            resources.map((each) => [Kind.key(each.kind.packageId, each.kind.name), each]),
        );
        this.#objects = new Map(
            objects.map((each) => [Kind.key(each.kind.packageId, each.kind.name), each]),
        );
    }

    /** List the kinds packages serve to workloads, such as secrets. */
    objects(): readonly ObjectBindingKind[] {
        return [...this.#objects.values()];
    }

    /** Serve databases: a purge drops a package's kept tables from a space's database. */
    databases() {
        return database.implement({
            methods: {
                purge: async (context) => {
                    // read the states the database applies
                    const { target, input } = context;
                    const { packageId } = input;
                    const digests = [...target.states, ...target.drainingStates];
                    const states = await Promise.all(
                        digests.map((digest) =>
                            context.execution.store.blobs.readJson(digest, DatabaseState),
                        ),
                    );

                    // refuse a package an installation still declares, whose tables it keeps
                    const isDeclared = states.some((state) =>
                        Object.values(state.tables).some((tables) =>
                            tables.some((table) => table.package.id === packageId),
                        ),
                    );
                    if (isDeclared) {
                        throw StackError.of(
                            "DATABASE_BLOCKED",
                            `cannot delete the data of package '${packageId}': an installation declares it`,
                        );
                    }

                    // drop the package's tables in the purge's commit
                    Binder.#migrateAtCommit(context, target.id, states, [packageId]);

                    return target;
                },
            },
        });
    }

    /** List the object types bindings target. */
    targets(): readonly ObjectType[] {
        return this.#kinds.map((kind) => kind.object);
    }

    /** Report whether a target is an object of a kind the binder binds on this machine. */
    isBindable(target: string): boolean {
        return this.#kinds.some((kind) => kind.object.identifies(target));
    }

    /** Report whether a declaration binds: every resource, and another kind by its identity. */
    binds(declaration: {
        readonly kind: string;
        readonly kindPackageId: string;
        readonly provisioned?: true;
    }): boolean {
        return (
            declaration.provisioned === true ||
            this.#kinds.some((each) => each.kind.has(declaration))
        );
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

    /** List the kinds of requirements no provider here serves yet, which wait for one. */
    unprovided(requirements: readonly ResourceRequirement[]): string[] {
        const kinds = requirements
            .filter(
                (requirement) =>
                    !this.#resources.has(Binder.#key(requirement)) &&
                    !this.#objects.has(Binder.#key(requirement)),
            )
            .map((requirement) => requirement.kind);

        return [...new Set(kinds)];
    }

    /** Bind each requirement to its installation's selected target, else to its declared record. */
    async bind(
        context: MethodContext,
        target: Installation,
        requirements: readonly ResourceRequirement[],
        running: readonly ResourceRequirement[],
    ): Promise<Binding[]> {
        // bind each requirement the installation selects a target for, at the target's version unless pinned
        const bound: Binding[] = [];
        const set = (requirement: ResourceRequirement, selection: BindingSelection) => {
            const selected = selection.target;
            bound.push({
                packageId: requirement.package.id,
                name: requirement.name,
                target: Binder.#targetOf(selected),
                targetSpaceId: Binder.#spaceOf(selected),
                version: selection.version ?? null,
            });
        };
        for (const requirement of requirements) {
            const declared = target.bindings[requirement.package.id]?.[requirement.name];
            if (declared !== undefined) {
                set(requirement, await this.#select(context, target, requirement, declared));
            }
        }

        // refuse an external kind's requirement nothing binds, as only the stack or the installing person supplies one
        const isUnselected = (requirement: ResourceRequirement) =>
            target.bindings[requirement.package.id]?.[requirement.name] === undefined;
        const unbound = requirements.find(
            (each) =>
                this.#resources.get(Binder.#key(each))?.kind.provisioned.management ===
                    "external" && isUnselected(each),
        );
        if (unbound !== undefined) {
            throw StackError.of(
                "RESOURCE_BLOCKED",
                `cannot install '${unbound.package.name}': it needs ${unbound.kind} '${unbound.name}', so bind one at install`,
            );
        }

        // declare the installation's own targets, then bind each to its managed record at its version
        await this.declare(context, target, { requirements, running }, bound);
        for (const requirement of requirements.filter((each) => this.#isOwned(target, each))) {
            const row = await this.#managed(context, target, requirement);
            if (row !== undefined) {
                const selection = {
                    target: { type: requirement.kind, id: schema.anyIdentifier().parse(row.id) },
                };
                set(requirement, await this.#select(context, target, requirement, selection));
            }
        }

        // order them by package and name, as the revision's digest covers them
        return bound.toSorted((left, right) =>
            `${left.packageId} ${left.name}`.localeCompare(`${right.packageId} ${right.name}`),
        );
    }

    /** Resolve a requirement to its selected target, at that target's version unless pinned. */
    async #select(
        context: MethodContext,
        target: Installation,
        requirement: ResourceRequirement,
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
                    `binding '${requirement.name}' of '${target.name}' is invalid: it targets unknown ${kind.object.name} '${selected.id}'`,
                );
            }
            const version = kind.version(row, declared.version ?? null);

            return { target: selected, ...(version === null ? {} : { version }) };
        }
        throw StackError.of(
            "DEFINITION_INVALID",
            `binding '${requirement.name}' of '${target.name}' is invalid: it selects an unresolved target`,
        );
    }

    /** Report whether a revision's bindings are those bind gives the installation now. */
    async isCurrent(
        reader: Pick<Snapshot, "read">,
        target: Installation,
        revision: InstallationRevision,
        build: BuildReader,
    ): Promise<boolean> {
        // require each of the build's own declarations the binder binds bound, as bind binds every one it owns
        const { bindings, packageId } = target;
        const declared = (await build.declarations()).filter((each) => this.binds(each));
        const isEachBound = declared.every((declaration) =>
            revision.bindings.some(
                (entry) => entry.packageId === packageId && entry.name === declaration.name,
            ),
        );
        if (!isEachBound) {
            return false;
        }

        // require each selected binding bound to the target it selects, none selected since the revision
        const selected = Object.entries(bindings).flatMap(([selectedPackage, names]) =>
            Object.entries(names).map(([name, selection]) => ({
                selectedPackage,
                name,
                selection,
            })),
        );
        const isSelectedBound = selected.every(({ selectedPackage, name, selection }) =>
            revision.bindings.some(
                (entry) =>
                    entry.packageId === selectedPackage &&
                    entry.name === name &&
                    entry.target === Binder.#targetOf(selection.target) &&
                    entry.targetSpaceId === Binder.#spaceOf(selection.target),
            ),
        );
        if (!isSelectedBound) {
            return false;
        }

        // require each other binding to target the record the installation still manages under its name
        for (const entry of revision.bindings) {
            if (bindings[entry.packageId]?.[entry.name] === undefined) {
                const kind = Binder.#kind(entry.target, this.#kinds);
                const [row] = await reader.read(Binder.#declarable(kind.object).table).findMany({
                    where: {
                        ...Binder.#managedBy(
                            target,
                            schema.identifier("package").parse(entry.packageId),
                        ),
                        managerName: entry.name,
                    },
                });
                if (row?.id !== entry.target) {
                    return false;
                }
            }
        }

        // require each local target at the version the revision bound
        return !(await this.isOutdated(reader, target, revision));
    }

    /** Report whether a bound target here moved past the version a revision runs it at. */
    async isOutdated(
        reader: Pick<Snapshot, "read">,
        target: Installation,
        revision: InstallationRevision,
    ): Promise<boolean> {
        // read each local target, and compare its version at the selection's pin
        for (const entry of revision.bindings.filter((each) => each.targetSpaceId === null)) {
            const kind = Binder.#kind(entry.target, this.#kinds);
            const row = await reader
                .read(kind.object.table)
                .findFirst({ where: { id: schema.anyIdentifier().parse(entry.target) } });
            const pinned = target.bindings[entry.packageId]?.[entry.name]?.version;
            if (row === undefined || kind.version(row, pinned ?? null) !== entry.version) {
                return true;
            }
        }

        return false;
    }

    /** Declare the installation's own targets as their manager, with what live deployments run. */
    async declare(
        context: MethodContext,
        target: Installation,
        declared: {
            readonly requirements: readonly ResourceRequirement[];
            readonly running: readonly ResourceRequirement[];
        },
        bindings: readonly Binding[],
    ): Promise<void> {
        // keep what a live deployment still runs beside the build's own requirements
        const owned = declared.requirements.filter((requirement) =>
            this.#isOwned(target, requirement),
        );
        const kept = declared.running.filter(
            (requirement) =>
                this.#isOwned(target, requirement) &&
                !owned.some(
                    (each) =>
                        each.package.id === requirement.package.id &&
                        each.name === requirement.name,
                ),
        );
        await this.#declare(context, target, [...owned, ...kept], bindings);
    }

    /** Report whether a requirement is the installation's own: a declared kind nothing binds. */
    #isOwned(target: Installation, requirement: ResourceRequirement): boolean {
        const isSelected =
            target.bindings[requirement.package.id]?.[requirement.name] !== undefined;

        return (
            (this.#resources.has(Binder.#key(requirement)) ||
                this.#objects.has(Binder.#key(requirement))) &&
            !isSelected
        );
    }

    /** Retire a removed installation's records: its own kinds' while it runs, then the others. */
    async retire(
        context: MethodContext,
        target: Installation,
        kept: readonly ResourceRequirement[],
        bindings: readonly Binding[],
    ): Promise<void> {
        // keep declaring the requirements of other packages' kinds, which their consumers stop using first
        const others = kept.filter(
            (requirement) =>
                this.#isOwned(target, requirement) &&
                this.#bindingKind(requirement).object.package.id !== target.packageId,
        );
        await this.#declare(context, target, others, bindings);
    }

    /** Read the bindable kind of a requirement's kind. */
    #bindingKind(requirement: ResourceRequirement): BindingKind {
        return present(
            this.#resources.get(Binder.#key(requirement)) ??
                this.#objects.get(Binder.#key(requirement)),
            `the bindable kind ${requirement.kind}`,
        );
    }

    /** Read the key of a requirement's kind: its package and name. */
    static #key(requirement: ResourceRequirement): string {
        return Kind.key(requirement.kindPackageId, requirement.kind);
    }

    /** Report whether a removed installation still manages records its own code finalizes. */
    async isFinalizing(context: MethodContext, target: Installation): Promise<boolean> {
        const own = this.#managedTypes().filter((object) => object.package.id === target.packageId);
        for (const object of own) {
            const [row] = await context.read(Binder.#declarable(object).table).findMany({
                where: { scope: target.scope, managerInstallationId: target.id },
            });
            if (row !== undefined) {
                return true;
            }
        }

        return false;
    }

    /** Let go of the retired records a removed installation still manages, which types keep. */
    async disown(context: MethodContext, target: Installation): Promise<void> {
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
        target: Installation,
        requirements: readonly ResourceRequirement[],
        bindings: readonly Binding[],
    ): Promise<void> {
        // apply a document for each package declaring a requirement or managing a record
        const packages = new Set([
            ...requirements.map((requirement) => requirement.package.id),
            ...(await this.#managers(context, target)),
        ]);
        for (const packageId of packages) {
            // declare each requirement's record in its type's collection, adopting a retained one of the same declaration
            const document: Record<string, Record<string, JsonObject>> = {};
            const adopted: Record<string, string> = {};
            for (const requirement of requirements.filter(
                (each) => each.package.id === packageId,
            )) {
                const declared = this.#declaration(requirement, target, bindings);
                if (declared !== undefined) {
                    const { collection } = declared.object;
                    document[collection] = {
                        ...document[collection],
                        [requirement.name]: declared.input,
                    };
                    const retained = await this.#retained(context, target, requirement);
                    if (retained !== undefined) {
                        adopted[`${collection}/${requirement.name}`] = retained;
                    }
                }
            }
            await this.#apply(context, target, packageId, { ...document, adopted });
        }
    }

    /** Read the record a requirement declares and its type, none while a binding is missing. */
    #declaration(
        requirement: ResourceRequirement,
        target: Installation,
        bindings: readonly Binding[],
    ): { readonly object: ObjectType; readonly input: JsonObject } | undefined {
        // declare a resource under the installation's name
        const resource = this.#resources.get(Binder.#key(requirement));
        if (resource !== undefined) {
            return {
                object: resource.object,
                input: resource.declare(requirement, target.name),
            };
        }

        // declare another kind's record once the bindings it needs exist
        const bindable = present(
            this.#objects.get(Binder.#key(requirement)),
            `the object kind ${requirement.kind}`,
        );
        const input = bindable.declare(requirement, bindings);

        return input === undefined ? undefined : { object: bindable.object, input };
    }

    /** Apply an installation's document of one package's declarations in the call's transaction. */
    async #apply(
        context: MethodContext,
        target: Installation,
        packageId: PackageId,
        document: JsonObject,
    ): Promise<void> {
        const { store, transaction } = context.execution;
        const stack = new Stack({
            database: store.database,
            objects: this.#managedTypes(),
            manager: { installationId: target.id, packageId },
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
    async #managers(context: MethodContext, target: Installation): Promise<readonly PackageId[]> {
        const packages = new Set<PackageId>();
        for (const type of this.#managedTypes()) {
            const rows = await context.read(Binder.#declarable(type).table).findMany({
                where: { scope: target.scope, managerInstallationId: target.id },
            });
            for (const row of rows) {
                packages.add(PackageId.parse(row.managerPackageId));
            }
        }

        return [...packages];
    }

    /** Find the record an installation manages for a requirement, absent while it waits. */
    async #managed(
        context: MethodContext,
        target: Installation,
        requirement: ResourceRequirement,
    ): Promise<{ readonly id: string } | undefined> {
        const kind = this.#bindingKind(requirement);
        const [row] = await context.read(Binder.#declarable(kind.object).table).findMany({
            where: {
                ...Binder.#managedBy(target, requirement.package.id),
                managerName: requirement.name,
            },
        });

        return row;
    }

    /** Find a retained, unmanaged resource of a requirement's declaration, to adopt again. */
    async #retained(
        context: MethodContext,
        target: Installation,
        requirement: ResourceRequirement,
    ): Promise<string | undefined> {
        // look for the retained resource of the installation's name only
        const kind = this.#resources.get(Binder.#key(requirement));
        if (kind === undefined) {
            return undefined;
        }
        const [named] = await context.read(kind.object.table).findMany({
            where: {
                scope: Installation.space(target),
                name: `${target.name}.${requirement.name}`,
            },
        });

        // adopt one of the same package's declaration, deleted with its content retained and unmanaged
        const isAdoptable =
            named !== undefined &&
            named.deletedAt !== null &&
            named.retention !== "delete" &&
            named.managerInstallationId === null &&
            named.definitionPackageId === requirement.package.id &&
            named.definitionName === requirement.name;

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

    /** Read the target a binding records for a selection: its record's id, else its name. */
    static #targetOf(selected: BindingSelection["target"]): string {
        return "id" in selected ? selected.id : selected.name;
    }

    /** Read the space a selection's target lives in, none for a target here. */
    static #spaceOf(selected: BindingSelection["target"]): Identifier<"space"> | null {
        return "spaceId" in selected ? selected.spaceId : null;
    }

    /** Match the records an installation manages under one package's declarations. */
    static #managedBy(target: Installation, packageId: PackageId) {
        return {
            scope: target.scope,
            managerInstallationId: target.id,
            managerPackageId: packageId,
        };
    }

    /** Read the object type of a bindable kind: a resource kind's, or an object kind's own. */
    #object(type: string, plan: StackPlan): ObjectType {
        return this.#resources.get(type)?.object ?? plan.object(type);
    }

    /** Ready the resources a revision requires at its states, answering the first not ready. */
    async ready(
        context: MethodContext,
        target: Installation,
        revision: Pick<InstallationRevision, "bindings" | "states">,
        requirements: readonly ResourceRequirement[],
    ): Promise<BoundTarget | undefined> {
        // ready each bound requirement whose target is in the space and whose kind prepares its targets
        const { bindings, states } = revision;
        let unready: BoundTarget | undefined;
        for (const requirement of requirements) {
            const bound = bindings.find(
                (entry) =>
                    entry.packageId === requirement.package.id &&
                    entry.name === requirement.name &&
                    entry.targetSpaceId === null,
            );
            // pass over an unbound requirement
            if (bound === undefined) {
                continue;
            }

            // ready a target its object kind prepares once its row and its provisioned parent arrived
            const bindable = this.#objects.get(Binder.#key(requirement));
            if (bindable !== undefined) {
                const declarer = Binder.#declarer(context, target, bindable.object);
                unready ??= await Binder.#readyObject(
                    context,
                    bindable,
                    declarer,
                    bound,
                    requirement,
                );
            }
            // ready a resource once its provider provisioned it and, for a stateful kind, it applied the revision's state
            else {
                const kind = Binder.#kind(bound.target, this.#resources.values());
                const id = kind.object.identifier(bound.target);
                const row = await context.read(kind.object.table).findFirst({ where: { id } });
                const provisioned = present(
                    kind.object.provisioned,
                    `the kind of ${kind.object.name}`,
                );
                const state = provisioned.kind.isStateful() ? states[requirement.name] : undefined;
                if (row === undefined || row.reference === null || !Binder.#holds(row, state)) {
                    unready ??= { object: kind.object, id };
                }
            }
        }

        // ready the space's own main database for a revision binding main nowhere else, once it applied its tables
        const tables = states[MAIN_DATABASE];
        const isMainBound = bindings.some(
            (entry) => entry.name === MAIN_DATABASE && entry.targetSpaceId === null,
        );
        if (tables !== undefined && !isMainBound) {
            const main = await Database.main(context, Installation.space(target));
            if (!Binder.#holds(main, tables)) {
                unready ??= { object: database, id: main.id };
            }
        }

        return unready;
    }

    /** Read whether a resource applied a state, true when none is required. */
    static #holds(row: Pick<Resource, "appliedStates">, state: Digest | undefined): boolean {
        return state === undefined || row.appliedStates.includes(state);
    }

    /** Ready an object target, preparing it once its parent is provisioned. */
    static async #readyObject(
        context: MethodContext,
        bindable: ObjectBindingKind,
        declarer: AccessContext,
        bound: Binding,
        requirement: ResourceRequirement,
    ): Promise<BoundTarget | undefined> {
        // wait for the target's row
        const waited = { object: bindable.object, id: bound.target };
        const row: Row | undefined = await context
            .read(bindable.object.table)
            .findFirst({ where: { id: bound.target } });
        if (row === undefined) {
            return waited;
        }

        // wait for a nested target's provisioned parent
        const parent = bindable.object.parent;
        const parentId = row["parentId"];
        if (!(await Binder.#isParentProvisioned(context, bindable.object, row))) {
            return parent !== undefined && parent.object !== "any" && typeof parentId === "string"
                ? { object: parent.object, id: parentId }
                : waited;
        }

        // take a ready target, else prepare it and wait
        if (await bindable.ready(context, bound, requirement)) {
            return undefined;
        }
        await bindable.prepare(context, declarer, bound, requirement);

        return waited;
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
        target: Installation,
        related: Digest | null,
    ): Promise<Digest> {
        // read the bindings of its live deployments' revisions to targets in its own space
        const live = await context.read(deployment.table).findMany({
            where: {
                ...Deployment.live(),
                scope: Installation.space(target),
                installationId: target.id,
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
            const consumer = principal.installation.reference(target.scope, target.id);
            const relating = consumed.filter(({ ids }) => related !== null || ids.length > 0);
            for (const { object, ids } of relating) {
                const declarer = Binder.#declarer(context, target, object);
                await context.mutate(declarer, [object.call("bind", { consumer, ids })]);
            }
        }

        return digest;
    }

    /** Find the kind whose objects have a target's identifier, refusing a target of no kind. */
    static #kind<Kind extends BindingKind>(target: string, kinds: Iterable<Kind>): Kind {
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
        target: Installation,
        object: ObjectType,
    ): AccessContext {
        return AccessContext.of(Binder.#caller(context, target, object), context.now);
    }

    /** Read the caller declaring an installation's records of a type, acting through the stack. */
    static #caller(context: MethodContext, target: Installation, object: ObjectType): Caller {
        const owner = principal.installation.reference(target.scope, target.id);
        const declarer =
            object.package.id === target.packageId ? owner : Installation.consenter(target);

        return Caller.onBehalfOf(declarer, context.self.subject);
    }

    /** Declare a projection per bindable kind writing each resource's states from its consumers. */
    specifications(store: ObjectStore): Projection[] {
        return this.#resourceObjects().map((object) => ({
            name: ObjectController.job(object, "specify"),
            key: "id",
            result: "process",
            all: { table: object.table, column: "id" },
            reads: [
                // take a resource itself
                {
                    table: object.table,
                    fields: Object.keys(object.table[TABLE].logged),
                    isObserving: true,
                },
                // take the space's resources once an installation targets another revision
                {
                    table: installation.table,
                    fields: ["targetRevisionId", "deletedAt"],
                    on: { scope: "scope" },
                },
                // take them once a deployment runs or stops running a revision
                {
                    table: deployment.table,
                    fields: ["status", "revisionId"],
                    on: { scope: "scope" },
                },
                // take the space's resources once its approval threshold or its placement changes
                { table: space.table, fields: ["approval"], on: { scope: "id" } },
                {
                    table: placement.table,
                    fields: ["machineId", "targetId", "status", "epoch"],
                    on: { scope: "spaceId" },
                },
            ],
            project: async (key) => this.#specify(key, store),
        }));
    }

    /** List the objects of the bindable resource kinds. */
    #resourceObjects(): readonly ResourceType<typeof space>[] {
        return [...this.#resources.values()].map((kind) => kind.object);
    }

    /** Write a resource's desired states and approval threshold. */
    async #specify(key: string, store: ObjectStore): Promise<void> {
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

        // write the states its consumers requirement and its space's approval threshold once they changed
        const isMain = object === database && row.name === MAIN_DATABASE;
        const input = await this.#specification(
            connection.snapshot(),
            store,
            object,
            row,
            served.approval,
        );
        if (input !== undefined) {
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

    /** Write the states a space's revisions require of its resources, in the call's transaction. */
    async specify(call: MethodContext, scope: Identifier<"space">): Promise<void> {
        // read the space's approval threshold
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };
        const served = await reader.read(space.table).findFirst({ where: { id: scope } });
        if (served === undefined) {
            return;
        }

        // specify each resource whose states or threshold moved
        const { store } = call.execution;
        for (const object of this.#resourceObjects()) {
            const rows = await reader.read(object.table).findMany({ where: { scope } });
            for (const row of rows) {
                const input = await this.#specification(
                    reader,
                    store,
                    object,
                    row,
                    served.approval,
                );
                if (input !== undefined) {
                    await call.mutate(store.as(object), [object.call("specify", input)]);
                }
            }
        }
    }

    /** Reconcile in the call the resources whose provider creates nothing outside the store. */
    async reconcilePure(call: MethodContext, scope: Identifier<"space">): Promise<void> {
        // read the space's resources of every kind
        const reader: Pick<Snapshot, "read"> = { read: (table) => call.read(table) };
        const { store } = call.execution;
        for (const object of this.#resourceObjects()) {
            const rows = await reader.read(object.table).findMany({ where: { scope } });
            for (const row of rows) {
                // reconcile one whose desired states are not applied, the space's own database at the commit
                const desired = [...row.states, ...row.drainingStates];
                const isApplied =
                    row.reference !== null &&
                    desired.join() === row.appliedStates.join() &&
                    row.conditions["Ready"]?.reason === "Applied";
                const isOwn = (row.provider ?? row.placement?.provider) === SPACE_PROVIDER;
                if (!isApplied && isOwn) {
                    await store.reconcileResource(call, object, row, async (record, states) => {
                        Binder.#migrateAtCommit(call, record.id, states, []);
                    });
                } else if (!isApplied && row.deletedAt === null && store.isPure(object, row)) {
                    await store.reconcileResource(call, object, row);
                }

                // declare the tables main replicates once it applied them, which replicas write and retire
                const isMain = object === database && row.name === MAIN_DATABASE;
                const reconciled = isApplied
                    ? row
                    : await reader.read(object.table).findFirst({ where: { id: row.id } });
                if (isMain && desired.join() === reconciled?.appliedStates.join()) {
                    store.database.state.declare(this.#replicatedTables);
                }
            }
        }
    }

    /** Migrate the space's own database to its desired states at the call's commit. */
    static #migrateAtCommit(
        call: Pick<MethodContext, "execution">,
        manager: string,
        states: readonly unknown[],
        purged: readonly string[],
    ): void {
        // plan and apply on the committing connection, so the tables commit with the rows
        const desired = schema.array(DatabaseState).parse(states);
        call.execution.transaction.record(
            async (committing) => {
                const declared = desired.map((state) => state.tables[committing.dialect]);
                await committing.migrate([], { states: declared, manager, purged });
            },
            `migrate:${manager}:${purged.join(",")}`,
        );
    }

    /** Read a resource's specification once its consumers' states or its threshold moved. */
    async #specification(
        reader: Pick<Snapshot, "read">,
        store: Pick<ObjectStore, "blobs">,
        object: ResourceType<typeof space>,
        row: Resource,
        approval: NonNullable<Resource["approval"]>,
    ) {
        // read the states its consumers requirement by digest, and main's replicated tables beside them
        const isMain = object === database && row.name === MAIN_DATABASE;
        const consumed = await consumedStates(reader, object, row, isMain);
        const desired = desiredStates(consumed);
        const replicated =
            isMain && this.#replicated !== undefined
                ? [await store.blobs.writeJson(this.#replicated)]
                : [];
        const active = [...replicated, ...desired.active];
        const { draining } = desired;
        const isChanged =
            active.join() !== row.states.join() ||
            draining.join() !== row.drainingStates.join() ||
            approval !== row.approval;

        return isChanged
            ? { id: row.id, states: active, drainingStates: draining, approval }
            : undefined;
    }

    /** Stop the draining deployments whose revision binds a resource. */
    async #stopDraining(
        row: Pick<Resource, "id" | "scope">,
        isMain: boolean,
        store: ObjectStore,
    ): Promise<void> {
        // find the deployments whose revision binds the resource on their way out: draining, or active on a revision their installation no longer targets
        const scope = schema.identifier("space").parse(row.scope);
        const { running } = await consumersOf(store.database.snapshot(), scope, row.id, isMain);
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
    reader: Pick<Snapshot, "read">,
    object: ResourceType<typeof space>,
    row: Resource,
    isMain: boolean,
): Promise<ConsumedStates> {
    // require no states of a kind without a state
    const { kind } = present(object.provisioned, `the kind of ${object.name}`);
    if (!kind.isStateful()) {
        return { targeted: [], running: [] };
    }

    // take the state each consumer's declaration requires of it
    const scope = schema.identifier("space").parse(row.scope);
    const { targeted, running } = await consumersOf(reader, scope, row.id, isMain);

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

/** Read the revisions binding a resource: those installations target and deployments run. */
async function consumersOf(
    reader: Pick<Snapshot, "read">,
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
    // read the space's installations and its available deployments
    const installed = await reader.read(installation.table).findMany({ where: { scope } });
    const deployed = await reader
        .read(deployment.table)
        .findMany({ where: { ...Deployment.available(), scope } });

    // read their revisions once
    const ids = [
        ...installed.flatMap((row) =>
            row.targetRevisionId === null ? [] : [row.targetRevisionId],
        ),
        ...deployed.map((entry) => entry.revisionId),
    ];
    const revisions = new Map(
        (
            await reader
                .read(installationRevision.table)
                .findMany({ where: { id: { in: [...new Set(ids)] } } })
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
    const targeted = installed.flatMap((row) => {
        const revision =
            row.targetRevisionId === null ? undefined : revisions.get(row.targetRevisionId);
        const isLive = !isMain || row.deletedAt === null;

        return revision === undefined || !isLive
            ? []
            : binders(revision).map((name) => ({ packageId: row.packageId, name, revision }));
    });
    const running = deployed.flatMap((entry) => {
        // count a deployment current while active on the revision its live installation targets
        const revision = revisions.get(entry.revisionId);
        const owner = installed.find((each) => each.id === entry.installationId);
        const isCurrent =
            entry.status === "active" &&
            owner !== undefined &&
            owner.deletedAt === null &&
            owner.targetRevisionId === entry.revisionId;

        return revision === undefined
            ? []
            : binders(revision).map((name) => ({ name, revision, deployment: entry, isCurrent }));
    });

    return { targeted, running };
}

/** List the distinct desired states of a resource, active and draining, by digest in order. */
function desiredStates(consumed: ConsumedStates): {
    readonly active: Digest[];
    readonly draining: Digest[];
} {
    // list each targeted or current state once, then the states of deployments on their way out as draining
    const active = new Set([
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
