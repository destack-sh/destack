import { type ErrorFamily, type ReasonDeclarations, ReasonDescription } from "@destack/error";
import { isContractProcedure } from "@orpc/contract";
import type { Table } from "@destack/db";
import {
    DeclarationName,
    ModuleMetadata,
    PACKAGE,
    PackageVersion,
    PackageError,
} from "@destack/package";
import type { Declaration, ResourceRequirement } from "@destack/package/declare";
import { mapValues, Version } from "@destack/schema";
import { AdapterPath, Capability } from "./description.ts";
import type { Command } from "./command.ts";
import type { ProcedureRouter } from "../procedure/index.ts";
import { GrantDeclaration, type Policy } from "@destack/access";
import type { ObjectTypeReference } from "@destack/sync";
import { ResourceDeclaration } from "@destack/resource";
import { type CamelCase, Casing } from "../object/name.ts";

/** A package's handler of a protocol others define, at a path below the package's mount. */
export interface AdapterDeclaration extends Declaration {
    /** The path below the package's mount the adapter answers, such as /npm. */
    readonly path: `/${string}`;
    /** The principal types whose callers the adapter answers, its installation alone, or anyone. */
    readonly callers?: readonly ObjectTypeReference[] | "self";
}

/** An object type's declaration, with its derived procedures. */
export interface ObjectDeclaration {
    /** The package declaring it. */
    readonly package: { readonly name: string };
    /** Its name within its package. */
    readonly name: string;
    /** Its declaring package and name. */
    readonly typeReference: { readonly packageId: string; readonly type: string };
    /** The phases its code attaches to its methods, by method name. */
    readonly hooks: Readonly<Record<string, readonly string[]>>;
    /** The declaration's procedures. */
    readonly procedures: ProcedureRouter;
    /** The procedures every declaration of its kind shares. */
    readonly shared: ProcedureRouter;
}

/** What a package declares: its objects, views and mounts, and what its installations bind. */
export interface PackageOptions {
    /** The object types the package serves, each routed by its name. */
    readonly objects?: readonly ObjectDeclaration[];
    /** The tables the package stores below its objects, such as protocol records. */
    readonly tables?: readonly Table[];
    /** The views the package offers. */
    readonly views?: readonly Declaration[];
    /** The commands the package declares on its objects. */
    readonly commands?: readonly Command[];
    /** The mounts serving protocols others define below the package. */
    readonly mounts?: readonly AdapterDeclaration[];
    /** The resources the package uses and the secrets it reads. */
    readonly resources?: readonly (ResourceRequirement | Declaration)[];
    /** Who the package's installations act as. */
    readonly access?: {
        /** The permissions the package's installations always request, by delegator. */
        readonly permissions?: GrantDeclaration;
    };
    /** What its installations can do beyond that, each with its name and what it needs. */
    readonly capabilities?: readonly (Capability & { readonly name: string })[];
    /** The reasons the package fails with, which its manifest lists. */
    readonly errors?: Pick<ErrorFamily<ReasonDeclarations>, "describe">;
    /** The oldest caller release the package serves, every release when absent. */
    readonly since?: Version;
}

/** A package's handle: its identity, what callers address, and what its installations bind. */
export interface Package<Options extends PackageOptions = PackageOptions> extends PackageVersion {
    /** The oldest caller release the package serves, every release when absent. */
    readonly since?: Version;
    /** The procedures its objects derive, by route name. */
    readonly router: PackageRouter<Options>;
    /** The object types the package serves, by their names in camel case. */
    readonly objects: ByRoute<Options["objects"]>;
    /** The tables the package stores below its objects, such as protocol records. */
    readonly tables: readonly Table[];
    /** The views the package offers, by name. */
    readonly views: ByName<Options["views"]>;
    /** The commands the package declares on its objects, by name. */
    readonly commands: ByName<Options["commands"]>;
    /** The mounts serving protocols others define below the package, by name. */
    readonly adapters: ByName<Options["mounts"]>;
    /** The resources the package uses, by name. */
    readonly resources: Keyed<ResourceOf<Listed<Options["resources"]>>>;
    /** The secrets the package reads, by name. */
    readonly secrets: Keyed<
        Exclude<Listed<Options["resources"]>, ResourceOf<Listed<Options["resources"]>>>
    >;
    /** The permissions the package's installations always request, by delegator. */
    readonly permissions: GrantDeclaration;
    /** What its installations can do beyond that, by name. */
    readonly capabilities: Readonly<Record<string, Capability>>;
    /** The reasons the package fails with, as its manifest lists them. */
    readonly errors: readonly ReasonDescription[];
}

/** A package's declarations of one kind, keyed by their names, none when it lists none. */
type ByName<List> = List extends readonly { readonly name: string }[]
    ? Keyed<List[number]>
    : Readonly<Record<string, never>>;

/** The declarations a package lists of one kind, none when it leaves the kind out. */
type Listed<List> = List extends readonly (infer Each)[] ? Each : never;

/** The listed declarations whose kind providers may provision, which bind as resources. */
type ResourceOf<Each> = Each extends { readonly kind: { readonly provisioned: infer Provisioned } }
    ? [Provisioned] extends [undefined]
        ? never
        : Each
    : never;

/** Declarations keyed by their names. */
type Keyed<Each extends { readonly name: string }> = {
    readonly [Named in Each as Named["name"]]: Named;
};

/** A package's object types keyed by their names in camel case, none when it lists none. */
type ByRoute<List> = List extends readonly ObjectDeclaration[]
    ? {
          readonly [Each in List[number] as RouteOf<Each>]: Each;
      } & (string extends List[number]["name"] ? Readonly<Record<string, List[number]>> : {})
    : Readonly<Record<string, never>>;

/** The route an object type's literal name gives it, none for a name typed as any string. */
type RouteOf<Each extends { readonly name: string }> = string extends Each["name"]
    ? never
    : CamelCase<Each["name"]>;

/** The router a package's objects derive. */
export type PackageRouter<Options extends PackageOptions> = Extract<
    Options["objects"] extends readonly ObjectDeclaration[]
        ? {
              readonly [Each in Options["objects"][number] as RouteOf<Each>]: Each["procedures"];
          } & Options["objects"][number]["shared"]
        : {},
    ProcedureRouter
>;

/** Declare the handle of the package the module belongs to. */
export function definePackage<const Options extends PackageOptions>(
    options: Options,
    module?: ModuleMetadata,
): Package<Options> {
    return Package.of(
        PackageVersion.parse(ModuleMetadata.require(module, "definePackage").package),
        options,
    );
}

/** The handles packages declare, and the ones hosts rebuild from installed descriptions. */
export const Package = {
    /** Build a package's handle from its identity and what it declares. */
    of<const Options extends PackageOptions>(
        owner: PackageVersion,
        options: Options,
    ): Package<Options> {
        // key what the package lists by name, each kind it leaves out empty, its resources apart from its secrets
        const listed = options.resources ?? [];
        const objects = byRoute(options.objects ?? []);
        const views = byName(options.views ?? []);
        const commands = byName(options.commands ?? []);
        const adapters = byName(options.mounts ?? []);
        const bound = Object.values(byName(listed));
        const resources = byName(bound.filter((each) => isProvisioned(each)));
        const secrets = byName(bound.filter((each) => !isProvisioned(each)));

        // refuse an object type another package defines, by its package's name
        const foreign = Object.values(objects).find((object) => object.package.name !== owner.name);
        if (foreign !== undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `object '${foreign.name}' is invalid: '${owner.name}' lists it, but only '${foreign.package.name}' can serve it`,
            );
        }

        // refuse a command on an object type the package does not serve, or named as one of its methods
        const served = new Set<unknown>(Object.values(objects));
        const stray = Object.values(commands).find((command) => !served.has(command.object));
        const renaming = Object.values(commands).find(
            (command) => command.name in command.object.methods,
        );
        if (stray !== undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `command '${stray.name}' is invalid: '${owner.name}' lists it on ${stray.object.name}, which it does not serve`,
            );
        } else if (renaming !== undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `command '${renaming.name}' is invalid: it takes the name of a method of ${renaming.object.name}`,
            );
        }

        // refuse an adapter answering at or below another's path
        const paths = Object.values(adapters).map((adapter) => adapter.path);
        const overlap = paths.find((path, index) =>
            paths.some(
                (other, at) => at !== index && (path === other || path.startsWith(`${other}/`)),
            ),
        );
        if (overlap !== undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `adapter path '${overlap}' is invalid: it overlaps another`,
            );
        }

        // refuse an adapter answering below an object's routes, which would take its calls
        const roots = new Set<string | undefined>();
        const routers: ProcedureRouter[] = Object.values(objects).map(
            (object) => object.procedures,
        );
        for (let router = routers.pop(); router !== undefined; router = routers.pop()) {
            // take a procedure's first path segment, and walk a nested router
            if (isContractProcedure(router)) {
                roots.add(router["~orpc"].route.path?.split("/")[1]);
            } else {
                routers.push(...Object.values(router));
            }
        }
        const shadowing = paths.find((path) => roots.has(path.split("/")[1]));
        if (shadowing !== undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `adapter path '${shadowing}' is invalid: it overlaps an object's routes`,
            );
        }

        // route the objects' procedures on first use, which spares a package's import the work
        let router: PackageRouter<Options> | undefined;

        return Object.freeze({
            [PACKAGE]: owner,
            ...owner,
            ...(options.since === undefined ? {} : { since: Version.parse(options.since) }),
            get router(): PackageRouter<Options> {
                router ??= mapValues<Record<string, unknown>, PackageRouter<Options>>(
                    routeObjects(owner, objects),
                    (procedures) => procedures,
                );

                return router;
            },
            objects: mapValues<typeof objects, ByRoute<Options["objects"]>>(
                objects,
                (each) => each,
            ),
            tables: [...(options.tables ?? [])],
            views: mapValues<typeof views, ByName<Options["views"]>>(views, (each) => each),
            commands: mapValues<typeof commands, ByName<Options["commands"]>>(
                commands,
                (each) => each,
            ),
            adapters: mapValues<typeof adapters, ByName<Options["mounts"]>>(
                adapters,
                (each) => each,
            ),
            resources: mapValues<typeof resources, Keyed<ResourceOf<Listed<Options["resources"]>>>>(
                resources,
                (each) => each,
            ),
            secrets: mapValues<
                typeof secrets,
                Keyed<
                    Exclude<Listed<Options["resources"]>, ResourceOf<Listed<Options["resources"]>>>
                >
            >(secrets, (each) => each),
            permissions: GrantDeclaration.parse(options.access?.permissions ?? {}),
            capabilities: Object.fromEntries(
                Object.values(byName(options.capabilities ?? [])).map(({ name, ...each }) => [
                    name,
                    Capability.parse(each),
                ]),
            ),
            errors: options.errors?.describe() ?? [],
        });
    },
};

/** Declare a mount serving a protocol others define below the package. */
export function defineMount<const Name extends string>(
    options: {
        /** The mount's name within the package. */
        readonly name: Name;
        /** The path below the package the mount answers, such as /npm. */
        readonly path: `/${string}`;
        /** The caller types the mount answers, or the installation itself, anyone when absent. */
        readonly callers?: readonly Pick<Policy, "definition">[] | "self";
    },
    module?: ModuleMetadata,
): AdapterDeclaration & { readonly name: Name } {
    // stamp the package and refuse a path of no segments
    const owner = PackageVersion.parse(ModuleMetadata.require(module, "defineMount").package);
    DeclarationName.parse(options.name);
    if (!AdapterPath.safeParse(options.path).success) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `mount '${options.name}' path '${options.path}' is invalid: it is not below the package`,
        );
    }

    // take the caller types by reference, or the installation itself
    const callers =
        options.callers === undefined || options.callers === "self"
            ? options.callers
            : options.callers.map(({ definition }) => ({
                  packageId: definition.packageId,
                  type: definition.name,
              }));

    return Object.freeze({
        package: owner,
        name: options.name,
        path: options.path,
        ...(callers === undefined ? {} : { callers }),
    });
}

/** Route each declaration under its name, with the shared procedures once. */
function routeObjects(
    owner: PackageVersion,
    objects: Readonly<Record<string, ObjectDeclaration>>,
): Record<string, unknown> {
    // route each declaration under its name and collect the shared procedures
    const derived: Record<string, unknown> = {};
    const shared: Record<string, unknown> = {};
    for (const [key, declaration] of Object.entries(objects)) {
        derived[key] = declaration.procedures;
        Object.assign(shared, declaration.shared);
    }

    // refuse a name taken twice
    const taken = [...Object.keys(objects), ...Object.keys(shared)];
    const collision = taken.find((key, position) => taken.indexOf(key) !== position);
    if (collision !== undefined) {
        throw new TypeError(`package '${owner.name}' routes two procedures under '${collision}'`);
    }

    return Object.assign(derived, shared);
}

/** Key declarations by their names, refusing two of one name. */
function byName<Named extends { readonly name: string }>(
    declarations: readonly Named[],
): Readonly<Record<string, Named>> {
    const keyed: Record<string, Named> = {};
    for (const declaration of declarations) {
        if (Object.hasOwn(keyed, declaration.name)) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `declaration '${declaration.name}' is invalid: it is a duplicate`,
            );
        }
        keyed[declaration.name] = declaration;
    }

    return keyed;
}

/** Key object types by their names in camel case, refusing two of one name. */
function byRoute(
    objects: readonly ObjectDeclaration[],
): Readonly<Record<string, ObjectDeclaration>> {
    const keyed: Record<string, ObjectDeclaration> = {};
    for (const declaration of objects) {
        // refuse a second object type of the same name
        const route = Casing.camel(declaration.name);
        if (Object.hasOwn(keyed, route)) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `object '${declaration.name}' is invalid: it is a duplicate`,
            );
        }
        keyed[route] = declaration;
    }

    return keyed;
}

/** Report whether providers provision a resource, as they do a bucket and never a secret. */
function isProvisioned(each: ResourceRequirement | Declaration): each is ResourceRequirement {
    return ResourceDeclaration.is(each) && each.kind.isProvisioned();
}
