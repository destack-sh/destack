import { type ObjectReference } from "@destack/sync";
import { principal, Relationship } from "@destack/access";
import { and, eq, inArray, type DatabaseConnection, type Select, type Table } from "@destack/db";
import type { Call, ObjectType } from "@destack/object";
import { PackageId } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import { identifier, type Identifier, schema } from "@destack/schema";
import { canonicalize } from "@destack/schema/json";
import { SpaceInstallation } from "../declare/installation.ts";
import type { SpaceBinding, SpaceResource } from "../declare/resource.ts";
import { SpaceError } from "../error/index.ts";
import * as base from "../object/index.ts";
import {
    type Binding,
    capture,
    type Capture,
    Deployment,
    deployment,
    type Installation,
    installation,
    type Resource,
    resource,
} from "../object/index.ts";

/** The relation through which installations read what their live deployments captured. */
const READER = "reader";

/** One declaration of an installation's package, bound by a stack. */
interface BindingDeclaration {
    /** The installation's declaration name. */
    readonly installation: string;
    /** The package declaring the need. */
    readonly packageId: PackageId;
    /** The package-local declaration. */
    readonly name: string;
    /** The binding the stack declares. */
    readonly binding: SpaceBinding;
}

/** A resource declaration of an installation's package, with the state it requires of its resource. */
export type ResourceNeed = SpaceResource["declaration"] & {
    /** The state the declaration requires, such as a database's tables. */
    readonly state: ResourceState;
};

/** A kind of object a binding targets, and the version a deployment captures of it. */
export interface Bindable {
    /** The bindable object type. */
    readonly object: ObjectType;
    /** The object types captures of this kind let installations read. */
    readonly readable: readonly ObjectType[];
    /** Read the version a deployment captures of a target: a binding's pin, or the target's current version or generation. */
    version(target: Readonly<Record<string, unknown>>, pin: number | null): number;
    /** Read the objects a capture lets its deployment's installation read. */
    reads(
        captured: Pick<Capture, "scope" | "target" | "version">,
        database: DatabaseConnection,
    ): Promise<readonly ObjectReference[]>;
}

/** Resources that deployments capture at their generation or at a binding's pinned one. */
export const resourceBindable: Bindable = {
    object: resource,
    readable: [resource],
    version: (target, pin) => pin ?? (target as Select<typeof resource.table>).generation,
    reads: async (captured) => [resource.reference(captured.scope, captured.target)],
};

/** The bindings of the installations a stack declares. */
export const binding = base.binding.declare({
    after: "every",
    keys: ["installations"],
    collect: (document) => {
        // flatten each installation's bindings by package and declaration name
        const declared = schema
            .record(schema.string(), SpaceInstallation)
            .parse(document.installations ?? {});
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
    },
    resolve: async (_name, declared: BindingDeclaration, context) => {
        // resolve a target the stack declares by its key
        const { target } = declared.binding;
        let targetId: string;
        if ("name" in target) {
            targetId = await context.require(target.type, target.name);
        }
        // require an existing target within the destination space
        else {
            const object = context.object(target.type);
            const table = object.table as Table & typeof resource.table;
            const [existing] = await context.database
                .select({ id: table.id })
                .from(table)
                .where(
                    and(eq(table.id, target.id as never), eq(table.scope, context.scope as never)),
                );
            if (existing === undefined) {
                throw new SpaceError(
                    "INVALID_DEFINITION",
                    `binding selects no ${target.type} ${target.id} in the destination space`,
                );
            }
            targetId = target.id;
        }

        // release the installation's own binding that the stack's binding replaces
        const installationId = identifier("installation").parse(
            await context.require(installation, declared.installation),
        );
        const [owned] = await context.database
            .select({ binding: base.binding.table })
            .from(base.binding.table)
            .innerJoin(
                resource.table,
                and(
                    eq(resource.table.id, base.binding.table.target),
                    eq(resource.table.ownerInstallationId, installationId),
                ),
            )
            .where(
                and(
                    eq(base.binding.table.installationId, installationId),
                    eq(base.binding.table.packageId, declared.packageId),
                    eq(base.binding.table.name, declared.name),
                ),
            );
        if (owned !== undefined) {
            await Binder.release(context, owned.binding);
        }

        return {
            installationId,
            packageId: declared.packageId,
            name: declared.name,
            target: targetId,
            version: declared.binding.version ?? null,
            state: declared.binding.state,
        };
    },
    values: (_name, resolved) => resolved,
});

/** Bind declarations to objects of each bindable kind, and capture what deployments run with. */
export class Binder {
    /** The bindable kinds, by the identifier prefix their objects take. */
    readonly #kinds: ReadonlyMap<string, Bindable>;

    /** Keep the bindable kinds a host serves. */
    constructor(kinds: readonly Bindable[]) {
        this.#kinds = new Map(kinds.map((kind) => [kind.object.identity, kind]));
    }

    /** Capture what a new deployment runs with: each binding at a version, with the states its build requires. */
    async capture(
        call: Pick<Call, "database" | "invoke">,
        created: Select<typeof deployment.table>,
        required: ReadonlyMap<string, ResourceState> = new Map(),
    ): Promise<Select<typeof capture.table>[]> {
        // read the installation's bindings, grouped by the kind of their targets
        const { database } = call;
        const bindings = await database
            .select()
            .from(base.binding.table)
            .where(
                and(
                    eq(base.binding.table.scope, created.scope),
                    eq(base.binding.table.installationId, created.installationId),
                ),
            );
        const byKind = Map.groupBy(bindings, (entry) => this.#kind(entry.target));

        // resolve each binding to the version of its target it runs with
        const captured: Select<typeof capture.table>[] = [];
        for (const [kind, entries] of byKind) {
            const table = kind.object.table as Table & typeof resource.table;
            const targets = await database
                .select()
                .from(table)
                .where(inArray(table.id, entries.map((entry) => entry.target) as never));
            const byId = new Map(targets.map((row) => [row.id as string, row]));
            for (const entry of entries) {
                const target = byId.get(entry.target);
                if (target === undefined) {
                    throw new SpaceError(
                        "INVALID_DEFINITION",
                        `binding ${entry.name} targets no ${kind.object.name} ${entry.target}`,
                    );
                }
                captured.push(
                    (await call.invoke(capture, "create", {
                        deploymentId: created.id,
                        packageId: entry.packageId,
                        name: entry.name,
                        target: entry.target,
                        version: kind.version(target, entry.version),
                        state: required.get(`${entry.packageId} ${entry.name}`) ?? entry.state,
                    })) as Select<typeof capture.table>,
                );
            }
        }

        return captured;
    }

    /** Bind each declaration no stack binding satisfies to a resource the installation owns, and release the owned ones no declaration needs. */
    async attach(
        call: Pick<Call, "database" | "invoke">,
        target: Installation,
        needs: readonly ResourceNeed[],
    ): Promise<void> {
        // read the installation's bindings by declaration with their owned resources
        const bindings = await call.database
            .select({ binding: base.binding.table, owned: resource.table })
            .from(base.binding.table)
            .leftJoin(
                resource.table,
                and(
                    eq(resource.table.id, base.binding.table.target),
                    eq(resource.table.ownerInstallationId, target.id),
                ),
            )
            .where(
                and(
                    eq(base.binding.table.scope, target.scope),
                    eq(base.binding.table.installationId, target.id),
                ),
            );
        const key = (packageId: string, name: string) => `${packageId} ${name}`;
        const bound = new Map(
            bindings.map((entry) => [key(entry.binding.packageId, entry.binding.name), entry]),
        );

        // own a resource for each declaration no binding satisfies, and keep the owned ones following their declaration
        for (const need of needs) {
            const existing = bound.get(key(need.package.id, need.name));
            if (existing === undefined) {
                await own(call, target, need);
            } else if (existing.owned !== null) {
                await follow(call, existing.binding, existing.owned, need);
            }
        }

        // release the owned bindings no declaration needs
        const needed = new Set(needs.map((need) => key(need.package.id, need.name)));
        for (const entry of bindings) {
            const isNeeded = needed.has(key(entry.binding.packageId, entry.binding.name));
            if (entry.owned !== null && !isNeeded) {
                await Binder.release(call, entry.binding);
            }
        }
    }

    /** Release a binding to a resource its installation owns and request the resource's deletion. */
    static async release(call: Pick<Call, "invoke">, owned: Binding): Promise<void> {
        await call.invoke(base.binding, "delete", { id: owned.id });
        await call.invoke(resource, "delete", { id: owned.target });
    }

    /** Relate an installation as reader of exactly the objects its live deployments captured. */
    async relate(
        database: DatabaseConnection,
        scope: Identifier<"space">,
        installationId: Identifier<"installation">,
        now: number,
    ): Promise<void> {
        // read the captures of the installation's live deployments
        const captured = await database
            .select({
                scope: capture.table.scope,
                target: capture.table.target,
                version: capture.table.version,
            })
            .from(capture.table)
            .innerJoin(deployment.table, eq(deployment.table.id, capture.table.deploymentId))
            .where(
                and(
                    eq(deployment.table.scope, scope),
                    eq(deployment.table.installationId, installationId),
                    Deployment.live(),
                ),
            );

        // read the objects each capture lets the installation read
        const wanted: ObjectReference[] = [];
        for (const entry of captured) {
            wanted.push(...(await this.#kind(entry.target).reads(entry, database)));
        }

        // relate the installation to exactly those objects of every readable type
        const readable = [...this.#kinds.values()].flatMap((kind) => kind.readable);
        await Relationship.replace(
            database,
            {
                scope,
                objects: readable.map((object) => ({
                    packageId: object.policy.definition.packageId,
                    type: object.name,
                })),
                relation: READER,
                subject: principal.installation.reference(scope, installationId),
            },
            wanted,
            now,
        );
    }

    /** Find the kind of a target by its identifier's prefix and refuse unknown kinds. */
    #kind(target: string): Bindable {
        const kind = [...this.#kinds].find(([prefix]) => target.startsWith(`${prefix}-`))?.[1];
        if (kind === undefined) {
            throw new SpaceError("INVALID_DEFINITION", `no bindable kind has ${target}`);
        }

        return kind;
    }
}

/** Bind a declaration to a resource the installation owns: the retained one of its name declared again, or a new one. */
async function own(
    call: Pick<Call, "database" | "invoke">,
    target: Installation,
    need: ResourceNeed,
): Promise<void> {
    // find a resource under the declaration's name
    const name = `${target.alias}.${need.name}`;
    const [named] = await call.database
        .select()
        .from(resource.table)
        .where(and(eq(resource.table.scope, target.scope), eq(resource.table.name, name)));

    // create the resource, owned by the installation
    let owned: Resource;
    if (named === undefined) {
        owned = (await call.invoke(resource, "create", {
            name,
            kind: need.kind,
            definitionPackageId: need.package.id,
            definitionVersion: need.package.version,
            definitionName: need.name,
            spec: need.spec,
            ownerInstallationId: target.id,
        })) as Resource;
    }
    // adopt a retained resource of the same package's declaration no other installation owns, cancelling its deletion
    else if (
        named.deletionRequestedAt !== null &&
        named.retention === "retain" &&
        named.kind === need.kind &&
        named.definitionPackageId === need.package.id &&
        named.definitionName === need.name &&
        (named.ownerInstallationId === null || named.ownerInstallationId === target.id)
    ) {
        owned = (await call.invoke(resource, "own", {
            id: named.id,
            ownerInstallationId: target.id,
        })) as Resource;
        await upgrade(call, owned, need);
    }
    // refuse a name another resource has
    else {
        throw new SpaceError(
            "INVALID_DEFINITION",
            `resource ${name} exists and is not a retained ${need.kind} to own again`,
        );
    }

    // bind the declaration to it
    await call.invoke(base.binding, "create", {
        installationId: target.id,
        packageId: need.package.id,
        name: need.name,
        target: owned.id,
        state: need.state,
    });
}

/** Update a binding to an owned resource, and the resource, to a declaration's current state, release and specification. */
async function follow(
    call: Pick<Call, "invoke">,
    bound: Binding,
    owned: Resource,
    need: ResourceNeed,
): Promise<void> {
    if (canonicalize(bound.state) !== canonicalize(need.state)) {
        await call.invoke(base.binding, "update", { id: bound.id, state: need.state });
    }
    await upgrade(call, owned, need);
}

/** Update an owned resource to its declaration's release and specification. */
async function upgrade(
    call: Pick<Call, "invoke">,
    owned: Resource,
    need: ResourceNeed,
): Promise<void> {
    const isChanged =
        owned.definitionVersion !== need.package.version ||
        canonicalize(owned.spec) !== canonicalize(need.spec);
    if (isChanged) {
        await call.invoke(resource, "update", {
            id: owned.id,
            definitionVersion: need.package.version,
            spec: need.spec,
        });
    }
}
