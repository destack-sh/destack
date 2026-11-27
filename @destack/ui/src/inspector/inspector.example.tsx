import { type Message, t } from "@destack/locale";
import { ActionInput, type ActionValue } from "@destack/object";
import { defineExample } from "@destack/package/declare";
import { type JsonObject, JsonValue, type schema, toJsonSchema } from "@destack/schema";
import {
    BoxSides,
    Corners,
    FillStack,
    FilterStack,
    Layout,
    Motion,
    Opacity,
    Overflow,
    Position,
    ShadowStack,
    Size,
    Stroke,
    TextFormat,
    Transform,
    Typography,
} from "@destack/style";
import * as style from "@destack/style";
import { createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { sharedValue, ValueField } from "../value-field/index.ts";
import { ActionInputField } from "../action/index.ts";
import { Inspector } from "./inspector.tsx";
import { FieldLegend, FieldSeparator, FieldSet } from "../field/index.ts";

/** The styles of the examples. */
const styles = style.create({
    panel: {
        width: "100%",
        maxWidth: "20rem",
    },
});

/** A value of a card's design: its name, schema and label. */
type DesignValue = readonly [name: string, definition: schema.Schema, label: Message];

/** The sections of a card's design, as a design tool orders them: a value, or a titled group. */
const SECTIONS: readonly (
    | DesignValue
    | { readonly title: Message; readonly values: readonly DesignValue[] }
)[] = [
    ["position", Position, t.context("label")`Position`],
    ["transform", Transform, t.context("label")`Transforms`],
    {
        title: t.context("title")`Layout`,
        values: [
            ["size", Size, t.context("label")`Size`],
            ["layout", Layout, t.context("label")`Auto layout`],
            ["overflow", Overflow, t.context("label")`Overflow`],
        ],
    },
    {
        title: t.context("title")`Appearance`,
        values: [
            ["opacity", Opacity, t.context("label")`Opacity`],
            ["corners", Corners, t.context("label")`Corners`],
        ],
    },
    {
        title: t.context("title")`Typography`,
        values: [
            ["typography", Typography.bindable, t.context("label")`Type`],
            ["text", TextFormat, t.context("label")`Text`],
        ],
    },
    ["fill", FillStack, t.context("label")`Fill`],
    ["stroke", Stroke, t.context("label")`Stroke`],
    {
        title: t.context("title")`Effects`,
        values: [
            ["shadows", ShadowStack.bindable, t.context("label")`Shadows`],
            ["backdrop", FilterStack, t.context("label")`Backdrop`],
        ],
    },
    ["motion", Motion, t.context("label")`Motion`],
];

/** A button's properties, each in a row of its name and its field. */
export const inspectorDefault = defineExample({
    of: Inspector,
    name: "default",
    description: t`A button's properties, each in a row of its name and its field.`,
    layout: "padded",
    render: () => <ButtonInspector />,
});

/** A card's design in sections, from its position to its motion, as a design tool's panel. */
export const inspectorDesign = defineExample({
    of: Inspector,
    name: "design",
    description: t`A card's design in sections, from its position to its motion, as a design tool's panel.`,
    layout: "padded",
    render: () => <DesignInspector objects={[CARD]} />,
});

/** A card and a popover edited at once, the values they hold apart shown as mixed. */
export const inspectorSelection = defineExample({
    of: Inspector,
    name: "selection",
    description: t`A card and a popover edited at once, the values they hold apart shown as mixed.`,
    layout: "padded",
    render: () => <DesignInspector objects={[CARD, POPOVER]} />,
});

/** Render an inspector of a button's properties. */
function ButtonInspector(): JSX.Element {
    // read the button's declared properties as inputs, and store what a person gives
    const inputs = Object.entries(Button.properties).flatMap(([name, definition]) =>
        definition === undefined ? [] : [ActionInput.of(name, definition)],
    );
    const [values, setValues] = createSignal<JsonObject>({
        variant: "default",
        size: "default",
        loading: false,
    });

    return (
        <Inspector aria-label="Button" xstyle={styles.panel}>
            <For each={inputs}>
                {(input) => (
                    <ActionInputField
                        input={input}
                        orientation="horizontal"
                        value={values()[input.name]}
                        onValueChange={(value) =>
                            setValues((current) => ({ ...current, [input.name]: value ?? null }))
                        }
                    />
                )}
            </For>
        </Inspector>
    );
}

/** A raised card bound to the theme. */
const CARD: JsonObject = {
    corners: Corners.all("{radius.3}"),
    layout: {
        direction: "column",
        gap: "{space.2}",
        align: "start",
        justify: "start",
        isWrapping: false,
        padding: { ...BoxSides.all("{space.4}"), top: { value: 24, unit: "px" } },
    },
    fill: [
        {
            paint: { kind: "solid", color: "{color.card}" },
            blend: "normal",
            isVisible: true,
        },
    ],
    stroke: {
        color: "{color.border}",
        width: "{stroke.border}",
        style: "solid",
        align: "inside",
    },
    shadows: "{shadow.raised}",
    backdrop: [{ kind: "blur", radius: { value: 16, unit: "px" } }],
    opacity: 0.96,
};

/** A popover beside the card: square, on the popover's color, without a backdrop. */
const POPOVER: JsonObject = {
    ...CARD,
    corners: Corners.all("{radius.2}"),
    fill: [
        { paint: { kind: "solid", color: "{color.popover}" }, blend: "normal", isVisible: true },
    ],
    backdrop: [],
};

/** Render an inspector of the design of some objects, each value shared, else mixed. */
function DesignInspector(properties: {
    /** The objects' designs as they start. */
    readonly objects: readonly JsonObject[];
}): JSX.Element {
    // store what the inputs give to every object, unsetting a reset value
    const locale = useLocale();
    const [objects, setObjects] = createSignal<readonly JsonObject[]>(properties.objects);
    const give = (name: string, value: JsonValue | undefined): void =>
        void setObjects((current) =>
            current.map(({ [name]: _dropped, ...rest }) =>
                value === undefined ? rest : { ...rest, [name]: value },
            ),
        );

    // render a value's field under its label
    const field = ([name, definition, label]: DesignValue): JSX.Element => (
        <ValueField
            label={locale.render(label)}
            type={typeOf(definition)}
            value={sharedValue(objects().map((object) => object[name]))}
            onValueChange={(value) => give(name, value)}
        />
    );

    return (
        <Inspector aria-label={locale.render(t.context("title")`Design`)} xstyle={styles.panel}>
            <For each={SECTIONS}>
                {(section, index) => (
                    <>
                        {/* A rule between sections */}
                        <Show when={index() > 0}>
                            <FieldSeparator />
                        </Show>

                        {/* A value of its own, or a titled group of values */}
                        {"title" in section ? (
                            <FieldSet>
                                <FieldLegend>{locale.render(section.title)}</FieldLegend>
                                <For each={section.values}>{field}</For>
                            </FieldSet>
                        ) : (
                            field(section)
                        )}
                    </>
                )}
            </For>
        </Inspector>
    );
}

/** Type a value by its schema, as a field of JSON. */
function typeOf(definition: schema.Schema): ActionValue {
    return { kind: "json", schema: JsonValue.object(toJsonSchema(definition)) };
}
