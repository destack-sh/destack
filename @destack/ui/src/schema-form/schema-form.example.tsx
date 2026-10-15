import { defineExample } from "@destack/package/declare";
import { JsonValue, schema, toJsonSchema } from "@destack/schema";
import { SchemaForm } from "./schema-form.tsx";

/** A section of an outline: a heading and the sections nested below it. */
type Section = { readonly heading: string; readonly sections?: readonly Section[] };

/** A section of an outline, nesting sections through itself. */
const SECTION: schema.Schema<Section> = schema.object({
    heading: schema.string().min(1).meta({ title: "Heading" }),
    sections: schema
        .array(schema.lazy(() => SECTION))
        .meta({ title: "Section" })
        .exactOptional(),
});

/** The input of a command that files a task. */
export const schemaFormTaskInput = defineExample({
    of: SchemaForm,
    name: "task-input",
    description:
        "the input of a command that files a task: a required title, a priority to choose and whether it is done",
    render: () => (
        <SchemaForm
            schema={JsonValue.object(
                toJsonSchema(
                    schema.object({
                        title: schema.string().min(1).meta({ title: "Title" }),
                        priority: schema
                            .enum(["low", "normal", "high"])
                            .meta({ title: "Priority" })
                            .exactOptional(),
                        done: schema
                            .boolean()
                            .meta({ title: "Done", description: "Filed as already done." })
                            .exactOptional(),
                    }),
                ),
            )}
            submit="File task"
            onSubmit={() => undefined}
        />
    ),
});

/** The input of a command that plans an event, every format in its control. */
export const schemaFormEventInput = defineExample({
    of: SchemaForm,
    name: "event-input",
    description: "the input of a command that plans an event, every format in its control",
    render: () => (
        <SchemaForm
            schema={JsonValue.object(
                toJsonSchema(
                    schema.object({
                        title: schema.string().meta({ title: "Title" }),
                        day: schema.iso.date().meta({ title: "Day" }),
                        reminder: schema.iso.datetime().meta({ title: "Reminder" }).exactOptional(),
                        organizer: schema.email().meta({ title: "Organizer" }),
                        link: schema.url().meta({ title: "Link" }).exactOptional(),
                        seats: schema.int().min(1).max(40).meta({ title: "Seats" }),
                        note: schema.string().nullable().meta({ title: "Note" }),
                    }),
                ),
            )}
            submit="Plan event"
            onSubmit={() => undefined}
        />
    ),
});

/** The input of a command that writes a recipe. */
export const schemaFormRecipeInput = defineExample({
    of: SchemaForm,
    name: "recipe-input",
    description:
        "the input of a command that writes a recipe: a nested source, a list of steps and several tags to choose",
    render: () => (
        <SchemaForm
            schema={JsonValue.object(
                toJsonSchema(
                    schema.object({
                        source: schema
                            .object({
                                book: schema.string().meta({ title: "Book" }),
                                page: schema.int().min(1).meta({ title: "Page" }).exactOptional(),
                            })
                            .meta({ title: "Source" }),
                        steps: schema.array(schema.string().min(1)).meta({ title: "Step" }),
                        tags: schema
                            .array(schema.enum(["quick", "vegetarian", "spicy"]))
                            .meta({ title: "Tags" }),
                    }),
                ),
            )}
            submit="Save recipe"
            onSubmit={() => undefined}
        />
    ),
});

/** The input of a command that adds a shape. */
export const schemaFormShapeInput = defineExample({
    of: SchemaForm,
    name: "shape-input",
    description:
        "the input of a command that adds a shape, its kind chosen first and the chosen kind's fields shown",
    render: () => (
        <SchemaForm
            schema={JsonValue.object(
                toJsonSchema(
                    schema.object({
                        shape: schema
                            .discriminatedUnion("kind", [
                                schema
                                    .object({
                                        kind: schema.literal("circle"),
                                        radius: schema.number().gt(0).meta({ title: "Radius" }),
                                    })
                                    .meta({ title: "Circle" }),
                                schema
                                    .object({
                                        kind: schema.literal("square"),
                                        side: schema.number().gt(0).meta({ title: "Side" }),
                                    })
                                    .meta({ title: "Square" }),
                            ])
                            .meta({ title: "Shape" }),
                    }),
                ),
            )}
            submit="Add shape"
            onSubmit={() => undefined}
        />
    ),
});

/** The input of a command that writes an outline. */
export const schemaFormOutlineInput = defineExample({
    of: SchemaForm,
    name: "outline-input",
    description:
        "the input of a command that writes an outline, whose sections nest sections through a reference",
    render: () => (
        <SchemaForm
            schema={JsonValue.object(
                toJsonSchema(schema.object({ outline: SECTION.meta({ title: "Outline" }) })),
            )}
            submit="Write outline"
            onSubmit={() => undefined}
        />
    ),
});
