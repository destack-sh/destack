import {
    FilterStack,
    Opacity,
    Overflow,
    Position,
    Size,
    TextFormat,
    BoxSides,
    Color,
    Corners,
    Dimension,
    FillStack,
    Gradient,
    Layout,
    Paint,
    Palette,
    ShadowStack,
    Stroke,
    Transform,
    Transition,
    Typography,
} from "@destack/style";
import { t } from "@destack/locale";
import {
    DateRange,
    Duration,
    type JsonObject,
    JsonValue,
    Money,
    Rating,
    RecurrenceRule,
    schema,
    toJsonSchema,
    Url,
} from "@destack/schema";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createSignal, For } from "@destack/view";
import { Field, FieldLabel } from "../field/index.ts";
import { ValueField, ValueFieldProvider } from "./value-field.tsx";

/** The styles of the examples. */
const styles = style.create({
    form: {
        display: "grid",
        gap: space[4],
        width: "100%",
        maxWidth: "22rem",
    },
});

/** The people a person picks from. */
const PEOPLE = ["Ada", "Grace", "Barbara"].map((name) => ({ value: name, title: name }));

/** The input of each kind of value, with what it gives. */
export const valueFieldKinds = defineExample({
    of: ValueField,
    name: "kinds",
    description: t`The input of each kind of value, with what it gives.`,
    layout: "padded",
    render: (properties) => {
        // keep what the inputs give
        const [given, setGiven] = createSignal<Readonly<Record<string, JsonValue | undefined>>>({
            share: 0.25,
            pinned: true,
            person: "Ada",
            label: "#f76b15",
            palette: ["#e0eaff", "#241d9a", "#f75092"],
            sky: {
                shape: { kind: "linear", angle: 180 },
                stops: [
                    { color: "#241d9a", position: 0 },
                    { color: "#ffc53d", position: 1 },
                ],
                space: "oklch",
                hue: "shorter",
            },
            card: { kind: "solid", color: "#ffffff" },
            background: [
                {
                    paint: { kind: "solid", color: "#24159a33" },
                    blend: "multiply",
                    isVisible: true,
                },
                { paint: { kind: "solid", color: "#fffdf7" }, blend: "normal", isVisible: true },
            ],
        });
        const of = (name: string) => ({
            get disabled() {
                return properties.disabled;
            },
            get value() {
                return given()[name];
            },
            onValueChange: (value: JsonValue | undefined) =>
                setGiven((current) => ({ ...current, [name]: value })),
        });

        return (
            <div {...style.attrs(styles.form)}>
                <Field>
                    <FieldLabel>Title</FieldLabel>
                    <ValueField type={{ kind: "text" }} {...of("title")} />
                </Field>
                <Field>
                    <FieldLabel>Share</FieldLabel>
                    <ValueField
                        type={{
                            kind: "number",
                            minimum: 0,
                            maximum: 1,
                            step: 0.05,
                            format: { style: "percent" },
                        }}
                        {...of("share")}
                    />
                </Field>
                <Field orientation="horizontal">
                    <FieldLabel>Pinned</FieldLabel>
                    <ValueField type={{ kind: "boolean" }} {...of("pinned")} />
                </Field>
                <Field>
                    <FieldLabel>Due</FieldLabel>
                    <ValueField type={{ kind: "time" }} {...of("due")} />
                </Field>
                <Field>
                    <FieldLabel>State</FieldLabel>
                    <ValueField
                        type={{ kind: "choice", choices: [{ value: "open" }, { value: "done" }] }}
                        choices={[
                            { value: "open", title: "Open" },
                            { value: "done", title: "Done" },
                        ]}
                        {...of("state")}
                    />
                </Field>
                <Field>
                    <FieldLabel>Person</FieldLabel>
                    <ValueField type={{ kind: "person" }} choices={PEOPLE} {...of("person")} />
                </Field>
                <Field>
                    <FieldLabel>Label</FieldLabel>
                    <ValueField
                        type={{ kind: "json", schema: describe(Color.schema) }}
                        {...of("label")}
                    />
                </Field>
                <Field>
                    <FieldLabel>Palette</FieldLabel>
                    <ValueField
                        type={{ kind: "json", schema: describe(Palette.min(2).max(5)) }}
                        {...of("palette")}
                    />
                </Field>
                <Field>
                    <FieldLabel>Sky</FieldLabel>
                    <ValueField
                        type={{ kind: "json", schema: describe(Gradient.schema) }}
                        {...of("sky")}
                    />
                </Field>
                <Field>
                    <FieldLabel>Card</FieldLabel>
                    <ValueField type={{ kind: "json", schema: describe(Paint) }} {...of("card")} />
                </Field>
                <Field>
                    <FieldLabel>Background</FieldLabel>
                    <ValueField
                        type={{ kind: "json", schema: describe(FillStack) }}
                        {...of("background")}
                    />
                </Field>
                <output>{JSON.stringify(given())}</output>
            </div>
        );
    },
});

/** The inputs of a card's look, from its corners to its motion. */
export const valueFieldCard = defineExample({
    of: ValueField,
    name: "card",
    description: t`The inputs of a card's look, with what they give.`,
    layout: "padded",
    render: (properties) => {
        // store what the inputs give, starting from a raised card
        const [given, setGiven] = createSignal<Readonly<Record<string, JsonValue | undefined>>>({
            corners: Corners.all("{radius.3}"),
            padding: { ...BoxSides.all("{space.4}"), top: { value: 24, unit: "px" } },
            stroke: {
                color: "{color.border}",
                width: "{stroke.border}",
                style: "solid",
                align: "inside",
            },
            shadows: "{shadow.raised}",
            backdrop: [{ kind: "blur", radius: { value: 16, unit: "px" } }],
            opacity: 0.96,
        });
        const of = (name: string) => ({
            get disabled() {
                return properties.disabled;
            },
            get value() {
                return given()[name];
            },
            onValueChange: (value: JsonValue | undefined) =>
                setGiven((current) => ({ ...current, [name]: value })),
        });
        const fields = [
            ["Corners", "corners", Corners],
            ["Padding", "padding", BoxSides],
            ["Stroke", "stroke", Stroke],
            ["Shadows", "shadows", ShadowStack.bindable],
            ["Backdrop", "backdrop", FilterStack],
            ["Opacity", "opacity", Opacity],
            ["Size", "size", Size],
            ["Position", "position", Position],
            ["Overflow", "overflow", Overflow],
            ["Type", "typography", Typography.bindable],
            ["Text", "text", TextFormat],
            ["Move", "transform", Transform],
            ["Layout", "layout", Layout],
            ["Motion", "transition", Transition],
            ["Width", "width", Dimension.bindable],
        ] as const;

        return (
            <div {...style.attrs(styles.form)}>
                <For each={fields}>
                    {([label, name, type]) => (
                        <Field>
                            <FieldLabel>{label}</FieldLabel>
                            <ValueField
                                type={{ kind: "json", schema: describe(type) }}
                                {...of(name)}
                            />
                        </Field>
                    )}
                </For>
                <output>{JSON.stringify(given())}</output>
            </div>
        );
    },
});

/** The inputs of a project's plan, from its dates to its budget. */
export const valueFieldPlan = defineExample({
    of: ValueField,
    name: "plan",
    description: t`The inputs of a project's plan, with what they give.`,
    layout: "padded",
    render: (properties) => {
        // store what the inputs give, starting from a December launch
        const [given, setGiven] = createSignal<Readonly<Record<string, JsonValue | undefined>>>({
            dates: { from: "2026-12-01", to: "2026-12-18" },
            estimate: { days: 3, hours: 4 },
            review: { frequency: "weekly", byDay: [{ day: 1 }, { day: 4 }] },
            budget: { amount: 480000, currency: "EUR" },
            priority: 4,
            brief: "https://example.com/launch",
        });
        const of = (name: string) => ({
            get disabled() {
                return properties.disabled;
            },
            get value() {
                return given()[name];
            },
            onValueChange: (value: JsonValue | undefined) =>
                setGiven((current) => ({ ...current, [name]: value })),
        });
        const fields = [
            ["Dates", "dates", DateRange],
            ["Estimate", "estimate", Duration],
            ["Review", "review", RecurrenceRule],
            ["Budget", "budget", Money],
            ["Priority", "priority", Rating.max(5)],
            ["Brief", "brief", Url],
        ] as const;

        return (
            <div {...style.attrs(styles.form)}>
                <For each={fields}>
                    {([label, name, type]) => (
                        <Field>
                            <FieldLabel>{label}</FieldLabel>
                            <ValueField
                                type={{ kind: "json", schema: describe(type) }}
                                {...of(name)}
                            />
                        </Field>
                    )}
                </For>
                <output>{JSON.stringify(given())}</output>
            </div>
        );
    },
});

/** A number an app takes by an input of its own. */
export const valueFieldReplaced = defineExample({
    of: ValueFieldProvider,
    name: "replaced",
    description: t`A number an app takes by an input of its own.`,
    render: () => (
        <ValueFieldProvider
            inputs={{
                number: (properties) => (
                    <input
                        type="range"
                        aria-label="Stars"
                        min={properties.type.minimum}
                        max={properties.type.maximum}
                        onInput={(event) =>
                            properties.onValueChange(Number(event.currentTarget.value))
                        }
                    />
                ),
            }}
        >
            <ValueField
                type={{ kind: "number", minimum: 0, maximum: 5, step: 1 }}
                value={undefined}
                onValueChange={() => {}}
            />
        </ValueFieldProvider>
    ),
});

/** Describe a schema as the JSON Schema a JSON input carries. */
function describe(definition: schema.Schema): JsonObject {
    return JsonValue.object(toJsonSchema(definition));
}
