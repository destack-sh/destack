import { Icon } from "@destack/ui/icon";
import caretDown from "@destack/icon/phosphor/caret-down";
import { type Direction, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    For,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { AutocompleteContext, AutocompleteControl } from "../autocomplete/index.ts";
import { badgeVariants } from "../badge/index.ts";
import { CollectionBuilder } from "../collection/index.ts";
import { InputGroup, InputGroupAddon, InputGroupInput } from "../input-group/index.ts";
import { TopLayer } from "../layer/index.ts";
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
    remove: {
        display: "inline-flex",
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
    /** Replace whether the list is open and tell the change handler. */
    readonly #setOpen: (isOpen: boolean) => void;
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
        const [isOpen, setOpen] = createControllableSignal({
            isControlled: () => properties.open !== undefined,
            value: () => properties.open === true,
            defaultValue: properties.defaultOpen === true,
            onChange: (isNext) => properties.onOpenChange?.(isNext),
        });
        this.autocomplete = new AutocompleteControl({
            get filter() {
                return properties.filter;
            },
            get value() {
                return properties.inputValue;
            },
            get defaultValue() {
                return properties.defaultInputValue;
            },
            onValueChange: (typed) => properties.onInputValueChange?.(typed),
        });

        // choose an option's value with its text from the list box
        this.list = new ListBoxControl({
            autocomplete: this.autocomplete,
            onAction: (option) => this.choose(option.value(), option.text()),
        });
        this.selection = new Selection(properties);
        this.properties = properties;
        this.isOpen = isOpen;
        this.items = new CollectionBuilder();
        this.#labels = new Map();
        this.#setOpen = setOpen;
        this.#input = undefined;
        this.#anchor = undefined;
        this.#content = undefined;
        this.#unplace = () => undefined;

        // stop placing the overlay and drop its anchor once its owner goes away
        onCleanup(() => this.#unplace());
    }

    /** Return the text of a chosen value, the value itself when it was chosen elsewhere. */
    labelOf(value: string): string {
        return this.#labels.get(value) ?? value;
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

    /** Open or close the list and tell the root's change handler. */
    open(isOpen: boolean): void {
        if (isOpen !== this.isOpen()) {
            this.#setOpen(isOpen);
        }
    }

    /** Open or close the list from its button, focusing the input. */
    toggle(): void {
        this.#input?.focus();
        this.open(!this.isOpen());
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
        this.autocomplete.type(label);
        this.open(false);
    }

    /** Report whether no item stands for a text, ignoring case, so an option can create it. */
    isNew(typed: string): boolean {
        const lowered = typed.toLowerCase();

        return lowered !== "" && !this.items.items().some((item) => item.toLowerCase() === lowered);
    }
}

/** The properties of a combobox root. */
export type ComboboxProperties = {
    /** Whether the list is open, which makes the open state controlled. */
    readonly open?: boolean;
    /** Whether the list starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean;
    /** Handle the list opening or closing. */
    readonly onOpenChange?: (open: boolean) => void;
    /** The text in the input, which makes it controlled. */
    readonly inputValue?: string;
    /** The text in the input at first while uncontrolled, empty by default. */
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
} & SelectionProperties;

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
};

/** Hold the chosen value of an input whose list of options filters as the person types. */
export function Combobox(properties: ComboboxProperties): JSX.Element {
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
        Omit<
            JSX.InputHTMLAttributes<HTMLInputElement>,
            "value" | "onInput" | "onKeyDown" | "onBlur" | "ref"
        >
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
                    control.open(true);
                }}
                onKeyDown={(event) => steer(event, control, locale.direction)}
                onBlur={() => control.open(false)}
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
            aria-label={locale.render(t`Show options`)}
            aria-expanded={control.isOpen() ? "true" : "false"}
            aria-controls={control.list.id}
            data-slot="combobox-trigger"
            onPointerDown={(event) => {
                // open from the input, which takes the focus and the keys
                event.preventDefault();
                control.toggle();
            }}
            {...style.attrs(styles.trigger)}
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
    const rest = omit(properties, "keepMounted", "xstyle", "style", "children");

    return (
        <Show when={properties.keepMounted === true || isChosen()}>
            <span
                data-slot="combobox-item-indicator"
                data-selected={isChosen() ? "" : undefined}
                {...rest}
                {...style.attributes([styles.indicator, properties.xstyle], properties.style)}
            >
                {properties.children ?? <Icon name="check" />}
            </span>
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
        control.open(true);
    } else if ((isVertical || event.key === "Enter") && control.isOpen()) {
        control.autocomplete.steer(event, direction);
    }
    // close the list, or clear the input when it is closed
    else if (event.key === "Escape") {
        event.preventDefault();
        if (control.isOpen()) {
            control.open(false);
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
    const rest = omit(properties, "xstyle", "style", "children");

    return (
        <div
            data-slot="combobox-chips"
            {...rest}
            {...style.attributes([styles.chips, properties.xstyle], properties.style)}
        >
            {properties.children ?? (
                <For each={control.selection.values()}>
                    {(value) => <ComboboxChip value={value} />}
                </For>
            )}
        </div>
    );
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
    const rest = omit(properties, "value", "xstyle", "style", "children");

    return (
        <ComboboxChipContext value={properties.value}>
            <span
                data-slot="combobox-chip"
                {...rest}
                {...style.attributes(
                    [badgeVariants({ variant: "secondary" }), properties.xstyle],
                    properties.style,
                )}
            >
                {properties.children ?? (
                    <>
                        {control.labelOf(properties.value)}
                        <ComboboxChipRemove />
                    </>
                )}
            </span>
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
    const rest = omit(properties, "xstyle", "style", "children");

    return (
        <button
            type="button"
            tabindex={-1}
            aria-label={locale.render(t`Remove ${control.labelOf(value)}`)}
            data-slot="combobox-chip-remove"
            {...rest}
            onClick={() => control.selection.toggle(value)}
            {...style.attributes([styles.remove, properties.xstyle], properties.style)}
        >
            {properties.children ?? <Icon name="x" size="0.75em" />}
        </button>
    );
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
