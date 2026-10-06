import * as style from "@destack/style";
import { color, motion, radius, shadow, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createEffect,
    createSignal,
    omit,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";
import {
    CommandControl,
    CommandEmpty,
    CommandGroup,
    CommandItem,
    CommandProvider,
    CommandSeparator,
    type CommandElementProperties,
    type CommandItemProperties,
} from "../command/index.ts";
import { placementStyle } from "../popover/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The condition under which a list sits below its input instead of the viewport's center. */
const ANCHORED = "@supports (position-area: block-end)";

/** The combobox of the nearest combobox root, null outside one. */
const ComboboxContext = createContext<ComboboxControl | null>(null);

/** The styles of a combobox's input and list. */
const styles = style.create({
    input: {
        width: "100%",
        height: size[3],
        paddingInline: space[3],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: { default: color.input, ":focus-visible": color.ring },
        borderRadius: radius[3],
        backgroundColor: "transparent",
        boxShadow: shadow.inset,
        color: color.foreground,
        transitionProperty: "border-color, outline-color",
        transitionDuration: motion.durationShort,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
        "::placeholder": { color: color.mutedForeground },
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
    /** The properties of the root, read for its controlled value and change handler. */
    readonly #properties: ComboboxProperties;
    /** The chosen value when uncontrolled. */
    readonly #ownValue: Accessor<string | undefined>;
    /** Replace the chosen value when uncontrolled. */
    readonly #setValue: Setter<string | undefined>;
    /** Replace whether the list is open when uncontrolled. */
    readonly #setOpen: Setter<boolean>;
    /** The input that anchors the list. */
    #input: HTMLInputElement | undefined;
    /** The list element. */
    #content: HTMLElement | undefined;

    /** Create a combobox whose list chooses its value, open when its properties ask for it. */
    constructor(properties: ComboboxProperties) {
        // start on the default value and open state, with the list choosing values
        const [ownValue, setValue] = createSignal(properties.defaultValue);
        const [isOpen, setOpen] = createSignal(properties.defaultOpen === true);
        this.list = new CommandControl({
            get shouldFilter() {
                return properties.shouldFilter !== false;
            },
        });
        this.isOpen = () => properties.open ?? isOpen();
        this.#properties = properties;
        this.#ownValue = ownValue;
        this.#setValue = setValue;
        this.#setOpen = setOpen;
        this.#input = undefined;
        this.#content = undefined;
        this.list.onChoose = (value, label) => this.choose(value, label);
    }

    /** Read the chosen value, controlled or the combobox's own. */
    value(): string | undefined {
        return this.#properties.value ?? this.#ownValue();
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
            this.#properties.onOpenChange?.(isOpen);
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

    /** Take an option's value, write its text into the input and close the list. */
    choose(value: string, label: string): void {
        // keep and report the value
        this.#setValue(value);
        this.#properties.onValueChange?.(value);

        // show the option's text in the input and close the list
        this.list.type(label);
        this.open(false);
    }
}

/** The properties of a combobox root. */
export interface ComboboxProperties {
    /** The chosen value, which makes the choice controlled. */
    readonly value?: string;
    /** The value chosen at first when the choice is uncontrolled. */
    readonly defaultValue?: string;
    /** Handle another value being chosen. */
    readonly onValueChange?: (value: string) => void;
    /** Whether the list is open, which makes the open state controlled. */
    readonly open?: boolean;
    /** Whether the list starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean;
    /** Handle the list opening or closing. */
    readonly onOpenChange?: (open: boolean) => void;
    /** Whether the list filters its options by the typed text, or its owner does, true by default. */
    readonly shouldFilter?: boolean;
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
    const rest = omit(properties, "style");

    return (
        <input
            role="combobox"
            aria-expanded={control.isOpen() ? "true" : "false"}
            aria-controls={list.listId}
            aria-autocomplete="list"
            aria-activedescendant={control.isOpen() ? list.highlighted() : undefined}
            autocomplete="off"
            data-slot="combobox-input"
            {...rest}
            ref={(element) => control.setInput(element)}
            value={list.search()}
            onInput={(event) => {
                // filter by the typed text and show the matches
                list.type(event.currentTarget.value);
                control.open(true);
            }}
            onKeyDown={(event) => steer(event, control)}
            onBlur={() => control.open(false)}
            {...style.attrs(text.callout, styles.input, properties.style)}
        />
    );
}

/** Render the list of options below the input. */
export function ComboboxContent(
    properties: CommandElementProperties<Omit<JSX.HTMLAttributes<HTMLDivElement>, "ref">>,
): JSX.Element {
    // show and hide the list as the combobox opens and closes
    const control = useCombobox();
    const rest = omit(properties, "style");
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
                {...style.attrs(
                    text.footnote,
                    styles.content,
                    placementStyle("bottom", "start"),
                    properties.style,
                )}
            />
        </TopLayer>
    );
}

/** Render an option that a click or Enter chooses as the combobox's value. */
export function ComboboxItem(properties: CommandItemProperties): JSX.Element {
    const control = useCombobox();
    const isChosen = (): boolean =>
        properties.value !== undefined && control.value() === properties.value;

    return (
        <CommandItem
            data-slot="combobox-item"
            data-checked={isChosen() ? "true" : undefined}
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
            control.list.type("");
        }
    }
}
