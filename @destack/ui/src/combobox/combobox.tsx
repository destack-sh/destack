import { contentOr, renderPart, type Render } from "../part/index.ts";
import { Icon } from "@destack/ui/icon";
import caretDown from "@destack/icon/phosphor/caret-down";
import { type Direction, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { color, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createUniqueId,
    createEffect,
    createSignal,
    flush,
    For,
    type JSX,
    omit,
    onCleanup,
    type Setter,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { AutocompleteContext, AutocompleteControl } from "../autocomplete/index.ts";
import { badgeVariants } from "../badge/index.ts";
import { CollectionBuilder } from "../collection/index.ts";
import type { InputProperties } from "../input/index.ts";
import { InputGroup, InputGroupAddon, InputGroupInput } from "../input-group/index.ts";
import { OpenChange, type OpenProperties, OpenState, TopLayer } from "../layer/index.ts";
import {
    ListBox,
    ListBoxContext,
    ListBoxControl,
    ListBoxEmpty,
    ListBoxItem,
    type ListBoxItemProperties,
    ListBoxLoading,
    ListBoxSection,
    type ListBoxSectionProperties,
    ListBoxSeparator,
} from "../list-box/index.ts";
import { Position } from "../position/index.ts";
import { Selection, type SelectionProperties } from "../selection/index.ts";
import { useFieldValue, readChoices } from "../field/control.ts";

/** The combobox of the nearest combobox root, null outside one. */
const ComboboxContext = createContext<ComboboxControl | null>(null);

/** The styles of a combobox's input and list. */
const styles = style.create({
    indicator: {
        display: "inline-flex",
        marginInlineStart: "auto",
    },
    chips: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[1],
    },
    chip: {
        overflow: { default: "hidden", [media.pointerCoarse]: "visible" },
    },
    remove: {
        display: "inline-flex",
        padding: { default: 0, [media.pointerCoarse]: space[2] },
        margin: { default: 0, [media.pointerCoarse]: `calc(-1 * ${space[2]})` },
        marginInlineStart: { default: 0, [media.pointerCoarse]: `calc(-1 * ${space[1]})` },
        cursor: "pointer",
    },
    trigger: {
        display: "inline-flex",
        cursor: { default: "pointer", ":disabled": "not-allowed" },
    },
    content: {
        width: "anchor-size(width)",
        maxHeight: `calc(8 * ${size[3]})`,
        overflowY: "auto",
        inset: "auto",
        padding: space[1],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: shadow.overlay,
    },
});

/** The chosen values and open state of a combobox. */
export class ComboboxControl {
    /** The search typed into the input, which filters the list box. */
    readonly autocomplete: AutocompleteControl;
    /** The list box of options. */
    readonly list: ListBoxControl;
    /** The chosen values, controlled or the combobox's own: one for a single combobox. */
    readonly selection: Selection;
    /** Whether the list is open. */
    readonly isOpen: Accessor<boolean>;
    /** The properties of the combobox root, read for its mode and handlers. */
    readonly properties: ComboboxProperties;
    /** The values the combobox's items stand for. */
    readonly items: CollectionBuilder<string>;
    /** The text of each value chosen here, which the chips show. */
    readonly #labels: Map<string, string>;
    /** What a person types, undefined while the chosen value shows. */
    readonly #typed: Accessor<string | undefined>;
    /** Replace what a person types. */
    readonly #setTyped: Setter<string | undefined>;
    /** The open state. */
    readonly #state: OpenState;
    /** The input. */
    #input: HTMLInputElement | undefined;
    /** The frame around the input, which anchors the list. */
    #anchor: HTMLElement | undefined;
    /** The list element. */
    #content: HTMLElement | undefined;
    /** Stop placing the shown list below the input. */
    #unplace: () => void;

    /** Create a combobox whose list chooses its values, open when its properties ask for it. */
    constructor(properties: ComboboxProperties) {
        // follow the chosen values, the open state and the typed text
        this.#state = new OpenState(properties, createUniqueId());
        this.selection = new Selection(properties);
        const [typed, setTyped] = createSignal(
            untrack(() => properties.defaultInputValue),
            { ownedWrite: true },
        );
        this.#typed = typed;
        this.#setTyped = setTyped;
        const shown = (): string => this.#shown();
        this.autocomplete = new AutocompleteControl({
            get filter() {
                return typed() === undefined && properties.inputValue === undefined
                    ? null
                    : properties.filter;
            },
            get value() {
                return shown();
            },
            onValueChange: (next) => this.#type(next),
        });

        // choose an option's value with its text from the list box
        this.list = new ListBoxControl({
            autocomplete: this.autocomplete,
            onAction: (option) => this.choose(option.value(), option.text()),
        });
        this.properties = properties;
        this.isOpen = this.#state.isOpen;
        this.items = new CollectionBuilder();
        this.#labels = new Map();
        this.#input = undefined;
        this.#anchor = undefined;
        this.#content = undefined;
        this.#unplace = () => undefined;

        // stop placing the overlay and drop its anchor once its owner goes away
        onCleanup(() => this.#unplace());
    }

    /** Return the text of a chosen value, its option's, else the value itself. */
    labelOf(value: string): string {
        const option = this.list.options.items().find((each) => each.value() === value);

        return this.#labels.get(value) ?? option?.text() ?? value;
    }

    /** Leave the search, showing the text of the chosen value again. */
    leave(): void {
        // forget the text typed, which chose nothing
        const before = this.autocomplete.value();
        this.#setTyped(undefined);
        flush();
        const after = this.#label();
        if (before !== after) {
            this.properties.onInputValueChange?.(after);
        }
    }

    /** Read the text of the input: the owner's, what a person types, else the chosen value's. */
    #shown(): string {
        return this.properties.inputValue ?? this.#typed() ?? this.#label();
    }

    /** Read the text of the chosen value, none in a multiple combobox, whose chips show theirs. */
    #label(): string {
        const [chosen] = this.selection.values();

        return this.properties.multiple === true || chosen === undefined
            ? ""
            : this.labelOf(chosen);
    }

    /** Take the text a person types, choosing nothing by an empty one. */
    #type(typed: string): void {
        // show it and tell the owner
        this.#setTyped(typed);
        this.properties.onInputValueChange?.(typed);

        // choose none by an empty text
        const isChosen = this.selection.values().length > 0;
        if (typed === "" && isChosen && this.properties.multiple !== true) {
            this.selection.clear();
        }
    }

    /** Set the input. */
    setInput(element: HTMLInputElement): void {
        this.#input = element;
    }

    /** Set the frame the list is placed beside, as wide as it. */
    setAnchor(element: HTMLElement): void {
        this.#anchor = element;
    }

    /** Set the list element. */
    setContent(element: HTMLElement): void {
        this.#content = element;
    }

    /** Open or close the list, unless its handler cancels it. */
    open(isOpen: boolean, change = new OpenChange()): void {
        if (isOpen !== this.isOpen()) {
            this.#state.set(isOpen, change);
        }
    }

    /** Open or close the list from its button, focusing the input. */
    toggle(event: PointerEvent): void {
        this.#input?.focus();
        this.open(!this.isOpen(), new OpenChange("trigger-press", event));
    }

    /** Show or hide the list element below the input. */
    sync(isOpen: boolean): void {
        if (
            this.#content === undefined ||
            this.#input === undefined ||
            this.#anchor === undefined
        ) {
            return;
        }
        // attach the list to its frame before showing, and place by script once it has a size
        this.#state.mark(this.#content);
        if (isOpen) {
            const detach = Position.attach(this.#content, this.#anchor, this.list.id);
            this.#content.showPopover({ source: this.#input });
            const unplace = Position.place(this.#content, this.#anchor);
            this.#unplace = () => {
                detach();
                unplace();
            };
        } else {
            this.#unplace();
            this.#content.hidePopover();
        }
    }

    /** Take an option. */
    choose(value: string, label: string): void {
        // remember the option's text for the chips
        this.#labels.set(value, label);

        // add or drop the value of a multiple combobox and clear the input for the next search
        if (this.properties.multiple === true) {
            this.selection.toggle(value);
            this.autocomplete.type("");
            return;
        }

        // set a single combobox's value and close the list on its text
        this.selection.add(value);
        this.properties.onInputValueChange?.(label);
        this.#setTyped(undefined);
        this.open(false, new OpenChange("item-press"));
    }

    /** Report whether no item stands for a text, ignoring case, so an option can create it. */
    isNew(typed: string): boolean {
        const lowered = typed.toLowerCase();

        return lowered !== "" && !this.items.items().some((item) => item.toLowerCase() === lowered);
    }
}

/** The properties of a combobox root. */
export type ComboboxProperties = {
    /** The text in the input, which makes it controlled. */
    readonly inputValue?: string;
    /** The text in the input at first while uncontrolled, the chosen value's by default. */
    readonly defaultInputValue?: string;
    /** Report whether an option matches the typed text, null where the owner filters. */
    readonly filter?:
        | ((value: string, text: string, keywords: readonly string[]) => boolean)
        | null;
    /** Handle the person typing, such as to load the options that match. */
    readonly onInputValueChange?: (text: string) => void;
    /** Handle the person creating an option from the typed text through `ComboboxCreate`. */
    readonly onCreate?: (text: string) => void;
    /** The input and list. */
    readonly children?: JSX.Element;
} & OpenProperties &
    SelectionProperties;

/** Read the combobox of the nearest combobox root, refusing elements outside one. */
export function useCombobox(): ComboboxControl {
    const control = useContext(ComboboxContext);
    if (control === null) {
        throw new TypeError("combobox elements need a combobox root around them");
    }

    return control;
}

/** The properties of an element of a combobox, the native element's attributes included. */
export type ComboboxElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
};

/** Hold the chosen value of an input whose list of options filters as the person types. */
export function Combobox(given: ComboboxProperties): JSX.Element {
    // edit the value of the field's form
    const properties = useFieldValue(given, {
        value: "value",
        change: "onValueChange",
        read: readChoices,
        settles: true,
    });

    const control = new ComboboxControl(properties);

    return (
        <ComboboxContext value={control}>
            <AutocompleteContext value={control.autocomplete}>
                <ListBoxContext value={control.list}>{properties.children}</ListBoxContext>
            </AutocompleteContext>
        </ComboboxContext>
    );
}

/** Render the input that filters the list, framed with the button that shows every option. */
export function ComboboxInput(
    properties: ComboboxElementProperties<
        Omit<InputProperties, "value" | "onInput" | "onKeyDown" | "onClick" | "onBlur" | "ref">
    >,
): JSX.Element {
    // read the combobox and the reader's direction
    const control = useCombobox();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");

    return (
        <InputGroup
            ref={(element) => control.setAnchor(element)}
            xstyle={properties.xstyle}
            style={properties.style}
        >
            <InputGroupInput
                data-slot="combobox-input"
                role="combobox"
                aria-expanded={control.isOpen() ? "true" : "false"}
                aria-controls={control.list.id}
                aria-autocomplete="list"
                aria-activedescendant={
                    control.isOpen() ? control.list.focus.descendant() : undefined
                }
                autocomplete="off"
                {...rest}
                ref={(element: HTMLInputElement) => control.setInput(element)}
                value={control.autocomplete.value()}
                onInput={(event) => {
                    // filter by the typed text and show the matches
                    control.autocomplete.type(event.currentTarget.value);
                    control.open(true, new OpenChange("input", event));
                }}
                onKeyDown={(event) => steer(event, control, locale.direction)}
                onClick={(event) => {
                    // show every option, the text of the chosen one ready to be typed over
                    const { currentTarget } = event;
                    if (!control.isOpen() && !currentTarget.readOnly) {
                        control.open(true, new OpenChange("trigger-press", event));
                        currentTarget.select();
                    }
                }}
                onBlur={(event) => {
                    control.open(false, new OpenChange("focus-out", event));
                    control.leave();
                }}
            />
            <InputGroupAddon align="inline-end">
                <ComboboxTrigger disabled={properties.disabled === true} />
            </InputGroupAddon>
        </InputGroup>
    );
}

/** Render the button that shows or hides every option, leaving the focus in the input. */
function ComboboxTrigger(properties: { readonly disabled: boolean }): JSX.Element {
    const control = useCombobox();
    const locale = useLocale();

    return (
        <button
            type="button"
            tabindex={-1}
            disabled={properties.disabled}
            aria-label={locale.render(t.context("action")`Show options`)}
            aria-expanded={control.isOpen() ? "true" : "false"}
            aria-controls={control.list.id}
            data-slot="combobox-trigger"
            onPointerDown={(event) => {
                // open from the input, which takes the focus and the keys
                event.preventDefault();
                control.toggle(event);
            }}
            {...style.attrs(state.touch, styles.trigger)}
        >
            <Icon icon={caretDown} />
        </button>
    );
}

/** Render the list box of options below the input. */
export function ComboboxContent(
    properties: ComboboxElementProperties<
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "ref" | "onKeyDown" | "onFocusOut">
    >,
): JSX.Element {
    // show and hide the list as the combobox opens and closes
    const control = useCombobox();
    createEffect(control.isOpen, (isOpen) => untrack(() => control.sync(isOpen)));

    return (
        <TopLayer>
            <ListBox
                control={control.list}
                popover="manual"
                data-slot="combobox-content"
                data-side="bottom"
                data-align="start"
                {...properties}
                ref={(element) => control.setContent(element)}
                xstyle={[
                    text.sm,
                    styles.content,
                    Position.beside("bottom", "start"),
                    properties.xstyle,
                ]}
            />
        </TopLayer>
    );
}

/** Render an option that a click or Enter chooses as the combobox's value. */
export function ComboboxItem(properties: ListBoxItemProperties): JSX.Element {
    // join the combobox's items under the item's value
    const control = useCombobox();
    const value = untrack(() => properties.value);
    if (value !== undefined) {
        control.items.add(value);
    }
    const isChosen = (): boolean =>
        properties.value !== undefined && control.selection.isSelected(properties.value);

    return (
        <ComboboxItemContext value={isChosen}>
            <ListBoxItem
                data-slot="combobox-item"
                data-selected={isChosen() ? "" : undefined}
                {...properties}
            />
        </ComboboxItemContext>
    );
}

/** Whether the nearest combobox item is chosen, null outside one. */
const ComboboxItemContext = createContext<Accessor<boolean> | null>(null);

/** The properties of a combobox item's indicator, the native element's attributes included. */
export interface ComboboxItemIndicatorProperties extends ComboboxElementProperties<
    JSX.HTMLAttributes<HTMLSpanElement>
> {
    /** Whether the indicator stays rendered while its item is not chosen, for animating it. */
    readonly keepMounted?: boolean;
}

/** Render the check of the nearest combobox item, shown while its value is chosen. */
export function ComboboxItemIndicator(properties: ComboboxItemIndicatorProperties): JSX.Element {
    // read the item, refusing an indicator outside one
    const isChosen = useContext(ComboboxItemContext);
    if (isChosen === null) {
        throw new TypeError("a combobox item indicator needs a combobox item around it");
    }
    const rest = omit(properties, "keepMounted", "children");

    return (
        <Show when={properties.keepMounted === true || isChosen()}>
            {renderPart("span", "combobox-item-indicator", rest, styles.indicator, {
                get "data-selected"() {
                    return isChosen() ? "" : undefined;
                },
                get children() {
                    return contentOr(properties, () => <Icon name="check" />);
                },
            })}
        </Show>
    );
}

/** Render a message while the typed text matches no option. */
export function ComboboxEmpty(
    properties: ComboboxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return <ListBoxEmpty data-slot="combobox-empty" {...properties} />;
}

/** Render a group of options under a heading. */
export function ComboboxGroup(properties: ListBoxSectionProperties): JSX.Element {
    return <ListBoxSection data-slot="combobox-group" {...properties} />;
}

/** Render a line between groups of options while the typed text is empty. */
export function ComboboxSeparator(
    properties: ComboboxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return <ListBoxSeparator data-slot="combobox-separator" {...properties} />;
}

/** Render the state of options the owner loads, such as a spinner and a message. */
export function ComboboxLoading(
    properties: ComboboxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return <ListBoxLoading data-slot="combobox-loading" {...properties} />;
}

/** Open, move through, choose from and close the list from the keyboard. */
function steer(event: KeyboardEvent, control: ComboboxControl, direction: Direction): void {
    // open with the down or up arrow key, else move the focus or choose with Enter
    const isVertical = event.key === "ArrowDown" || event.key === "ArrowUp";
    if (isVertical && !control.isOpen()) {
        event.preventDefault();
        control.open(true, new OpenChange("trigger-press", event));
    } else if ((isVertical || event.key === "Enter") && control.isOpen()) {
        control.autocomplete.steer(event, direction);
    }
    // close the list, or clear the input when it is closed
    else if (event.key === "Escape") {
        event.preventDefault();
        if (control.isOpen()) {
            control.open(false, new OpenChange("escape-key", event));
        } else {
            control.autocomplete.type("");
        }
    }
    // drop the last chip of a multiple combobox from an empty input
    else if (
        event.key === "Backspace" &&
        control.properties.multiple === true &&
        control.autocomplete.value() === ""
    ) {
        const last = control.selection.values().at(-1);
        if (last !== undefined) {
            control.selection.toggle(last);
        }
    }
}

/** Render the chosen values of a multiple combobox as chips. */
export function ComboboxChips(
    properties: ComboboxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useCombobox();
    const rest = omit(properties, "children");

    return renderPart("div", "combobox-chips", rest, styles.chips, {
        get children() {
            return contentOr(properties, () => (
                <For each={control.selection.values()}>
                    {(value) => <ComboboxChip value={value} />}
                </For>
            ));
        },
    });
}

/** The properties of a chip of a multiple combobox, the native element's attributes included. */
export interface ComboboxChipProperties extends ComboboxElementProperties<
    JSX.HTMLAttributes<HTMLSpanElement>
> {
    /** The chosen value the chip shows. */
    readonly value: string;
}

/** The value of the nearest chip, null outside one. */
const ComboboxChipContext = createContext<string | null>(null);
/** Render one chosen value of a multiple combobox as a chip. */
export function ComboboxChip(properties: ComboboxChipProperties): JSX.Element {
    const control = useCombobox();
    const rest = omit(properties, "value", "children");

    return (
        <ComboboxChipContext value={properties.value}>
            {renderPart(
                "span",
                "combobox-chip",
                rest,
                () => [badgeVariants({ variant: "secondary" }), styles.chip],
                {
                    get children() {
                        return contentOr(properties, () => (
                            <>
                                {control.labelOf(properties.value)}
                                <ComboboxChipRemove />
                            </>
                        ));
                    },
                },
            )}
        </ComboboxChipContext>
    );
}

/** Render the button that drops the nearest chip's value. */
export function ComboboxChipRemove(
    properties: ComboboxElementProperties<
        Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "onClick">
    >,
): JSX.Element {
    // read the chip's value and label the button by its text
    const control = useCombobox();
    const locale = useLocale();
    const value = useContext(ComboboxChipContext);
    if (value === null) {
        throw new TypeError("a combobox chip remove button needs a combobox chip around it");
    }
    const rest = omit(properties, "children");

    return renderPart("button", "combobox-chip-remove", rest, [state.touch, styles.remove], {
        type: "button" as const,
        tabindex: -1,
        get "aria-label"() {
            return locale.render(t`Remove ${control.labelOf(value)}`);
        },
        onClick: () => control.selection.toggle(value),
        get children() {
            return contentOr(properties, () => <Icon name="x" size="0.75em" />);
        },
    });
}

/** Render an option that creates one from the typed text while no item has that text. */
export function ComboboxCreate(properties: {
    /** The option's text for the typed text, `Create "…"` by default. */
    readonly children?: (text: string) => JSX.Element;
}): JSX.Element {
    // offer the option while the typed text matches no item
    const control = useCombobox();
    const locale = useLocale();
    const typed = (): string => control.autocomplete.value().trim();

    return (
        <Show when={control.isNew(typed())}>
            <ListBoxItem
                data-slot="combobox-create"
                value={typed()}
                textValue={typed()}
                keywords={[typed()]}
                onSelect={(created) => control.properties.onCreate?.(created)}
            >
                {properties.children?.(typed()) ?? locale.render(t`Create "${typed()}"`)}
            </ListBoxItem>
        </Show>
    );
}
