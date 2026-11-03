import { PackageManifest } from "@destack/package/manifest";
import { graph, Package } from "@destack/package";
import type { EventStore } from "@destack/event";
import { type Authorizer, decisionTables, Policy, type TableMapping } from "@destack/access";
import { ObjectTypeReference, type Subject } from "@destack/sync";
import { type BlobStore, type DatabaseConnection, TABLE, type Table } from "@destack/db";
import type { Identifier } from "@destack/schema";
import { PackageContext, ServerContext } from "../workload/workload.ts";
import { PACKAGE_DECLARATION, PackageDescription } from "../declare/description.ts";
import { PackageHandle } from "../declare/package.ts";
import type { Follow } from "../replica/shape.ts";
import { branchType, type BranchType } from "../branch/index.ts";
import { ObjectType } from "../object/object.ts";
import { ObjectPolicy } from "../object/access.ts";
import { ObjectTypePool, ObjectDescription, type KnownDeclaration } from "../object/description.ts";
import type { ClaimPrimary } from "../claim/reservation.ts";
import type { EphemeralStorage } from "./ephemeral.ts";
import type { ExternalStorage } from "./external.ts";
import { Provisioned } from "../trait/provisioned.ts";
import { ResourceController, type ResourceControllerOptions } from "./provisioned.ts";
import { TrackedHistory } from "./tracked.ts";
import type { HookSandbox } from "../hook/sandbox.ts";
import type {
    PackageImplementation,
    ObjectPackage,
    ImplementationInput,
} from "./implementation.ts";
import { installation } from "../installation/installation.ts";
import { mapInstalledPackages } from "../installation/package.ts";

/** A policy a store authorizes beside its objects: a policy, a type, or a table mapping. */
type CatalogPolicy = Policy | ObjectType | TableMapping;

/** A served package: its handle, its code, its types and the installation serving it. */
type Served = Catalog["served"][number];

/** A served type's installation and the package whose code implements it. */
export interface ServedType {
    /** The installation serving the type. */
    readonly server: ServerContext;
    /** The package implementing the type. */
    readonly package: Package;
}

/** What a catalog compiles from: a store's servers and sandboxes over its database. */
export interface CatalogOptions {
    /** The package servers. */
    readonly servers: readonly PackageImplementation[];
    /** The sandboxes of the installations whose packages run here without a workload. */
    readonly sandboxes?: readonly HookSandbox[];
    /** The installation the store serves as, absent for a store running as its principal. */
    readonly server?: ServerContext;
    /** The principal of the store's work. */
    readonly principal: Subject;
    /** The partition the store's database is home to, which its policies decide below. */
    readonly partition?: string;
    /** Further policies and table mappings to authorize. */
    readonly policies?: readonly CatalogPolicy[];
    /** The database with the objects. */
    readonly database: DatabaseConnection;
    /** The partition's blobs, which package code reads through its context. */
    readonly blobs: BlobStore;
    /** The machine running the servers' provisioned resources, which providers require. */
    readonly machine?: Identifier<"machine">;
    /** The claim type recording the names of unique indexes, and the store serving it. */
    readonly claims?: ClaimPrimary;
    /** The memory store with the served ephemeral objects. */
    readonly ephemeral?: EphemeralStorage;
    /** The event log of the store's database, with its calls and served events types. */
    readonly events?: EventStore;
    /** Report a sandbox whose types refer to ones the store does not serve. */
    readonly report: (error: unknown) => void;
}

/** The packages a store serves, with their types, routes, tables and access rules. */
export class Catalog {
    /** The package servers it compiled from. */
    readonly servers: readonly PackageImplementation[];
    /** The sandboxes it compiled from. */
    readonly sandboxes: readonly HookSandbox[];
    /** The served packages, each with its code, its types and the installation serving it. */
    readonly served: readonly {
        /** The package. */
        readonly package: ObjectPackage;
        /** The package's code. */
        readonly definition: ImplementationInput;
        /** The package's types, its handled ones in place of their declared defaults. */
        readonly objects: readonly ObjectType[];
        /** The installation serving the package, absent for the platform's store. */
        readonly server: ServerContext | undefined;
        /** The server or sandbox the package was read from, which a later compile reuses it for. */
        readonly source: PackageImplementation | HookSandbox;
    }[];
    /** The types the packages declare, without the chunk types serving their text. */
    readonly declared: readonly ObjectType[];
    /** The object types served. */
    readonly objects: readonly ObjectType[];
    /** The served durable object types. */
    readonly durable: readonly ObjectType[];
    /** The types whose queries run over rows replicated here, never followed nor changed here. */
    readonly replicas: readonly ObjectType[];
    /** The policies authorized beside the objects: the given, the referenced and the followed. */
    readonly policies: readonly CatalogPolicy[];
    /** The installations serving objects and their implementing packages, by type key. */
    readonly installed: ReadonlyMap<string, ServedType>;
    /** The shapes the served code reads replicated, with the keys it follows them at. */
    readonly follows: readonly Follow[];
    /** The on-demand follows of the rows standing for callers. */
    readonly standing: readonly Extract<Follow, { readonly onDemand: unknown }>[];
    /** The files with the served external objects. */
    readonly external: readonly ExternalStorage[];
    /** The replicated scope tables the served types live in or relate to, which relist chains. */
    readonly chained: readonly Table[];
    /** The evaluator of the served objects' policies. */
    readonly authorizer: Authorizer;
    /** The durable types the store serves or decides, whose rows stand for their principals. */
    readonly decided: readonly ObjectType[];
    /** The served or authorized durable types whose rows the database replicates from home. */
    readonly replicated: readonly ObjectType[];
    /** The types follower replicas compile against: the followed shapes' and the decided. */
    readonly followed: readonly ObjectType[];
    /** The tables the store's feed follows: the decided types' and the decisions'. */
    readonly tables: readonly Table[];
    /** The served kinds' resources, each controlled by the loop its trait builds. */
    readonly provisioned: readonly ObjectType[];
    /** The providers reconciling the served kinds' resources. */
    readonly providers: ResourceControllerOptions["providers"];
    /** The declarations installed object types resolve against: the types, policies and kinds. */
    readonly known: readonly KnownDeclaration[];
    /** The scope's branch types, absent for a store without branches. */
    readonly branch: BranchType | undefined;

    /** Hold a compiled catalog. */
    private constructor(
        fields: Omit<{ readonly [Key in keyof Catalog]: Catalog[Key] }, "isServing">,
    ) {
        // keep each compiled field
        this.servers = fields.servers;
        this.sandboxes = fields.sandboxes;
        this.served = fields.served;
        this.declared = fields.declared;
        this.objects = fields.objects;
        this.durable = fields.durable;
        this.replicas = fields.replicas;
        this.policies = fields.policies;
        this.installed = fields.installed;
        this.follows = fields.follows;
        this.standing = fields.standing;
        this.external = fields.external;
        this.chained = fields.chained;
        this.authorizer = fields.authorizer;
        this.decided = fields.decided;
        this.replicated = fields.replicated;
        this.followed = fields.followed;
        this.tables = fields.tables;
        this.provisioned = fields.provisioned;
        this.providers = fields.providers;
        this.known = fields.known;
        this.branch = fields.branch;
    }

    /** Whether the store serves methods, which its journal records. */
    get isServing(): boolean {
        return this.declared.length > 0;
    }

    /** Compile a store's servers and sandboxes into its types, routes, tables and access. */
    static compile(given: CatalogOptions, previous?: Catalog): Catalog {
        // read each server's package, reusing a previous catalog's read of the same server
        const { servers, sandboxes = [], machine, server, database } = given;
        const reused = (source: PackageImplementation | HookSandbox) =>
            previous?.served.filter((entry) => entry.source === source) ?? [];
        const implementations = servers.map(
            (each) =>
                reused(each)[0] ??
                Catalog.#packageOf(each, each.installation ?? server, given.blobs),
        );

        // require the machine running any provisioned resources
        const providers = implementations.flatMap((each) => each.definition.providers ?? []);
        if (providers.length > 0 && machine === undefined) {
            throw new TypeError("provisioned resources need a machine");
        }

        // serve the sandboxed installations' packages, declared against what the servers make known
        const givenPolicies = given.policies ?? [];
        const implemented = implementations.flatMap((each) => each.objects);
        const implementedFollows = implementations
            .flatMap((each) => Catalog.#follows(each.definition))
            .flatMap((each) => each.shape.objects);
        const known = ObjectType.known(implemented, [
            ...givenPolicies,
            ...ObjectType.referenced(implemented),
            ...implementedFollows,
        ]);
        const served: Served[] = [
            ...implementations,
            ...sandboxes.flatMap((sandbox) => {
                // reuse a previous read, else leave a sandbox referring to types this store does not serve yet unserved, reporting it
                const earlier = reused(sandbox);
                if (earlier.length > 0) {
                    return earlier;
                }
                try {
                    return Catalog.#sandboxPackages(sandbox, known);
                } catch (error) {
                    given.report(
                        new Error(
                            `installation ${sandbox.server.installation} serves nothing: its types refer to ones this start does not serve`,
                            { cause: error },
                        ),
                    );

                    return [];
                }
            }),
        ];

        // route every package's types, refusing a package served twice
        const routes = Catalog.#routes(served);
        const servedIds = new Set<string>();
        for (const each of served) {
            if (servedIds.has(each.package.id)) {
                throw new TypeError(`package ${each.package.name} is served twice`);
            }
            servedIds.add(each.package.id);
        }

        // answer queries of replicated types nothing here serves, and list the follows and files
        const replicas = known
            .filter((type) => type instanceof ObjectType)
            .filter(
                (type) =>
                    !routes.has(ObjectTypeReference.key(type.typeReference)) &&
                    database.replicates(type.table),
            );
        const declared = [...routes.values()].map((route) => route.type);
        const follows = served.flatMap((each) => Catalog.#follows(each.definition));
        const external = served.flatMap((each) =>
            each.definition.external === undefined ? [] : [each.definition.external],
        );

        // serve each type at its package's installation, else at the store's, refusing one served at none
        const installed = new Map(
            [...routes].map(([key, { type, entry }]) => {
                const context = entry.server ?? server;
                if (context === undefined) {
                    throw new TypeError(`${type.name} is served at no installation`);
                }

                return [key, { server: context, package: Package.of(entry.package) }] as const;
            }),
        );

        // add the referenced replicated types, then the followed shapes' types, to the policies
        const referenced = ObjectType.referenced(declared).filter((type) =>
            database.replicates(type.table),
        );
        const referencing = Catalog.#policies(declared, givenPolicies, referenced);
        const chained = Catalog.#chainedTables(
            database,
            declared,
            [...declared, ...referencing].filter((each) => each instanceof ObjectType),
        );
        const shapes = [...new Set(follows.flatMap((each) => each.shape.objects))];
        const policies = Catalog.#policies(declared, referencing, shapes);

        // require each object type once apart from the shared procedures, and the stores its storage needs
        const objects = ObjectType.served(declared);
        for (const type of declared) {
            type.requireValid();
        }
        Catalog.#requireRoutes(declared);
        Catalog.#requireStores(objects, { ...given, external });

        // authorize the objects and the replicas the policies do not already decide
        const policed = new Set(policies.map((each) => Catalog.#policyKey(each)));
        const unpoliced = replicas.filter((type) => !policed.has(Catalog.#policyKey(type)));
        const authorizer = Catalog.#authorizer([...objects, ...unpoliced], {
            database,
            policies,
            installed,
            ...(given.partition === undefined ? {} : { partition: given.partition }),
        });
        TrackedHistory.require(objects, authorizer);

        // follow the durable tables and the decision tables, the replicated ones no installation here serves
        const decided = Catalog.#durable(objects, policies);
        const replicated = decided.filter(
            (object) =>
                database.replicates(object.table) &&
                !installed.has(ObjectTypeReference.key(object.typeReference)),
        );
        const tables = [
            ...new Set([...decided.flatMap((object) => object.tables), ...decisionTables]),
        ];

        return new Catalog({
            servers,
            sandboxes,
            served,
            declared,
            objects,
            durable: objects.filter((object) => object.storage === "durable"),
            replicas,
            policies,
            installed,
            follows,
            standing: served.flatMap((each) => each.definition.standing ?? []),
            external,
            chained,
            authorizer,
            decided,
            replicated,
            followed: [...shapes, ...decided],
            tables,
            provisioned: Catalog.#control(
                objects,
                providers.length === 0 || machine === undefined
                    ? undefined
                    : { providers, machine },
            ),
            providers,
            known: ObjectType.known(declared, policies),
            branch: objects.some((type) => type.same(branchType.object)) ? branchType : undefined,
        });
    }

    /** List a store's policies with the types whose policy it maps no other way. */
    static #policies(
        objects: readonly ObjectType[],
        policies: readonly CatalogPolicy[],
        types: readonly ObjectType[],
    ): CatalogPolicy[] {
        // add each type once, unless an object or policy already maps its policy
        const mapped = new Set([...objects, ...policies].map((each) => Catalog.#policyKey(each)));
        const added = types.filter((type) => {
            // add a type whose policy nothing maps yet, once
            const key = Catalog.#policyKey(type);
            const isAdded = !mapped.has(key);
            mapped.add(key);

            return isAdded;
        });

        return [...policies, ...added];
    }

    /** List the durable object types a store serves or decides. */
    static #durable(
        objects: readonly ObjectType[],
        policies: readonly (Policy | ObjectType | TableMapping)[],
    ): ObjectType[] {
        // list each durable type once, though several extensions name it
        const durable = [...objects, ...policies.filter((other) => other instanceof ObjectType)];

        return durable.filter(
            (object, index) =>
                object.storage === "durable" &&
                durable.findIndex((each) => each.same(object)) === index,
        );
    }

    /** Control each served kind's resources that a provider here provides. */
    static #control(
        objects: readonly ObjectType[],
        provisioned: ResourceControllerOptions | undefined,
    ): ObjectType[] {
        return provisioned === undefined
            ? []
            : objects
                  .filter((object) => Provisioned.is(object))
                  .filter((object) =>
                      provisioned.providers.some((provider) => provider.object.same(object)),
                  )
                  .map((object) =>
                      object.implement({
                          controller: new ResourceController(object, provisioned),
                      }),
                  );
    }

    /** Build the authorizer over the objects, their scope types and other policies. */
    static #authorizer(
        objects: readonly ObjectType[],
        options: {
            readonly database: DatabaseConnection;
            readonly policies: readonly CatalogPolicy[];
            readonly installed: ReadonlyMap<string, ServedType>;
            readonly partition?: string;
        },
    ): Authorizer {
        // map the types attached to the objects whose tables the database has beside them
        const stored = new Set(options.database.tables.map((table) => table[TABLE].sqlName));
        const attached: ObjectType[] = [];
        const pending = objects.flatMap((object) => object.attachments.map((each) => each.object));
        for (let type = pending.shift(); type !== undefined; type = pending.shift()) {
            const isKnown = [...objects, ...attached].some((each) => each.same(type));
            if (!isKnown && stored.has(type.table[TABLE].sqlName)) {
                attached.push(type);
                pending.push(...type.attachments.map((each) => each.object));
            }
        }

        // map packages onto their installations where the database stores them, each serving its types there
        const isInstalled = stored.has(installation.table[TABLE].sqlName);
        const sets = isInstalled ? [mapInstalledPackages(installation.table)] : [];
        const servers = new Map(
            isInstalled
                ? [...options.installed].map(([key, served]) => [key, served.package.id])
                : [],
        );

        return ObjectPolicy.authorizer(
            [...objects, ...attached],
            options.policies,
            new Set(options.database.tables.filter((table) => options.database.replicates(table))),
            { partition: options.partition, sets, servers },
        );
    }

    /** Require each object type once, apart from the shared procedures. */
    static #requireRoutes(objects: readonly ObjectType[]): void {
        const routed = new Set<string>();
        for (const object of objects) {
            // refuse a type served twice, and one under the shared replica procedures' name
            const key = ObjectTypeReference.key(object.typeReference);
            if (routed.has(key)) {
                throw new TypeError(
                    `object ${object.name} of ${object.package.id} is served twice`,
                );
            } else if (Object.hasOwn(object.shared, object.key)) {
                throw new TypeError(
                    `object ${object.name} has the same name as the shared ${object.key} procedures`,
                );
            }
            routed.add(key);
        }
    }

    /** Refuse others' local types, require claims and files, store ephemeral types. */
    static #requireStores(
        objects: readonly ObjectType[],
        options: Pick<
            CatalogOptions,
            "database" | "claims" | "ephemeral" | "events" | "principal"
        > & { readonly external: readonly ExternalStorage[] },
    ): void {
        // refuse local objects outside the scope of the store's principal
        const local = objects.find(
            (object) =>
                object.storage === "local" &&
                !object.scopes.some((scope) => scope.policy.is(options.principal)),
        );
        if (local !== undefined) {
            throw new TypeError(`local object ${local.name} only exists in its scope's store`);
        }

        // require claims for indexed objects
        const indexed = objects.find((object) => Object.keys(object.indexes).length > 0);
        if (indexed !== undefined && options.claims === undefined) {
            throw new TypeError(
                `object ${indexed.name} declares indexes but has no claim type for their names`,
            );
        }

        // require a memory store for ephemeral objects, storing them in it
        const ephemeral = objects.filter((object) => object.storage === "ephemeral");
        const [unstored] = ephemeral;
        if (unstored !== undefined && options.ephemeral === undefined) {
            throw new TypeError(`ephemeral object ${unstored.name} needs a store`);
        }
        options.ephemeral?.serve(ephemeral);

        // require the event log over the store's database for events types, storing their kinds in it
        const events = objects.filter((object) => object.storage === "events");
        const [unlogged] = events;
        const isLogged =
            options.events !== undefined && options.events.database === options.database;
        if (unlogged !== undefined && !isLogged) {
            throw new TypeError(`events object ${unlogged.name} needs the store's event log`);
        }
        options.events?.register(events.map((object) => object.eventKind()));

        // require files for external objects
        const unfiled = objects.find(
            (object) =>
                object.storage === "external" &&
                !options.external.some((storage) =>
                    storage.objects.some((stored) => stored.same(object)),
                ),
        );
        if (unfiled !== undefined) {
            throw new TypeError(`external object ${unfiled.name} needs files`);
        }
    }

    /** List a package's follows, its standing ones among them. */
    static #follows(definition: ImplementationInput): readonly Follow[] {
        return [...(definition.follows ?? []), ...(definition.standing ?? [])];
    }

    /** Read a server's package for an installation, its handled types over their defaults. */
    static #packageOf(
        server: PackageImplementation,
        context: ServerContext | undefined,
        blobs: BlobStore,
    ): Served {
        // refuse a handled type of the package that its handle neither declares nor its providers provision
        const definition = server.definition(PackageContext.of(context, blobs));
        const handled = definition.objects ?? [];
        const served = Object.values(server.package.objects);
        const provisioned = (definition.providers ?? []).map((provider) => provider.object);
        const others = handled.filter((type) => !served.some((each) => each.same(type)));
        const stray = others.find(
            (type) =>
                type.package.id === server.package.id &&
                !provisioned.some((each) => each.same(type)),
        );
        if (stray !== undefined) {
            throw new TypeError(`${server.package.name} does not declare object ${stray.name}`);
        }

        // serve the declared types, external ones only with their files, provisioned ones only handled or provisioned
        const filed = definition.external?.objects ?? [];
        const declared = served
            .filter((type) => type.storage !== "external" || filed.some((each) => each.same(type)))
            .filter(
                (type) =>
                    type.provisioned === undefined ||
                    [...handled, ...provisioned].some((each) => each.same(type)),
            )
            .map((type) => handled.find((each) => each.same(type)) ?? type);

        return {
            package: server.package,
            definition,
            objects: [...declared, ...others],
            server: context,
            source: server,
        };
    }

    /** Route each package's declared types to it, refusing two packages implementing one. */
    static #routes(
        served: readonly Served[],
    ): ReadonlyMap<string, { readonly type: ObjectType; readonly entry: Served }> {
        // implement each package's declared types, once
        const routes = new Map<string, { readonly type: ObjectType; readonly entry: Served }>();
        for (const entry of served) {
            for (const type of entry.objects) {
                const key = ObjectTypeReference.key(type.typeReference);
                const earlier = routes.get(key);
                if (earlier !== undefined) {
                    throw new TypeError(
                        `${Catalog.#servedBy(earlier.entry)} and ${Catalog.#servedBy(entry)} both implement ${type.name} objects of ${type.package.id}`,
                    );
                }
                routes.set(key, { type, entry });
            }
        }

        return routes;
    }

    /** Describe who serves a package: its installation, else the package itself. */
    static #servedBy(entry: Served): string {
        return entry.server === undefined
            ? `package ${entry.package.name}`
            : `installation ${entry.server.installation}`;
    }

    /** Serve an installation's package, running the code of its types in its sandbox. */
    static #sandboxPackages(sandbox: HookSandbox, known: Iterable<KnownDeclaration>): Served[] {
        // declare the installation's types and index them beside the known ones by reference
        const { declarations, server } = sandbox;
        const platform = [...known];
        const types = ObjectTypePool.of(platform).declare(
            ObjectDescription.installed(declarations),
        );
        const byReference = new Map(
            [...platform, ...types]
                .filter((declared) => declared instanceof ObjectType)
                .map((type) => [ObjectTypeReference.key(type.typeReference), type]),
        );
        const owner = PackageManifest.package(server.build.manifest);
        const local = declarations.filter(
            (entry) => graph.Moniker.packageId(entry.symbol) === owner.id,
        );

        // route each described type, implementing the package's types in the sandbox
        const route = (routes: PackageDescription["objects"]): Record<string, ObjectType> =>
            Object.fromEntries(
                Object.entries(routes).map(([name, routed]) => {
                    // refuse a route of a type nothing here declares
                    const type = byReference.get(ObjectTypeReference.key(routed.type));
                    if (type === undefined) {
                        throw new TypeError(
                            `installation ${server.installation} routes ${routed.type.type} objects of ${routed.type.packageId}, but nothing here implements or replicates them`,
                        );
                    }

                    // implement the package's types in the sandbox and route other types as declared
                    if (routed.type.packageId === owner.id) {
                        return [name, sandbox.implement(type, routed.hooks)];
                    } else if (Object.keys(routed.hooks).length > 0) {
                        throw new TypeError(
                            `installation ${server.installation} cannot hook ${type.name} of ${type.package.id}: the type's package implements it`,
                        );
                    }

                    return [name, type];
                }),
            );

        // serve the package's objects, its adapters left to its workloads
        return local
            .filter((entry) => entry.kind === PACKAGE_DECLARATION)
            .map((entry) => PackageDescription.parse(entry.description))
            .flatMap((described): Served[] => {
                // leave a package of no objects unserved, its adapters to its workloads
                if (Object.keys(described.objects).length === 0) {
                    return [];
                }

                // implement the package's objects as the installation
                const objects = route(described.objects);
                const since = described.since === undefined ? {} : { since: described.since };
                const handle = PackageHandle.of(owner, { objects, ...since });

                return [
                    {
                        package: handle,
                        definition: {},
                        objects: Object.values(objects),
                        server,
                        source: sandbox,
                    },
                ];
            });
    }

    /** Key a type's policy by its package and name, the identity the authorizer maps once. */
    static #policyKey(declared: Policy | ObjectType | TableMapping): string {
        const { policy } = declared instanceof Policy ? { policy: declared } : declared;

        return `${policy.definition.packageId} ${policy.name}`;
    }

    /** List the replicated scope tables served types live in or relate to, which relist chains. */
    static #chainedTables(
        database: DatabaseConnection,
        objects: readonly ObjectType[],
        types: readonly ObjectType[],
    ): Table[] {
        return types
            .filter(
                (type) =>
                    database.replicates(type.table) &&
                    objects.some((object) =>
                        [...object.scopes, ...object.related].some((each) => each.same(type)),
                    ),
            )
            .map((type) => type.table);
    }
}
