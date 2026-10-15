import { DeclarationName, ModuleMetadata, PACKAGE, Package, PackageError } from "@destack/package";
import type { Declaration, ResourceRequirement } from "@destack/package/declare";
import { mapValues, Version } from "@destack/schema";
import { AdapterPath } from "../runtime/description.ts";
import type { ProcedureRouter } from "../runtime/runtime.ts";
import type { Policy } from "@destack/access";
import type { ObjectTypeReference } from "@destack/sync";

/** A package's handler of a protocol others define, at a path below the package's mount. */
export interface AdapterDeclaration extends Declaration {
    /** The path below the package's mount the adapter answers, such as /npm. */
    readonly path: `/${string}`;
    /** The principal types whose signed callers the adapter answers, anyone's when absent. */
    readonly callers?: readonly ObjectTypeReference[];
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
    /** The declaration's own procedures. */
    readonly procedures: ProcedureRouter;
    /** The procedures every declaration of its kind shares. */
    readonly shared: ProcedureRouter;
}

/** What a package declares: its objects, views and adapters, and what its installations bind. */
export interface PackageInput {
    /** The object types the package serves, by route name. */
    readonly objects?: Readonly<Record<string, ObjectDeclaration>>;
    /** The views the package offers, by name. */
    readonly views?: Readonly<Record<string, Declaration>>;
    /** The adapters of protocols others define, by name. */
    readonly adapters?: Readonly<Record<string, AdapterDeclaration>>;
    /** The resources the package uses, by name. */
    readonly resources?: Readonly<Record<string, ResourceRequirement>>;
    /** The secrets the package reads, by name. */
    readonly secrets?: Readonly<Record<string, Declaration>>;
    /** The oldest caller release the package serves, every release when absent. */
    readonly since?: Version;
}

/** A package's handle: its identity, what callers address, and what its installations bind. */
export interface PackageHandle<Input extends PackageInput = PackageInput> extends Package {
    /** The oldest caller release the package serves, every release when absent. */
    readonly since?: Version;
    /** The procedures its objects derive, by route name. */
    readonly router: PackageRoutes<Input>;
    /** The object types the package serves, by route name. */
    readonly objects: Declared<Input["objects"]>;
    /** The views the package offers, by name. */
    readonly views: Declared<Input["views"]>;
    /** The adapters of protocols others define, by name. */
    readonly adapters: Declared<Input["adapters"]>;
    /** The resources the package uses, by name. */
    readonly resources: Declared<Input["resources"]>;
    /** The secrets the package reads, by name. */
    readonly secrets: Declared<Input["secrets"]>;
}

/** The declarations of one kind a package lists, none when it leaves the kind out. */
type Declared<Listed> = (Listed extends Readonly<Record<string, unknown>>
    ? Listed
    : Readonly<Record<string, never>>) &
    object;

/** The router a package's objects derive. */
export type PackageRoutes<Input extends PackageInput> = Extract<
    Input["objects"] extends Readonly<Record<string, ObjectDeclaration>>
        ? {
              readonly [Name in keyof Input["objects"]]: Input["objects"][Name]["procedures"];
          } & Input["objects"][keyof Input["objects"]]["shared"]
        : {},
    ProcedureRouter
>;

/** Declare the handle of the package the module belongs to. */
export function definePackage<const Input extends PackageInput>(
    input: Input,
    module?: ModuleMetadata,
): PackageHandle<Input> {
    return PackageHandle.of(
        Package.parse(ModuleMetadata.require(module, "definePackage").package),
        input,
    );
}

/** The handles packages declare, and the ones hosts rebuild from installed descriptions. */
export const PackageHandle = {
    /** Build a package's handle from its identity and what it declares. */
    of<const Input extends PackageInput>(owner: Package, input: Input): PackageHandle<Input> {
        // read what the package declares, each kind it leaves out empty
        const { objects = {}, views = {}, adapters = {}, resources = {}, secrets = {} } = input;

        // key each named declaration by its own name, once across the handle
        const names = new Set<string>();
        const named = [views, adapters, resources, secrets].flatMap((map) => Object.entries(map));
        for (const [key, declaration] of named) {
            if (key !== declaration.name) {
                throw new PackageError(
                    "INVALID_DEFINITION",
                    `declaration ${declaration.name} is listed under another name: ${key}`,
                );
            } else if (names.has(key)) {
                throw new PackageError("INVALID_DEFINITION", `duplicate declaration: ${key}`);
            }
            names.add(key);
        }

        // refuse an adapter answering at or below another's path
        const paths = Object.values(adapters).map((adapter) => adapter.path);
        const overlap = paths.find((path, index) =>
            paths.some(
                (other, at) => at !== index && (path === other || path.startsWith(`${other}/`)),
            ),
        );
        if (overlap !== undefined) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `adapter path ${overlap} overlaps another`,
            );
        }

        return Object.freeze({
            [PACKAGE]: owner,
            ...owner,
            ...(input.since === undefined ? {} : { since: Version.parse(input.since) }),
            router: mapValues<Record<string, unknown>, PackageRoutes<Input>>(
                routeObjects(owner, objects),
                (procedures) => procedures,
            ),
            objects: mapValues<typeof objects, Declared<Input["objects"]>>(objects, (each) => each),
            views: mapValues<typeof views, Declared<Input["views"]>>(views, (each) => each),
            adapters: mapValues<typeof adapters, Declared<Input["adapters"]>>(
                adapters,
                (each) => each,
            ),
            resources: mapValues<typeof resources, Declared<Input["resources"]>>(
                resources,
                (each) => each,
            ),
            secrets: mapValues<typeof secrets, Declared<Input["secrets"]>>(secrets, (each) => each),
        });
    },
};

/** Declare an adapter of a protocol others define, answered below the package's mount. */
export function defineAdapter(
    name: string,
    declaration: {
        readonly path: `/${string}`;
        readonly callers?: readonly Pick<Policy, "definition">[];
    },
    module?: ModuleMetadata,
): AdapterDeclaration {
    // stamp the package and refuse a path of no segments
    const owner = Package.parse(ModuleMetadata.require(module, "defineAdapter").package);
    if (!AdapterPath.safeParse(declaration.path).success) {
        throw new PackageError(
            "INVALID_DEFINITION",
            `adapter ${name} has no path below the mount: ${declaration.path}`,
        );
    }

    return Object.freeze({
        package: owner,
        name: DeclarationName.parse(name),
        path: declaration.path,
        ...(declaration.callers === undefined
            ? {}
            : {
                  callers: declaration.callers.map(({ definition }) => ({
                      packageId: definition.packageId,
                      type: definition.name,
                  })),
              }),
    });
}

/** Route each declaration under its name, with the shared procedures once. */
function routeObjects(
    owner: Package,
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
        throw new TypeError(`package ${owner.name} routes two procedures under ${collision}`);
    }

    return Object.assign(derived, shared);
}
