import { field } from "@destack/object";
import check from "@destack/icon/phosphor/check";
import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import { type Direction, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, space, stroke, weight, size } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createMemo,
    createSignal,
    createUniqueId,
    type JSX,
    merge,
    omit,
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
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The key of the section of options outside any section. */
const UNSECTIONED = "";

/** The list box of the nearest list box, null outside one. */
export const ListBoxContext = createContext<ListBoxControl | null>(null);

/** The section of the nearest list box section, null outside one. */
const ListBoxSectionContext = createContext<ListBoxSectionControl | null>(null);

/** The attributes of a note in place of options: an option nobody picks. */
const NOTE = { role: "option", "aria-disabled": "true", "aria-selected": "false" } as const;

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
        minHeight: size[2],
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[2],
        cursor: "default",
        userSelect: "none",
    },
    check: {
        marginInlineStart: "auto",
        color: color.mutedForeground,
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

/** How a list box lays out its options for the keyboard. */
export type ListBoxLayout = keyof typeof LIST_BOX_LAYOUTS;

/** Whether choosing an option adds it to the selection or drops it. */
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
    /** The search field that drives the list box. */
    readonly autocomplete?: AutocompleteControl | undefined;
    /** The selected values, none without one. */
    readonly selection?: Selection | undefined;
    /** Whether choosing toggles an option's value or the focus moving selects it. */
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
    /** Whether the options lie in a grid. */
    readonly isGrid: boolean;
    /** Whether the choice follows the focus. */
    readonly isFollowing: boolean;
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
        this.isGrid = options.layout === "grid";
        this.isFollowing = options.selectionBehavior === "replace";
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
                        this.selection?.add(option.value());
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

    /** Read the value of the option the highlight is on, undefined while none is. */
    activeValue(): string | undefined {
        const key = this.focus.current();

        return key === undefined ? undefined : this.collection.item(key)?.value();
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
            this.selection?.add(option.value());
        }
    }

    /** Report whether a search filters the options. */
    isSearching(): boolean {
        return (
            this.#options.autocomplete !== undefined && this.#options.autocomplete.value() !== ""
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
    readonly label: (id: string) => void;
}

/** The properties of an element of a list box, the native element's attributes included. */
export type ListBoxElementProperties<Attributes> = Omit<Attributes, "class"> &
    ElementPartProperties;

/** The layouts of a list box's options. */
const LIST_BOX_LAYOUTS = {
    list: {
        title: t.context("option")`List`,
        description: t`One after another.`,
    },
    grid: {
        title: t.context("option")`Grid`,
        description: t`In rows of a column count.`,
    },
};

/** The properties people change in a list box's controls and inspector. */
const LIST_BOX_PROPERTIES = {
    layout: field
        .enum(LIST_BOX_LAYOUTS)
        .labeled(t.context("label")`Layout`)
        .described(t`Whether the options run one after another or in rows.`)
        .default("list"),
};

/** The properties of a list box besides its declared ones: its selection and focus. */
export type ListBoxAttributes = ListBoxElementProperties<JSX.HTMLAttributes<HTMLDivElement>> &
    Omit<ListBoxOptions, "selection" | "autocomplete" | "layout"> & {
        /** The list box state a composite created. */
        readonly control?: ListBoxControl;
        /** The selected values, none for a list box of actions. */
        readonly selection?: SelectionProperties;
    };

/** The properties people change in a list box option's controls and inspector. */
const LIST_BOX_ITEM_PROPERTIES = {
    disabled: field
        .boolean()
        .labeled(t.context("label")`Disabled`)
        .described(t`Refuses the option where it cannot be chosen now.`)
        .default(false),
};

/** The properties of an option besides its declared ones: its value, words and attributes. */
export interface ListBoxItemAttributes extends ListBoxElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onSelect">
> {
    /** The value the option stands for, its text by default. */
    readonly value?: string;
    /** The text the option stands for once chosen, its content by default. */
    readonly textValue?: string;
    /** Further words a search matches, such as synonyms. */
    readonly keywords?: readonly string[];
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

/** Render a list box of options, or a composite's list box state. */
export const ListBox = defineComponent({
    name: "listBox",
    properties: LIST_BOX_PROPERTIES,
    render(
        properties: RenderProperties<ListBoxAttributes, typeof LIST_BOX_PROPERTIES>,
    ): JSX.Element {
        // take a composite's state, else the list box's selection and focus under any search around it
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
                            : new Selection(merge(() => properties.selection ?? {})),
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
        );
        const isGrid = (): boolean => properties.layout === "grid";

        return (
            <ListBoxContext value={control}>
                {renderPart(
                    "div",
                    "list-box",
                    rest,
                    () => [styles.listBox, isGrid() && styles.grid(properties.columns ?? 1)],
                    {
                        id: control.id,
                        role: "listbox" as const,
                        get "aria-multiselectable"() {
                            return control.selection?.isMultiple() === true ? "true" : undefined;
                        },
                        get "aria-orientation"() {
                            return isGrid() || properties.orientation === "both"
                                ? undefined
                                : properties.orientation;
                        },
                        get "data-layout"() {
                            return properties.layout;
                        },
                        onKeyDown: (event) => steer(event, control, locale.direction),
                        onFocusOut: (event) => control.focus.focusOut(event),
                    },
                )}
            </ListBoxContext>
        );
    },
});

/** The properties of a list box: its selection, layout and focus. */
export type ListBoxProperties = ComponentProperties<typeof ListBox>;

/** Render a section of options under a heading, hidden while the search shows none of them. */
export function ListBoxSection(properties: ListBoxSectionProperties): JSX.Element {
    // label the section by its heading and hide it once none of its options show
    const control = useListBox();
    const headings = new CollectionBuilder<string>();
    const section: ListBoxSectionControl = {
        key: createUniqueId(),
        isLabelled: () => headings.size() > 0,
        label: (id) => {
            headings.add(id);
        },
    };
    const rest = omit(properties, "heading", "forceMount", "children");

    return (
        <ListBoxSectionContext value={section}>
            {renderPart("div", "list-box-section", rest, null, {
                role: "group" as const,
                get "aria-labelledby"() {
                    return section.isLabelled() ? `${section.key}-heading` : undefined;
                },
                get hidden() {
                    return properties.forceMount !== true && !control.isSectionShown(section.key);
                },
                get children() {
                    return (
                        <>
                            <Show when={"heading" in properties}>
                                <ListBoxSectionHeading>{properties.heading}</ListBoxSectionHeading>
                            </Show>
                            {properties.children}
                        </>
                    );
                },
            })}
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
    section.label(`${section.key}-heading`);

    return renderPart("div", "list-box-section-heading", properties, [text.xs, styles.heading], {
        get id() {
            return `${section.key}-heading`;
        },
        "aria-hidden": "true",
    });
}

/** Render an option the search filters and a click or Enter chooses. */
export const ListBoxItem = defineComponent({
    name: "listBoxItem",
    properties: LIST_BOX_ITEM_PROPERTIES,
    render(
        properties: RenderProperties<ListBoxItemAttributes, typeof LIST_BOX_ITEM_PROPERTIES>,
    ): JSX.Element {
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
            "children",
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
            isDisabled: () => properties.disabled,
            isForced: () => properties.forceMount === true,
            act: () => properties.onSelect?.(option.value()),
            element: () => element,
        };
        control.options.add(option);
        control.focus.bind(() => option.key, option.element);
        const isActive = (): boolean => control.focus.isActive(option.key);
        const isVirtual = control.focus.mode === "virtual";

        // mark a chosen option: filled where the choice follows the focus or in a grid, else by a check
        const isChosen = (): boolean =>
            control.selection !== undefined && !isVirtual && control.isSelected(option);
        const isFilled = control.isGrid || control.isFollowing;

        return renderPart(
            "div",
            "list-box-item",
            rest,
            () => [
                state.focus,
                styles.option,
                isVirtual && isActive() && state.selected,
                isChosen() && isFilled && state.selected,
                properties.disabled && state.inert,
            ],
            {
                get id() {
                    return control.focus.id(option.key);
                },
                role: "option" as const,
                get "aria-selected"() {
                    return control.isSelected(option) ? "true" : "false";
                },
                get "aria-disabled"() {
                    return properties.disabled ? "true" : undefined;
                },
                get tabindex() {
                    return isVirtual ? undefined : isActive() ? 0 : -1;
                },
                get hidden() {
                    return !control.isShown(option.key);
                },
                get "data-highlighted"() {
                    return isActive() ? "" : undefined;
                },
                get "data-disabled"() {
                    return properties.disabled ? "" : undefined;
                },
                get "data-value"() {
                    return option.value();
                },
                ref: (item) => (element = item),
                onPointerDown: (event) => {
                    // leave the focus in the search field of a virtual list box
                    if (isVirtual) {
                        event.preventDefault();
                    }
                },
                onPointerMove: () => {
                    // focus an available option under the pointer behind a search field
                    if (isVirtual && !option.isDisabled()) {
                        control.focus.focus(option.key);
                    }
                },
                onFocus: () => control.focus.focusIn(option.key),
                onClick: () => control.choose(option.key),
                get children() {
                    return (
                        <>
                            {properties.children}
                            <Show when={isChosen() && !isFilled}>
                                <Icon icon={check} xstyle={styles.check} />
                            </Show>
                        </>
                    );
                },
            },
        );
    },
});

/** The properties of an option of a list box, the native element's attributes included. */
export type ListBoxItemProperties = ComponentProperties<typeof ListBoxItem>;

/** Render a message while no option shows, such as when the search matches none. */
export function ListBoxEmpty(
    properties: ListBoxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useListBox();

    return (
        <Show when={control.collection.size() === 0}>
            {renderPart("div", "list-box-empty", properties, styles.note, NOTE)}
        </Show>
    );
}

/** Render the state of options the owner loads, announced as it changes. */
export function ListBoxLoading(
    properties: ListBoxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return renderPart("div", "list-box-loading", properties, styles.note, {
        ...NOTE,
        "aria-live": "polite",
    });
}

/** Render a line between sections of options, hidden while a search filters them. */
export function ListBoxSeparator(
    properties: ListBoxElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const control = useListBox();

    return (
        <Show when={!control.isSearching()}>
            {renderPart("div", "list-box-separator", properties, styles.separator, {
                role: "presentation",
            })}
        </Show>
    );
}

/** Read the list box options a list box's properties have, without its element's attributes. */
function ownOptions(
    properties: RenderProperties<ListBoxAttributes, typeof LIST_BOX_PROPERTIES>,
): ListBoxOptions {
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

/** Group the options a search shows by section. */
function sectioned(
    options: readonly ListBoxOption[],
    autocomplete: AutocompleteControl | undefined,
): CollectionSection<ListBoxOption>[] {
    const sections = new Map<string, ListBoxOption[]>();
    for (const option of options) {
        // show a forced option, else one the search matches
        if (
            autocomplete !== undefined &&
            !option.isForced() &&
            !autocomplete.matches(option.value(), [option.text(), ...option.keywords()])
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
