import type { Direction } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createMemo,
    createSignal,
    createUniqueId,
    type JSX,
    omit,
    onCleanup,
    onSettled,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { AutocompleteContext, type AutocompleteControl } from "../autocomplete/index.ts";
import { Collection, CollectionBuilder, type CollectionSection } from "../collection/index.ts";
import { Focus, GridDelegate, ListDelegate, type Orientation } from "../focus/index.ts";
import { Selection, type SelectionProperties } from "../selection/index.ts";

/** The key of the section of options outside any section. */
const UNSECTIONED = "";

/** The list box of the nearest list box, null outside one. */
export const ListBoxContext = createContext<ListBoxControl | null>(null);

/** The section of the nearest list box section, null outside one. */
const ListBoxSectionContext = createContext<ListBoxSectionControl | null>(null);

/** The styles of a list box and its parts. */
const styles = style.create({
    listBox: {
        outlineStyle: "none",
    },
    grid: (columns: number) => ({
        display: "grid",
        gridTemplateColumns: `repeat(${String(columns)}, max-content)`,
        gap: space[1],
    }),
    option: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        paddingBlock: space[2],
        paddingInline: space[2],
        borderRadius: radius[2],
        cursor: "default",
        userSelect: "none",
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    active: {
        backgroundColor: color.accent,
        color: color.accentForeground,
    },
    disabled: {
        opacity: 0.5,
        pointerEvents: "none",
    },
    note: {
        paddingBlock: space[5],
        textAlign: "center",
    },
    separator: {
        height: 0,
        marginInline: `calc(-1 * ${space[1]})`,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        borderTopColor: color.border,
    },
    heading: {
        paddingBlock: space[1],
        paddingInline: space[2],
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
});

/** How a list box lays out its options for the keyboard: one after another, or in rows of a column count. */
export type ListBoxLayout = "list" | "grid";

/** Whether choosing an option adds it to the selection or drops it, or the focus moving selects it. */
export type SelectionBehavior = "toggle" | "replace";

/** An option of a list box, which joins its list box as it mounts. */
export interface ListBoxOption {
    /** The key of the option, unique in its list box, which names its element in virtual focus. */
    readonly key: string;
    /** The key of the section the option sits in, empty outside a section. */
    readonly section: string;
    /** The value the option stands for. */
    readonly value: Accessor<string>;
    /** The text the option shows, which typeahead matches. */
    readonly text: Accessor<string>;
    /** Further words a search matches. */
    readonly keywords: Accessor<readonly string[]>;
    /** Whether the option is unavailable. */
    readonly isDisabled: Accessor<boolean>;
    /** Whether the option shows whatever the search. */
    readonly isForced: Accessor<boolean>;
    /** Run the option's own action, as a click or Enter does. */
    readonly act: () => void;
    /** The option's element, which orders it among the others, undefined until it renders. */
    readonly element: Accessor<HTMLElement | undefined>;
}

/** The options a list box is created with. */
export interface ListBoxOptions {
    /** The search field that drives the list box, whose search filters the options and keeps the focus virtual. */
    readonly autocomplete?: AutocompleteControl | undefined;
    /** The selected values, none without one. */
    readonly selection?: Selection | undefined;
    /** Whether choosing toggles an option's value or the focus moving selects it, toggle by default. */
    readonly selectionBehavior?: SelectionBehavior | undefined;
    /** Lay the options out one after another or in rows of a column count, a list by default. */
    readonly layout?: ListBoxLayout | undefined;
    /** The options in each row of a grid layout, one by default. */
    readonly columns?: number | undefined;
    /** The arrow keys that move the focus through a list layout, vertical by default. */
    readonly orientation?: Orientation | undefined;
    /** Whether the arrow keys wrap from the last option to the first and back in a list layout. */
    readonly isLooping?: boolean | undefined;
    /** The value of the focused option, which makes the focus controlled. */
    readonly activeValue?: string | undefined;
    /** The value of the option focused at first while uncontrolled, the first option by default. */
    readonly defaultActiveValue?: string | undefined;
    /** Handle another option taking the focus. */
    readonly onActiveChange?: ((value: string) => void) | undefined;
    /** Handle an option being chosen with a click or Enter, after its own action. */
    readonly onAction?: ((option: ListBoxOption) => void) | undefined;
}

/** The options, focus and selection of a list box, which its options and sections share. */
export class ListBoxControl {
    /** The id of the list box element. */
    readonly id: string;
    /** The options in document order. */
    readonly options: CollectionBuilder<ListBoxOption>;
    /** The options the search shows, by section. */
    readonly collection: Collection<ListBoxOption>;
    /** The focused option. */
    readonly focus: Focus<string>;
    /** The selected values, undefined for a list box that only runs actions. */
    readonly selection: Selection | undefined;
    /** The layout the keys move through. */
    readonly delegate: ListDelegate<ListBoxOption> | GridDelegate<ListBoxOption>;
    /** The options the list box was created with. */
    readonly #options: ListBoxOptions;
    /** The keys of the sections that show an option. */
    readonly #shownSections: Accessor<ReadonlySet<string>>;

    /** Create an empty list box, virtually focused and filtered under a search field. */
    constructor(options: ListBoxOptions) {
        // collect the options the search shows, grouped by section in document order
        const autocomplete = options.autocomplete;
        this.id = createUniqueId();
        this.options = new CollectionBuilder((option) => option.element());
        this.#options = options;
        this.selection = options.selection;
        this.collection = new Collection({
            sections: () => sectioned(this.options.items(), autocomplete),
            key: (option) => option.key,
            text: (option) => option.text(),
            isDisabled: (option) => option.isDisabled(),
        });
        this.#shownSections = createMemo(
            () => new Set(this.collection.sections().map((section) => section.key)),
        );

        // move through the options one after another or in rows
        this.delegate =
            options.layout === "grid"
                ? new GridDelegate(this.collection, () => options.columns ?? 1)
                : new ListDelegate(this.collection, {
                      get orientation() {
                          return options.orientation ?? "vertical";
                      },
                      get isLooping() {
                          return options.isLooping === true;
                      },
                      isTypeahead: autocomplete === undefined,
                  });

        // focus the option of the controlled or first value, virtually under a search field
        this.focus = new Focus<string>({
            delegate: this.delegate,
            mode: autocomplete === undefined ? "roving" : "virtual",
            controlled: () => this.keyOf(options.activeValue),
            initial: () => this.keyOf(options.defaultActiveValue) ?? this.#firstSelected(),
            onChange: (key) => {
                // tell the owner of the value and select it when selection follows the focus
                const option = this.collection.item(key);
                if (option !== undefined) {
                    options.onActiveChange?.(option.value());
                    if (options.selectionBehavior === "replace") {
                        this.selection?.select(option.value());
                    }
                }
            },
        });
        autocomplete?.connect({
            id: this.id,
            focus: this.focus,
            isVertical: options.layout !== "grid",
            act: (key) => this.choose(key),
        });
    }

    /** Report whether an option shows under the search. */
    isShown(key: string): boolean {
        return this.collection.has(key);
    }

    /** Report whether a section shows an option under the search. */
    isSectionShown(key: string): boolean {
        return this.#shownSections().has(key);
    }

    /** Report whether an option's value is selected, or focused in a list box without selection. */
    isSelected(option: ListBoxOption): boolean {
        if (this.selection === undefined || this.focus.mode === "virtual") {
            return this.focus.isActive(option.key);
        }

        return this.selection.isSelected(option.value());
    }

    /** Read the key of the shown option of a value, undefined for no value or no such option. */
    keyOf(value: string | undefined): string | undefined {
        if (value === undefined) {
            return undefined;
        }

        return this.collection.keys().find((key) => this.collection.item(key)?.value() === value);
    }

    /** Choose an available option: run its action, then select its value. */
    choose(key: string): void {
        // ignore an unavailable option
        const option = this.collection.item(key);
        if (option === undefined || option.isDisabled()) {
            return;
        }

        // run its action, then add or replace its value
        option.act();
        this.#options.onAction?.(option);
        if (this.selection?.isMultiple() === true) {
            this.selection.toggle(option.value());
        } else {
            this.selection?.select(option.value());
        }
    }

    /** Report whether a search filters the options. */
    isSearching(): boolean {
        return (
            this.#options.autocomplete !== undefined && this.#options.autocomplete.search() !== ""
        );
    }

    /** Read the key of the first selected value's option, undefined without one. */
    #firstSelected(): string | undefined {
        return this.keyOf(this.selection?.values()[0]);
    }
}

/** The id and heading of a list box section, which its options and heading read. */
interface ListBoxSectionControl {
    /** The key of the section, the id of its element. */
    readonly key: string;
    /** Whether a heading labels the section. */
    readonly isLabelled: Accessor<boolean>;
    /** Label the section by its heading until the heading unmounts. */
    readonly label: () => void;
}

/** The properties of an element of a list box, the native element's attributes included. */
export type ListBoxElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a list box: its selection, layout and focus, the native element's attributes included. */
export type ListBoxProperties = ListBoxElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown" | "onFocusOut">
> &
    Omit<ListBoxOptions, "selection" | "autocomplete"> & {
        /** The list box state a composite created, which the element renders in place of its own. */
        readonly control?: ListBoxControl;
        /** The selected values, none for a list box of actions. */
        readonly selection?: SelectionProperties;
    };

/** The properties of an option of a list box, the native element's attributes included. */
export interface ListBoxItemProperties extends ListBoxElementProperties<
    Omit<
        JSX.HTMLAttributes<HTMLDivElement>,
        "onSelect" | "onClick" | "onPointerMove" | "onPointerDown" | "onFocus" | "ref"
    >
> {
    /** The value the option stands for, its text by default. */
    readonly value?: string;
    /** The text the option stands for once chosen, its content by default. */
    readonly textValue?: string;
    /** Further words a search matches, such as synonyms. */
    readonly keywords?: readonly string[];
    /** Whether the option is unavailable. */
    readonly disabled?: boolean;
    /** Whether the option shows whatever the search. */
    readonly forceMount?: boolean;
    /** Handle the option being chosen with a click or Enter. */
    readonly onSelect?: (value: string) => void;
}

/** The properties of a section of a list box, the native element's attributes included. */
export interface ListBoxSectionProperties extends ListBoxElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The heading that names the section, rendered as its heading part. */
    readonly heading?: JSX.Element;
    /** Whether the section shows whatever the search. */
    readonly forceMount?: boolean;
}

/** Read the list box of the nearest list box, refusing parts outside one. */
export function useListBox(): ListBoxControl {
    const control = useContext(ListBoxContext);
    if (control === null) {
        throw new TypeError("a list box part needs a list box around it");
    }

    return control;
}

/** Render a list box of options, which arrow keys, Home, End and typed letters move through and Enter, Space or a click choose, or a composite's list box state, driven by the search of an autocomplete around it. */
export function ListBox(properties: ListBoxProperties): JSX.Element {
    // take a composite's state, else hold the list box's own selection and focus under any search around it
    const locale = useLocale();
    const autocomplete = useContext(AutocompleteContext) ?? undefined;
    const control = untrack(
        () =>
            properties.control ??
            new ListBoxControl({
                ...ownOptions(properties),
                autocomplete,
                selection:
                    properties.selection === undefined
                        ? undefined
                        : new Selection(properties.selection),
            }),
    );
    const rest = omit(
        properties,
        "control",
        "selection",
        "selectionBehavior",
        "layout",
        "columns",
        "orientation",
        "isLooping",
        "activeValue",
        "defaultActiveValue",
        "onActiveChange",
        "onAction",
        "xstyle",
        "style",
        "children",
    );
    const isGrid = (): boolean => properties.layout === "grid";

    return (
        <ListBoxContext value={control}>
            <div
                id={control.id}
                role="listbox"
                aria-multiselectable={control.selection?.isMultiple() === true ? "true" : undefined}
                aria-orientation={
                    isGrid() || properties.orientation === "both"
                        ? undefined
                        : properties.orientation
                }
                data-slot="list-box"
                data-layout={properties.layout ?? "list"}
                {...rest}
                onKeyDown={(event) => steer(event, control, locale.direction)}
                onFocusOut={(event) => control.focus.focusOut(event)}
                {...style.attributes(
                    [
                        styles.listBox,
                        isGrid() && styles.grid(properties.columns ?? 1),
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            >
                {properties.children}
            </div>
        </ListBoxContext>
    );
}

/** Render a section of options under a heading, hidden while the search shows none of them. */
export function ListBoxSection(properties: ListBoxSectionProperties): JSX.Element {
    // label the section by its heading and hide it once none of its options show
    const control = useListBox();
    const [isLabelled, setLabelled] = createSignal(false, { ownedWrite: true });
    const section: ListBoxSectionControl = {
        key: createUniqueId(),
        isLabelled,
        label: () => {
            setLabelled(true);
            onCleanup(() => setLabelled(false));
        },
    };
    const rest = omit(properties, "heading", "forceMount", "xstyle", "style", "children");

    return (
        <ListBoxSectionContext value={section}>
            <div
                role="group"
                aria-labelledby={section.isLabelled() ? `${section.key}-heading` : undefined}
                hidden={properties.forceMount !== true && !control.isSectionShown(section.key)}
                data-slot="list-box-section"
                {...rest}
                {...style.attributes([properties.xstyle], properties.style)}
            >
                <Show when={"heading" in properties}>
                    <ListBoxSectionHeading>{properties.heading}</ListBoxSectionHeading>
                </Show>
                {properties.children}
            </div>
        </ListBoxSectionContext>
    );
}

/** Render the heading that labels the nearest list box section. */
export function ListBoxSectionHeading(
    properties: ListBoxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // label the section for as long as the heading renders
    const section = useContext(ListBoxSectionContext);
    if (section === null) {
        throw new TypeError("a list box section heading needs a list box section around it");
    }
    section.label();
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            id={`${section.key}-heading`}
            aria-hidden="true"
            data-slot="list-box-section-heading"
            {...rest}
            {...style.attributes(
                [text.caption, styles.heading, properties.xstyle],
                properties.style,
            )}
        />
    );
}

/** Render an option the search filters and a click or Enter chooses, marked while focused and selected. */
export function ListBoxItem(properties: ListBoxItemProperties): JSX.Element {
    // join the list box with the option's value, its text unless one is passed
    const control = useListBox();
    const rest = omit(
        properties,
        "value",
        "textValue",
        "keywords",
        "disabled",
        "forceMount",
        "onSelect",
        "xstyle",
        "style",
    );
    let element: HTMLDivElement | undefined;
    const [content, setContent] = createSignal("", { ownedWrite: true });
    onSettled(() => {
        setContent(element?.textContent?.trim() ?? "");
    });
    const option: ListBoxOption = {
        key: createUniqueId(),
        section: useContext(ListBoxSectionContext)?.key ?? UNSECTIONED,
        value: () => properties.value ?? content(),
        text: () => properties.textValue ?? content(),
        keywords: () => properties.keywords ?? [],
        isDisabled: () => properties.disabled === true,
        isForced: () => properties.forceMount === true,
        act: () => properties.onSelect?.(option.value()),
        element: () => element,
    };
    control.options.add(option);
    control.focus.bind(() => option.key, option.element);
    const isActive = (): boolean => control.focus.isActive(option.key);
    const isVirtual = control.focus.mode === "virtual";

    return (
        <div
            id={control.focus.id(option.key)}
            role="option"
            aria-selected={control.isSelected(option) ? "true" : "false"}
            aria-disabled={properties.disabled === true ? "true" : undefined}
            tabindex={isVirtual ? undefined : isActive() ? 0 : -1}
            hidden={!control.isShown(option.key)}
            data-slot="list-box-item"
            data-highlighted={isActive() ? "" : undefined}
            data-disabled={properties.disabled === true ? "" : undefined}
            data-value={option.value()}
            {...rest}
            ref={(item) => (element = item)}
            onPointerDown={(event) => {
                // keep the focus in the search field of a virtual list box
                if (isVirtual) {
                    event.preventDefault();
                }
            }}
            onPointerMove={() => {
                // focus an available option under the pointer behind a search field
                if (isVirtual && !option.isDisabled()) {
                    control.focus.focus(option.key);
                }
            }}
            onFocus={() => control.focus.focusIn(option.key)}
            onClick={() => control.choose(option.key)}
            {...style.attributes(
                [
                    styles.option,
                    isVirtual && isActive() && styles.active,
                    properties.disabled === true && styles.disabled,
                    properties.xstyle,
                ],
                properties.style,
            )}
        />
    );
}

/** Render a message while no option shows, such as when the search matches none. */
export function ListBoxEmpty(
    properties: ListBoxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useListBox();
    const rest = omit(properties, "xstyle", "style");

    return (
        <Show when={control.collection.size() === 0}>
            <div
                role="presentation"
                data-slot="list-box-empty"
                {...rest}
                {...style.attributes([styles.note, properties.xstyle], properties.style)}
            />
        </Show>
    );
}

/** Render the state of options the owner loads, announced as it changes. */
export function ListBoxLoading(
    properties: ListBoxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            role="status"
            data-slot="list-box-loading"
            {...rest}
            {...style.attributes([styles.note, properties.xstyle], properties.style)}
        />
    );
}

/** Render a line between sections of options, hidden while a search filters them. */
export function ListBoxSeparator(
    properties: ListBoxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useListBox();
    const rest = omit(properties, "xstyle", "style");

    return (
        <Show when={!control.isSearching()}>
            <div
                role="separator"
                data-slot="list-box-separator"
                {...rest}
                {...style.attributes([styles.separator, properties.xstyle], properties.style)}
            />
        </Show>
    );
}

/** Read the list box options a list box's properties hold, without its element's attributes. */
function ownOptions(properties: ListBoxProperties): ListBoxOptions {
    return {
        get selectionBehavior() {
            return properties.selectionBehavior;
        },
        get layout() {
            return properties.layout;
        },
        get columns() {
            return properties.columns;
        },
        get orientation() {
            return properties.orientation;
        },
        get isLooping() {
            return properties.isLooping;
        },
        get activeValue() {
            return properties.activeValue;
        },
        get defaultActiveValue() {
            return properties.defaultActiveValue;
        },
        get onActiveChange() {
            return properties.onActiveChange;
        },
        get onAction() {
            return properties.onAction;
        },
    };
}

/** Group the options a search shows by section, in the order each section's first option mounted. */
function sectioned(
    options: readonly ListBoxOption[],
    autocomplete: AutocompleteControl | undefined,
): CollectionSection<ListBoxOption>[] {
    const sections = new Map<string, ListBoxOption[]>();
    for (const option of options) {
        // keep a forced option, else one the search matches
        if (
            autocomplete !== undefined &&
            !option.isForced() &&
            !autocomplete.matches(option.value(), option.keywords())
        ) {
            continue;
        }
        const items = sections.get(option.section);
        if (items === undefined) {
            sections.set(option.section, [option]);
        } else {
            items.push(option);
        }
    }

    return [...sections].map(([key, items]) => ({ key, items }));
}

/** Move the focus of a roving list box, and choose its focused option with Enter or Space. */
function steer(event: KeyboardEvent, control: ListBoxControl, direction: Direction): void {
    // leave keys to the search field of a virtual list box
    if (control.focus.mode === "virtual" || control.focus.move(event, direction) !== undefined) {
        return;
    }

    // choose the focused option
    const active = control.focus.current();
    if ((event.key === "Enter" || event.key === " ") && active !== undefined) {
        event.preventDefault();
        control.choose(active);
    }
}
