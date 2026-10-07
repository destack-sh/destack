import { defineSchema, type JsonObject, type JsonValue, schema } from "@destack/schema";
import { ModuleMetadata } from "../definition/metadata.ts";
import { DeclarationName } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import { Moniker } from "../graph/moniker.ts";
import type { Declaration } from "./declaration.ts";

/** The schema of the property values an example sets, each a JSON value a control can change. */
const Properties = schema.record(schema.string(), schema.json());

/** The property values an example of a declaration sets: some of a function's properties, or JSON values. */
export type PropertiesOf<Of> = Of extends (properties: infer Properties) => unknown
    ? Partial<Properties>
    : Readonly<Record<string, JsonValue>>;

/** An example of a declaration as its module defines it. */
export interface ExampleDefinition<Of, Instance> {
    /** The declaration the example shows, such as a component, a view or an object type. */
    readonly of: Of;
    /** The name of the state it shows, unique among the declaration's examples, such as `history-unavailable`. */
    readonly name: string;
    /** One sentence saying what the example shows. */
    readonly description: string;
    /** The property values the example sets, each a JSON value a control can change. */
    readonly properties?: PropertiesOf<Of>;
    /** The calls bringing the declaration's objects into the example's state, by the scope it opens them in, such as a view's space. */
    readonly objects?: ExampleObjects;
    /** Render the declaration with the properties, absent for a declaration its host renders, such as a view. */
    readonly render?: (properties: PropertiesOf<Of>) => Instance;
}

/** The calls an example makes to bring objects into its state, by scope: methods of object types, named, with their inputs. */
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
    /** The declaration the example shows. */
    readonly of: unknown;
    /** One sentence saying what the example shows. */
    readonly description: string;
    /** The property values the example sets. */
    readonly properties: Properties;
    /** The calls bringing the declaration's objects into the example's state, by scope. */
    readonly objects: ExampleObjects;
    /** Render the declaration with the example's properties, or with the ones a control changed, absent for a declaration its host renders. */
    readonly render: ((properties?: Properties) => Instance) | undefined;
}

/** Declare an example of a declaration in a given state. */
export function defineExample<Of, Instance = never>(
    definition: ExampleDefinition<Of, Instance>,
    module?: ModuleMetadata,
): Example<PropertiesOf<Of>, Instance> {
    // stamp the package supplied by the module transform and validate the name
    const owner = ModuleMetadata.require(module, "defineExample").package;
    const { of, name, description } = definition;
    DeclarationName.parse(name);
    requireDescription("example", name, description);

    // require JSON properties
    const properties = definition.properties ?? {};
    if (!isProperties<Of>(properties)) {
        throw new PackageError(
            "INVALID_DEFINITION",
            `example ${name} sets properties that are no JSON`,
        );
    }

    // render through the definition, or leave rendering to the host for a declaration it renders
    const { render } = definition;

    return Object.freeze({
        package: owner,
        name,
        of,
        description,
        properties,
        objects: definition.objects ?? {},
        render:
            render === undefined
                ? undefined
                : (changed: PropertiesOf<Of> = properties) => render(changed),
    });
}

/** Require one sentence saying what a declaration shows. */
export function requireDescription(kind: string, name: string, description: string): void {
    if (description.trim() === "") {
        throw new PackageError("INVALID_DEFINITION", `${kind} ${name} needs a description`);
    }
}

/** Report whether property values are JSON values a control can change. */
function isProperties<Of>(properties: object): properties is PropertiesOf<Of> {
    return Properties.safeParse(properties).success;
}

/** A JSON scalar a select control offers, such as a variant's name. */
const Option = schema.union([schema.string(), schema.number(), schema.boolean()]);

/** The input a control shows for a property, from the property's type. */
export const ControlInput = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** A switch, for a boolean. */
            kind: schema.literal("boolean"),
        }),
        schema.object({
            /** A number field, for a number. */
            kind: schema.literal("number"),
        }),
        schema.object({
            /** A text field, for a string. */
            kind: schema.literal("text"),
        }),
        schema.object({
            /** A select, for a union of literals. */
            kind: schema.literal("select"),
            /** The literals of the union, in declaration order. */
            options: schema.array(Option).min(2),
        }),
    ]),
);
/** The input a control shows for a property. */
export type ControlInput = schema.Infer<typeof ControlInput>;

/** A control an editor shows for one property of the declaration an example shows. */
export const Control = defineSchema(
    schema.object({
        /** The property the control sets. */
        property: schema.string().min(1),
        /** The property's documentation, absent without one. */
        label: schema.string().min(1).exactOptional(),
        /** Whether the property can stay unset. */
        isOptional: schema.boolean(),
        /** The input the control shows. */
        input: ControlInput,
    }),
);
/** A control an editor shows for one property of the declaration an example shows. */
export type Control = schema.Infer<typeof Control>;

/** The description of an example declaration in the graph. */
export const ExampleDescription = defineSchema(
    schema.object({
        /** One sentence saying what the example shows. */
        description: schema.string().min(1),
        /** The property values the example sets, by property. */
        properties: schema.record(schema.string(), schema.json()),
        /** The controls of the properties the declaration's package declares, by property. */
        controls: schema.array(Control),
        /** The calls bringing the declaration's objects into the example's state, by scope, each object type by moniker. */
        objects: schema.record(
            schema.string(),
            schema.array(
                schema.object({
                    /** The object type. */
                    object: Moniker,
                    /** The method. */
                    method: schema.string().min(1),
                    /** The method's input. */
                    input: schema.record(schema.string(), schema.json()),
                }),
            ),
        ),
    }),
);
/** The description of an example declaration in the graph. */
export type ExampleDescription = schema.Infer<typeof ExampleDescription>;
