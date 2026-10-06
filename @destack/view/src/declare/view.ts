import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import { type ViewPresentationPriority } from "@destack/package/manifest";
import { PermissionScope } from "@destack/access";
import { type Permission, PermissionReference } from "@destack/access";
import type { ObjectType } from "@destack/object";
import type { Service } from "@destack/service";
import type { Component } from "solid-js";

/** A view as its package defines it. */
export interface ViewDefinition {
    /** The name, unique among the package's views. */
    readonly name: string;
    /** The object types the view reads and changes, opened before it renders in each scope it requests their permissions in. */
    readonly objects?: readonly ObjectType[];
    /** The permissions the view requests, by the scope they apply in relative to its context. */
    readonly permissions?: { readonly [Scope in PermissionScope]?: readonly Permission[] };
    /** The object types the view presents, and how strongly, so opening an object picks its view. */
    readonly presents?: readonly Presentation[];
    /** The platform services the view calls as its person, such as the observability service. */
    readonly services?: readonly Service[];
    /** Load the module whose default export is the root component. */
    readonly component: () => Promise<{ readonly default: Component }>;
}

/** An object type a view presents: the one it opens by default, or one offered beside it. */
export interface Presentation {
    /** The presented object type, among the view's objects. */
    readonly object: ObjectType;
    /** How strongly the view presents it. */
    readonly priority: ViewPresentationPriority;
}

/** A view a package declares. */
export class View {
    /** The declaring package. */
    readonly package: Package;
    /** The name. */
    readonly name: string;
    /** The object types the view reads and changes. */
    readonly objects: readonly ObjectType[];
    /** The permissions the view requests, by the scope they apply in. */
    readonly permissions: NonNullable<ViewDefinition["permissions"]>;
    /** The object types the view presents. */
    readonly presents: readonly Presentation[];
    /** The platform services the view calls as its person. */
    readonly services: readonly Service[];
    /** Load the module whose default export is the root component. */
    readonly component: ViewDefinition["component"];

    /** Keep a declared view. */
    constructor(owner: Package, definition: ViewDefinition) {
        // keep the package and the definition
        this.package = owner;
        this.name = definition.name;
        this.objects = definition.objects ?? [];
        this.permissions = definition.permissions ?? {};
        this.presents = definition.presents ?? [];
        this.services = definition.services ?? [];
        this.component = definition.component;
    }

    /** List the object types the view opens in a scope: those it requests permissions on there. */
    opens(scope: PermissionScope): ObjectType[] {
        const permissions = this.permissions[scope] ?? [];

        return this.objects.filter((object) =>
            permissions.some((permission) => isPermissionOf(permission, object)),
        );
    }
}

/** Declare a view. */
export function defineView(definition: ViewDefinition, module?: ModuleMetadata): View {
    // stamp the declaring package and validate the name
    const owner = ModuleMetadata.require(module, "defineView").package;
    DeclarationName.parse(definition.name);

    // list each requested permission with its scope
    const objects = definition.objects ?? [];
    const requested = PermissionScope.options.flatMap((scope) =>
        (definition.permissions?.[scope] ?? []).map((permission) => ({ scope, permission })),
    );

    // refuse a permission requested in two scopes
    const twice = requested.find((entry) =>
        requested.some(
            (other) =>
                other.scope !== entry.scope &&
                PermissionReference.key(other.permission) ===
                    PermissionReference.key(entry.permission),
        ),
    );
    if (twice !== undefined) {
        const { type, name } = twice.permission;
        throw new TypeError(
            `view ${definition.name} requests ${type} ${name} in more than one scope`,
        );
    }

    // require each permission to be one of an object type the view opens
    const unopened = requested.find(
        ({ permission }) => !objects.some((object) => isPermissionOf(permission, object)),
    );
    if (unopened !== undefined) {
        const { type, name } = unopened.permission;
        throw new TypeError(
            `view ${definition.name} requests ${type} ${name} but opens no ${type} objects`,
        );
    }

    // require each opened object type to have a permission placing it in a scope
    const unplaced = objects.find(
        (object) => !requested.some(({ permission }) => isPermissionOf(permission, object)),
    );
    if (unplaced !== undefined) {
        throw new TypeError(
            `view ${definition.name} opens ${unplaced.name} but requests no ${unplaced.name} permission`,
        );
    }

    // require each presented object type to be one the view opens
    const unpresentable = (definition.presents ?? []).find(
        ({ object }) => !objects.some((opened) => opened.same(object)),
    );
    if (unpresentable !== undefined) {
        throw new TypeError(
            `view ${definition.name} presents ${unpresentable.object.name} but opens no ${unpresentable.object.name} objects`,
        );
    }

    return new View(owner, definition);
}

/** Report whether a permission is one of an object type's. */
function isPermissionOf(permission: Permission, object: ObjectType): boolean {
    return (
        object.typeReference.packageId === permission.packageId &&
        object.typeReference.type === permission.type
    );
}
