import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import type { ViewPresentationPriority } from "@destack/package/view";
import type { Permission } from "@destack/access";
import type { ObjectType } from "@destack/object";
import type { Component } from "solid-js";

/** A view as its package defines it. */
export interface ViewDefinition {
    /** The name, unique among the package's views. */
    readonly name: string;
    /** The object types the view reads and changes, opened in their scopes before it renders. */
    readonly objects?: readonly ObjectType[];
    /** The object types the view opens in the person's home space, such as their notifications. */
    readonly home?: readonly ObjectType[];
    /** The permissions the view requests. */
    readonly permissions?: readonly Permission[];
    /** The object types the view presents, and how strongly, so opening an object picks its view. */
    readonly presents?: readonly Presentation[];
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
    /** The object types the view opens in the person's home space. */
    readonly home: readonly ObjectType[];
    /** The permissions the view requests. */
    readonly permissions: readonly Permission[];
    /** The object types the view presents. */
    readonly presents: readonly Presentation[];
    /** Load the module whose default export is the root component. */
    readonly component: ViewDefinition["component"];

    /** Keep a declared view. */
    constructor(owner: Package, definition: ViewDefinition) {
        // keep the package and the definition
        this.package = owner;
        this.name = definition.name;
        this.objects = definition.objects ?? [];
        this.home = definition.home ?? [];
        this.permissions = definition.permissions ?? [];
        this.presents = definition.presents ?? [];
        this.component = definition.component;
    }
}

/** Declare a view. */
export function defineView(definition: ViewDefinition, module?: ModuleMetadata): View {
    // stamp the declaring package and validate the name
    const owner = ModuleMetadata.require(module, "defineView").package;
    DeclarationName.parse(definition.name);

    // refuse a type opened both in the view's scopes and in the person's home
    const objects = definition.objects ?? [];
    const home = definition.home ?? [];
    const twice = home.find((type) => objects.some((opened) => opened.same(type)));
    if (twice !== undefined) {
        throw new TypeError(
            `view ${definition.name} opens ${twice.name} both in its scopes and in the home`,
        );
    }

    // require each permission to be one of an object type the view opens
    const unopened = (definition.permissions ?? []).find(
        (permission) =>
            ![...objects, ...home].some(
                ({ typeReference }) =>
                    typeReference.packageId === permission.packageId &&
                    typeReference.type === permission.type,
            ),
    );
    if (unopened !== undefined) {
        throw new TypeError(
            `view ${definition.name} requests ${unopened.type} ${unopened.name} but opens no ${unopened.type} objects`,
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
