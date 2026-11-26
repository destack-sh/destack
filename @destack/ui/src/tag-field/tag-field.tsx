import { field } from "@destack/object";
import xIcon from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import { color, motion, radius, size, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createSignal,
    flush,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { badgeVariants } from "../badge/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import { type FieldControl, useFieldControl, useFieldValue } from "../field/control.ts";
import { Icon } from "../icon/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { VisuallyHidden } from "../visually-hidden/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** What divides the tags a person types or pastes, commas and line breaks by default. */
const DELIMITER = /[,\n]/u;

/** The least room the text to type in takes before it goes to a line of its own. */
const LEAST_TEXT = `calc(16 * ${space[1]})`;

/** The properties people change in a tag field's controls and inspector. */
const TAG_FIELD_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(
            t`Refuses input where the tags cannot change now, as its field does when absent.`,
        )
        .optional(),
    readOnly: field
        .boolean()
        .labeled(t.context("label")`Read only`)
        .described(t`Shows the tags, which a person cannot change.`)
        .default(false),
};

/** The styles of a tags field and its parts. */
const styles = style.create({
    field: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[1],
        width: "100%",
        minWidth: 0,
        minHeight: size[2],
        paddingBlock: `calc(${space[1]} - ${stroke.border})`,
        paddingInline: space[2],
        borderWidth: stroke.border,
        borderColor: {
            default: color.input,
            ":has(input:focus-visible)": color.ring,
            ":has([aria-invalid=true])": swatch.destructiveMark,
        },
        borderRadius: radius[3],
        opacity: { default: 1, ":is([data-disabled])": 0.5 },
        cursor: { default: "text", ":is([data-disabled])": "not-allowed" },
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        outlineStyle: { default: "none", ":has(input:focus-visible)": "solid" },
        outlineWidth: stroke.ring,
        outlineOffset: `calc(-1 * ${stroke.border})`,
        outlineColor: {
            default: color.ring,
            ":has([aria-invalid=true])": `color-mix(in oklab, ${swatch.destructiveMark} 20%, transparent)`,
        },
    },
    list: {
        display: "contents",
    },
    tag: {
        maxWidth: "100%",
        cursor: "default",
        userSelect: "none",
        overflow: { default: "hidden", [media.pointerCoarse]: "visible" },
    },
    words: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
    },
    edit: {
        minWidth: "1ch",
        maxWidth: "100%",
        padding: 0,
        borderWidth: 0,
        outlineStyle: "none",
        backgroundColor: "transparent",
        color: "inherit",
        fontFamily: "inherit",
        fontSize: "inherit",
        fieldSizing: "content",
    },
    remove: {
        display: "inline-flex",
        padding: { default: 0, [media.pointerCoarse]: space[2] },
        margin: { default: 0, [media.pointerCoarse]: `calc(-1 * ${space[2]})` },
        marginInlineStart: { default: 0, [media.pointerCoarse]: `calc(-1 * ${space[1]})` },
        borderWidth: 0,
        backgroundColor: "transparent",
        color: "inherit",
        cursor: { default: "pointer", ":disabled": "not-allowed" },
    },
    clear: {
        marginInlineStart: "auto",
        marginBlock: `calc(-1 * ${space[1]})`,
    },
    input: {
        flexGrow: 1,
        flexBasis: LEAST_TEXT,
        minWidth: LEAST_TEXT,
        paddingInline: space[1],
        borderWidth: 0,
        outlineStyle: "none",
        backgroundColor: "transparent",
        color: "inherit",
        fontFamily: "inherit",
    },
});

/** Why a tag is refused: it is there already, there are enough, or the owner does not take it. */
export type TagRefusal = "duplicate" | "max" | "invalid";

/** The properties of an element of a tags field, the native element's attributes included. */
export type TagFieldElementProperties<Target extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties of a tag field besides its declared ones: its tags, rules and attributes. */
export type TagFieldAttributes = TagFieldElementProperties & {
    /** The tags. */
    readonly value?: readonly string[];
    /** The tags at first while uncontrolled, none by default. */
    readonly defaultValue?: readonly string[];
    /** Handle the tags as one comes, goes or changes. */
    readonly onValueChange?: (value: readonly string[]) => void;
    /** The text typed so far. */
    readonly inputValue?: string;
    /** Handle the text as it is typed. */
    readonly onInputValueChange?: (text: string) => void;
    /** The most tags it takes, any number by default. */
    readonly max?: number;
    /** Whether it takes more, and is invalid while it has them. */
    readonly allowOverflow?: boolean;
    /** Whether it takes a tag twice. */
    readonly allowDuplicates?: boolean;
    /** The most characters a tag has, any number by default. */
    readonly maxLength?: number;
    /** What divides the tags a person types or pastes, commas and line breaks by default. */
    readonly delimiter?: RegExp;
    /** Write a tag as it is kept, without the spaces around it by default. */
    readonly sanitizeValue?: (tag: string) => string;
    /** Report whether a tag is one it takes, any by default. */
    readonly validate?: (tag: string, tags: readonly string[]) => boolean;
    /** Handle a tag it does not take. */
    readonly onValueInvalid?: (tag: string, refusal: TagRefusal) => void;
    /** What becomes of the text as a person leaves the field: a tag by default. */
    readonly blurBehavior?: "add" | "clear" | "keep";
    /** Whether a person changes a tag in place, as they do by default. */
    readonly editable?: boolean;
    /** The form field name each tag is submitted under. */
    readonly name?: string;
    /** The id of the form it belongs to, its ancestor form by default. */
    readonly form?: string;
};

/** The tags of a field and the text of the next one. */
export class TagFieldControl {
    /** The tags. */
    readonly values: Accessor<readonly string[]>;
    /** The text typed so far. */
    readonly typed: Accessor<string>;
    /** Why the text is refused, undefined while it is not. */
    readonly refusal: Accessor<TagRefusal | undefined>;
    /** The tag a person changes in place, by its place. */
    readonly edited: Accessor<number | undefined>;
    /** What was last told of. */
    readonly news: Accessor<
        { readonly kind: "added" | "removed"; readonly tag: string } | undefined
    >;
    /** The field around it, null outside one. */
    readonly field: FieldControl | null;
    /** The properties. */
    readonly properties: RenderProperties<TagFieldAttributes, typeof TAG_FIELD_PROPERTIES>;
    /** The element of the field. */
    element: HTMLElement | undefined;
    /** Set the tags. */
    readonly #setValues: (values: readonly string[]) => void;
    /** Set the text. */
    readonly #setTyped: (text: string) => void;
    /** Set why the text is refused. */
    readonly #setRefusal: (refusal: TagRefusal | undefined) => void;
    /** Set the tag a person changes. */
    readonly #setEdited: (place: number | undefined) => void;
    /** Set what was last told of. */
    readonly #setNews: (kind: "added" | "removed", tag: string) => void;

    /** Follow a field's tags. */
    constructor(
        properties: RenderProperties<TagFieldAttributes, typeof TAG_FIELD_PROPERTIES>,
        fieldControl: FieldControl | null,
    ) {
        // follow the controlled tags and text, or the field's own
        const [values, setValues] = createControllableSignal<readonly string[]>({
            isControlled: () => "value" in properties,
            value: () => properties.value ?? [],
            defaultValue: untrack(() => properties.defaultValue) ?? [],
            onChange: (next) => properties.onValueChange?.(next),
        });
        const [typed, setTyped] = createControllableSignal<string>({
            isControlled: () => "inputValue" in properties,
            value: () => properties.inputValue ?? "",
            defaultValue: "",
            onChange: (next) => properties.onInputValueChange?.(next),
        });
        this.values = values;
        this.typed = typed;
        this.properties = properties;
        this.field = fieldControl;
        this.#setValues = setValues;
        this.#setTyped = setTyped;

        // start without a refusal, a tag to change, or news
        const owned = { ownedWrite: true };
        const [refusal, setRefusal] = createSignal<TagRefusal | undefined>(undefined, owned);
        const [edited, setEdited] = createSignal<number | undefined>(undefined, owned);
        const [news, setNews] = createSignal<ReturnType<typeof this.news>>(undefined, owned);
        this.refusal = refusal;
        this.edited = edited;
        this.news = news;
        this.#setRefusal = (next) => void setRefusal(next);
        this.#setEdited = (next) => void setEdited(next);
        this.#setNews = (kind, tag) => void setNews({ kind, tag });
    }

    /** Report whether the field refuses the person, by its own value or its field's. */
    isDisabled(): boolean {
        return this.properties.disabled === true || this.field?.isDisabled() === true;
    }

    /** Report whether a person changes the tags. */
    isEditable(): boolean {
        return !this.isDisabled() && !this.properties.readOnly;
    }

    /** Report whether there are more tags than it takes. */
    isOverflowing(): boolean {
        return this.values().length > (this.properties.max ?? Number.POSITIVE_INFINITY);
    }

    /** Type text, adding each tag it has whole. */
    type(words: string): void {
        // add those before the last delimiter, and type on after it
        const parts = words.split(this.#delimiter());
        const rest = parts.pop() ?? "";
        this.#setRefusal(undefined);
        const refused = parts.filter((part) => this.#take(part) !== undefined);
        this.#setTyped([...refused, rest].join(refused.length > 0 && rest !== "" ? " " : ""));
        flush();
        this.#tell(refused);
    }

    /** Add the text as a tag, and report whether there was one. */
    submit(): boolean {
        // keep a text it does not take, for the person to change
        const typed = this.typed();
        const isTyped = typed.trim() !== "";
        if (isTyped && this.#take(typed) === undefined) {
            this.#setTyped("");
        }
        flush();
        this.#tell(isTyped && this.refusal() !== undefined ? [typed] : []);

        return isTyped;
    }

    /** Clear the text. */
    clearText(): void {
        this.#setRefusal(undefined);
        this.#setTyped("");
    }

    /** Remove a tag. */
    remove(place: number): void {
        // tell of it, as it is gone from the screen
        const tag = this.values()[place];
        if (tag !== undefined && this.isEditable()) {
            this.#setValues(this.values().filter((_, each) => each !== place));
            this.#setNews("removed", tag);
            flush();
        }
    }

    /** Remove every tag. */
    clear(): void {
        if (this.isEditable()) {
            this.#setValues([]);
            this.clearText();
        }
    }

    /** Change a tag in place. */
    edit(place: number | undefined): void {
        if (this.isEditable() && this.properties.editable !== false) {
            this.#setEdited(place);
            flush();
        }
    }

    /** Replace a tag changed in place, removing it where nothing is left. */
    replace(place: number, words: string): void {
        // keep the tag as it was where the new one is not one it takes
        const before = this.values()[place];
        const tag = this.#sanitize(words);
        const others = this.values().filter((_, each) => each !== place);
        const refusal = this.#refusalOf(tag, others);
        this.#setEdited(undefined);
        if (tag === "") {
            this.remove(place);
        } else if (refusal !== undefined) {
            this.properties.onValueInvalid?.(tag, refusal);
        } else if (tag !== before) {
            this.#setValues(this.values().map((each, at) => (at === place ? tag : each)));
        }
        flush();
    }

    /** Focus a tag, the text past the last one. */
    focus(place: number): void {
        const tags = this.element?.querySelectorAll<HTMLElement>("[data-slot=tag-field-item]");
        const input = this.element?.querySelector<HTMLElement>("[data-slot=tag-field-input]");
        (tags?.[Math.max(place, 0)] ?? input)?.focus();
    }

    /** Tell the owner of tags the field does not take, once the text has them. */
    #tell(refused: readonly string[]): void {
        // say why of each, as the next may be refused for another reason
        for (const words of refused) {
            const tag = this.#sanitize(words);
            const refusal = this.#refusalOf(tag, this.values());
            if (refusal !== undefined) {
                this.properties.onValueInvalid?.(tag, refusal);
            }
        }
    }

    /** Add a tag, and say why it is not one the field takes, undefined where it is. */
    #take(words: string): TagRefusal | undefined {
        // leave out what is empty
        const tag = this.#sanitize(words);
        if (tag === "") {
            return undefined;
        }
        const refusal = this.#refusalOf(tag, this.values());
        this.#setRefusal(refusal);
        if (refusal === undefined) {
            this.#setValues([...this.values(), tag]);
            this.#setNews("added", tag);
            flush();
        }

        return refusal;
    }

    /** Say why a tag is not one the field takes, undefined where it is. */
    #refusalOf(tag: string, others: readonly string[]): TagRefusal | undefined {
        // compare it with the others, their count and what the owner takes
        const { max, maxLength, validate } = this.properties;
        const isLong = maxLength !== undefined && tag.length > maxLength;
        if (others.includes(tag) && this.properties.allowDuplicates !== true) {
            return "duplicate";
        }
        if (max !== undefined && others.length >= max && this.properties.allowOverflow !== true) {
            return "max";
        }

        return isLong || validate?.(tag, others) === false ? "invalid" : undefined;
    }

    /** Write a tag as it is kept. */
    #sanitize(words: string): string {
        return this.properties.sanitizeValue?.(words) ?? words.trim();
    }

    /** Read what divides tags. */
    #delimiter(): RegExp {
        return this.properties.delimiter ?? DELIMITER;
    }
}

/** The nearest tags field, null outside one. */
const TagFieldContext = createContext<TagFieldControl | null>(null);

/** The place of the nearest tag, null outside one. */
const TagContext = createContext<Accessor<number> | null>(null);

/** Read the nearest tags field, refusing a part outside one. */
export function useTagField(): TagFieldControl {
    const control = useContext(TagFieldContext);
    if (control === null) {
        throw new TypeError("a tags field part needs a tags field around it");
    }

    return control;
}

/** Render tags a person types, removes and changes in one field. */
export const TagField = defineComponent({
    name: "tagField",
    properties: TAG_FIELD_PROPERTIES,
    render(given: RenderProperties<TagFieldAttributes, typeof TAG_FIELD_PROPERTIES>): JSX.Element {
        // edit the value of the field's form
        const properties = useFieldValue(given, {
            value: "value",
            change: "onValueChange",
            read: (value) =>
                Array.isArray(value) ? value.filter((each) => typeof each === "string") : [],
            settles: true,
        });
        const control = new TagFieldControl(properties, useFieldControl());
        const locale = useLocale();
        const rest = omit(
            properties,
            "value",
            "defaultValue",
            "onValueChange",
            "inputValue",
            "onInputValueChange",
            "max",
            "allowOverflow",
            "allowDuplicates",
            "maxLength",
            "delimiter",
            "sanitizeValue",
            "validate",
            "onValueInvalid",
            "blurBehavior",
            "editable",
            "disabled",
            "readOnly",
            "name",
            "form",
            "children",
        );
        const news = (): string => {
            // tell of the last tag that came or went
            const last = control.news();
            if (last === undefined) {
                return "";
            }

            return last.kind === "added"
                ? locale.render(t`Added ${last.tag}`)
                : locale.render(t`Removed ${last.tag}`);
        };

        return (
            <TagFieldContext value={control}>
                {renderPart("div", "tag-field", rest, styles.field, {
                    role: "group" as const,
                    ref: (element: HTMLElement) => (control.element = element),
                    get "data-disabled"() {
                        return control.isDisabled() ? "" : undefined;
                    },
                    get "data-readonly"() {
                        return properties.readOnly ? "" : undefined;
                    },
                    onPointerDown: (event: PointerEvent) => {
                        // type by a press between the tags
                        if (event.target === event.currentTarget) {
                            event.preventDefault();
                            control.focus(control.values().length);
                        }
                    },
                    get children() {
                        return (
                            <>
                                {contentOr(properties, () => (
                                    <>
                                        <TagFieldList />
                                        <TagFieldInput />
                                    </>
                                ))}
                                <VisuallyHidden role="status">{news()}</VisuallyHidden>
                                <Show when={properties.name}>
                                    <For each={control.values()}>
                                        {(value) => (
                                            <input
                                                type="hidden"
                                                name={properties.name}
                                                form={properties.form}
                                                value={value}
                                            />
                                        )}
                                    </For>
                                </Show>
                            </>
                        );
                    },
                })}
            </TagFieldContext>
        );
    },
});

/** The properties of a tag field, its tags, rules and attributes included. */
export type TagFieldProperties = ComponentProperties<typeof TagField>;

/** Render the tags of the nearest field, each as the caller does. */
export function TagFieldList(
    properties: Omit<TagFieldElementProperties, "children"> & {
        /** Render a tag, with its text and a button that removes it by default. */
        readonly children?: (tag: string, place: Accessor<number>) => JSX.Element;
    },
): JSX.Element {
    // name the list by the tags it has
    const control = useTagField();
    const locale = useLocale();
    const rest = omit(properties, "children");

    return renderPart("div", "tag-field-list", rest, styles.list, {
        get role() {
            return control.values().length === 0 ? undefined : ("list" as const);
        },
        get "aria-label"() {
            return locale.render(t.context("title")`Tags`);
        },
        get children() {
            return (
                <For each={control.values()}>
                    {(tag, place) =>
                        properties.children?.(tag, place) ?? (
                            <TagFieldItem value={tag} place={place()} />
                        )
                    }
                </For>
            );
        },
    });
}

/** The properties of a tag. */
export type TagFieldItemProperties = TagFieldElementProperties<HTMLSpanElement> & {
    /** The tag. */
    readonly value: string;
    /** Its place among the tags. */
    readonly place: number;
};

/** Render one tag of the nearest field, which the keys move between, change and remove. */
export function TagFieldItem(properties: TagFieldItemProperties): JSX.Element {
    // let the keys reach it from the text
    const control = useTagField();
    const locale = useLocale();
    const rest = omit(properties, "value", "place", "children");
    const isEdited = (): boolean => control.edited() === properties.place;
    const press = (event: KeyboardEvent): void => {
        // leave keys inside it, such as the text changed in place
        if (event.target !== event.currentTarget) {
            return;
        }
        const forward = locale.direction === "rtl" ? "ArrowLeft" : "ArrowRight";
        const backward = locale.direction === "rtl" ? "ArrowRight" : "ArrowLeft";
        const { place } = properties;
        const isTaken = [forward, backward, "Home", "End", "Enter", "F2", "Delete", "Backspace"];
        if (!isTaken.includes(event.key)) {
            return;
        }

        // move, change or remove it
        event.preventDefault();
        if (event.key === forward) {
            control.focus(place + 1);
        } else if (event.key === backward) {
            control.focus(place - 1);
        } else if (event.key === "Home") {
            control.focus(0);
        } else if (event.key === "End") {
            control.focus(control.values().length);
        } else if (event.key === "Enter" || event.key === "F2") {
            control.edit(place);
        } else {
            control.remove(place);
            control.focus(event.key === "Delete" ? place : place - 1);
        }
    };

    return (
        <TagContext value={() => properties.place}>
            {renderPart(
                "span",
                "tag-field-item",
                rest,
                () => [badgeVariants({ variant: "secondary" }), state.focus, styles.tag],
                {
                    role: "listitem" as const,
                    tabindex: -1,
                    get "data-editing"() {
                        return isEdited() ? "" : undefined;
                    },
                    onKeyDown: press,
                    onDblClick: () => control.edit(properties.place),
                    get children() {
                        return (
                            <Show
                                when={isEdited()}
                                fallback={contentOr(properties, () => (
                                    <>
                                        <span {...style.attrs(styles.words)}>
                                            {properties.value}
                                        </span>
                                        <TagFieldRemove />
                                    </>
                                ))}
                            >
                                <TagEdit value={properties.value} place={properties.place} />
                            </Show>
                        );
                    },
                },
            )}
        </TagContext>
    );
}

/** Render the text of a tag a person changes in place. */
function TagEdit(properties: { readonly value: string; readonly place: number }): JSX.Element {
    // take the change by Enter or leaving, and leave it by Escape
    const control = useTagField();
    const locale = useLocale();
    let isDone = false;
    const finish = (words: string, isFocused: boolean): void => {
        // finish once, as the input leaves the screen with the focus in it
        if (!isDone) {
            isDone = true;
            const { place } = properties;
            control.replace(place, words);
            if (isFocused) {
                control.focus(place);
            }
        }
    };

    return (
        <input
            data-slot="tag-field-edit"
            aria-label={locale.render(t`Edit ${properties.value}`)}
            ref={(element) =>
                queueMicrotask(() => {
                    element.focus();
                    element.select();
                })
            }
            value={properties.value}
            onKeyDown={(event) => {
                // keep the keys from the tag
                event.stopPropagation();
                if (event.key === "Enter" && !event.isComposing) {
                    event.preventDefault();
                    finish(event.currentTarget.value, true);
                } else if (event.key === "Escape") {
                    event.preventDefault();
                    finish(properties.value, true);
                }
            }}
            onBlur={(event) => finish(event.currentTarget.value, false)}
            {...style.attrs(state.typed, text.xs, styles.edit)}
        />
    );
}

/** Render the button that removes the nearest tag. */
export function TagFieldRemove(
    properties: Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class"> & ElementPartProperties,
): JSX.Element {
    // say which tag it removes, and go on typing
    const control = useTagField();
    const locale = useLocale();
    const place = useContext(TagContext);
    if (place === null) {
        throw new TypeError("a tag's remove button needs a tag around it");
    }

    return (
        <Show when={!control.properties.readOnly}>
            {renderPart("button", "tag-field-remove", properties, [state.touch, styles.remove], {
                type: "button" as const,
                tabindex: -1,
                get disabled() {
                    return control.isDisabled();
                },
                get "aria-label"() {
                    return locale.render(t`Remove ${control.values()[place()] ?? ""}`);
                },
                onClick: () => {
                    control.remove(place());
                    control.focus(control.values().length);
                },
                get children() {
                    return contentOr(properties, () => <Icon icon={xIcon} size="0.75em" />);
                },
            })}
        </Show>
    );
}

/** Render the text of the next tag of the nearest field. */
export function TagFieldInput(
    properties: Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "class" | "value" | "onInput"> &
        ElementPartProperties,
): JSX.Element {
    // add by Enter and a delimiter, and go to the tags by the keys before them
    const control = useTagField();
    const locale = useLocale();
    const press = (event: KeyboardEvent & { readonly currentTarget: HTMLInputElement }): void => {
        // leave keys while a word is composed
        const { currentTarget } = event;
        const isStart = currentTarget.selectionStart === 0 && currentTarget.selectionEnd === 0;
        const backward = locale.direction === "rtl" ? "ArrowRight" : "ArrowLeft";
        if (event.isComposing) {
            return;
        }
        if (event.key === "Enter" && control.submit()) {
            event.preventDefault();
        } else if ((event.key === backward || event.key === "Backspace") && isStart) {
            event.preventDefault();
            control.focus(control.values().length - 1);
        } else if (event.key === "Escape" && control.typed() !== "") {
            event.preventDefault();
            control.clearText();
        }
    };
    const leave = (): void => {
        // keep what is typed as a tag, unless the owner says otherwise
        const { blurBehavior = "add" } = control.properties;
        if (blurBehavior === "add") {
            control.submit();
        } else if (blurBehavior === "clear") {
            control.clearText();
        }
    };

    return (
        <Show when={!control.properties.readOnly}>
            {renderPart(
                "input",
                "tag-field-input",
                properties,
                [state.typed, text.sm, styles.input],
                mergeProperties(control.field?.attributes() ?? {}, {
                    autocomplete: "off" as const,
                    get "aria-label"() {
                        return control.field === null
                            ? locale.render(t.context("label")`Add tag`)
                            : undefined;
                    },
                    enterkeyhint: "done" as const,
                    get "aria-invalid"() {
                        const isInvalid = control.field?.isInvalid() === true;

                        return control.refusal() !== undefined ||
                            control.isOverflowing() ||
                            isInvalid
                            ? ("true" as const)
                            : undefined;
                    },
                    get disabled() {
                        return control.isDisabled();
                    },
                    get value() {
                        return control.typed();
                    },
                    onInput: (event: InputEvent & { readonly currentTarget: HTMLInputElement }) => {
                        control.type(event.currentTarget.value);
                        event.currentTarget.value = control.typed();
                    },
                    onPaste: (event: ClipboardEvent) => {
                        // add what a list has, whose line breaks an input leaves out
                        const pasted = event.clipboardData?.getData("text/plain") ?? "";
                        if (pasted.includes("\n")) {
                            event.preventDefault();
                            control.type(`${control.typed()}${pasted}\n`);
                        }
                    },
                    onKeyDown: press,
                    onBlur: leave,
                }),
            )}
        </Show>
    );
}

/** Render the button that removes every tag of the nearest field, while there are any. */
export function TagFieldClear(properties: ButtonProperties): JSX.Element {
    const control = useTagField();
    const locale = useLocale();

    return (
        <Show when={control.values().length > 0 && control.isEditable()}>
            <Button
                isIconOnly
                data-slot="tag-field-clear"
                variant="ghost"
                size="sm"
                aria-label={locale.render(t.context("action")`Remove all`)}
                xstyle={styles.clear}
                {...mergeProperties(
                    {
                        onClick: () => {
                            control.clear();
                            flush();
                            control.focus(0);
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
