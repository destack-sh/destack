import { Field } from "@destack/object";
import { DeclarationName, ModuleMetadata, type PackageVersion } from "@destack/package";
import type { JSX } from "@solidjs/web";
import { merge } from "../solid/reactive.ts";

/** The properties people change in a component's controls and inspector, each a field, by name. */
export type ComponentFields = Readonly<Record<string, Field>>;

/** The value a field takes. */
type ValueOf<Of> = Of extends Field<infer Configuration> ? Configuration["value"] : never;

/** The names of a component's fields that declare a default. */
type DefaultedName<Fields extends ComponentFields> = {
    readonly [Name in keyof Fields]: Fields[Name] extends Field<infer Configuration>
        ? Configuration["default"] extends true
            ? Name
            : never
        : never;
}[keyof Fields];

/** The names of a component's fields a caller must give: required ones without a default. */
type RequiredName<Fields extends ComponentFields> = {
    readonly [Name in keyof Fields]: Fields[Name] extends Field<infer Configuration>
        ? Configuration extends { readonly required: true; readonly default: false }
            ? Name
            : never
        : never;
}[keyof Fields];

/** A component's declared properties as callers give them, an optional one unset when undefined. */
export type GivenProperties<Fields extends ComponentFields> = {
    readonly [Name in RequiredName<Fields>]: ValueOf<Fields[Name]>;
} & {
    readonly [Name in Exclude<keyof Fields, RequiredName<Fields>>]?:
        | ValueOf<Fields[Name]>
        | undefined;
};

/** A component's declared properties as its render reads them: each defaulted one filled in. */
export type DeclaredProperties<Fields extends ComponentFields> = {
    readonly [Name in RequiredName<Fields> | DefaultedName<Fields>]: ValueOf<Fields[Name]>;
} & {
    readonly [Name in Exclude<keyof Fields, RequiredName<Fields> | DefaultedName<Fields>>]?:
        | ValueOf<Fields[Name]>
        | undefined;
};

/** A type without some keys, each member of a union on its own. */
type Without<Type, Keys extends PropertyKey> = Type extends unknown ? Omit<Type, Keys> : never;

/** A component's properties as its render reads them: its attributes and its declared ones. */
export type RenderProperties<Attributes, Fields extends ComponentFields> = Attributes &
    DeclaredProperties<Fields>;

/** What a package declares about a component beside its render function. */
export interface ComponentDeclaration<
    Fields extends ComponentFields = ComponentFields,
    Defaults extends Readonly<Record<string, unknown>> = Readonly<Record<string, unknown>>,
> {
    /** The kind, which tells a component from other declarations. */
    readonly kind: "component";
    /** The declaring package. */
    readonly package: PackageVersion;
    /** The name, unique in its package, such as `button` or `tabsTrigger`. */
    readonly name: string;
    /** The properties people change in its controls and inspector. */
    readonly properties: Fields;
    /** The value each declared property takes when none is given, by property. */
    readonly defaults: Defaults;
}

/** A component a package declares: its render function, with its declaration. */
export type Component<
    Properties = never,
    Fields extends ComponentFields = ComponentFields,
    Defaults extends Readonly<Record<string, unknown>> = Readonly<Record<string, unknown>>,
> = ((properties: Properties) => JSX.Element) & ComponentDeclaration<Fields, Defaults>;

/** The declared properties of a component that a default fills in, with their defaults. */
export type DeclaredDefaults<Fields extends ComponentFields> = Pick<
    DeclaredProperties<Fields>,
    DefaultedName<Fields>
>;

/** The properties a component's callers give it, its declared ones and the rest. */
export type ComponentProperties<Of> = Of extends (properties: infer Properties) => JSX.Element
    ? Properties
    : never;

/** A component as its package defines it: its name, its declared properties and its render. */
export interface ComponentDefinition<
    Fields extends ComponentFields,
    Properties extends DeclaredProperties<Fields>,
> {
    /** The name, unique in its package, such as `button` or `tabsTrigger`. */
    readonly name: string;
    /** The properties people change in its controls and inspector. */
    readonly properties: Fields;
    /** Render it from its properties, each declared one with a default filled in. */
    readonly render: (properties: Properties) => JSX.Element;
}

/** Declare a component: its name, the properties people change in it, and its render. */
export function defineComponent<
    Fields extends ComponentFields,
    Properties extends DeclaredProperties<Fields>,
>(
    definition: ComponentDefinition<Fields, Properties>,
    module?: ModuleMetadata,
): Component<
    Without<Properties, keyof Fields> & GivenProperties<Fields>,
    Fields,
    DeclaredDefaults<Fields>
> {
    // stamp the declaring package and validate the name
    const owner = ModuleMetadata.require(module, "defineComponent").package;
    const { name, properties, render } = definition;
    DeclarationName.parse(name);

    // render with each declared default under what the caller gives, the component named as declared
    // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- each default is its field's own initial value
    const defaults = defaultsOf(properties) as DeclaredDefaults<Fields>;
    const component = (given: Without<Properties, keyof Fields> & GivenProperties<Fields>) =>
        // oxlint-disable-next-line typescript/consistent-type-assertions, typescript/no-unsafe-type-assertion -- the defaults are these fields' own
        render(merge(defaults, given) as unknown as Properties);
    Object.defineProperty(component, "name", { value: name });

    return Object.assign(component, {
        kind: "component" as const,
        package: owner,
        properties,
        defaults,
    });
}

/** A component a package declares, as a value to tell components by. */
export const Component = {
    /** Report whether a value is a declared component. */
    is(value: unknown): value is Component {
        return typeof value === "function" && Reflect.get(value, "kind") === "component";
    },
};

/** Read the value each field takes when none is given, by its name. */
function defaultsOf(fields: ComponentFields): Record<string, unknown> {
    // take the value each field starts with
    const defaults: Record<string, unknown> = {};
    for (const [name, field] of Object.entries(fields)) {
        if (field.initial !== undefined) {
            defaults[name] = field.initial.value;
        }
    }

    return defaults;
}
