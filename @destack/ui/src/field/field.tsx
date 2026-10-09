import * as style from "@destack/style";
import { color, space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { children, createUniqueId, For, type JSX, merge, omit, Show } from "@destack/view";
import { Label, type LabelProperties } from "../label/index.ts";
import { FieldContext, FieldControl, useFieldControl } from "./control.ts";
import { Separator } from "../separator/index.ts";
import { renderPart } from "../part/index.ts";

/** The width of a field group above which responsive fields lay out in a row. */
const RESPONSIVE_QUERY = "@container field-group (min-width: 28rem)";

/** The orientation of a field that sets none. */
const DEFAULTS: Required<Pick<FieldProperties, "orientation">> = { orientation: "vertical" };

/** The variant of a legend that sets none. */
const LEGEND_DEFAULTS: Required<Pick<FieldLegendProperties, "variant">> = { variant: "legend" };

/** The styles of a field and its elements. */
const styles = style.create({
    field: {
        display: "flex",
        width: "100%",
        gap: space[3],
    },
    invalid: {
        color: color.destructive,
    },
    set: {
        display: "flex",
        flexDirection: "column",
        gap: space[5],
        minWidth: 0,
    },
    legend: {
        marginBottom: space[3],
        fontWeight: weight.medium,
    },
    group: {
        containerName: "field-group",
        containerType: "inline-size",
        display: "flex",
        flexDirection: "column",
        width: "100%",
        gap: space[6],
    },
    content: {
        display: "flex",
        flex: 1,
        flexDirection: "column",
        gap: space[2],
    },
    title: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        width: "fit-content",
        fontWeight: weight.medium,
    },
    disabled: {
        opacity: 0.5,
    },
    description: {
        color: color.mutedForeground,
    },
    error: {
        color: color.destructive,
    },
    errors: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        paddingInlineStart: space[4],
    },
    separator: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        color: color.mutedForeground,
    },
    line: {
        position: "absolute",
        insetInline: 0,
        top: "50%",
    },
    separatorContent: {
        position: "relative",
        paddingInline: space[2],
        backgroundColor: color.background,
    },
});

/** The layout of a field's label and control in each orientation. */
const orientations = style.create({
    vertical: {
        flexDirection: "column",
    },
    horizontal: {
        flexDirection: "row",
        alignItems: "center",
    },
    responsive: {
        flexDirection: { default: "column", [RESPONSIVE_QUERY]: "row" },
        alignItems: { default: "stretch", [RESPONSIVE_QUERY]: "center" },
    },
});

/** The text style of a legend in each variant. */
const legends = { legend: text.callout, label: text.footnote } as const;

/** The direction a field lays out its label and control in. */
export type FieldOrientation = "vertical" | "horizontal" | "responsive";

/** The look of a legend: a section heading or a label. */
export type FieldLegendVariant = "legend" | "label";

/** The properties of an element of a field, the native element's attributes included. */
export type FieldElementProperties<Target extends HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a field. */
export interface FieldProperties extends FieldElementProperties<HTMLDivElement> {
    /** The direction of its label and control, vertical by default. */
    readonly orientation?: FieldOrientation;
    /** Whether the value is invalid, which marks the control and colors the field. */
    readonly invalid?: boolean;
    /** Whether the field is disabled, which disables the control. */
    readonly disabled?: boolean;
}

/** The properties of a legend. */
export interface FieldLegendProperties extends FieldElementProperties<HTMLLegendElement> {
    /** The look, legend by default. */
    readonly variant?: FieldLegendVariant;
}

/** The properties of a field's error message. */
export interface FieldErrorProperties extends FieldElementProperties<HTMLDivElement> {
    /** The errors to list when there are no children, each shown once by message. */
    readonly errors?: readonly ({ readonly message?: string | undefined } | undefined)[];
}

/** Render a field that connects its label, control, descriptions and errors. */
export function Field(properties: FieldProperties): JSX.Element {
    // share one control with the field's elements
    const field = merge(DEFAULTS, properties);
    const rest = omit(field, "orientation", "invalid", "disabled", "xstyle", "style", "children");
    const control = new FieldControl(
        () => field.invalid === true,
        () => field.disabled === true,
    );

    return (
        <FieldContext value={control}>
            <div
                role="group"
                data-slot="field"
                data-orientation={field.orientation}
                data-invalid={control.isInvalid() ? "true" : undefined}
                data-disabled={control.isDisabled() ? "true" : undefined}
                {...rest}
                {...style.attributes(
                    [
                        styles.field,
                        orientations[field.orientation],
                        control.isInvalid() && styles.invalid,
                        field.xstyle,
                    ],
                    field.style,
                )}
            >
                {field.children}
            </div>
        </FieldContext>
    );
}

/** Render the label of the nearest field's control. */
export function FieldLabel(properties: LabelProperties): JSX.Element {
    const control = useFieldControl();
    const rest = omit(properties, "xstyle", "style");

    return (
        <Label
            id={control?.isGrouped() === true ? control.labelId : undefined}
            data-slot="field-label"
            for={control?.isGrouped() === true ? undefined : control?.id}
            {...rest}
            xstyle={[control?.isDisabled() === true && styles.disabled, properties.xstyle]}
        />
    );
}

/** Render a description of the nearest field's control. */
export function FieldDescription(
    properties: FieldElementProperties<HTMLParagraphElement>,
): JSX.Element {
    // describe the field's control for as long as the description renders
    const id = createUniqueId();
    useFieldControl()?.describe(id, () => true);

    return renderPart("p", "field-description", properties, [text.footnote, styles.description], {
        id,
    });
}

/** Render the nearest field's errors as an alert, or nothing when there are none. */
export function FieldError(properties: FieldErrorProperties): JSX.Element {
    // list the children, else each distinct error message
    const rest = omit(properties, "errors", "xstyle", "style", "children");
    const content = children(() => properties.children);
    const messages = (): string[] => [
        ...new Set(properties.errors?.flatMap((error) => error?.message ?? []) ?? []),
    ];

    // describe the field's control while there is something to show
    const id = createUniqueId();
    const isShown = (): boolean => content.toArray().length > 0 || messages().length > 0;
    useFieldControl()?.describe(id, isShown);

    return (
        <Show when={isShown()}>
            <div
                id={id}
                role="alert"
                data-slot="field-error"
                {...rest}
                {...style.attributes(
                    [text.footnote, styles.error, properties.xstyle],
                    properties.style,
                )}
            >
                <Show when={content.toArray().length === 0} fallback={content()}>
                    <Show when={messages().length > 1} fallback={messages()[0]}>
                        <ul {...style.attrs(styles.errors)}>
                            <For each={messages()}>{(message) => <li>{message}</li>}</For>
                        </ul>
                    </Show>
                </Show>
            </div>
        </Show>
    );
}

/** Render a fieldset that groups related fields. */
export function FieldSet(properties: FieldElementProperties<HTMLFieldSetElement>): JSX.Element {
    return renderPart("fieldset", "field-set", properties, styles.set);
}

/** Render the legend of a fieldset as a heading or a label. */
export function FieldLegend(properties: FieldLegendProperties): JSX.Element {
    const legend = merge(LEGEND_DEFAULTS, properties);
    const rest = omit(legend, "variant", "xstyle", "style");

    return (
        <legend
            data-slot="field-legend"
            data-variant={legend.variant}
            {...rest}
            {...style.attributes(
                [legends[legend.variant], styles.legend, legend.xstyle],
                legend.style,
            )}
        />
    );
}

/** Render a stack of fields that responsive fields measure their width against. */
export function FieldGroup(properties: FieldElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "field-group", properties, styles.group);
}

/** Render a column beside a control that holds its label and description. */
export function FieldContent(properties: FieldElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "field-content", properties, styles.content);
}

/** Render the title of a field as text instead of a label. */
export function FieldTitle(properties: FieldElementProperties<HTMLDivElement>): JSX.Element {
    const control = useFieldControl();

    return renderPart("div", "field-label", properties, () => [
        text.callout,
        styles.title,
        control?.isDisabled() === true && styles.disabled,
    ]);
}

/** Render a line between fields, with optional text in its middle. */
export function FieldSeparator(properties: FieldElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "xstyle", "style", "children");
    const content = children(() => properties.children);

    return (
        <div
            data-slot="field-separator"
            data-content={content.toArray().length > 0 ? "true" : "false"}
            {...rest}
            {...style.attributes(
                [text.footnote, styles.separator, properties.xstyle],
                properties.style,
            )}
        >
            <Separator xstyle={styles.line} />
            <Show when={content.toArray().length > 0}>
                <span data-slot="field-separator-content" {...style.attrs(styles.separatorContent)}>
                    {content()}
                </span>
            </Show>
        </div>
    );
}
