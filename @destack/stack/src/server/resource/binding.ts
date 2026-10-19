import { type space } from "@destack/account/object";
import { ObjectReference, Subject } from "@destack/sync";
import type { ResourceRow, MethodContext, ObjectType, ResourceType } from "@destack/object";
import { principal } from "@destack/access";
import type { Select, Snapshot } from "@destack/db";
import type { Stack } from "@destack/object/server";
import { PackageId } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import { RESOURCE_DECLARATION, ResourceDefinition, type ResourceKind } from "@destack/resource";
import { canonicalize, Digest, type Identifier, type JsonObject, schema } from "@destack/schema";
import { StackDefinition } from "../../declare/stack.ts";
import { RESOURCE_TARGET, type StackBinding } from "../../declare/binding.ts";
import { StackError } from "../../error/index.ts";
import {
    Binding,
    binding,
    capture,
    Database,
    MAIN_DATABASE,
    type Capture,
    Deployment,
    deployment,
    type Installation,
    installation,
} from "../../object/index.ts";

/** One declaration of an installation's package, bound by a stack. */
interface BindingDeclaration {
    /** The installation's declaration name. */
    readonly installation: string;
    /** The package declaring the need. */
    readonly packageId: PackageId;
    /** The package-local declaration. */
    readonly name: string;
    /** The binding the stack declares. */
    readonly binding: StackBinding;
}

/** A resource declaration of an installation's package, with the state it requires. */
export type ResourceNeed = ResourceDefinition["declaration"] & {
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
export interface ServedBindable<Object extends ObjectType = ObjectType> extends Bindable<Object> {
    /** The provider workloads read the bound objects through, such as vault. */
    readonly provider: string;
    /** The address workloads call the package serving the bound objects at. */
    readonly reference: string;
    /** Provision the target its installation owns for an unbound need. */
    readonly provision: (
        context: MethodContext,
        target: Installation,
        need: ResourceNeed,
        bindings: readonly Binding[],
    ) => Promise<string | undefined>;
    /** Make a bound target ready to capture, and report whether it is. */
    readonly ready: (
        context: MethodContext,
        bound: Binding,
        need: ResourceNeed,
    ) => Promise<boolean>;
}

/** The kinds of objects bindings target. */
export const Bindable = {
    /** Bind resources of one kind, captured at their generation or at a binding's pin. */
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

    /** Keep the resource kinds and the other bindable kinds a machine serves. */
    constructor(resources: readonly ResourceBindable[], served: readonly ServedBindable[]) {
        // index the kinds by resource kind and by declaration kind
        this.#kinds = [...resources, ...served];
        this.#resources = new Map(resources.map((kind) => [kind.kind.name, kind]));
        this.#served = new Map(served.map((kind) => [kind.declaration, kind]));
    }

    /** Report whether a bindable kind binds the declarations of a kind. */
    binds(declaration: string): boolean {
        return this.#kinds.some((kind) => kind.declaration === declaration);
    }

    /** Serve bindings: declare those of a stack's installations, and release a binding. */
    declare() {
        return binding.implement({
            methods: {
                release: async (call, next) => {
                    // disown the resource for deletion and delete the binding
                    const resource = Binder.#kind(
                        call.target.target,
                        this.#resources.values(),
                    ).object;
                    await call.mutate(call.self, [
                        resource.call("adopt", { id: call.target.target, owner: null }),
                        resource.call("delete", { id: call.target.target }),
                    ]);

                    return next();
                },
            },
            declaration: {
                after: "every",
                keys: ["installations"],
                collect: (document) => flatten(document),
                resolve: async (_name, declared: BindingDeclaration, context) => {
                    // resolve the target and release the installation's own binding it replaces
                    const resolved = await resolveTarget(declared.binding, context);
                    const installationId = schema
                        .identifier("installation")
                        .parse(await context.require(installation, declared.installation));
                    await this.#releaseReplaced(context, declared, installationId);

                    return {
                        installationId,
                        packageId: declared.packageId,
                        name: declared.name,
                        ...resolved,
                        version: declared.binding.version ?? null,
                        state: declared.binding.state,
                    };
                },
                values: (_name, resolved) => resolved,
            },
        });
    }

    /** Release the installation's own binding that a stack's binding replaces. */
    async #releaseReplaced(
        context: Stack,
        declared: BindingDeclaration,
        installationId: Identifier<"installation">,
    ): Promise<void> {
        // read the installation's own binding of the declaration
        const snapshot = context.transaction.view();
        const [current] = await snapshot.read(binding.table).findMany({
            where: {
                installationId,
                packageId: declared.packageId,
                name: declared.name,
            },
        });

        // release it when it claims a resource
        if (
            current !== undefined &&
            (await this.owned(snapshot, current, installationId)) !== undefined
        ) {
            await context.mutate([binding.call("release", { id: current.id })]);
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
                    throw new StackError(
                        "INVALID_DEFINITION",
                        `binding ${entry.name} targets no ${kind.object.name} ${entry.target}`,
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

    /** Bind each declaration no stack binding satisfies to a target the installation owns. */
    async attach(
        context: MethodContext,
        target: Installation,
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

        // create an owned resource for each unbound declaration and keep owned ones following their declaration
        const resources = needs.filter((need) => this.#resources.has(need.kind));
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
            // keep an owned resource following its declaration
            else if (existing.owned !== undefined) {
                await this.#follow(context, existing.binding, existing.owned, need);
            }
        }

        // create the other owned targets of unbound declarations
        const others = needs.filter((need) => !this.#resources.has(need.kind));
        for (const need of others.filter(
            (each) => !bound.has(Binding.key(each.package.id, each.name)),
        )) {
            await this.#ownOther(context, target, need);
        }

        // release the owned bindings no declaration needs, and delete a main binding no table needs
        const needed = new Set(resources.map((need) => Binding.key(need.package.id, need.name)));
        for (const entry of bindings) {
            const isNeeded = needed.has(Binding.key(entry.binding.packageId, entry.binding.name));
            // release an owned resource
            if (entry.owned !== undefined && !isNeeded) {
                await context.mutate(context.self, [
                    binding.call("release", { id: entry.binding.id }),
                ]);
            }
            // delete the binding to the main database
            else if (entry.binding.name === MAIN_DATABASE && !isNeeded) {
                await context.mutate(context.self, [
                    binding.call("delete", { id: entry.binding.id }),
                ]);
            }
        }
    }

    /** Make every bound target of an installation's needs ready to capture where it can. */
    async ready(
        context: MethodContext,
        target: Installation,
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

            // ready a target its served kind prepares once its row arrived
            const served = this.#served.get(need.kind);
            if (served !== undefined) {
                const row = await context
                    .read(served.object.table)
                    .findFirst({ where: { id: bound.target } });
                isReady =
                    row !== undefined && (await served.ready(context, bound, need)) && isReady;
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

    /** Read the resource a binding targets when its installation owns it, else absent. */
    async owned(
        snapshot: Pick<Snapshot, "read">,
        bound: Binding,
        installationId: Identifier<"installation">,
    ): Promise<ResourceRow | undefined> {
        // look only at resource targets
        const kind = [...this.#resources.values()].find((each) =>
            each.object.identifies(bound.target),
        );
        if (kind === undefined) {
            return undefined;
        }

        // read the target when the installation owns it
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
        scope: Identifier<"space">,
        installationId: Identifier<"installation">,
        related: Digest | null,
    ): Promise<Digest> {
        // read the captures of the installation's live deployments of targets in its own space
        const live = await context.read(deployment.table).findMany({
            where: {
                ...Deployment.live(),
                scope,
                installationId,
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

        // list exactly those objects of each used type the space serves, as only its installed packages keep objects
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
        );
        if (digest !== related) {
            const consumer = principal.installation.reference(scope, installationId);
            await context.mutate(
                context.self,
                consumed.map(({ object, ids }) => object.call("bind", { consumer, ids })),
            );
        }

        return digest;
    }

    /** Find the kind whose objects have a target's identifier, refusing a target of no kind. */
    static #kind<Kind extends Bindable>(target: string, kinds: Iterable<Kind>): Kind {
        const kind = [...kinds].find((each) => each.object.identifies(target));
        if (kind === undefined) {
            throw new StackError("INVALID_DEFINITION", `no bindable kind has ${target}`);
        }

        return kind;
    }

    /** Bind a declaration to a resource the installation owns, reusing a retained one. */
    async #own(context: MethodContext, target: Installation, need: ResourceNeed): Promise<void> {
        // require a served kind of the declaration's resource
        const kind = this.#resources.get(need.kind);
        if (kind === undefined) {
            throw new StackError(
                "INVALID_DEFINITION",
                `no machine here serves ${need.kind} resources`,
            );
        }

        // find the kind's resource of the name in the space
        const name = `${target.name}.${need.name}`;
        const owner = principal.installation.reference(target.scope, target.id);
        const [named] = await context.read(kind.object.table).findMany({
            where: {
                scope: target.scope,
                name,
            },
        });

        // create the resource owned by the installation under the identifier the binding takes
        let ownedId: string;
        if (named === undefined) {
            ownedId = kind.object.generateId();
            await context.mutate(context.self, [
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
        // adopt a retained resource of the same package's declaration no other installation owns and cancel its deletion
        else if (
            named.deletedAt !== null &&
            named.retention !== "delete" &&
            named.definitionPackageId === need.package.id &&
            named.definitionName === need.name &&
            (named.ownerId === null || Subject.same(ObjectReference.at("owner", named), owner))
        ) {
            ownedId = named.id;
            await context.mutate(context.self, [
                kind.object.call("adopt", { id: named.id, owner }),
            ]);
            await this.#upgrade(context, kind.object, named, need);
        }
        // refuse a name another resource has
        else {
            throw new StackError(
                "INVALID_DEFINITION",
                `resource ${name} exists and is not a retained ${need.kind} to own again`,
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
        target: Installation,
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

    /** Bind a declaration of another kind to the target its kind owns for it. */
    async #ownOther(
        context: MethodContext,
        target: Installation,
        need: ResourceNeed,
    ): Promise<void> {
        // require a kind binding the declaration's kind
        const kind = this.#served.get(need.kind);
        if (kind === undefined) {
            throw new StackError(
                "INVALID_DEFINITION",
                `no machine here binds ${need.kind} declarations`,
            );
        }

        // create the target among the bindings so far and bind the declaration to it
        const bindings = await Binder.#bindings(context, target);
        const provisioned = await kind.provision(context, target, need, bindings);
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

    /** Read an installation's bindings. */
    static async #bindings(
        snapshot: Pick<Snapshot, "read">,
        target: Installation,
    ): Promise<Binding[]> {
        return snapshot
            .read(binding.table)
            .findMany({ where: { scope: target.scope, installationId: target.id } });
    }

    /** Update an owned resource and its binding to a declaration. */
    async #follow(
        context: MethodContext,
        bound: Binding,
        owned: ResourceRow,
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
            Binder.#kind(owned.id, this.#resources.values()).object,
            owned,
            need,
        );
    }

    /** Update an owned resource to its declaration's release and specification. */
    async #upgrade(
        context: MethodContext,
        object: ObjectType,
        owned: ResourceRow,
        need: ResourceNeed,
    ): Promise<void> {
        const isChanged =
            owned.definitionVersion !== need.package.version ||
            canonicalize(owned.spec) !== canonicalize(need.spec);
        if (isChanged) {
            await context.mutate(context.self, [
                object.call("update", {
                    id: owned.id,
                    definitionVersion: need.package.version,
                    spec: need.spec,
                }),
            ]);
        }
    }
}

/** Flatten each installation's bindings in a stack document by package and declaration name. */
function flatten(document: JsonObject): Record<string, BindingDeclaration> {
    // key each binding by its installation, package and declaration
    const declared = StackDefinition.collections(document).installations;
    const bindings: Record<string, BindingDeclaration> = {};
    for (const [name, entry] of Object.entries(declared)) {
        for (const [packageId, names] of Object.entries(entry.bindings)) {
            for (const [declaration, declaredBinding] of Object.entries(names)) {
                bindings[`installations/${name}/${packageId}/${declaration}`] = {
                    installation: name,
                    packageId: PackageId.parse(packageId),
                    name: declaration,
                    binding: declaredBinding,
                };
            }
        }
    }

    return bindings;
}

/** Read the resource kind a stack declares under a key, refusing a key without a resource. */
function declaredKind(context: Stack, key: string): string {
    // read the resource declared under the key
    const { resources } = StackDefinition.collections(context.document);
    const declared = Object.hasOwn(resources, key) ? resources[key] : undefined;
    if (declared === undefined) {
        throw new StackError("INVALID_DEFINITION", `the stack declares no resource ${key}`);
    }

    return declared.declaration.kind;
}

/** Resolve a binding's target. */
async function resolveTarget(
    declared: StackBinding,
    context: Stack,
): Promise<{ readonly target: string; readonly targetSpaceId: Identifier<"space"> | null }> {
    // take an installation of another space by its name as it stands there, unpinned
    const { target } = declared;
    if ("spaceId" in target) {
        if (target.spaceId === context.scope || declared.version !== undefined) {
            throw new StackError(
                "INVALID_DEFINITION",
                `a binding selects another space's installation by name, unpinned, not ${target.name} in ${target.spaceId}`,
            );
        }

        return { target: target.name, targetSpaceId: target.spaceId };
    }
    // resolve a target the stack declares by its key, a resource as the kind its declaration names
    else if ("name" in target) {
        const type =
            target.type === RESOURCE_TARGET ? declaredKind(context, target.name) : target.type;

        return { target: await context.require(type, target.name), targetSpaceId: null };
    }
    // require an existing target within the destination space
    else {
        const object = context.object(target.type);
        const [existing] = await context.transaction.read(object.table).findMany({
            where: {
                id: target.id,
                scope: context.scope,
            },
        });
        if (existing === undefined) {
            throw new StackError(
                "INVALID_DEFINITION",
                `binding selects no ${target.type} ${target.id} in the destination space`,
            );
        }

        return { target: target.id, targetSpaceId: null };
    }
}
