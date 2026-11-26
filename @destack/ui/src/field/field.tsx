import { state } from "@destack/theme/state";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, size, space, weight, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    children,
    createUniqueId,
    For,
    type FormField,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { t } from "@destack/locale";
import { field } from "@destack/object";
import { Button } from "../button/index.ts";
import { Label, type LabelProperties } from "../label/index.ts";
import { FieldContext, FieldControl, useFieldControl } from "./control.ts";
import { Separator } from "../separator/index.ts";
import { mergeProperties, renderPart, type Render } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The width of a field group above which responsive fields lay out in a row. */
const RESPONSIVE_QUERY = "@container field-group (min-width: 28rem)";

/** The directions a field lays out its label and control in. */
const ORIENTATIONS = {
    vertical: { title: t.context("option")`Vertical` },
    horizontal: { title: t.context("option")`Horizontal` },
    responsive: {
        title: t.context("option")`Responsive`,
        description: t`In a row where its field group is wide enough, else in a column.`,
    },
};

/** The looks of a legend: a section heading or a label. */
const LEGEND_VARIANTS = {
    legend: { title: t.context("option")`Legend` },
    label: { title: t.context("option")`Label` },
};

/** The properties people change in a field's controls and inspector. */
const FIELD_PROPERTIES = {
    orientation: field
        .enum(ORIENTATIONS)
        .labeled(t.context("label")`Orientation`)
        .described(t`Whether the label sits above its control or beside it.`)
        .default("vertical"),
    invalid: field
        .boolean()
        .labeled(t.context("label")`Invalid`)
        .described(t`Marks the control and colors the field, as for a refused value.`)
        .default(false),
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses input in the field's control.`)
        .default(false),
};

/** The properties people change in a legend's controls and inspector. */
const FIELD_LEGEND_PROPERTIES = {
    variant: field
        .enum(LEGEND_VARIANTS)
        .labeled(t.context("label")`Variant`)
        .described(t`Whether the legend heads a section or reads as a label.`)
        .default("legend"),
};

/** An error as it appears. */
const appear = style.keyframes({
    from: { opacity: 0, transform: "translateY(-0.25rem)" },
});

/** The styles of a field and its elements. */
const styles = style.create({
    field: {
        display: "flex",
        width: "100%",
        gap: space[3],
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
    description: {
        color: color.mutedForeground,
    },
    error: {
        color: swatch.destructiveText,
        animationName: { default: appear, [media.motionReduce]: "none" },
        animationDuration: motion.durationShort,
        animationTimingFunction: motion.easingStandard,
    },
    below: {
        flexBasis: "100%",
    },
    label: {
        position: "relative",
        // push a control after it to the end
        flexGrow: { default: null, ":first-child": 1 },
        flexShrink: 0,
        maxWidth: `calc(100% - ${size[3]})`,
        "::after": {
            content: "''",
            position: "absolute",
            insetBlock: `calc(-1 * ${space[1]})`,
            insetInline: `calc(-1 * ${space[3]})`,
        },
    },
    conflict: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
        color: color.mutedForeground,
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
        flexWrap: { default: "nowrap", ":has(> [data-slot=field-error])": "wrap" },
        alignItems: {
            default: "center",
            ":has(> [data-slot=field-content])": "flex-start",
        },
    },
    responsive: {
        flexDirection: { default: "column", [RESPONSIVE_QUERY]: "row" },
        alignItems: { default: "stretch", [RESPONSIVE_QUERY]: "center" },
    },
});

/** The text style of a legend in each variant. */
const legends = { legend: text.base, label: text.xs } as const;

/** The direction a field lays out its label and control in. */
export type FieldOrientation = keyof typeof ORIENTATIONS;

/** The look of a legend: a section heading or a label. */
export type FieldLegendVariant = keyof typeof LEGEND_VARIANTS;

/** The properties of an element of a field, the native element's attributes included. */
export type FieldElementProperties<Target extends HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
};

/** The properties of a field besides its declared ones: its native attributes and form value. */
export interface FieldAttributes extends FieldElementProperties<HTMLDivElement> {
    /** The value of a form its control edits, which tells what is wrong with it. */
    readonly field?: FormField<unknown> | undefined;
}

/** The properties of a legend besides its declared ones: its native attributes. */
export type FieldLegendAttributes = FieldElementProperties<HTMLLegendElement>;

/** The properties of a field's error message. */
export interface FieldErrorProperties extends FieldElementProperties<HTMLDivElement> {
    /** The errors to list without children, each shown once, the field's own by default. */
    readonly errors?: readonly ({ readonly message?: string | undefined } | undefined)[];
}

/** Render a field that connects its label, control, descriptions and errors. */
export const Field = defineComponent({
    name: "field",
    properties: FIELD_PROPERTIES,
    render(properties: RenderProperties<FieldAttributes, typeof FIELD_PROPERTIES>): JSX.Element {
        // share one control with the field's elements
        const rest = omit(properties, "orientation", "invalid", "disabled", "field");
        const control = new FieldControl(
            () => properties.invalid,
            () => properties.disabled,
            untrack(() => properties.field),
            untrack(() => properties.orientation) === "horizontal",
        );

        // show what the browser says is wrong, and settle on the value as the focus leaves
        const recheck = (event: Event): void => {
            if (isCheckable(event.target)) {
                control.recheck(event.target);
            }
        };
        const refuse = (event: Event): void => {
            // take the browser's place, and its focus on the form's first refused control
            const { target } = event;
            if (isCheckable(target) && control.refuse(target)) {
                event.preventDefault();
                const first = [...(target.form?.elements ?? [])].find(
                    (each) => isCheckable(each) && !each.validity.valid,
                );
                if (first === target) {
                    target.focus();
                }
            }
        };

        // settle on the value as the focus leaves
        const { clock } = useEnvironment();
        let press: AbortController | undefined;
        let isPressed = false;
        onCleanup(() => press?.abort());
        const enter = (event: FocusEvent & { readonly currentTarget: HTMLElement }): void => {
            // tell whether a press outside takes the focus
            if (press === undefined) {
                // follow presses until the value is settled on
                press = new AbortController();
                const { signal } = press;
                const { ownerDocument } = event.currentTarget;
                const options = { signal, capture: true };
                ownerDocument.addEventListener("pointerdown", () => (isPressed = true), options);
                ownerDocument.addEventListener("pointerup", () => (isPressed = false), options);
                ownerDocument.addEventListener("pointercancel", () => (isPressed = false), options);
            }
        };
        const leave = (event: FocusEvent & { readonly currentTarget: HTMLElement }): void => {
            // stay while the focus does, or leaves with the window
            const { currentTarget, relatedTarget } = event;
            const { ownerDocument } = currentTarget;
            const isInside = relatedTarget instanceof Node && currentTarget.contains(relatedTarget);
            if (isInside || !ownerDocument.hasFocus()) {
                return;
            }
            const settle = (): void => {
                // stop following presses
                press?.abort();
                press = undefined;
                isPressed = false;
                control.value?.settle();
            };

            // settle once a press is over, so an error never moves what is pressed
            if (isPressed && press !== undefined) {
                const options = { signal: press.signal, capture: true, once: true };
                const wait = (): void => void clock.setTimeout(settle, 0);
                ownerDocument.addEventListener("pointerup", wait, options);
                ownerDocument.addEventListener("pointercancel", wait, options);
            } else {
                settle();
            }
        };

        return (
            <FieldContext value={control}>
                {renderPart(
                    "div",
                    "field",
                    rest,
                    () => [styles.field, orientations[properties.orientation]],
                    mergeProperties(
                        {
                            role: "group" as const,
                            get "data-orientation"() {
                                return properties.orientation;
                            },
                            get "data-invalid"() {
                                return control.isInvalid() ? "" : undefined;
                            },
                            get "data-disabled"() {
                                return control.isDisabled() ? "" : undefined;
                            },
                        },
                        {
                            onFocusIn: enter,
                            onFocusOut: leave,
                            onInput: recheck,
                            onChange: recheck,
                            ref: (element: HTMLElement) =>
                                element.addEventListener("invalid", refuse, { capture: true }),
                        },
                    ),
                )}
            </FieldContext>
        );
    },
});

/** The properties of a field, its native attributes included. */
export type FieldProperties = ComponentProperties<typeof Field>;

/** Render the label of the nearest field's control. */
export function FieldLabel(properties: LabelProperties): JSX.Element {
    const control = useFieldControl();
    const rest = omit(properties, "xstyle", "style");

    return (
        <Label
            id={control?.labelId}
            data-slot="field-label"
            for={control?.isGrouped() === true ? undefined : control?.id}
            {...rest}
            xstyle={[
                control?.isAcross === true && styles.label,
                control?.isDisabled() === true && state.inert,
                properties.xstyle,
            ]}
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

    return renderPart("p", "field-description", properties, [text.xs, styles.description], {
        id,
    });
}

/** Render the nearest field's errors as an alert, or nothing when there are none. */
export function FieldError(properties: FieldErrorProperties): JSX.Element {
    // list the children, else each distinct error message
    const rest = omit(properties, "errors", "children");
    const content = children(() => properties.children);
    const control = useFieldControl();
    const given = (): readonly ({ readonly message?: string | undefined } | undefined)[] =>
        properties.errors ?? [{ message: control?.problem() }];
    const messages = (): string[] => [...new Set(given().flatMap((error) => error?.message ?? []))];

    // describe the field's control while there is something to show
    const id = createUniqueId();
    const isShown = (): boolean => content.toArray().length > 0 || messages().length > 0;
    control?.describe(id, isShown);
    control?.showError(id);

    return (
        <Show when={isShown()}>
            {renderPart(
                "div",
                "field-error",
                rest,
                () => [text.xs, styles.error, control?.isAcross === true && styles.below],
                {
                    id: id,
                    role: "alert" as const,
                    get children() {
                        return (
                            <Show when={content.toArray().length === 0} fallback={content()}>
                                <Show when={messages().length > 1} fallback={messages()[0]}>
                                    <ul {...style.attrs(styles.errors)}>
                                        <For each={messages()}>
                                            {(message) => <li>{message}</li>}
                                        </For>
                                    </ul>
                                </Show>
                            </Show>
                        );
                    },
                },
            )}
        </Show>
    );
}

/** The properties of what a field shows of a value someone else set. */
export type FieldConflictProperties = Omit<FieldElementProperties<HTMLDivElement>, "children"> & {
    /** Write their value, by default as it is where it is text or a number. */
    readonly children?: (value: unknown) => JSX.Element;
};

/** Render what someone else set the field's value to meanwhile, to take or to leave. */
export function FieldConflict(properties: FieldConflictProperties): JSX.Element {
    // write their value
    const locale = useLocale();
    const control = useFieldControl();
    const value = control?.value;
    const rest = omit(properties, "children");
    const write = (theirs: unknown): JSX.Element =>
        properties.children?.(theirs) ??
        (typeof theirs === "number"
            ? locale.number(theirs)
            : typeof theirs === "string"
              ? theirs
              : "");

    // describe the field's control while there is something to show
    const id = createUniqueId();
    control?.describe(id, () => value?.conflict() !== undefined);

    return (
        <Show when={value?.conflict()}>
            {(conflict) =>
                renderPart("div", "field-conflict", rest, [text.xs, styles.conflict], {
                    id: id,
                    role: "status" as const,
                    get children() {
                        return (
                            <>
                                <span>
                                    {locale.render(t.context("title")`Changed meanwhile:`)}{" "}
                                    <b>{write(conflict().value)}</b>
                                </span>
                                <Button size="sm" variant="outline" onClick={() => value?.take()}>
                                    {locale.render(t.context("action")`Use this`)}
                                </Button>
                                <Button size="sm" variant="ghost" onClick={() => value?.keep()}>
                                    {locale.render(t.context("action")`Keep mine`)}
                                </Button>
                            </>
                        );
                    },
                })
            }
        </Show>
    );
}

/** Report whether an element is a control the browser checks. */
function isCheckable(
    element: unknown,
): element is HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement {
    return (
        element instanceof HTMLInputElement ||
        element instanceof HTMLSelectElement ||
        element instanceof HTMLTextAreaElement
    );
}

/** Render a fieldset that groups related fields. */
export function FieldSet(properties: FieldElementProperties<HTMLFieldSetElement>): JSX.Element {
    return renderPart("fieldset", "field-set", properties, styles.set);
}

/** Render the legend of a fieldset as a heading or a label. */
export const FieldLegend = defineComponent({
    name: "fieldLegend",
    properties: FIELD_LEGEND_PROPERTIES,
    render(
        properties: RenderProperties<FieldLegendAttributes, typeof FIELD_LEGEND_PROPERTIES>,
    ): JSX.Element {
        const rest = omit(properties, "variant");

        return renderPart(
            "legend",
            "field-legend",
            rest,
            () => [legends[properties.variant], styles.legend],
            {
                get "data-variant"() {
                    return properties.variant;
                },
            },
        );
    },
});

/** The properties of a legend, its native attributes included. */
export type FieldLegendProperties = ComponentProperties<typeof FieldLegend>;

/** Render a stack of fields that responsive fields measure their width against. */
export function FieldGroup(properties: FieldElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "field-group", properties, styles.group);
}

/** Render a column beside a control with its label and description. */
export function FieldContent(properties: FieldElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "field-content", properties, styles.content);
}

/** Render the title of a field as text instead of a label. */
export function FieldTitle(properties: FieldElementProperties<HTMLDivElement>): JSX.Element {
    const control = useFieldControl();

    return renderPart("div", "field-label", properties, () => [
        text.sm,
        styles.title,
        control?.isDisabled() === true && state.inert,
    ]);
}

/** Render a line between fields, with optional text in its middle. */
export function FieldSeparator(properties: FieldElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "children");
    const content = children(() => properties.children);

    return renderPart("div", "field-separator", rest, [text.xs, styles.separator], {
        get "data-content"() {
            return content.toArray().length > 0 ? "" : undefined;
        },
        get children() {
            return (
                <>
                    <Separator xstyle={styles.line} />
                    <Show when={content.toArray().length > 0}>
                        <span
                            data-slot="field-separator-content"
                            {...style.attrs(styles.separatorContent)}
                        >
                            {content()}
                        </span>
                    </Show>
                </>
            );
        },
    });
}
