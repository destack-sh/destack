import { Icon } from "@destack/icon";
import { t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
import * as style from "@destack/style";
import { color, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    createSignal,
    For,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
} from "@destack/view";
import {
    CommandControl,
    CommandEmpty,
    CommandGroup,
    CommandItem,
    CommandLoading,
    CommandProvider,
    CommandSeparator,
    type CommandElementProperties,
    type CommandItemProperties,
} from "../command/index.ts";
import { badgeStyle } from "../badge/index.ts";
import { type Choice, createChoice } from "../choice/index.ts";
import { useFieldControl } from "../field/control.ts";
import { inputStyle } from "../input/index.ts";
import { placementStyle } from "../popover/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The condition under which a list sits below its input instead of the viewport's center. */
const ANCHORED = "@supports (position-area: block-end)";

/** The combobox of the nearest combobox root, null outside one. */
const ComboboxContext = createContext<ComboboxControl | null>(null);

/** The styles of a combobox's input and list. */
const styles = style.create({
    chips: {
        display: "flex",
        flexWrap: "wrap",
        gap: space[1],
    },
    remove: {
        display: "inline-flex",
        padding: 0,
        borderWidth: 0,
        backgroundColor: "transparent",
        color: "inherit",
        cursor: "pointer",
    },
    input: {
        width: "100%",
        height: size[3],
        cursor: { default: "text", ":disabled": "not-allowed" },
    },
    content: {
        boxSizing: "border-box",
        width: "anchor-size(width)",
        maxHeight: `calc(8 * ${size[3]})`,
        overflowY: "auto",
        inset: { default: 0, [ANCHORED]: "auto" },
        margin: { default: "auto", [ANCHORED]: space[1] },
        padding: space[1],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.popoverForeground,
        boxShadow: shadow.overlay,
    },
});

/** The chosen value and open state of a combobox, around the command list its options share. */
export class ComboboxControl {
    /** The list of options, filtered by the input's text. */
    readonly list: CommandControl;
    /** Whether the list is open. */
    readonly isOpen: Accessor<boolean>;
    /** The chosen values, controlled or the combobox's own: one for a single combobox. */
    readonly values: Accessor<readonly string[]>;
    /** The properties of the combobox root, read for its mode and handlers. */
    readonly properties: ComboboxProperties;
    /** The values of the combobox's items. */
    readonly items: Accessor<ReadonlySet<string>>;
    /** The text of each value chosen here, which the chips show. */
    readonly #labels: Map<string, string>;
    /** Replace the values of the combobox's items. */
    readonly #setItems: (items: ReadonlySet<string>) => void;
    /** Replace the chosen values and tell the change handler. */
    readonly #setValues: (values: readonly string[]) => void;
    /** Replace whether the list is open and tell the change handler. */
    readonly #setOpen: (isOpen: boolean) => void;
    /** The input that anchors the list. */
    #input: HTMLInputElement | undefined;
    /** The list element. */
    #content: HTMLElement | undefined;

    /** Create a combobox whose list chooses its values, open when its properties ask for it. */
    constructor(properties: ComboboxProperties) {
        // start on the default values and open state
        const [values, setValues] = createChoice(properties);
        const [isOpen, setOpen] = createControllableSignal({
            isControlled: () => properties.open !== undefined,
            value: () => properties.open === true,
            defaultValue: properties.defaultOpen === true,
            onChange: (isNext) => properties.onOpenChange?.(isNext),
        });
        this.list = new CommandControl({
            get shouldFilter() {
                return properties.shouldFilter !== false;
            },
        });
        this.properties = properties;
        this.isOpen = isOpen;
        this.values = values;
        const [items, setItems] = createSignal<ReadonlySet<string>>(new Set(), {
            ownedWrite: true,
        });
        this.items = items;
        this.#setItems = setItems;
        this.#labels = new Map();
        this.#setValues = setValues;
        this.#setOpen = setOpen;
        this.#input = undefined;
        this.#content = undefined;
        this.list.onChoose = (value, label) => this.choose(value, label);
    }

    /** Add an item's value until the item unmounts. */
    register(value: string): void {
        this.#setItems(new Set([...this.items(), value]));
        onCleanup(() =>
            this.#setItems(new Set([...this.items()].filter((entry) => entry !== value))),
        );
    }

    /** Report whether a value is chosen. */
    isChosen(value: string): boolean {
        return this.values().includes(value);
    }

    /** Return the text of a chosen value, the value itself when it was chosen elsewhere. */
    labelOf(value: string): string {
        return this.#labels.get(value) ?? value;
    }

    /** Set the input that anchors the list. */
    setInput(element: HTMLInputElement): void {
        this.#input = element;
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

    /** Show or hide the list element below the input. */
    sync(isOpen: boolean): void {
        if (this.#content === undefined || this.#input === undefined) {
            return;
        }
        if (isOpen) {
            this.#content.showPopover({ source: this.#input });
        } else {
            this.#content.hidePopover();
        }
    }

    /** Type into the input, filtering the list and telling the owner of the text. */
    type(search: string): void {
        this.list.type(search);
        this.properties.onInputValueChange?.(search);
    }

    /** Take an option: a single combobox writes its text into the input and closes, a multiple one adds or drops it and keeps the list open. */
    choose(value: string, label: string): void {
        // remember the option's text for the chips
        this.#labels.set(value, label);

        // add or drop the value of a multiple combobox and clear the input for the next search
        if (this.properties.multiple === true) {
            this.#setValues(
                this.isChosen(value)
                    ? this.values().filter((entry) => entry !== value)
                    : [...this.values(), value],
            );
            this.type("");
            return;
        }

        // keep a single combobox's value and close the list on its text
        this.#setValues([value]);
        this.type(label);
        this.open(false);
    }

    /** Drop a chosen value. */
    remove(value: string): void {
        this.#setValues(this.values().filter((entry) => entry !== value));
    }
}

/** The properties of a combobox root. */
export type ComboboxProperties = ComboboxBehaviour & Choice;

/** The open state, search and creation of a combobox. */
export interface ComboboxBehaviour {
    /** Whether the list is open, which makes the open state controlled. */
    readonly open?: boolean;
    /** Whether the list starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean;
    /** Handle the list opening or closing. */
    readonly onOpenChange?: (open: boolean) => void;
    /** Whether the list filters its options by the typed text, or its owner does, as for options it loads, true by default. */
    readonly shouldFilter?: boolean;
    /** Handle the person typing, such as to load the options that match. */
    readonly onInputValueChange?: (text: string) => void;
    /** Handle the person creating an option from the typed text through `ComboboxCreate`. */
    readonly onCreate?: (text: string) => void;
    /** The input and list. */
    readonly children?: JSX.Element;
}

/** Read the combobox of the nearest combobox root, refusing elements outside one. */
export function useCombobox(): ComboboxControl {
    const control = useContext(ComboboxContext);
    if (control === null) {
        throw new TypeError("combobox elements need a combobox root around them");
    }

    return control;
}

/** Hold the chosen value of an input whose list of options filters as the person types. */
export function Combobox(properties: ComboboxProperties): JSX.Element {
    const control = new ComboboxControl(properties);

    return (
        <ComboboxContext value={control}>
            <CommandProvider control={control.list}>{properties.children}</CommandProvider>
        </ComboboxContext>
    );
}

/** Render the input that filters the list, opening it as the person types or presses the down arrow key. */
export function ComboboxInput(
    properties: CommandElementProperties<
        Omit<
            JSX.InputHTMLAttributes<HTMLInputElement>,
            "value" | "onInput" | "onKeyDown" | "onBlur" | "ref"
        >
    >,
): JSX.Element {
    // read the combobox and its list
    const control = useCombobox();
    const list = control.list;
    const field = useFieldControl();
    const rest = omit(properties, "xstyle", "style");
    const isInvalid = (): boolean =>
        field?.isInvalid() === true || properties["aria-invalid"] === "true";

    return (
        <input
            role="combobox"
            aria-expanded={control.isOpen() ? "true" : "false"}
            aria-controls={list.listId}
            aria-autocomplete="list"
            aria-activedescendant={control.isOpen() ? list.highlighted() : undefined}
            autocomplete="off"
            data-slot="combobox-input"
            {...field?.attributes()}
            {...rest}
            ref={(element) => control.setInput(element)}
            value={list.search()}
            onInput={(event) => {
                // filter by the typed text and show the matches
                control.type(event.currentTarget.value);
                control.open(true);
            }}
            onKeyDown={(event) => steer(event, control)}
            onBlur={() => control.open(false)}
            {...style.attributes(
                [inputStyle({ invalid: isInvalid() }), styles.input, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render the list of options below the input. */
export function ComboboxContent(
    properties: CommandElementProperties<Omit<JSX.HTMLAttributes<HTMLDivElement>, "ref">>,
): JSX.Element {
    // show and hide the list as the combobox opens and closes
    const control = useCombobox();
    const rest = omit(properties, "xstyle", "style");
    createEffect(control.isOpen, (isOpen) => control.sync(isOpen));

    return (
        <TopLayer>
            <div
                id={control.list.listId}
                role="listbox"
                popover="manual"
                data-slot="combobox-content"
                {...rest}
                ref={(element) => control.setContent(element)}
                {...style.attributes(
                    [
                        text.footnote,
                        styles.content,
                        placementStyle("bottom", "start"),
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            />
        </TopLayer>
    );
}

/** Render an option that a click or Enter chooses as the combobox's value. */
export function ComboboxItem(properties: CommandItemProperties): JSX.Element {
    // join the combobox's items under the item's value
    const control = useCombobox();
    const value = untrack(() => properties.value);
    if (value !== undefined) {
        control.register(value);
    }
    const isChosen = (): boolean =>
        properties.value !== undefined && control.isChosen(properties.value);

    return (
        <CommandItem
            data-slot="combobox-item"
            data-state={isChosen() ? "checked" : "unchecked"}
            {...properties}
        />
    );
}

/** Render a message while the typed text matches no option. */
export const ComboboxEmpty = CommandEmpty;

/** Render a group of options under a heading. */
export const ComboboxGroup = CommandGroup;

/** Render a line between groups of options. */
export const ComboboxSeparator = CommandSeparator;

/** Open, move through, choose from and close the list from the keyboard. */
function steer(event: KeyboardEvent, control: ComboboxControl): void {
    // open with the down arrow key, else move the highlight
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        if (control.isOpen()) {
            control.list.move(event.key === "ArrowDown" ? 1 : -1);
        } else {
            control.open(true);
        }
    }
    // choose the highlighted option
    else if (event.key === "Enter" && control.isOpen()) {
        event.preventDefault();
        control.list.chooseHighlighted();
    }
    // close the list, or clear the input when it is closed
    else if (event.key === "Escape") {
        event.preventDefault();
        if (control.isOpen()) {
            control.open(false);
        } else {
            control.type("");
        }
    }
    // drop the last chip of a multiple combobox from an empty input
    else if (
        event.key === "Backspace" &&
        control.properties.multiple === true &&
        control.list.search() === ""
    ) {
        const last = control.values().at(-1);
        if (last !== undefined) {
            control.remove(last);
        }
    }
}

/** Render the chosen values of a multiple combobox as chips, each with a button that drops it. */
export function ComboboxChips(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // read the chosen values and their texts
    const control = useCombobox();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="combobox-chips"
            {...rest}
            {...style.attributes([styles.chips, properties.xstyle], properties.style)}
        >
            <For each={control.values()}>
                {(value) => (
                    <span
                        data-slot="combobox-chip"
                        {...style.attrs(badgeStyle({ variant: "secondary" }))}
                    >
                        {control.labelOf(value)}
                        <button
                            type="button"
                            tabindex={-1}
                            aria-label={locale.render(t`Remove ${control.labelOf(value)}`)}
                            data-slot="combobox-chip-remove"
                            onClick={() => control.remove(value)}
                            {...style.attrs(styles.remove)}
                        >
                            <Icon name="x" size="0.75em" />
                        </button>
                    </span>
                )}
            </For>
        </div>
    );
}

/** Render an option that creates one from the typed text while no item has that text. */
export function ComboboxCreate(properties: {
    /** The option's text for the typed text, `Create "…"` by default. */
    readonly children?: (text: string) => JSX.Element;
}): JSX.Element {
    // offer the option while the typed text names no item
    const control = useCombobox();
    const locale = useLocale();
    const typed = (): string => control.list.search().trim();
    const isNew = (): boolean => {
        const lowered = typed().toLowerCase();

        return (
            lowered !== "" && ![...control.items()].some((item) => item.toLowerCase() === lowered)
        );
    };

    return (
        <Show when={isNew()}>
            <CommandItem
                data-slot="combobox-create"
                value={typed()}
                textValue={typed()}
                keywords={[typed()]}
                onSelect={(created) => control.properties.onCreate?.(created)}
            >
                {properties.children?.(typed()) ?? locale.render(t`Create "${typed()}"`)}
            </CommandItem>
        </Show>
    );
}

/** Render the state of options the owner loads, such as a spinner and a message. */
export const ComboboxLoading = CommandLoading;
