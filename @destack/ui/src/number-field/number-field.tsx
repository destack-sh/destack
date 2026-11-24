import caretDownIcon from "@destack/icon/phosphor/caret-down";
import caretUpIcon from "@destack/icon/phosphor/caret-up";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { size, space } from "@destack/theme/tokens.stylex";
import {
    flush,
    type JSX,
    merge,
    omit,
    onCleanup,
    Show,
    untrack,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { type ButtonProperties } from "../button/index.ts";
import { useFieldControl, useFieldValue } from "../field/control.ts";
import { FormInput, isFormControl } from "../form-input/index.ts";
import { Icon } from "../icon/index.ts";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupInput,
} from "../input-group/index.ts";
import type { InputProperties } from "../input/index.ts";
import { Move } from "../interaction/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";
import {
    NumberFieldContext,
    NumberFieldControl,
    type NumberFieldOptions,
    type NumberFieldStep,
    useNumberField,
} from "./control.ts";

/** How long a button is pressed before it steps again, in milliseconds, as Base UI's. */
const REPEAT_DELAY = 400;

/** How often a pressed button steps, in milliseconds, as Base UI's. */
const REPEAT_INTERVAL = 60;

/** How far a pointer moves for each step it scrubs, in pixels, as Base UI's. */
const SCRUB_DISTANCE = 2;

/** The styles of a number field. */
const styles = style.create({
    field: {
        position: "relative",
        width: "100%",
        minWidth: 0,
    },
    input: {
        fontVariantNumeric: "tabular-nums",
    },
    steps: {
        gap: 0,
        paddingInline: space[1],
    },
    stacked: {
        display: "grid",
        gridAutoFlow: { default: "row", [media.pointerCoarse]: "column" },
    },
    step: {
        width: { default: space[5], [media.pointerCoarse]: size[2] },
        height: { default: space[3], [media.pointerCoarse]: size[1] },
        padding: 0,
        fontSize: { default: "0.625rem", [media.pointerCoarse]: "1rem" },
    },
    decrement: {
        order: { default: null, [media.pointerCoarse]: -1 },
    },
    scrub: {
        width: "fit-content",
        cursor: "ew-resize",
        userSelect: "none",
        touchAction: "none",
    },
});

/** The names of what only a number field reads. */
const OPTIONS = [
    "value",
    "defaultValue",
    "onValueChange",
    "onValueCommitted",
    "min",
    "max",
    "step",
    "smallStep",
    "largeStep",
    "snapOnStep",
    "allowOutOfRange",
    "allowWheelScrub",
    "format",
    "disabled",
    "readOnly",
    "required",
    "name",
    "form",
] as const;

/** The attributes of a number field that are its input's. */
const ATTRIBUTES = [
    "placeholder",
    "id",
    "aria-label",
    "aria-labelledby",
    "aria-describedby",
    "aria-invalid",
] as const;

/** The properties of a number field, with its element's attributes. */
export type NumberFieldProperties = NumberFieldOptions &
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | keyof NumberFieldOptions> &
    ElementPartProperties;

/** Render a field of a number, written and read as people of the language write it. */
export function NumberField(given: NumberFieldProperties): JSX.Element {
    // edit the value of the field's form
    const properties = useFieldValue(given, {
        value: "value",
        change: "onValueChange",
        read: (value) => (typeof value === "number" ? value : undefined),
    });

    // share the number with the parts
    const control = new NumberFieldControl(properties, useFieldControl(), useLocale());
    let element: HTMLElement | undefined;
    const rest = omit(properties, ...OPTIONS, ...ATTRIBUTES, "children");

    return (
        <NumberFieldContext value={control}>
            {renderPart("div", "number-field", rest, styles.field, {
                ref: (created: HTMLElement) => (element = created),
                get "data-disabled"() {
                    return control.isDisabled() ? "" : undefined;
                },
                get children() {
                    return (
                        <>
                            {properties.children ?? <NumberFieldParts />}
                            <Show when={isFormControl(properties) || control.refusal() !== ""}>
                                <FormInput
                                    name={properties.name}
                                    form={properties.form}
                                    disabled={control.isDisabled()}
                                    value={String(control.value() ?? "")}
                                    error={control.refusal()}
                                    onReset={() => control.reset()}
                                    onFocus={() => element?.querySelector("input")?.focus()}
                                />
                            </Show>
                        </>
                    );
                },
            })}
        </NumberFieldContext>
    );
}

/** Render the usual parts of a number field. */
function NumberFieldParts(): JSX.Element {
    return (
        <InputGroup>
            <NumberFieldInput />
            <InputGroupAddon align="inline-end" xstyle={styles.steps}>
                <span {...style.attrs(styles.stacked)}>
                    <NumberFieldIncrement xstyle={styles.step}>
                        <Icon icon={caretUpIcon} weight="bold" />
                    </NumberFieldIncrement>
                    <NumberFieldDecrement xstyle={[styles.step, styles.decrement]}>
                        <Icon icon={caretDownIcon} weight="bold" />
                    </NumberFieldDecrement>
                </span>
            </InputGroupAddon>
        </InputGroup>
    );
}

/** Render the input of a number field, which the arrow keys step. */
export function NumberFieldInput(
    properties: Omit<InputProperties, "value" | "type" | "name" | "form">,
): JSX.Element {
    // step by keys, and by the wheel where it may
    const control = useNumberField();
    const locale = useLocale();
    const options = control.options;
    const attributes = merge(() => attributesOf(options));
    const keys = (event: KeyboardEvent): void => {
        // step, or settle by Enter and leave the form its submit
        const direction = directionOf(event);
        if (event.isComposing) {
            return;
        } else if (event.key === "Enter") {
            control.commit();
        } else if (direction !== undefined && !event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            control.step(direction, sizeOf(event));
            flush();
            control.finish();
        }
        // move to a limit, which leaves the caret where the field has none
        else if (event.key === "Home" && options.min !== undefined) {
            event.preventDefault();
            control.toLimit("min");
        } else if (event.key === "End" && options.max !== undefined) {
            event.preventDefault();
            control.toLimit("max");
        }
    };
    const wheel = (event: WheelEvent): void => {
        // leave the wheel to the page unless the field takes it
        const isFocused = event.currentTarget === document.activeElement;
        if (options.allowWheelScrub === true && isFocused && event.deltaY !== 0) {
            event.preventDefault();
            control.step(event.deltaY < 0 ? 1 : -1, sizeOf(event));
            flush();
            control.finish();
        }
    };

    return (
        <InputGroupInput
            {...mergeProperties(
                {
                    type: "text",
                    get inputmode() {
                        return control.keyboard();
                    },
                    autocomplete: "off",
                    spellcheck: "false" as const,
                    "aria-roledescription": locale.render(t`Number field`),
                    onKeyDown: keys,
                    onWheel: wheel,
                    onInput: (event: InputEvent & { readonly currentTarget: HTMLInputElement }) =>
                        control.type(event.currentTarget.value),
                    onBlur: () => control.commit(),
                },
                merge(
                    () => (control.refusal() === "" ? {} : { "aria-invalid": "true" as const }),
                    attributes,
                    omit(properties, "xstyle"),
                ),
            )}
            value={control.shown()}
            disabled={control.isDisabled()}
            readonly={options.readOnly}
            required={options.required}
            {...(options.form === undefined ? {} : { form: options.form })}
            xstyle={[styles.input, properties.xstyle]}
        />
    );
}

/** Render the button that steps a number up. */
export function NumberFieldIncrement(properties: ButtonProperties): JSX.Element {
    const locale = useLocale();

    return (
        <NumberFieldStepButton {...properties} direction={1} label={locale.render(t`Increase`)} />
    );
}

/** Render the button that steps a number down. */
export function NumberFieldDecrement(properties: ButtonProperties): JSX.Element {
    const locale = useLocale();

    return (
        <NumberFieldStepButton {...properties} direction={-1} label={locale.render(t`Decrease`)} />
    );
}

/** Render a button that steps a number, again and again while it is pressed. */
function NumberFieldStepButton(
    properties: ButtonProperties & {
        /** The way it steps. */
        readonly direction: 1 | -1;
        /** What it does. */
        readonly label: string;
    },
): JSX.Element {
    // step as it is pressed, then again after a while
    const control = useNumberField();
    const { clock } = useEnvironment();
    let timer: number | undefined;
    let isPressed = false;
    const stop = (): void => {
        // settle once, as the button is let go
        clock.clearTimeout(timer);
        if (isPressed) {
            isPressed = false;
            control.finish();
        }
    };
    const repeat = (amount: NumberFieldStep, delay: number): void => {
        timer = clock.setTimeout(() => {
            // stop at the limit, where the button takes no more events
            control.step(properties.direction, amount);
            flush();
            if (control.canStep(properties.direction)) {
                repeat(amount, REPEAT_INTERVAL);
            } else {
                stop();
            }
        }, delay);
    };
    onCleanup(stop);
    const start = (event: PointerEvent): void => {
        // leave the focus where it is, and other buttons of the pointer alone
        if (event.button === 0) {
            event.preventDefault();
            stop();
            isPressed = true;
            control.step(properties.direction, sizeOf(event));
            flush();
            repeat(sizeOf(event), REPEAT_DELAY);
        }
    };

    return (
        <InputGroupButton
            data-slot={
                properties.direction === 1 ? "number-field-increment" : "number-field-decrement"
            }
            tabindex={-1}
            aria-label={properties.label}
            disabled={!control.canStep(properties.direction)}
            onPointerDown={start}
            onPointerUp={stop}
            onPointerLeave={stop}
            onPointerCancel={stop}
            onClick={(event) => {
                // step by what presses without a pointer, such as a screen reader
                if (event.detail === 0) {
                    control.step(properties.direction);
                    flush();
                    control.finish();
                }
            }}
            {...omit(properties, "direction", "label")}
        />
    );
}

/** Render what a pointer drags sideways to step a number, such as its label. */
export function NumberFieldScrubArea(
    properties: Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> & ElementPartProperties,
): JSX.Element {
    // step for each few pixels
    const control = useNumberField();
    const { clock } = useEnvironment();
    let left = 0;
    const move = new Move(
        {
            orientation: "horizontal",
            keys: [],
            distance: SCRUB_DISTANCE,
            onMoveStart: () => (left = 0),
            onMoveEnd: () => control.finish(),
            onMove: (event) => {
                left += event.deltaX;
                const steps = Math.trunc(left / SCRUB_DISTANCE);
                if (steps !== 0) {
                    left -= steps * SCRUB_DISTANCE;
                    const amount = event.shiftKey ? "large" : event.altKey ? "small" : "step";
                    control.step(steps > 0 ? 1 : -1, amount, Math.abs(steps));
                    flush();
                }
            },
        },
        clock,
    );

    return renderPart(
        "span",
        "number-field-scrub-area",
        untrack(() => properties),
        styles.scrub,
        { role: "presentation" as const, ...move.attributes() },
    );
}

/** Read the attributes of a number field that are its input's. */
function attributesOf(
    options: NumberFieldOptions,
): Pick<InputProperties, (typeof ATTRIBUTES)[number]> {
    // leave out those not set, which the field around it sets
    const label = options["aria-label"];
    const labels = options["aria-labelledby"];
    const descriptions = options["aria-describedby"];
    const isInvalid = options["aria-invalid"];

    return {
        ...(options.id === undefined ? {} : { id: options.id }),
        ...(options.placeholder === undefined ? {} : { placeholder: options.placeholder }),
        ...(label === undefined ? {} : { "aria-label": label }),
        ...(labels === undefined ? {} : { "aria-labelledby": labels }),
        ...(descriptions === undefined ? {} : { "aria-describedby": descriptions }),
        ...(isInvalid === undefined ? {} : { "aria-invalid": isInvalid }),
    };
}

/** Read the way a key steps, undefined for another key. */
function directionOf(event: KeyboardEvent): 1 | -1 | undefined {
    return event.key === "ArrowUp" || event.key === "PageUp"
        ? 1
        : event.key === "ArrowDown" || event.key === "PageDown"
          ? -1
          : undefined;
}

/** Read how far an event steps: finer with Alt, coarser with Shift and by pages. */
function sizeOf(event: KeyboardEvent | WheelEvent | PointerEvent): NumberFieldStep {
    const isPage = event instanceof KeyboardEvent && event.key.startsWith("Page");

    return event.shiftKey || isPage ? "large" : event.altKey ? "small" : "step";
}
