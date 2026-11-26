import { type JsonObject, type JsonValue, schema } from "@destack/schema";
import { Message } from "../definition/message.ts";
import { ModuleMetadata } from "../definition/metadata.ts";
import { DeclarationName } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import { Moniker } from "../graph/moniker.ts";
import type { Declaration } from "./declaration.ts";

/** The schema of the property values an example sets, each a JSON value a control can change. */
const Properties = schema.jsonObject();

/** The property values an example sets: some of a function's properties, or JSON values. */
export type PropertiesOf<Of> = Of extends (properties: infer Properties) => unknown
    ? Partial<Properties>
    : Readonly<Record<string, JsonValue>>;

/** Where a host puts an example on its stage, after Storybook's layouts. */
export type ExampleLayout = "centered" | "padded" | "fullscreen";

/** An example of a declaration as its module defines it. */
export interface ExampleDefinition<Of, Instance> {
    /** The declaration the example shows, such as a component, a view or an object type. */
    readonly of: Of;
    /** The state it shows in camelCase, unique per shown declaration, such as `notesEmpty`. */
    readonly name: string;
    /** One sentence saying what the example shows, as a message people read in their language. */
    readonly description: Message;
    /** Where a host puts the example, centred by default. */
    readonly layout?: ExampleLayout;
    /** The property values the example sets, each a JSON value a control can change. */
    readonly properties?: PropertiesOf<Of>;
    /** The calls bringing the declaration's objects into the example's state, by scope. */
    readonly objects?: ExampleObjects;
    /** Render the declaration with the properties, absent if its host renders it, as for a view. */
    readonly render?: (properties: PropertiesOf<Of>) => Instance;
}

/** The calls bringing objects into an example's state, by scope: object type methods and inputs. */
export type ExampleObjects = Readonly<
    Record<
        string,
        readonly {
            /** The object type, as the declaration opens it. */
            readonly object: { readonly name: string };
            /** The method. */
            readonly method: string;
            /** The method's input. */
            readonly input: JsonObject;
        }[]
    >
>;

/** One declaration in a given state, rendered or instantiated by a host and never stepped. */
export interface Example<
    Properties extends object = object,
    Instance = unknown,
> extends Declaration {
    /** The kind of declaration, which a host tells examples from scenarios by. */
    readonly kind: "example";
    /** The declaration the example shows. */
    readonly of: unknown;
    /** One sentence saying what the example shows, as a message people read in their language. */
    readonly description: Message;
    /** Where a host puts the example. */
    readonly layout: ExampleLayout;
    /** The property values the example sets. */
    readonly properties: Properties;
    /** The calls bringing the declaration's objects into the example's state, by scope. */
    readonly objects: ExampleObjects;
    /** Render the declaration with the example's properties, or the ones a control changed. */
    render?(properties?: Properties): Instance;
}

/** Declare an example of a declaration in a given state. */
export function defineExample<Of, Instance = never>(
    definition: ExampleDefinition<Of, Instance>,
    module?: ModuleMetadata,
): Example<PropertiesOf<Of>, Instance> {
    // stamp the package supplied by the module transform and validate the name
    const owner = ModuleMetadata.require(module, "defineExample").package;
    const { of, name, description } = definition;
    requireQualified("example", name);
    requireDescription("example", name, description);

    // require JSON properties
    const properties = definition.properties ?? {};
    if (!isProperties<Of>(properties)) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `example '${name}' is invalid: it sets properties that are not JSON`,
        );
    }

    // render through the definition, or leave rendering to the host for a declaration it renders
    const { render } = definition;

    return Object.freeze({
        kind: "example",
        package: owner,
        name,
        of,
        description,
        layout: definition.layout ?? "centered",
        properties,
        objects: definition.objects ?? {},
        ...(render === undefined
            ? {}
            : { render: (changed: PropertiesOf<Of> = properties) => render(changed) }),
    });
}

/** Require a name the module transform qualified by what its declaration shows. */
export function requireQualified(kind: string, name: string): void {
    DeclarationName.parse(name);
    if (!name.includes(".")) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `${kind} '${name}' is invalid: the module transform qualifies its name by what it shows`,
        );
    }
}

/** Require one sentence saying what a declaration shows. */
export function requireDescription(kind: string, name: string, description: Message): void {
    if (description.source.trim() === "") {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `${kind} '${name}' is invalid: it needs a description`,
        );
    }
}

/** Report whether property values are JSON values a control can change. */
function isProperties<Of>(properties: object): properties is PropertiesOf<Of> {
    return Properties.safeParse(properties).success;
}

/** The description of an example declaration in the graph. */
export const ExampleDescription = schema.object({
    /** One sentence saying what the example shows. */
    description: Message,
    /** The property values the example sets, by property. */
    properties: schema.jsonObject(),
    /** The calls bringing objects into the example's state, by scope and type moniker. */
    objects: schema.record(
        schema.string(),
        schema.array(
            schema.object({
                /** The object type. */
                object: Moniker,
                /** The method. */
                method: schema.string().min(1),
                /** The method's input. */
                input: schema.jsonObject(),
            }),
        ),
    ),
});
/** The description of an example declaration in the graph. */
export type ExampleDescription = schema.Infer<typeof ExampleDescription>;
