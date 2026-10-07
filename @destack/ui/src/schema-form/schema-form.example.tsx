import { defineExample } from "@destack/package/declare";
import { SchemaForm } from "./schema-form.tsx";

/** The input of a command that files a task: a required title, a priority to choose and whether it is done. */
export const schemaFormTaskInput = defineExample({
    of: SchemaForm,
    name: "task-input",
    description:
        "the input of a command that files a task: a required title, a priority to choose and whether it is done",
    render: () => (
        <SchemaForm
            schema={{
                type: "object",
                properties: {
                    title: { type: "string", title: "Title", minLength: 1 },
                    priority: { title: "Priority", enum: ["low", "normal", "high"] },
                    done: { type: "boolean", title: "Done", description: "Filed as already done." },
                },
                required: ["title"],
            }}
            submit="File task"
            onSubmit={() => undefined}
        />
    ),
});

/** The input of a command that plans an event, every format in its control and the defaults filled in. */
export const schemaFormEventInput = defineExample({
    of: SchemaForm,
    name: "event-input",
    description:
        "the input of a command that plans an event, every format in its control and the defaults filled in",
    render: () => (
        <SchemaForm
            schema={{
                type: "object",
                properties: {
                    title: { type: "string", title: "Title", default: "Team lunch" },
                    day: { type: "string", title: "Day", format: "date", default: "2026-10-09" },
                    starts: { type: "string", title: "Starts", format: "time" },
                    reminder: { type: "string", title: "Reminder", format: "date-time" },
                    organizer: { type: "string", title: "Organizer", format: "email" },
                    link: { type: "string", title: "Link", format: "uri" },
                    seats: { type: "integer", title: "Seats", minimum: 1, maximum: 40, default: 8 },
                    note: { type: ["string", "null"], title: "Note" },
                },
                required: ["title", "day", "organizer", "seats", "note"],
            }}
            submit="Plan event"
            onSubmit={() => undefined}
        />
    ),
});

/** The input of a command that writes a recipe: a nested source, a list of steps and several tags to choose. */
export const schemaFormRecipeInput = defineExample({
    of: SchemaForm,
    name: "recipe-input",
    description:
        "the input of a command that writes a recipe: a nested source, a list of steps and several tags to choose",
    render: () => (
        <SchemaForm
            schema={{
                type: "object",
                properties: {
                    source: {
                        type: "object",
                        title: "Source",
                        properties: {
                            book: { type: "string", title: "Book" },
                            page: { type: "integer", title: "Page", minimum: 1 },
                        },
                        required: ["book"],
                    },
                    steps: {
                        type: "array",
                        title: "Step",
                        items: { type: "string", minLength: 1 },
                        default: ["Boil the water", "Add the pasta"],
                    },
                    tags: {
                        type: "array",
                        title: "Tags",
                        items: { enum: ["quick", "vegetarian", "spicy"] },
                        default: ["quick"],
                    },
                },
                required: ["source", "steps", "tags"],
            }}
            submit="Save recipe"
            onSubmit={() => undefined}
        />
    ),
});

/** The input of a command that adds a shape, its kind chosen first and the chosen kind's fields shown. */
export const schemaFormShapeInput = defineExample({
    of: SchemaForm,
    name: "shape-input",
    description:
        "the input of a command that adds a shape, its kind chosen first and the chosen kind's fields shown",
    render: () => (
        <SchemaForm
            schema={{
                type: "object",
                properties: {
                    shape: {
                        title: "Shape",
                        oneOf: [
                            {
                                type: "object",
                                title: "Circle",
                                properties: {
                                    kind: { const: "circle" },
                                    radius: {
                                        type: "number",
                                        title: "Radius",
                                        exclusiveMinimum: 0,
                                    },
                                },
                                required: ["kind", "radius"],
                            },
                            {
                                type: "object",
                                title: "Square",
                                properties: {
                                    kind: { const: "square" },
                                    side: { type: "number", title: "Side", exclusiveMinimum: 0 },
                                },
                                required: ["kind", "side"],
                            },
                        ],
                    },
                },
                required: ["shape"],
            }}
            submit="Add shape"
            onSubmit={() => undefined}
        />
    ),
});

/** The input of a command that writes an outline, whose sections nest sections through a reference. */
export const schemaFormOutlineInput = defineExample({
    of: SchemaForm,
    name: "outline-input",
    description:
        "the input of a command that writes an outline, whose sections nest sections through a reference",
    render: () => (
        <SchemaForm
            schema={{
                type: "object",
                $defs: {
                    section: {
                        type: "object",
                        properties: {
                            heading: { type: "string", title: "Heading", minLength: 1 },
                            sections: {
                                type: "array",
                                title: "Section",
                                items: { $ref: "#/$defs/section" },
                            },
                        },
                        required: ["heading"],
                    },
                },
                properties: { outline: { $ref: "#/$defs/section", title: "Outline" } },
                required: ["outline"],
            }}
            submit="Write outline"
            onSubmit={() => undefined}
        />
    ),
});
