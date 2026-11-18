import { PackageManifest } from "@destack/package/manifest";
import { graph, PackageVersion } from "@destack/package";
import { RelationGraph } from "../query/query.ts";
import type { EventStore } from "@destack/event";
import { type Authorizer, decisionTables, Policy, type TableMapping } from "@destack/access";
import { ObjectTypeReference, type Subject } from "@destack/sync";
import { type BlobStore, type DatabaseConnection, TABLE, type Table } from "@destack/db";
import { found, type Identifier } from "@destack/schema";
import { PackageContext, type ServerContext } from "../workload/workload.ts";
import { PACKAGE_KIND, PackageDescription } from "../declare/description.ts";
import { Package } from "../declare/package.ts";
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
import type { Isolate } from "../hook/isolate.ts";
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

/** How a store implements an object type: its package, installation and capabilities. */
export interface ObjectTypeImplementation {
    /** The installation serving the type, absent for its scope's own package. */
    readonly server: ServerContext | undefined;
    /** The package implementing the type. */
    readonly package: PackageVersion;
    /** The capabilities it runs with: its installation's, else its package's required ones. */
    readonly capabilities: readonly string[];
}

/** An installation a catalog leaves out, and why. */
export interface CatalogExclusion {
    /** The installation left out. */
    readonly installation: Identifier<"installation">;
    /** Why the catalog leaves it out. */
    readonly error: Error;
}

/** The stores a catalog's types need beside its database, and the principal owning local ones. */
type CatalogStores = Pick<
    CatalogOptions,
    "database" | "claims" | "ephemeral" | "events" | "principal" | "device"
> & {
    /** The files of the external types. */
    readonly external: readonly ExternalStorage[];
};

/** What a catalog compiles from: a store's servers and isolates over its database. */
export interface CatalogOptions {
    /** The package servers. */
    readonly servers: readonly PackageImplementation[];
    /** The isolates of the installations whose packages run here without a workload. */
    readonly isolates?: readonly Isolate[];
    /** The installation the store serves as, absent for a store running as its principal. */
    readonly server?: ServerContext;
    /** The principal of the store's work. */
    readonly principal: Subject;
    /** The device the store runs on, whose partition stores device-scoped local objects. */
    readonly device?: Subject;
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
}

/** The packages a store serves, with their types, routes, tables and access rules. */
export class Catalog {
    /** The package servers it compiled from. */
    readonly servers: readonly PackageImplementation[];
    /** The isolates it compiled from. */
    readonly isolates: readonly Isolate[];
    /** The served packages, each with its code, its types and the installation serving it. */
    readonly served: readonly {
        /** The package. */
        readonly package: ObjectPackage;
        /** The package's code. */
        readonly definition: ImplementationInput;
        /** The package's types, its handled ones in place of their declared defaults. */
        readonly objects: readonly ObjectType[];
        /** The installation serving the package, absent for its scope's own package. */
        readonly server: ServerContext | undefined;
        /** The server or isolate the package was read from, which a later compile reuses it for. */
        readonly source: PackageImplementation | Isolate;
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
    readonly installed: ReadonlyMap<string, ObjectTypeImplementation>;
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
    /** The types follower replicas compile against: followed, decided and served local ones. */
    readonly followed: RelationGraph;
    /** The relations between the object types served, which the served queries compile against. */
    readonly relations: RelationGraph;
    /** The tables the store's feed follows: decided, served local and decision tables. */
    readonly tables: readonly Table[];
    /** The served kinds' resources, each controlled by the loop its trait builds. */
    readonly provisioned: readonly ObjectType[];
    /** The providers reconciling the served kinds' resources. */
    readonly providers: ResourceControllerOptions["providers"];
    /** The machine running the provisioned resources, absent for a store reconciling none. */
    readonly machine: Identifier<"machine"> | undefined;
    /** The declarations installed object types resolve against: the types, policies and kinds. */
    readonly known: readonly KnownDeclaration[];
    /** The scope's branch types, absent for a store without branches. */
    readonly branch: BranchType | undefined;
    /** The installations it refuses, with why: their declarations conflict with the others'. */
    readonly refused: readonly CatalogExclusion[];
    /** The installations it leaves out until types they refer to are served, with why. */
    readonly waiting: readonly CatalogExclusion[];

    /** Hold a compiled catalog. */
    private constructor(
        fields: Omit<
            { readonly [Key in keyof Catalog]: Catalog[Key] },
            "isServing" | "hasMethods" | "relations"
        >,
    ) {
        // keep each compiled field
        this.servers = fields.servers;
        this.isolates = fields.isolates;
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
        this.relations = new RelationGraph(fields.objects);
        this.tables = fields.tables;
        this.provisioned = fields.provisioned;
        this.providers = fields.providers;
        this.machine = fields.machine;
        this.known = fields.known;
        this.branch = fields.branch;
        this.refused = fields.refused;
        this.waiting = fields.waiting;
    }

    /** Whether the store serves methods its journal records, beside its local objects. */
    get isServing(): boolean {
        return this.declared.some((type) => type.storage !== "local");
    }

    /** Whether the store runs methods at all, its own local objects' among them. */
    get hasMethods(): boolean {
        return this.declared.length > 0;
    }

    /** Compile a store's servers and isolates into its types, routes, tables and access. */
    static compile(given: CatalogOptions, previous?: Catalog): Catalog {
        return Catalog.#compile(given, previous, []);
    }

    /** Compile a catalog, leaving out the installations refused so far. */
    static #compile(
        given: CatalogOptions,
        previous: Catalog | undefined,
        refused: readonly CatalogExclusion[],
    ): Catalog {
        // read each server's package, reusing a previous catalog's read of the same server
        const { servers, isolates = [], machine, server, database } = given;
        const reused = (source: PackageImplementation | Isolate) =>
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

        // serve the isolated installations' packages, declared against what the servers make known
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
        const waiting: CatalogExclusion[] = [];
        const served: Served[] = [
            ...implementations,
            ...isolates.flatMap((isolate) => {
                // reuse a previous read, else leave an isolate referring to types this store does not serve yet waiting
                const earlier = reused(isolate);
                if (earlier.length > 0) {
                    return earlier;
                }
                try {
                    return Catalog.#isolatePackages(isolate, known);
                } catch (error) {
                    waiting.push({
                        installation: isolate.server.installation,
                        error: new Error("its types refer to ones this start does not serve yet", {
                            cause: error,
                        }),
                    });

                    return [];
                }
            }),
        ];

        // refuse each installation whose package or types this store cannot serve, failing loudly on its own
        const external = served.flatMap((each) =>
            each.definition.external === undefined ? [] : [each.definition.external],
        );
        const left = Catalog.#refuse(Catalog.#refusals(served, { ...given, external }), {
            given,
            previous,
            refused,
        });
        if (left !== undefined) {
            return left;
        }

        // route every package's types
        const routes = Catalog.#routes(served);

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

        // serve each type at its package's installation, else the store's, else as the scope's own
        const installed = new Map(
            [...routes].map(([key, { entry }]) => {
                const context = entry.server ?? server;
                const capabilities =
                    context?.capabilities ??
                    Object.entries(entry.package.capabilities)
                        .filter(([, capability]) => capability.optional !== true)
                        .map(([name]) => name);

                return [
                    key,
                    { server: context, package: PackageVersion.of(entry.package), capabilities },
                ] as const;
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

        // require each type valid, then store the ephemeral and events types
        const objects = ObjectType.served(declared);
        for (const type of declared) {
            type.requireValid();
        }
        Catalog.#serveStores(objects, given);

        // authorize the objects and the replicas the policies do not already decide
        const policed = new Set(policies.map((each) => Catalog.#policyKey(each)));
        const unpoliced = replicas.filter((type) => !policed.has(Catalog.#policyKey(type)));
        const authorizer = Catalog.#authorizer([...objects, ...unpoliced], {
            database,
            policies,
            installed,
            ...(given.partition === undefined ? {} : { partition: given.partition }),
        });

        // refuse each installation whose tracked types read through a type without history
        const tracked = Catalog.#refuse(
            new Map(
                TrackedHistory.refusals(objects, authorizer)
                    .toReversed()
                    .map(({ object, error }) => [
                        found(routes, ObjectTypeReference.key(object.typeReference)).entry,
                        error,
                    ]),
            ),
            { given, previous, refused },
        );
        if (tracked !== undefined) {
            return tracked;
        }

        // follow the durable tables and the decision tables, the replicated ones no installation here serves
        const decided = Catalog.#durable(objects, policies);
        const replicated = decided.filter(
            (object) =>
                database.replicates(object.table) &&
                !installed.has(ObjectTypeReference.key(object.typeReference)),
        );
        const local = objects.filter((object) => object.storage === "local");
        const tables = [
            ...new Set([
                ...decided.flatMap((object) => object.tables),
                ...local.flatMap((object) => object.tables),
                ...decisionTables,
            ]),
        ];

        return new Catalog({
            servers,
            isolates,
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
            followed: new RelationGraph([...shapes, ...decided, ...local]),
            tables,
            provisioned: Catalog.#control(
                objects,
                machine === undefined ? undefined : { providers, machine },
            ),
            providers,
            machine,
            known: ObjectType.known(declared, policies),
            branch: objects.some((type) => type.same(branchType.object)) ? branchType : undefined,
            refused,
            waiting,
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

    /** Control each served internal kind's resources that a provider here provides. */
    static #control(
        objects: readonly ObjectType[],
        provisioned: ResourceControllerOptions | undefined,
    ): ObjectType[] {
        return provisioned === undefined
            ? []
            : objects
                  .filter((object) => Provisioned.is(object))
                  .filter(
                      (object) =>
                          object.provisioned?.kind.provisioned.management !== "external" &&
                          provisioned.providers.some((provider) => provider.object.same(object)),
                  )
                  .map((object) =>
                      object.implement({
                          controllers: new ResourceController(object, provisioned).controllers(),
                      }),
                  );
    }

    /** Build the authorizer over the objects, their scope types and other policies. */
    static #authorizer(
        objects: readonly ObjectType[],
        options: {
            readonly database: DatabaseConnection;
            readonly policies: readonly CatalogPolicy[];
            readonly installed: ReadonlyMap<string, ObjectTypeImplementation>;
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
            options.database,
            { partition: options.partition, sets, servers },
        );
    }

    /** List the served entries a store cannot serve, with why: served twice, or lacking a store. */
    static #refusals(served: readonly Served[], options: CatalogStores): Map<Served, TypeError> {
        // check the store's own packages first, so an installation never takes their types
        const packages = new Set<string>();
        const types = new Map<string, Served>();
        const refusals = new Map<Served, TypeError>();
        const ordered = [
            ...served.filter((entry) => entry.server === undefined),
            ...served.filter((entry) => entry.server !== undefined),
        ];
        for (const entry of ordered) {
            // refuse the entry's first type the store cannot serve, else its package served twice
            const refused = entry.objects
                .map((type) => Catalog.#typeRefusal(type, entry, types, options))
                .find((each) => each !== undefined);
            const error =
                refused ??
                (packages.has(entry.package.id)
                    ? new TypeError(`package '${entry.package.name}' is served twice`)
                    : undefined);

            // record a refusal, else serve the entry's package and types
            if (error !== undefined) {
                refusals.set(entry, error);
            } else {
                packages.add(entry.package.id);
                for (const type of entry.objects) {
                    types.set(ObjectTypeReference.key(type.typeReference), entry);
                }
            }
        }

        return refusals;
    }

    /** Explain why a store cannot serve a type, absent when it can. */
    static #typeRefusal(
        type: ObjectType,
        entry: Served,
        routed: ReadonlyMap<string, Served>,
        options: CatalogStores,
    ): TypeError | undefined {
        // refuse a type served twice, and one under the shared replica procedures' name
        const earlier = routed.get(ObjectTypeReference.key(type.typeReference));
        if (earlier !== undefined) {
            const earlierDescription = Catalog.#servedBy(earlier);
            const laterDescription = Catalog.#servedBy(entry);

            return new TypeError(
                `${earlierDescription} and ${laterDescription} both implement ${type.name} objects of '${type.package.id}'`,
            );
        }
        // refuse a local type outside the scope of the store's principal and device
        else if (
            type.storage === "local" &&
            !type.scopes.some(
                (scope) =>
                    scope.policy.is(options.principal) ||
                    (options.device !== undefined && scope.policy.is(options.device)),
            )
        ) {
            return new TypeError(`local object ${type.name} only exists in its scope's store`);
        }
        // refuse indexes without the claims recording their names
        else if (Object.keys(type.indexes).length > 0 && options.claims === undefined) {
            return new TypeError(
                `object ${type.name} declares indexes but has no claim type for their names`,
            );
        }
        // refuse an ephemeral type without a memory store
        else if (type.storage === "ephemeral" && options.ephemeral === undefined) {
            return new TypeError(`ephemeral object ${type.name} needs a store`);
        }
        // refuse an events type without the event log over the store's database
        else if (type.storage === "events" && options.events?.database !== options.database) {
            return new TypeError(`events object ${type.name} needs the store's event log`);
        }
        // refuse an external type without files
        else if (
            type.storage === "external" &&
            !options.external.some((storage) => storage.objects.some((each) => each.same(type)))
        ) {
            return new TypeError(`external object ${type.name} needs files`);
        }

        return undefined;
    }

    /** Compile again without the refused installations, failing on the store's own package. */
    static #refuse(
        refusals: ReadonlyMap<Served, Error>,
        compiling: {
            readonly given: CatalogOptions;
            readonly previous: Catalog | undefined;
            readonly refused: readonly CatalogExclusion[];
        },
    ): Catalog | undefined {
        // fail on the store's own package, which no installation stands for
        const entries = [...refusals];
        const own = entries.find(([entry]) => entry.server === undefined);
        if (own !== undefined) {
            throw own[1];
        }

        // compile again without the refused installations, recording why each is left out
        if (entries.length === 0) {
            return undefined;
        }
        const { given, previous, refused } = compiling;
        const sources = new Set(entries.map(([entry]) => entry.source));
        const left = entries.flatMap(([entry, error]) =>
            entry.server === undefined ? [] : [{ installation: entry.server.installation, error }],
        );

        return Catalog.#compile(
            {
                ...given,
                servers: given.servers.filter((each) => !sources.has(each)),
                isolates: (given.isolates ?? []).filter((each) => !sources.has(each)),
            },
            previous,
            [...refused, ...left],
        );
    }

    /** Store the ephemeral and events types in their stores. */
    static #serveStores(
        objects: readonly ObjectType[],
        options: Pick<CatalogOptions, "ephemeral" | "events">,
    ): void {
        options.ephemeral?.serve(objects.filter((object) => object.storage === "ephemeral"));
        options.events?.register(
            objects
                .filter((object) => object.storage === "events")
                .map((object) => object.eventKind()),
        );
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
            throw new TypeError(`'${server.package.name}' does not declare object '${stray.name}'`);
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
        // implement each package's declared types, which the refusals left served once
        return new Map(
            served.flatMap((entry) =>
                entry.objects.map(
                    (type) =>
                        [ObjectTypeReference.key(type.typeReference), { type, entry }] as const,
                ),
            ),
        );
    }

    /** Describe who serves a package: its installation, else its scope as its own. */
    static #servedBy(entry: Served): string {
        return entry.server !== undefined
            ? `installation '${entry.server.installation}'`
            : `the scope's own '${entry.package.name}'`;
    }

    /** Serve an installation's package, running the code of its types in its isolate. */
    static #isolatePackages(isolate: Isolate, known: Iterable<KnownDeclaration>): Served[] {
        // declare the installation's types and index them beside the known ones by reference
        const { declarations, server } = isolate;
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

        // route each described type, implementing the package's types in the isolate
        const route = (routes: PackageDescription["objects"]): Record<string, ObjectType> =>
            Object.fromEntries(
                Object.entries(routes).map(([name, routed]) => {
                    // refuse a route of a type nothing here declares
                    const type = byReference.get(ObjectTypeReference.key(routed.type));
                    if (type === undefined) {
                        throw new TypeError(
                            `installation '${server.installation}' routes ${routed.type.type} objects of '${routed.type.packageId}', but nothing here implements or replicates them`,
                        );
                    }

                    // implement the package's types in the isolate and route other types as declared
                    if (routed.type.packageId === owner.id) {
                        return [name, isolate.implement(type, routed.hooks)];
                    } else if (Object.keys(routed.hooks).length > 0) {
                        throw new TypeError(
                            `installation '${server.installation}' cannot hook ${type.name} of '${type.package.id}': the type's package implements it`,
                        );
                    }

                    return [name, type];
                }),
            );

        // serve the package's objects, its mounts left to its workloads
        return local
            .filter((entry) => PACKAGE_KIND.has(entry))
            .map((entry) => PackageDescription.parse(entry.description))
            .flatMap((described): Served[] => {
                // leave a package of no objects unserved, its mounts to its workloads
                if (Object.keys(described.objects).length === 0) {
                    return [];
                }

                // implement the package's objects as the installation
                const objects = route(described.objects);
                const since = described.since === undefined ? {} : { since: described.since };
                const handle = Package.of(owner, { objects: Object.values(objects), ...since });

                return [
                    {
                        package: handle,
                        definition: {},
                        objects: Object.values(objects),
                        server,
                        source: isolate,
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
