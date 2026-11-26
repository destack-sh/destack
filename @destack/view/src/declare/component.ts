import type { ActionInputDefinition } from "@destack/object";
import { DeclarationName, ModuleMetadata, type PackageVersion } from "@destack/package";
import type { JSX } from "@solidjs/web";

/** A function that renders a component from its properties. */
export type ComponentRender = (properties: never) => JSX.Element;

/** The properties people change in a component's controls and inspector, as an action's inputs. */
export type ComponentProperties<Properties> = {
    readonly [Key in keyof Properties]?: ActionInputDefinition;
};

/** What a package declares about a component beside its render function. */
export interface ComponentDeclaration {
    /** The kind, which tells a component from other declarations. */
    readonly kind: "component";
    /** The declaring package. */
    readonly package: PackageVersion;
    /** The name, unique in its package, such as `button` or `tabsTrigger`. */
    readonly name: string;
    /** The properties people change in its controls and inspector. */
    readonly properties: ComponentProperties<Record<string, unknown>>;
}

/** A component a package declares: its render function, with its declaration. */
export type Component<Render extends ComponentRender = ComponentRender> = Render &
    ComponentDeclaration;

/** Declare a component: its name, its render, and the properties people change in it. */
export function defineComponent<Render extends ComponentRender>(
    name: string,
    render: Render,
    properties: ComponentProperties<Parameters<Render>[0]>,
    module?: ModuleMetadata,
): Component<Render> {
    // stamp the declaring package and validate the name
    const owner = ModuleMetadata.require(module, "defineComponent").package;
    DeclarationName.parse(name);

    // name the render function as the component, its read-only name through a definition
    Object.defineProperty(render, "name", { value: name });

    // give it the rest of the declaration
    return Object.assign(render, { kind: "component" as const, package: owner, properties });
}

/** Report whether a value is a declared component. */
export function isComponent(value: unknown): value is Component {
    return typeof value === "function" && Reflect.get(value, "kind") === "component";
}
