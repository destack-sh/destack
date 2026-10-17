import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import { type ViewPresentationPriority } from "@destack/package/manifest";
import { account, space } from "@destack/account/object";
import type { ObjectType } from "@destack/object";
import type { ContextScope } from "./context.ts";
import type { PackageHandle } from "@destack/runtime";
import type { Component } from "solid-js";

/** A view as its package defines it. */
export interface ViewDefinition {
    /** The name, unique among the package's views. */
    readonly name: string;
    /** The object types the view reads and changes, wherever its context opens them. */
    readonly objects?: readonly ObjectType[];
    /** The object types the view presents, and how strongly. */
    readonly presents?: readonly Presentation[];
    /** The packages the view calls as its user, such as observability. */
    readonly packages?: readonly PackageHandle[];
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

/** A view a package declares, acting under its user's grant to the package's installation. */
export class View {
    /** The declaring package. */
    readonly package: Package;
    /** The name. */
    readonly name: string;
    /** The object types the view reads and changes, wherever its context opens them. */
    readonly objects: readonly ObjectType[];
    /** The object types the view presents. */
    readonly presents: readonly Presentation[];
    /** The packages the view calls as its user. */
    readonly packages: readonly PackageHandle[];
    /** Load the module whose default export is the root component. */
    readonly component: ViewDefinition["component"];

    /** Create a view from its package and definition. */
    constructor(owner: Package, definition: ViewDefinition) {
        // copy the package and the definition
        this.package = owner;
        this.name = definition.name;
        this.objects = definition.objects ?? [];
        this.presents = definition.presents ?? [];
        this.packages = definition.packages ?? [];
        this.component = definition.component;
    }

    /** List the object types the view opens in one of its context's scopes, by where each lives. */
    opens(scope: ContextScope): readonly ObjectType[] {
        // open space types in the space and the home, account types in the account, others anywhere
        const lives = scope === "account" ? account : space;

        return this.objects.filter(
            (object) =>
                object.scopes.length === 0 || object.scopes.some((each) => each.same(lives)),
        );
    }
}

/** Declare a view. */
export function defineView(definition: ViewDefinition, module?: ModuleMetadata): View {
    // stamp the declaring package and validate the name
    const owner = ModuleMetadata.require(module, "defineView").package;
    DeclarationName.parse(definition.name);

    // require each presented object type to be one the view opens
    const opened = definition.objects ?? [];
    const unpresentable = (definition.presents ?? []).find(
        ({ object }) => !opened.some((each) => each.same(object)),
    );
    if (unpresentable !== undefined) {
        throw new TypeError(
            `view ${definition.name} presents ${unpresentable.object.name} but does not open ${unpresentable.object.name} objects`,
        );
    }

    return new View(owner, definition);
}
