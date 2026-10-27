import { type space } from "@destack/account/object";
import {
    type InstallationBindings,
    MAIN_DATABASE,
    type StackBinding,
    type StackBindingTarget,
} from "../../declare/index.ts";
import { ObjectReference, Subject } from "@destack/sync";
import {
    type MethodContext,
    type ObjectType,
    Provisioned,
    type Resource,
    type ResourceType,
} from "@destack/object";
import { AccessContext, Caller, principal } from "@destack/access";
import type { Row, Select, Snapshot, Table } from "@destack/db";
import type { ObjectStore, StackPlan } from "@destack/object/server";
import { type Package, PackageId } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import {
    RESOURCE_DECLARATION,
    type ResourceDescription,
    type ResourceKind,
} from "@destack/resource";
import { canonicalize, Digest, type Identifier, nth, present, schema } from "@destack/schema";
import { StackError } from "../../error/index.ts";
import {
    Binding,
    binding,
    capture,
    Database,
    type Capture,
    Deployment,
    deployment,
    Rollout,
    rollout,
    type NamedInstallation,
} from "../../object/index.ts";

/** A resource declaration of an installation's package, with the state it requires. */
export type ResourceNeed = ResourceDescription & {
    /** The package declaring the resource. */
    readonly package: Package;
    /** The state the declaration requires, such as a database's tables. */
    readonly state: ResourceState;
};

/** A kind of object a binding targets, and the version a deployment captures of it. */
export interface Bindable<Object extends ObjectType = ObjectType> {
    /** The bindable object type. */
    readonly object: Object;
    /** The object types captures of this kind let installations use. */
    readonly used: readonly ObjectType[];
    /** Read the version a deployment captures of a target, null for a secret without a version. */
    version(target: Select<Object["table"]>, pin: number | null): number | null;
    /** Read the objects a capture of a target in the deployment's space lets it use. */
    readonly uses: (
        captured: Pick<Capture, "scope" | "target"> & { readonly version: number | null },
        snapshot: Pick<Snapshot, "read">,
    ) => Promise<readonly ObjectReference[]>;
    /** The declaration kind whose needs this kind binds, such as resource or secret. */
    readonly declaration: string;
}

/** A resource kind bindings target, whose providers provision its objects. */
export interface ResourceBindable extends Bindable<ResourceType<typeof space>> {
    /** The resource kind the objects are. */
    readonly kind: ResourceKind;
}

/** A kind of object a package serves to workloads, such as secrets. */
export interface ServedBindable<
    Object extends ObjectType = ObjectType,
    Client = unknown,
> extends Bindable<Object> {
    /** The provider workloads read the bound objects through, such as vault. */
    readonly provider: string;
    /** The address workloads call the package serving the bound objects at. */
    readonly reference: string;
    /** Provision the owned target of its installation for an unbound need, as its declarer. */
    readonly provision: (
        context: MethodContext,
        declarer: AccessContext,
        target: Rollout & { readonly installation: NamedInstallation },
        need: ResourceNeed,
        bindings: readonly Binding[],
    ) => Promise<string | undefined>;
    /** Report whether a bound target is ready to capture, reading it alone. */
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
    /** Connect an in-process installation to a binding's target at its captured version. */
    readonly connect: (
        store: Promise<ObjectStore>,
        row: Rollout & { readonly installation: NamedInstallation },
        bound: Binding,
    ) => Promise<Client>;
}

/** The kinds of objects bindings target. */
export const Bindable = {
    /** Bind resources of one kind, captured at their generation or a binding's pinned version. */
    resource(kind: ResourceKind, object: ResourceType<typeof space>): ResourceBindable {
        return {
            object,
            kind,
            declaration: RESOURCE_DECLARATION,
            used: [object],
            version: (target, pin) => pin ?? target.generation,
            uses: async (captured) => [object.reference(captured.scope, captured.target)],
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
    readonly #served: ReadonlyMap<string, ServedBindable>;

    /** Index the resource kinds and the other bindable kinds a machine serves. */
    constructor(resources: readonly ResourceBindable[], served: readonly ServedBindable[]) {
        // index the kinds by resource kind and by declaration kind
        this.#kinds = [...resources, ...served];
        this.#resources = new Map(resources.map((kind) => [kind.kind.name, kind]));
        this.#served = new Map(served.map((kind) => [kind.declaration, kind]));
    }

    /** List the object types bindings target. */
    targets(): readonly ObjectType[] {
        return this.#kinds.map((kind) => kind.object);
    }

    /** Report whether a bindable kind binds the declarations of a kind. */
    binds(declaration: string): boolean {
        return this.#kinds.some((kind) => kind.declaration === declaration);
    }

    /** Serve bindings, each released with the owned resource of its installation. */
    serve() {
        return binding.implement({
            methods: {
                release: async (call, next) => {
                    // delete the binding, then disown its resource for deletion as its installation declares it
                    const released = await next();
                    const resource = Binder.#kind(
                        call.target.target,
                        this.#resources.values(),
                    ).object;
                    const [row] = await call
                        .read(rollout.table)
                        .findMany({ where: { id: call.target.installationId } });
                    const declaring = await Rollout.read(call, [
                        present(row, `the rollout of ${call.target.installationId}`),
                    ]);
                    const declarer = Binder.#declarer(call, nth(declaring, 0), resource);
                    await call.mutate(declarer, [
                        resource.call("adopt", { id: call.target.target, owner: null }),
                        resource.call("delete", { id: call.target.target }),
                    ]);

                    return released;
                },
            },
        });
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
    async #resolveTarget(declared: StackBinding, plan: StackPlan): Promise<StackBindingTarget> {
        // take an installation of another space by its name as it stands there, unpinned
        const { target } = declared;
        if ("spaceId" in target) {
            if (target.spaceId === plan.stack.scope || declared.version !== undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `a binding must select an installation of another space by name without a version, not ${target.name} in ${target.spaceId}`,
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
                `${target.type} ${target.id} not found in the destination space`,
            );
        }
    }

    /** Capture what a new deployment runs with. */
    async capture(
        context: MethodContext,
        created: Select<typeof deployment.table>,
        required: ReadonlyMap<string, ResourceState> = new Map(),
    ): Promise<Select<typeof capture.table>[]> {
        // read the installation's bindings
        const bindings = await context.read(binding.table).findMany({
            where: {
                scope: created.scope,
                installationId: created.installationId,
            },
        });
        const byKind = Map.groupBy(
            bindings.filter((entry) => entry.targetSpaceId === null),
            (entry) => Binder.#kind(entry.target, this.#kinds),
        );

        // capture each binding to another space's target
        const captures = bindings.flatMap((entry) =>
            entry.targetSpaceId === null
                ? []
                : [
                      Binder.#captureCall(
                          created,
                          entry,
                          { targetSpaceId: entry.targetSpaceId },
                          required,
                      ),
                  ],
        );

        // resolve each binding in the deployment's space to the version of its target it runs with
        for (const [kind, entries] of byKind) {
            const targets = await context.read(kind.object.table).findMany({
                where: {
                    id: { in: entries.map((entry) => entry.target) },
                },
            });
            const byId = new Map(targets.map((row) => [row["id"], row]));
            for (const entry of entries) {
                const target = byId.get(entry.target);
                if (target === undefined) {
                    throw StackError.of(
                        "DEFINITION_INVALID",
                        `binding ${entry.name} targets unknown ${kind.object.name} ${entry.target}`,
                    );
                }
                const at = { version: kind.version(target, entry.version) };
                captures.push(Binder.#captureCall(created, entry, at, required));
            }
        }

        return [...(await context.mutate(context.self, captures))];
    }

    /** Describe what a deployment runs with for one binding. */
    static #captureCall(
        created: Select<typeof deployment.table>,
        entry: Binding,
        at: { readonly version: number | null } | { readonly targetSpaceId: Identifier<"space"> },
        required: ReadonlyMap<string, ResourceState>,
    ) {
        return capture.call("create", {
            deploymentId: created.id,
            packageId: entry.packageId,
            name: entry.name,
            target: entry.target,
            ...at,
            state: required.get(Binding.key(entry.packageId, entry.name)) ?? entry.state,
        });
    }

    /** List the kinds of some needs no provider here serves yet, which wait until one does. */
    unprovided(needs: readonly ResourceNeed[]): string[] {
        const kinds = needs
            .map((need) => need.kind)
            .filter((kind) => !this.#resources.has(kind) && !this.#served.has(kind));

        return [...new Set(kinds)];
    }

    /** Bind each declaration no stack binding satisfies to an owned target of the installation. */
    async attach(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        needs: readonly ResourceNeed[],
    ): Promise<void> {
        // read the installation's bindings by declaration with their owned resources
        const rows = await Binder.#bindings(context, target);
        const bindings = await Promise.all(
            rows.map(async (entry) => ({
                binding: entry,
                owned: await this.owned(context, entry, target.id),
            })),
        );
        const bound = new Map(
            bindings.map((entry) => [
                Binding.key(entry.binding.packageId, entry.binding.name),
                entry,
            ]),
        );

        // bind each declaration the installation selects a target for, replacing an owned binding of it
        const selected = Object.entries(target.bindings).flatMap(([packageId, names]) =>
            Object.entries(names).map(([name, declared]) => ({ packageId, name, declared })),
        );
        for (const { packageId, name, declared } of selected) {
            await this.#bindSelected(
                context,
                target,
                packageId,
                name,
                declared,
                bound.get(Binding.key(packageId, name)),
            );
        }
        const isSelected = (need: ResourceNeed) =>
            target.bindings[need.package.id]?.[need.name] !== undefined;

        // create an owned resource for each unbound declaration, and update owned ones to follow their declaration
        const resources = needs.filter(
            (need) => this.#resources.has(need.kind) && !isSelected(need),
        );
        for (const need of resources) {
            const existing = bound.get(Binding.key(need.package.id, need.name));
            // bind the installation's tables to the space's main database
            if (need.name === MAIN_DATABASE) {
                await Binder.#bindMain(context, target, need, existing?.binding);
            }
            // create an owned resource for an unbound declaration
            else if (existing === undefined) {
                await this.#own(context, target, need);
            }
            // update an owned resource to follow its declaration
            else if (existing.owned !== undefined) {
                await this.#follow(context, target, existing.binding, existing.owned, need);
            }
        }

        // create the other owned targets of unbound declarations
        const others = needs.filter((need) => !this.#resources.has(need.kind) && !isSelected(need));
        for (const need of others.filter(
            (each) => !bound.has(Binding.key(each.package.id, each.name)),
        )) {
            await this.#ownOther(context, target, need);
        }

        // delete the bindings no declaration needs to targets of other kinds, which resources may contain
        const needed = new Set([
            ...needs.map((need) => Binding.key(need.package.id, need.name)),
            ...selected.map((each) => Binding.key(each.packageId, each.name)),
        ]);
        const unneeded = bindings.filter(
            (entry) => !needed.has(Binding.key(entry.binding.packageId, entry.binding.name)),
        );
        const contained = unneeded.filter(
            (entry) =>
                entry.binding.name !== MAIN_DATABASE &&
                ![...this.#resources.values()].some((kind) =>
                    kind.object.identifies(entry.binding.target),
                ),
        );
        if (contained.length > 0) {
            await context.mutate(
                context.self,
                contained.map((entry) => binding.call("delete", { id: entry.binding.id })),
            );
        }

        // then release the owned resources no declaration needs, and delete a main binding no table needs
        for (const entry of unneeded) {
            // release an owned resource
            if (entry.owned !== undefined) {
                await context.mutate(context.self, [
                    binding.call("release", { id: entry.binding.id }),
                ]);
            }
            // delete the binding to the main database
            else if (entry.binding.name === MAIN_DATABASE) {
                await context.mutate(context.self, [
                    binding.call("delete", { id: entry.binding.id }),
                ]);
            }
        }
    }

    /** Read the object type of a bindable kind: a resource kind's, or a served kind's own. */
    #object(type: string, plan: StackPlan): ObjectType {
        return this.#resources.get(type)?.object ?? plan.object(type);
    }

    /** Bind a declaration to the target its installation selects, releasing an owned binding. */
    async #bindSelected(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        packageId: string,
        name: string,
        declared: StackBinding,
        existing:
            | { readonly binding: Select<typeof binding.table>; readonly owned?: unknown }
            | undefined,
    ): Promise<void> {
        // read the selected target: another space's installation by name, or a record here
        const selected =
            "spaceId" in declared.target
                ? { target: declared.target.name, targetSpaceId: declared.target.spaceId }
                : "id" in declared.target
                  ? { target: declared.target.id, targetSpaceId: null }
                  : undefined;
        if (selected === undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `binding ${name} of ${target.installation.name} selects an unresolved target`,
            );
        }
        const version = declared.version ?? null;

        // reuse a binding to the selected target, refreshing its state
        const current = existing?.binding;
        if (
            current !== undefined &&
            current.target === selected.target &&
            current.targetSpaceId === selected.targetSpaceId &&
            current.version === version
        ) {
            if (canonicalize(current.state) !== canonicalize(declared.state)) {
                await context.mutate(context.self, [
                    binding.call("update", { ...binding.target(current), state: declared.state }),
                ]);
            }

            return;
        }

        // release an owned binding or delete another one, then bind the selected target
        if (current !== undefined) {
            await context.mutate(context.self, [
                binding.call(existing?.owned === undefined ? "delete" : "release", {
                    id: current.id,
                }),
            ]);
        }
        await context.mutate(context.self, [
            binding.call("create", {
                installationId: target.id,
                packageId: PackageId.parse(packageId),
                name,
                ...selected,
                version,
                state: declared.state,
            }),
        ]);
    }

    /** Make every bound target of an installation's needs ready to capture where it can. */
    async ready(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        needs: readonly ResourceNeed[],
    ): Promise<boolean> {
        // read the installation's bindings
        const bindings = await Binder.#bindings(context, target);

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

            // ready a target its served kind prepares once its row and its provisioned parent arrived
            const served = this.#served.get(need.kind);
            if (served !== undefined) {
                const declarer = Binder.#declarer(context, target, served.object);
                isReady =
                    (await Binder.#readyServed(context, served, declarer, bound, need)) && isReady;
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

    /** Report whether a served target is ready, preparing one whose provisioned parent is. */
    static async #readyServed(
        context: MethodContext,
        served: ServedBindable,
        declarer: AccessContext,
        bound: Binding,
        need: ResourceNeed,
    ): Promise<boolean> {
        // wait for the target's row, and for a nested target's provisioned parent
        const row: Row | undefined = await context
            .read(served.object.table)
            .findFirst({ where: { id: bound.target } });
        if (
            row === undefined ||
            !(await Binder.#isParentProvisioned(context, served.object, row))
        ) {
            return false;
        }

        // take a ready target, else prepare it and wait
        if (await served.ready(context, bound, need)) {
            return true;
        }
        await served.prepare(context, declarer, bound, need);

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

    /** Read the resource a binding targets when its installation is the owner, else absent. */
    async owned(
        snapshot: Pick<Snapshot, "read">,
        bound: Binding,
        installationId: Identifier<"installation">,
    ): Promise<Resource | undefined> {
        // look only at resource targets
        const kind = [...this.#resources.values()].find((each) =>
            each.object.identifies(bound.target),
        );
        if (kind === undefined) {
            return undefined;
        }

        // read the target when the installation is the owner
        const table = kind.object.table;
        const owner = principal.installation.reference(bound.scope, installationId);
        const id = schema.identifier(kind.object.identity).parse(bound.target);

        return snapshot
            .read(table)
            .findFirst({ where: { id, ...ObjectReference.columns("owner", owner) } });
    }

    /** Relate an installation as consumer of exactly the objects its live deployments captured. */
    async relate(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        related: Digest | null,
    ): Promise<Digest> {
        // read the captures of the installation's live deployments of targets in its own space
        const live = await context.read(deployment.table).findMany({
            where: {
                ...Deployment.live(),
                scope: target.scope,
                installationId: target.id,
            },
        });
        const captured = await context.read(capture.table).findMany({
            where: {
                deploymentId: { in: live.map((entry) => entry.id) },
                targetSpaceId: { isNull: true },
            },
        });

        // read the objects each capture lets the installation use
        const wanted = (
            await Promise.all(
                captured.map((entry) =>
                    Binder.#kind(entry.target, this.#kinds).uses(entry, context),
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
            // relate it to the objects of each used type, as the installation's declarer
            const consumer = principal.installation.reference(target.scope, target.id);
            for (const { object, ids } of consumed) {
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
                `${target} is not an identifier of any bindable kind`,
            );
        }

        return kind;
    }

    /** Bind a declaration to an owned resource of the installation, reusing a retained one. */
    async #own(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        need: ResourceNeed,
    ): Promise<void> {
        // require a served kind of the declaration's resource
        const kind = this.#resources.get(need.kind);
        if (kind === undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `no machine here provides ${need.kind} resources`,
            );
        }

        // find the kind's resource of the name in the space
        const name = `${target.installation.name}.${need.name}`;
        const owner = principal.installation.reference(target.scope, target.id);
        const [named] = await context.read(kind.object.table).findMany({
            where: {
                scope: target.scope,
                name,
            },
        });

        // create the resource owned by the installation under the identifier the binding takes
        const owning = Binder.#declarer(context, target, kind.object);
        let ownedId: string;
        if (named === undefined) {
            ownedId = kind.object.generateId(context.environment);
            await context.mutate(owning, [
                kind.object.call("create", {
                    id: ownedId,
                    name,
                    definitionPackageId: need.package.id,
                    definitionVersion: need.package.version,
                    definitionName: need.name,
                    spec: need.spec,
                    ...ObjectReference.columns("owner", owner),
                }),
            ]);
        }
        // adopt a retained resource of the same package's declaration without another owner, and cancel its deletion
        else if (
            named.deletedAt !== null &&
            named.retention !== "delete" &&
            named.definitionPackageId === need.package.id &&
            named.definitionName === need.name &&
            (named.ownerId === null || Subject.same(ObjectReference.at("owner", named), owner))
        ) {
            ownedId = named.id;
            await context.mutate(owning, [kind.object.call("adopt", { id: named.id, owner })]);
            await this.#upgrade(context, target, kind.object, named, need);
        }
        // refuse a name another resource has
        else {
            throw StackError.of(
                "DEFINITION_INVALID",
                `resource ${name} already exists and is not a retained ${need.kind} that can be adopted`,
            );
        }

        // bind the declaration to it
        await context.mutate(context.self, [
            binding.call("create", {
                installationId: target.id,
                packageId: need.package.id,
                name: need.name,
                target: ownedId,
                state: need.state,
            }),
        ]);
    }

    /** Bind an installation's tables to its space's main database, following their state. */
    static async #bindMain(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        need: ResourceNeed,
        bound: Binding | undefined,
    ): Promise<void> {
        // bind the tables to the main database the space has from its birth
        if (bound === undefined) {
            const main = await Database.main(context, target.scope);
            await context.mutate(context.self, [
                binding.call("create", {
                    installationId: target.id,
                    packageId: need.package.id,
                    name: need.name,
                    target: main.id,
                    state: need.state,
                }),
            ]);
        }
        // follow the tables the installation's objects have now
        else if (canonicalize(bound.state) !== canonicalize(need.state)) {
            await context.mutate(context.self, [
                binding.call("update", { id: bound.id, state: need.state }),
            ]);
        }
    }

    /** Bind a declaration of another kind to the target its kind provides for it. */
    async #ownOther(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        need: ResourceNeed,
    ): Promise<void> {
        // require a kind binding the declaration's kind
        const kind = this.#served.get(need.kind);
        if (kind === undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `no machine here binds ${need.kind} declarations`,
            );
        }

        // create the target among the bindings so far and bind the declaration to it
        const bindings = await Binder.#bindings(context, target);
        const declarer = Binder.#declarer(context, target, kind.object);
        const provisioned = await kind.provision(context, declarer, target, need, bindings);
        if (provisioned !== undefined) {
            await context.mutate(context.self, [
                binding.call("create", {
                    installationId: target.id,
                    packageId: need.package.id,
                    name: need.name,
                    target: provisioned,
                    state: need.state,
                }),
            ]);
        }
    }

    /** Read who declares an installation's resource: itself for its kind, else its consenter. */
    static #declarer(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        object: ObjectType,
    ): AccessContext {
        const owner = principal.installation.reference(target.scope, target.id);
        const declarer =
            object.package.id === target.installation.packageId ? owner : Rollout.consenter(target);

        return AccessContext.of(Caller.acting(declarer, context.self.subject), context.now);
    }

    /** Read an installation's bindings. */
    static async #bindings(
        snapshot: Pick<Snapshot, "read">,
        target: Rollout & { readonly installation: NamedInstallation },
    ): Promise<Binding[]> {
        return snapshot
            .read(binding.table)
            .findMany({ where: { scope: target.scope, installationId: target.id } });
    }

    /** Update an owned resource and its binding to a declaration. */
    async #follow(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        bound: Binding,
        owned: Resource,
        need: ResourceNeed,
    ): Promise<void> {
        // update the binding's state and the resource's release and specification
        if (canonicalize(bound.state) !== canonicalize(need.state)) {
            await context.mutate(context.self, [
                binding.call("update", { id: bound.id, state: need.state }),
            ]);
        }
        await this.#upgrade(
            context,
            target,
            Binder.#kind(owned.id, this.#resources.values()).object,
            owned,
            need,
        );
    }

    /** Update an owned resource to its declaration's release and specification. */
    async #upgrade(
        context: MethodContext,
        target: Rollout & { readonly installation: NamedInstallation },
        object: ObjectType,
        owned: Resource,
        need: ResourceNeed,
    ): Promise<void> {
        const isChanged =
            owned.definitionVersion !== need.package.version ||
            canonicalize(owned.spec) !== canonicalize(need.spec);
        if (isChanged) {
            await context.mutate(Binder.#declarer(context, target, object), [
                object.call("update", {
                    id: owned.id,
                    definitionVersion: need.package.version,
                    spec: need.spec,
                }),
            ]);
        }
    }
}
