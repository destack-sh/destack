import checkIcon from "@destack/icon/phosphor/check";
import pencilIcon from "@destack/icon/phosphor/pencil-simple";
import xIcon from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { color, radius, space } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    flush,
    type JSX,
    merge,
    omit,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { type FieldControl, readText, useFieldControl, useFieldValue } from "../field/control.ts";
import { Icon } from "../icon/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";

/** How a text is changed and kept where its owner does not say. */
const DEFAULTS = { activationMode: "click", submitMode: "both" } as const;

/** The styles of a text changed in place and its parts. */
const styles = style.create({
    editable: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
    },
    // take the same room read and written, whatever the text around it is
    text: {
        boxSizing: "border-box",
        minWidth: 0,
        margin: 0,
        marginInline: `calc(-1 * ${space[1]})`,
        paddingInline: space[1],
        paddingBlock: 0,
        borderWidth: 0,
        borderRadius: radius[2],
        backgroundColor: "transparent",
        color: "inherit",
        fontFamily: "inherit",
        fontSize: "inherit",
        fontWeight: "inherit",
        lineHeight: "inherit",
        letterSpacing: "inherit",
        textAlign: "inherit",
        overflowWrap: "anywhere",
    },
    preview: {
        whiteSpace: "pre-wrap",
        cursor: { default: "text", ":is([data-disabled])": "default" },
        backgroundColor: {
            default: "transparent",
            ":hover": { default: "transparent", [media.hover]: color.hover },
            ":is([data-disabled]):hover": "transparent",
        },
        minHeight: "1lh",
    },
    empty: {
        color: color.mutedForeground,
    },
    input: {
        width: "100%",
        outlineStyle: "none",
        backgroundColor: color.muted,
        resize: "none",
        fieldSizing: "content",
        maxWidth: "100%",
    },
    line: {
        minWidth: "4ch",
        width: "auto",
    },
});

/** The properties of an element of a text changed in place, the native attributes included. */
export type EditableElementProperties<Target extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties of a text changed in place: its text, how it is changed and kept, its form. */
export type EditableProperties = EditableElementProperties & {
    /** The text. */
    readonly value?: string;
    /** The text at first while uncontrolled, none by default. */
    readonly defaultValue?: string;
    /** Handle the text as it is typed. */
    readonly onValueChange?: (value: string) => void;
    /** Handle the text a person keeps. */
    readonly onValueCommitted?: (value: string) => void;
    /** Handle the text as it was, as a person leaves the change. */
    readonly onValueReverted?: (value: string) => void;
    /** Whether the text is being changed. */
    readonly edit?: boolean;
    /** Whether the text is being changed at first while uncontrolled. */
    readonly defaultEdit?: boolean;
    /** Handle the change beginning and ending. */
    readonly onEditChange?: (isEdited: boolean) => void;
    /** What begins the change: a press by default, two, or only the owner's button. */
    readonly activationMode?: "click" | "dblclick" | "none";
    /** What keeps the change: Enter, leaving it, both by default, or only the owner's button. */
    readonly submitMode?: "enter" | "blur" | "both" | "none";
    /** Whether the text is selected as the change begins, as it is by default. */
    readonly selectOnFocus?: boolean;
    /** Whether the text has lines, which Enter breaks and the command key with Enter keeps. */
    readonly multiline?: boolean;
    /** What it shows while it is empty. */
    readonly placeholder?: string;
    /** The most characters it has, any number by default. */
    readonly maxLength?: number;
    /** Whether an empty text is put back as it was, as a title's is. */
    readonly required?: boolean;
    /** Whether it ignores the person. */
    readonly disabled?: boolean;
    /** Whether the person reads the text but does not change it. */
    readonly readOnly?: boolean;
    /** The form field name it submits its text under. */
    readonly name?: string;
    /** The id of the form it belongs to, its ancestor form by default. */
    readonly form?: string;
};

/** A text and whether it is being changed. */
export class EditableControl {
    /** The text. */
    readonly value: Accessor<string>;
    /** Whether the text is being changed. */
    readonly isEdited: Accessor<boolean>;
    /** The field around it, null outside one. */
    readonly field: FieldControl | null;
    /** The properties. */
    readonly properties: EditableProperties;
    /** The element around it. */
    element: HTMLElement | undefined;
    /** Set the text. */
    readonly #setValue: (value: string) => void;
    /** Set whether the text is being changed. */
    readonly #setEdited: (isEdited: boolean) => void;
    /** The text before the change. */
    #before = "";

    /** Follow a text. */
    constructor(properties: EditableProperties, field: FieldControl | null) {
        // follow the controlled text and change, or its own
        const [value, setValue] = createControllableSignal<string>({
            isControlled: () => "value" in properties,
            value: () => properties.value ?? "",
            defaultValue: untrack(() => properties.defaultValue) ?? "",
            onChange: (next) => properties.onValueChange?.(next),
        });
        const [isEdited, setEdited] = createControllableSignal<boolean>({
            isControlled: () => "edit" in properties,
            value: () => properties.edit === true,
            defaultValue: untrack(() => properties.defaultEdit) === true,
            onChange: (next) => properties.onEditChange?.(next),
        });
        this.value = value;
        this.isEdited = isEdited;
        this.properties = properties;
        this.field = field;
        this.#setValue = setValue;
        this.#setEdited = setEdited;
    }

    /** Report whether a person changes it. */
    isEditable(): boolean {
        return this.properties.disabled !== true && this.properties.readOnly !== true;
    }

    /** Begin the change. */
    begin(): void {
        if (this.isEditable() && !this.isEdited()) {
            this.#before = this.value();
            this.#setEdited(true);
            flush();
        }
    }

    /** Type text. */
    type(text: string): void {
        this.#setValue(text);
        flush();
    }

    /** Remember the text a change begins with, also where the owner begins it. */
    remember(): void {
        this.#before = untrack(this.value);
    }

    /** Keep the change, or put a required text back where nothing is left. */
    submit(): void {
        // end it once
        if (!this.isEdited()) {
            return;
        }
        if (this.properties.required === true && this.value().trim() === "") {
            this.cancel();
        } else {
            this.#setEdited(false);
            if (this.value() !== this.#before) {
                this.properties.onValueCommitted?.(this.value());
            }
            flush();
        }
    }

    /** Leave the change, and put the text back. */
    cancel(): void {
        if (this.isEdited()) {
            this.#setEdited(false);
            this.#setValue(this.#before);
            this.properties.onValueReverted?.(this.#before);
            flush();
        }
    }

    /** Focus the text as it is read. */
    focus(): void {
        this.element?.querySelector<HTMLElement>("[data-slot=editable-preview]")?.focus();
    }
}

/** The nearest text changed in place, null outside one. */
const EditableContext = createContext<EditableControl | null>(null);

/** Read the nearest text changed in place, refusing a part outside one. */
export function useEditable(): EditableControl {
    const control = useContext(EditableContext);
    if (control === null) {
        throw new TypeError("an editable part needs an editable around it");
    }

    return control;
}

/** Render a text a person changes where it stands. */
export function Editable(given: EditableProperties): JSX.Element {
    // edit the value of the field's form
    const bound = useFieldValue(given, { value: "value", change: "onValueChange", read: readText });
    const properties = merge(DEFAULTS, bound);
    const control = new EditableControl(properties, useFieldControl());
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "onValueCommitted",
        "onValueReverted",
        "edit",
        "defaultEdit",
        "onEditChange",
        "activationMode",
        "submitMode",
        "selectOnFocus",
        "multiline",
        "placeholder",
        "maxLength",
        "required",
        "disabled",
        "readOnly",
        "name",
        "form",
        "children",
    );

    return (
        <EditableContext value={control}>
            {renderPart("div", "editable", rest, styles.editable, {
                ref: (element: HTMLElement) => (control.element = element),
                get "data-editing"() {
                    return control.isEdited() ? "" : undefined;
                },
                get "data-disabled"() {
                    return properties.disabled === true ? "" : undefined;
                },
                get "data-readonly"() {
                    return properties.readOnly === true ? "" : undefined;
                },
                get children() {
                    return (
                        <>
                            {contentOr(properties, () => (
                                <>
                                    <EditablePreview />
                                    <EditableInput />
                                </>
                            ))}
                            <Show when={properties.name}>
                                <input
                                    type="hidden"
                                    name={properties.name}
                                    form={properties.form}
                                    value={control.value()}
                                />
                            </Show>
                        </>
                    );
                },
            })}
        </EditableContext>
    );
}

/** Render the text as it is read, which a press or Enter begins to change. */
export function EditablePreview(
    properties: EditableElementProperties<HTMLSpanElement>,
): JSX.Element {
    // show the placeholder while it is empty
    const control = useEditable();
    const locale = useLocale();
    const isActivated = (): boolean =>
        control.isEditable() && control.properties.activationMode !== "none";

    return (
        <Show when={!control.isEdited()}>
            {renderPart(
                "span",
                "editable-preview",
                properties,
                () => [
                    state.focus,
                    styles.text,
                    styles.preview,
                    control.value() === "" && styles.empty,
                ],
                {
                    get role() {
                        return isActivated() ? ("button" as const) : undefined;
                    },
                    get tabindex() {
                        return isActivated() ? 0 : undefined;
                    },
                    get "aria-description"() {
                        return isActivated() ? locale.render(t`Press Enter to edit`) : undefined;
                    },
                    get "data-disabled"() {
                        return control.isEditable() ? undefined : "";
                    },
                    onClick: ({
                        currentTarget,
                    }: Event & { readonly currentTarget: HTMLElement }) => {
                        // leave a press that selects text
                        const selection = currentTarget.ownerDocument.getSelection();
                        const isSelecting = selection?.isCollapsed === false;
                        if (control.properties.activationMode === "click" && !isSelecting) {
                            control.begin();
                        }
                    },
                    onDblClick: () => {
                        if (control.properties.activationMode === "dblclick") {
                            control.begin();
                        }
                    },
                    onKeyDown: (event: KeyboardEvent) => {
                        const isBegun = event.key === "Enter" || event.key === "F2";
                        if (isBegun && isActivated() && event.target === event.currentTarget) {
                            event.preventDefault();
                            control.begin();
                        }
                    },
                    get children() {
                        return contentOr(properties, () =>
                            control.value() === ""
                                ? (control.properties.placeholder ?? "")
                                : control.value(),
                        );
                    },
                },
            )}
        </Show>
    );
}

/** The properties of the text as it is written, the native element's attributes included. */
export type EditableInputProperties = Omit<
    JSX.InputHTMLAttributes<HTMLInputElement> & JSX.TextareaHTMLAttributes<HTMLTextAreaElement>,
    "class" | "value" | "onInput" | "ref" | "onKeyDown" | "onBlur"
> &
    ElementPartProperties;

/** Render the text as it is written, selected whole. */
export function EditableInput(properties: EditableInputProperties): JSX.Element {
    const control = useEditable();

    return (
        <Show when={control.isEdited()}>
            <EditableText {...properties} />
        </Show>
    );
}

/** Render the text while it is changed, kept by Enter or leaving it, left by Escape. */
function EditableText(properties: EditableInputProperties): JSX.Element {
    // take the focus, the text selected
    const control = useEditable();
    const locale = useLocale();
    control.remember();
    type Text = HTMLInputElement | HTMLTextAreaElement;
    const take = (element: Text): void =>
        queueMicrotask(() => {
            element.focus();
            if (control.properties.selectOnFocus !== false) {
                element.select();
            }
        });
    const press = (event: KeyboardEvent): void => {
        // break a line by Enter in a text with lines
        const isHeld = event.metaKey || event.ctrlKey;
        const isEnter = event.key === "Enter" && !event.isComposing;
        const isKept = control.properties.multiline === true ? isHeld : !event.shiftKey;
        const mode = control.properties.submitMode;
        if (isEnter && isKept && (mode === "enter" || mode === "both")) {
            event.preventDefault();
            control.submit();
            control.focus();
        } else if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            control.cancel();
            control.focus();
        }
    };
    const leave = (event: FocusEvent): void => {
        // stay while the focus goes to a button beside it
        const { relatedTarget } = event;
        const isInside =
            relatedTarget instanceof Node && control.element?.contains(relatedTarget) === true;
        const mode = control.properties.submitMode;
        if (!isInside && (mode === "blur" || mode === "both")) {
            control.submit();
        }
    };
    const rest = omit(properties, "xstyle", "style");
    const shared = {
        ...control.field?.attributes(),
        "data-slot": "editable-input",
        get "aria-label"() {
            return control.field === null ? locale.render(t.context("label")`Text`) : undefined;
        },
        get placeholder() {
            return control.properties.placeholder;
        },
        get maxlength() {
            return control.properties.maxLength;
        },
        get value() {
            return control.value();
        },
    };

    return (
        <Show
            when={control.properties.multiline === true}
            fallback={
                <input
                    {...shared}
                    {...rest}
                    ref={take}
                    onInput={(event) => control.type(event.currentTarget.value)}
                    onKeyDown={press}
                    onBlur={leave}
                    {...style.attributes(
                        [state.typed, styles.text, styles.input, styles.line, properties.xstyle],
                        properties.style,
                    )}
                />
            }
        >
            <textarea
                {...shared}
                {...rest}
                rows={1}
                ref={take}
                onInput={(event) => control.type(event.currentTarget.value)}
                onKeyDown={press}
                onBlur={leave}
                {...style.attributes(
                    [state.typed, styles.text, styles.input, properties.xstyle],
                    properties.style,
                )}
            />
        </Show>
    );
}

/** Render the button that begins the change, gone while it goes on. */
export function EditableTrigger(properties: ButtonProperties): JSX.Element {
    const control = useEditable();
    const locale = useLocale();

    return (
        <Show when={!control.isEdited() && control.isEditable()}>
            <Button
                isIconOnly
                data-slot="editable-trigger"
                variant="ghost"
                size="sm"
                aria-label={locale.render(t.context("action")`Edit`)}
                {...mergeProperties({ onClick: () => control.begin() }, properties)}
            >
                {contentOr(properties, () => (
                    <Icon icon={pencilIcon} />
                ))}
            </Button>
        </Show>
    );
}

/** Render the button that keeps the change, there while it goes on. */
export function EditableSubmit(properties: ButtonProperties): JSX.Element {
    const control = useEditable();
    const locale = useLocale();

    return (
        <Show when={control.isEdited()}>
            <Button
                isIconOnly
                data-slot="editable-submit"
                size="sm"
                aria-label={locale.render(t.context("action")`Save`)}
                {...mergeProperties(
                    {
                        onClick: () => {
                            control.submit();
                            control.focus();
                        },
                    },
                    properties,
                )}
            >
                {contentOr(properties, () => (
                    <Icon icon={checkIcon} />
                ))}
            </Button>
        </Show>
    );
}

/** Render the button that leaves the change, there while it goes on. */
export function EditableCancel(properties: ButtonProperties): JSX.Element {
    const control = useEditable();
    const locale = useLocale();

    return (
        <Show when={control.isEdited()}>
            <Button
                isIconOnly
                data-slot="editable-cancel"
                variant="ghost"
                size="sm"
                aria-label={locale.render(t.context("action")`Cancel`)}
                {...mergeProperties(
                    {
                        onClick: () => {
                            control.cancel();
                            control.focus();
                        },
                    },
                    properties,
                )}
            >
                {contentOr(properties, () => (
                    <Icon icon={xIcon} />
                ))}
            </Button>
        </Show>
    );
}
