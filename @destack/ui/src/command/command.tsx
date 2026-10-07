import { Icon } from "@destack/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createSignal,
    createUniqueId,
    type JSX,
    omit,
    onCleanup,
    type Setter,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogTitle,
    type DialogProperties,
} from "../dialog/index.ts";

/** The command list of the nearest command, null outside one. */
const CommandContext = createContext<CommandControl | null>(null);

/** The id of the nearest command group, null outside one. */
const CommandGroupContext = createContext<string | null>(null);

/** The styles of a command and its elements. */
const styles = style.create({
    command: {
        display: "flex",
        flexDirection: "column",
        width: "100%",
        overflow: "hidden",
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.popoverForeground,
    },
    search: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        height: size[3],
        paddingInline: space[3],
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        borderBottomColor: color.border,
        color: color.mutedForeground,
    },
    input: {
        flex: 1,
        height: "100%",
        padding: 0,
        borderWidth: 0,
        outlineStyle: "none",
        backgroundColor: "transparent",
        color: color.foreground,
        "::placeholder": { color: color.mutedForeground },
    },
    list: {
        maxHeight: `calc(8 * ${size[3]})`,
        overflowX: "hidden",
        overflowY: "auto",
        padding: space[1],
    },
    empty: {
        paddingBlock: space[5],
        textAlign: "center",
    },
    heading: {
        paddingBlock: space[1],
        paddingInline: space[2],
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
    item: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        paddingBlock: space[2],
        paddingInline: space[2],
        borderRadius: radius[2],
        cursor: "default",
        userSelect: "none",
    },
    active: {
        backgroundColor: color.accent,
        color: color.accentForeground,
    },
    disabled: {
        opacity: 0.5,
        pointerEvents: "none",
    },
    separator: {
        height: 0,
        marginInline: `calc(-1 * ${space[1]})`,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        borderTopColor: color.border,
    },
    shortcut: {
        marginInlineStart: "auto",
        color: color.mutedForeground,
        letterSpacing: "0.1em",
    },
    hidden: {
        position: "absolute",
        width: stroke.border,
        height: stroke.border,
        overflow: "hidden",
        clipPath: "inset(50%)",
        whiteSpace: "nowrap",
    },
});

/** An option of a command list, which the list filters and highlights. */
export interface CommandOption {
    /** The id of the option element. */
    readonly id: string;
    /** The id of the option's group, null outside a group. */
    readonly group: string | null;
    /** The text the search matches, the option's value or its text. */
    readonly value: () => string;
    /** Further words the search matches. */
    readonly keywords: readonly string[];
    /** Whether the option is unavailable. */
    readonly isDisabled: () => boolean;
    /** Choose the option. */
    readonly choose: () => void;
}

/** The search, options and highlighted option of a command list, which its input, list and options share. */
export class CommandControl {
    /** The id of the list element. */
    readonly listId: string;
    /** The text typed into the search. */
    readonly search: Accessor<string>;
    /** The options in document order. */
    readonly options: Accessor<readonly CommandOption[]>;
    /** The properties of the list's owner, read for filtering and the controlled highlight. */
    readonly #properties: CommandListProperties;
    /** Handle an option being chosen with its value and text, such as a combobox taking it. */
    onChoose: ((value: string, text: string) => void) | undefined;
    /** The value of the option the person last highlighted, when uncontrolled. */
    readonly #highlighted: Accessor<string | undefined>;
    /** Replace the search. */
    readonly #setSearch: Setter<string>;
    /** Replace the options. */
    readonly #setOptions: Setter<readonly CommandOption[]>;
    /** Replace the highlighted option. */
    readonly #setHighlighted: Setter<string | undefined>;

    /** Create an empty command list, filtering by its search unless its owner asks not to. */
    constructor(properties: CommandListProperties) {
        // start with an empty search and no options
        const [search, setSearch] = createSignal("");
        const [options, setOptions] = createSignal<readonly CommandOption[]>([], {
            ownedWrite: true,
        });
        const [highlighted, setHighlighted] = createSignal<string | undefined>(undefined);
        this.listId = createUniqueId();
        this.search = search;
        this.options = options;
        this.#properties = properties;
        this.onChoose = undefined;
        this.#highlighted = highlighted;
        this.#setSearch = setSearch;
        this.#setOptions = setOptions;
        this.#setHighlighted = setHighlighted;
    }

    /** Add an option until it unmounts. */
    register(option: CommandOption): void {
        this.#setOptions((options) => [...options, option]);
        onCleanup(() => this.#setOptions((options) => options.filter((entry) => entry !== option)));
    }

    /** Replace the search, highlighting the first match again. */
    type(search: string): void {
        this.#setSearch(search);
        this.#setHighlighted(undefined);
    }

    /** Report whether an option matches the search, ignoring case. */
    matches(option: CommandOption): boolean {
        const search = this.search().trim().toLowerCase();
        if (this.#properties.shouldFilter === false || search === "") {
            return true;
        }

        return [option.value(), ...option.keywords].some((word) =>
            word.toLowerCase().includes(search),
        );
    }

    /** List the options the search shows, in document order. */
    shown(): CommandOption[] {
        return this.options().filter((option) => this.matches(option));
    }

    /** Read the highlighted option's id: the last one highlighted while shown, else the first enabled one. */
    highlighted(): string | undefined {
        const enabled = this.shown().filter((option) => !option.isDisabled());
        const value = this.#properties.value ?? this.#highlighted();

        return (enabled.find((option) => option.value() === value) ?? enabled[0])?.id;
    }

    /** Highlight an option and tell the owner's change handler its value. */
    highlight(option: CommandOption): void {
        // skip the option already highlighted
        const value = option.value();
        if (option.id === this.highlighted()) {
            return;
        }
        this.#setHighlighted(value);
        this.#properties.onValueChange?.(value);
    }

    /** Highlight the next or previous enabled option, stopping at the ends, and scroll it into view. */
    move(offset: 1 | -1): void {
        // step from the highlighted option, staying within the list
        const enabled = this.shown().filter((option) => !option.isDisabled());
        const index = enabled.findIndex((option) => option.id === this.highlighted());
        const next = enabled[Math.min(Math.max(index + offset, 0), enabled.length - 1)];
        if (next === undefined) {
            return;
        }

        // highlight it and bring it into view
        this.highlight(next);
        document.getElementById(next.id)?.scrollIntoView?.({ block: "nearest" });
    }

    /** Choose the highlighted option, if any. */
    chooseHighlighted(): void {
        const id = this.highlighted();
        this.options()
            .find((option) => option.id === id)
            ?.choose();
    }
}

/** The filtering and highlight of a command list, which its owner controls. */
export interface CommandListProperties {
    /** Whether the list filters its options by the search, true by default. */
    readonly shouldFilter?: boolean;
    /** The highlighted option's value, which makes the highlight controlled. */
    readonly value?: string;
    /** Handle another option being highlighted. */
    readonly onValueChange?: (value: string) => void;
}

/** The properties of a command, the native element's attributes included. */
export interface CommandProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class">, CommandListProperties {
    /** The StyleX styles applied after the command's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an element of a command, the native element's attributes included. */
export type CommandElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a command option. */
export interface CommandItemProperties extends CommandElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onSelect" | "onClick" | "onPointerMove" | "ref">
> {
    /** The text the option stands for once chosen, its content by default. */
    readonly textValue?: string;
    /** The text the search matches and the value chosen, the option's text by default. */
    readonly value?: string;
    /** Further words the search matches, such as synonyms. */
    readonly keywords?: readonly string[];
    /** Whether the option is unavailable. */
    readonly disabled?: boolean;
    /** Handle the option being chosen with a click or Enter. */
    readonly onSelect?: (value: string) => void;
}

/** The properties of a command dialog. */
export interface CommandDialogProperties extends DialogProperties {
    /** The title assistive technology announces, a generic one by default. */
    readonly title?: string;
    /** The description assistive technology announces, a generic one by default. */
    readonly description?: string;
}

/** Read the command list of the nearest command, refusing elements outside one. */
export function useCommand(): CommandControl {
    const control = useContext(CommandContext);
    if (control === null) {
        throw new TypeError("command elements need a command around them");
    }

    return control;
}

/** Provide a command list to elements that build one, such as a combobox. */
export function CommandProvider(properties: {
    readonly control: CommandControl;
    readonly children?: JSX.Element;
}): JSX.Element {
    return <CommandContext value={properties.control}>{properties.children}</CommandContext>;
}

/** Render a searchable list of commands, its search filtering them as the person types. */
export function Command(properties: CommandProperties): JSX.Element {
    const control = new CommandControl(properties);
    const rest = omit(properties, "shouldFilter", "value", "onValueChange", "xstyle", "style");

    return (
        <CommandProvider control={control}>
            <div
                data-slot="command"
                {...rest}
                {...style.attributes(
                    [text.footnote, styles.command, properties.xstyle],
                    properties.style,
                )}
            />
        </CommandProvider>
    );
}

/** Render the search of a command, which the arrow keys and Enter steer through the list. */
export function CommandInput(
    properties: CommandElementProperties<
        Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "value" | "onInput" | "onKeyDown">
    >,
): JSX.Element {
    const control = useCommand();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div data-slot="command-input-wrapper" {...style.attrs(styles.search)}>
            <Icon name="magnifying-glass" />
            <input
                role="combobox"
                aria-expanded="true"
                aria-controls={control.listId}
                aria-autocomplete="list"
                aria-activedescendant={control.highlighted()}
                autocomplete="off"
                spellcheck={false}
                data-slot="command-input"
                {...rest}
                value={control.search()}
                onInput={(event) => control.type(event.currentTarget.value)}
                onKeyDown={(event) => steer(event, control)}
                {...style.attributes(
                    [text.callout, styles.input, properties.xstyle],
                    properties.style,
                )}
            />
        </div>
    );
}

/** Render the list box of a command's options. */
export function CommandList(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useCommand();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            id={control.listId}
            role="listbox"
            data-slot="command-list"
            {...rest}
            {...style.attributes([styles.list, properties.xstyle], properties.style)}
        />
    );
}

/** Render a message while the search matches no option. */
export function CommandEmpty(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useCommand();
    const rest = omit(properties, "xstyle", "style");

    return (
        <Show when={control.shown().length === 0}>
            <div
                role="presentation"
                data-slot="command-empty"
                {...rest}
                {...style.attributes([styles.empty, properties.xstyle], properties.style)}
            />
        </Show>
    );
}

/** Render the state of options the owner loads, announced as it changes. */
export function CommandLoading(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            role="status"
            data-slot="command-loading"
            {...rest}
            {...style.attributes([styles.empty, properties.xstyle], properties.style)}
        />
    );
}

/** Render a group of options under a heading, hidden while the search matches none of them. */
export function CommandGroup(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>> & {
        readonly heading?: JSX.Element;
    },
): JSX.Element {
    // name the group and hide it once none of its options match
    const control = useCommand();
    const id = createUniqueId();
    const rest = omit(properties, "heading", "xstyle", "style", "children");
    const isEmpty = (): boolean => !control.shown().some((option) => option.group === id);

    return (
        <CommandGroupContext value={id}>
            <div
                role="group"
                aria-labelledby={"heading" in properties ? `${id}-heading` : undefined}
                hidden={isEmpty()}
                data-slot="command-group"
                {...rest}
                {...style.attributes([properties.xstyle], properties.style)}
            >
                <Show when={"heading" in properties}>
                    <div
                        id={`${id}-heading`}
                        aria-hidden="true"
                        {...style.attrs(text.caption, styles.heading)}
                    >
                        {properties.heading}
                    </div>
                </Show>
                {properties.children}
            </div>
        </CommandGroupContext>
    );
}

/** Render an option that the search filters and a click or Enter chooses. */
export function CommandItem(properties: CommandItemProperties): JSX.Element {
    // join the list with the option's value, its text unless one is passed
    const control = useCommand();
    const rest = omit(
        properties,
        "value",
        "textValue",
        "keywords",
        "disabled",
        "onSelect",
        "xstyle",
        "style",
    );
    let element: HTMLDivElement | undefined;
    const option: CommandOption = {
        id: createUniqueId(),
        group: useContext(CommandGroupContext),
        value: () => properties.value ?? element?.textContent?.trim() ?? "",
        get keywords() {
            return properties.keywords ?? [];
        },
        isDisabled: () => properties.disabled === true,
        choose: () => {
            properties.onSelect?.(option.value());
            control.onChoose?.(
                option.value(),
                properties.textValue ?? element?.textContent?.trim() ?? option.value(),
            );
        },
    };
    control.register(option);
    const isHighlighted = (): boolean => control.highlighted() === option.id;

    return (
        <div
            id={option.id}
            role="option"
            aria-selected={isHighlighted() ? "true" : "false"}
            aria-disabled={properties.disabled === true ? "true" : undefined}
            hidden={!control.matches(option)}
            data-slot="command-item"
            data-highlighted={isHighlighted() ? "" : undefined}
            data-disabled={properties.disabled === true ? "" : undefined}
            {...rest}
            ref={(item) => (element = item)}
            onPointerDown={(event) => event.preventDefault()}
            onPointerMove={() => {
                // highlight an enabled option under the pointer
                if (!option.isDisabled()) {
                    control.highlight(option);
                }
            }}
            onClick={() => {
                // choose an enabled option
                if (!option.isDisabled()) {
                    option.choose();
                }
            }}
            {...style.attributes(
                [
                    styles.item,
                    isHighlighted() && styles.active,
                    properties.disabled === true && styles.disabled,
                    properties.xstyle,
                ],
                properties.style,
            )}
        />
    );
}

/** Render a line between groups of options. */
export function CommandSeparator(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useCommand();
    const rest = omit(properties, "xstyle", "style");

    return (
        <Show when={control.search() === ""}>
            <div
                role="separator"
                data-slot="command-separator"
                {...rest}
                {...style.attributes([styles.separator, properties.xstyle], properties.style)}
            />
        </Show>
    );
}

/** Render the keyboard shortcut of an option at its end. */
export function CommandShortcut(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <span
            data-slot="command-shortcut"
            {...rest}
            {...style.attributes(
                [text.caption, styles.shortcut, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render a command in a modal dialog, titled for assistive technology. */
export function CommandDialog(properties: CommandDialogProperties): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "title", "description", "children");

    return (
        <Dialog {...rest}>
            <DialogContent showCloseButton={false}>
                <DialogTitle xstyle={styles.hidden}>
                    {properties.title ?? locale.render(t`Command palette`)}
                </DialogTitle>
                <DialogDescription xstyle={styles.hidden}>
                    {properties.description ?? locale.render(t`Search for a command to run`)}
                </DialogDescription>
                <Command>{properties.children}</Command>
            </DialogContent>
        </Dialog>
    );
}

/** Move the highlight with the up and down arrow keys and choose it with Enter. */
function steer(event: KeyboardEvent, control: CommandControl): void {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        control.move(event.key === "ArrowDown" ? 1 : -1);
    } else if (event.key === "Enter") {
        event.preventDefault();
        control.chooseHighlighted();
    }
}
