import { Icon } from "@destack/ui/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { media } from "@destack/style/media.stylex";
import { color, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import { transition } from "@destack/theme/motion";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createEffect,
    createSignal,
    For,
    type JSX,
    merge,
    omit,
    type Setter,
    Show,
    useContext,
    untrack,
    useLocale,
} from "@destack/view";
import {
    AutocompleteContext,
    AutocompleteControl,
    type AutocompleteProperties,
    type AutocompleteValueProperties,
} from "../autocomplete/index.ts";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogTitle,
    type DialogProperties,
} from "../dialog/index.ts";
import {
    ListBox,
    ListBoxContext,
    ListBoxControl,
    ListBoxEmpty,
    ListBoxItem,
    type ListBoxItemProperties,
    ListBoxLoading,
    ListBoxSection,
    ListBoxSectionHeading,
    ListBoxSeparator,
    useListBox,
} from "../list-box/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";
import { KbdShortcut } from "../kbd/index.ts";
import { OpenContext } from "../layer/index.ts";
import { mergeProperties, renderPart } from "../part/index.ts";

/** The distance of a command dialog from the viewport's top. */
const DIALOG_OFFSET = "12vh";

/** The size of a command dialog's search: the base text style's, a step above the options'. */
const DIALOG_INPUT_SIZE = "var(--destack-text-base-font-size)";

/** The widest a command dialog grows, 720px. */
const DIALOG_WIDTH = "45rem";

/** The width of the column a command dialog's icons share, 20px. */
const DIALOG_ICON = "1.25rem";

/** The widest a small command dialog grows, 480px. */
const DIALOG_SMALL_WIDTH = "30rem";

/** The widest a command dialog grows beside a detail, 960px. */
const DIALOG_DETAIL_WIDTH = "60rem";

/** The viewport height a command dialog's search, footer and offsets take. */
const DIALOG_CHROME = `(2 * ${DIALOG_OFFSET} + 2 * ${size[4]})`;

/** The tallest a command dialog's options grow, and their fixed height in a body. */
const DIALOG_LIST_HEIGHT = `min(25rem, 100dvh - ${DIALOG_CHROME})`;

/** The inset of a command dialog's icons, headings and footer: the list's and an option's. */
const DIALOG_INSET = `calc(${space[2]} + ${space[3]})`;

/** The styles of a command and its elements. */
const styles = style.create({
    dialog: {
        // open and close at once, as a palette opened many times a day should
        transitionDuration: "0s",
        "::backdrop": { transitionDuration: "0s" },
        width: {
            default: `min(100% - 2 * ${space[4]}, ${DIALOG_WIDTH})`,
            ':has([data-slot="command-detail"])': `min(100% - 2 * ${space[4]}, ${DIALOG_DETAIL_WIDTH})`,
        },
        marginBlockStart: DIALOG_OFFSET,
        marginBlockEnd: "auto",
        padding: 0,
        gap: 0,
        overflow: "hidden",
    },
    small: {
        width: `min(100% - 2 * ${space[4]}, ${DIALOG_SMALL_WIDTH})`,
    },
    command: {
        display: "flex",
        flexDirection: "column",
        width: "100%",
        overflow: "hidden",
        borderRadius: radius[3],
        backgroundColor: color.popover,
        color: color.foreground,
    },
    search: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        flexShrink: 0,
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
        outlineStyle: "none",
        color: color.foreground,
        "::placeholder": { color: color.mutedForeground },
    },
    icon: {
        display: "inline-flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        width: "1em",
    },
    list: {
        flexGrow: 1,
        minWidth: 0,
        maxHeight: `calc(8 * ${size[3]})`,
        overflowX: "hidden",
        overflowY: "auto",
        padding: space[1],
    },
    separator: {
        marginBlock: space[1],
    },
    shortcut: {
        marginInlineStart: "auto",
        color: color.mutedForeground,
    },
    description: {
        // shrink before the option's name wraps
        flexShrink: 9999,
        overflow: "hidden",
        color: color.mutedForeground,
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
    },
    meta: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        gap: space[2],
        marginInlineStart: "auto",
        color: color.mutedForeground,
    },
    footer: {
        display: "flex",
        alignItems: "center",
        justifyContent: "flex-end",
        gap: space[4],
        paddingBlock: space[2],
        paddingInline: space[3],
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        borderTopColor: color.border,
        color: color.mutedForeground,
    },
    hint: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    body: {
        display: "flex",
        flex: 1,
        minHeight: 0,
    },
    fields: {
        display: "grid",
        gridTemplateColumns: "auto 1fr",
        columnGap: space[4],
        rowGap: space[2],
        marginBlock: space[4],
    },
    label: {
        color: color.mutedForeground,
    },
    value: {
        margin: 0,
    },
    location: {
        marginInlineEnd: "auto",
    },
    detail: {
        display: { default: null, [media.maxMd]: "none" },
        flex: "0 0 40%",
        minWidth: 0,
        overflowY: "auto",
        padding: space[4],
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: stroke.border,
        borderInlineStartColor: color.border,
    },
    page: {
        flexShrink: 0,
        paddingInline: space[2],
        paddingBlock: space[1],
        borderRadius: radius[2],
        backgroundColor: color.muted,
        color: color.foreground,
    },
});

/** The styles a command and its elements add inside a dialog. */
const dialogStyles = style.create({
    command: {
        borderRadius: radius[5],
    },
    search: {
        gap: space[2],
        height: size[4],
        paddingInline: DIALOG_INSET,
    },
    input: {
        fontSize: DIALOG_INPUT_SIZE,
    },
    list: {
        maxHeight: DIALOG_LIST_HEIGHT,
        padding: space[2],
    },
    icon: {
        width: DIALOG_ICON,
    },
    group: {
        marginBlockStart: { default: space[2], ":first-child": 0 },
    },
    body: {
        flex: "1 1 auto",
        height: DIALOG_LIST_HEIGHT,
    },
    heading: {
        paddingBlockStart: space[1],
        paddingInline: `calc(${space[3]} + (${DIALOG_ICON} - 1rem) / 2)`,
    },
    item: {
        gap: space[2],
        minHeight: `calc(${size[2]} + ${space[1]})`,
        paddingInline: space[3],
        borderRadius: radius[3],
        whiteSpace: "nowrap",
    },
    separator: {
        marginBlock: `${space[2]} 0`,
        marginInline: `calc(-1 * ${space[2]})`,
    },
    footer: {
        paddingInline: DIALOG_INSET,
    },
    detail: {
        paddingInline: DIALOG_INSET,
    },
});

/** Whether a command sits in a command dialog. */
const CommandDialogContext = createContext(false);

/** A page a command shows in place of its root, such as the people an assignment picks from. */
export interface CommandPageEntry {
    /** The page's name, which its `CommandPage` shows under. */
    readonly name: string;
    /** The title its chip shows in the search. */
    readonly title: string;
}

/** The search of a command and the pages it has opened over its root, after cmdk's pages. */
export class CommandControl {
    /** The search that filters the options. */
    readonly search: AutocompleteControl;
    /** The pages opened over the root, the shown one last. */
    readonly pages: Accessor<readonly CommandPageEntry[]>;
    /** Replace the pages. */
    readonly #setPages: Setter<readonly CommandPageEntry[]>;

    /** Create the pages of a command over its search. */
    constructor(search: AutocompleteControl) {
        // start on the root
        const [pages, setPages] = createSignal<readonly CommandPageEntry[]>([], {
            ownedWrite: true,
        });
        this.search = search;
        this.pages = pages;
        this.#setPages = setPages;
    }

    /** Read the name of the page shown, none on the root. */
    page(): string | undefined {
        return this.pages().at(-1)?.name;
    }

    /** Open a page over the shown one, with an empty search. */
    push(page: CommandPageEntry): void {
        this.#setPages((pages) => [...pages, page]);
        this.search.type("");
    }

    /** Go back to the page under the shown one, with an empty search. */
    pop(): void {
        this.#setPages((pages) => pages.slice(0, -1));
        this.search.type("");
    }

    /** Start over on the root, with an empty search. */
    restart(): void {
        this.#setPages([]);
        this.search.type("");
    }

    /** Go back on Escape or on Backspace in an empty search, reporting whether it went back. */
    back(event: KeyboardEvent): boolean {
        const isBack =
            this.pages().length > 0 &&
            (event.key === "Escape" || (event.key === "Backspace" && this.search.value() === ""));
        if (isBack) {
            event.preventDefault();
            this.pop();
        }

        return isBack;
    }
}

/** The command around an element, null outside one. */
const CommandContext = createContext<CommandControl | null>(null);

/** The properties of a command: its search, filtering and highlight. */
export interface CommandProperties
    extends
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "class">,
        Omit<AutocompleteProperties, "filter" | keyof AutocompleteValueProperties> {
    /** Whether the command filters its options by the search, or its owner does, as cmdk's. */
    readonly shouldFilter?: boolean;
    /** Report whether an option's value or keywords match the search, as cmdk's. */
    readonly filter?: (value: string, search: string, keywords: readonly string[]) => boolean;
    /** The highlighted option's value, which makes the highlight controlled. */
    readonly value?: string;
    /** The highlighted option's value at first while uncontrolled, the first option by default. */
    readonly defaultValue?: string;
    /** Handle another option being highlighted. */
    readonly onValueChange?: (value: string) => void;
    /** Whether the arrow keys wrap from the last option to the first and back, false by default. */
    readonly loop?: boolean;
    /** The StyleX styles applied after the command's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an element of a command, the native element's attributes included. */
export type CommandElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a command option. */
export interface CommandItemProperties extends ListBoxItemProperties {
    /** The icon before its title, whose room a dialog's options keep without one. */
    readonly icon?: JSX.Element;
}

/** The properties of a command dialog. */
export interface CommandDialogProperties
    extends Omit<DialogProperties, "children">, Omit<CommandProperties, "title"> {
    /** The title assistive technology announces, a generic one by default. */
    readonly title?: string;
    /** The description assistive technology announces, a generic one by default. */
    readonly description?: string;
    /** How wide it grows: a palette's width by default, small for a plain list. */
    readonly size?: "default" | "sm";
}

/** The properties of a command group, the native element's attributes included. */
export interface CommandGroupProperties extends CommandElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The heading that names the group, rendered as its heading part. */
    readonly heading?: JSX.Element;
    /** Whether the group shows whatever the search. */
    readonly forceMount?: boolean;
}

/** Read the nearest command, refusing elements outside one. */
export function useCommand(): CommandControl {
    const command = useContext(CommandContext);
    if (command === null) {
        throw new TypeError("command parts need a command around them");
    }

    return command;
}

/** Render a searchable list of commands. */
export function Command(properties: CommandProperties): JSX.Element {
    // steer one list box of options from the search, with the pages over it
    const autocomplete = new AutocompleteControl({
        get filter() {
            return properties.shouldFilter === false ? null : properties.filter;
        },
    });
    const command = new CommandControl(autocomplete);
    const isOpen = useContext(OpenContext);
    createEffect(isOpen, (isShown, wasShown) => {
        // start over each time the overlay around opens again
        if (isShown && wasShown === false) {
            untrack(() => command.restart());
        }
    });
    const isDialog = useContext(CommandDialogContext);
    const list = new ListBoxControl({
        autocomplete,
        get activeValue() {
            return properties.value;
        },
        get defaultActiveValue() {
            return properties.defaultValue;
        },
        onActiveChange: (value) => properties.onValueChange?.(value),
        get isLooping() {
            return properties.loop === true;
        },
    });
    const rest = omit(
        properties,
        "shouldFilter",
        "filter",
        "value",
        "defaultValue",
        "onValueChange",
        "loop",
        "xstyle",
        "style",
    );

    return (
        <CommandContext value={command}>
            <AutocompleteContext value={autocomplete}>
                <ListBoxContext value={list}>
                    <div
                        data-slot="command"
                        {...rest}
                        {...style.attributes(
                            [
                                text.sm,
                                styles.command,
                                isDialog && dialogStyles.command,
                                properties.xstyle,
                            ],
                            properties.style,
                        )}
                    />
                </ListBoxContext>
            </AutocompleteContext>
        </CommandContext>
    );
}

/** Render the search of a command. */
export function CommandInput(
    properties: CommandElementProperties<
        Omit<
            JSX.InputHTMLAttributes<HTMLInputElement>,
            "value" | "defaultValue" | "onInput" | "onKeyDown"
        >
    > &
        AutocompleteValueProperties & {
            /** Handle a key before the command goes back or steers. */
            readonly onKeyDown?: (event: KeyboardEvent) => void;
        },
): JSX.Element {
    // take the search field's attributes from the command's search, going back before steering
    const command = useCommand();
    const isDialog = useContext(CommandDialogContext);
    const locale = useLocale();
    command.search.follow(properties);
    const search = command.search.input(() => locale.direction);
    const back = {
        onKeyDown: (event: KeyboardEvent) => {
            properties.onKeyDown?.(event);
            if (!event.defaultPrevented && !command.back(event)) {
                search.onKeyDown(event);
            }
        },
    };

    return (
        <div
            data-slot="command-input-wrapper"
            {...style.attrs(styles.search, isDialog && dialogStyles.search)}
        >
            <span {...style.attrs(styles.icon, isDialog && dialogStyles.icon)}>
                <Icon name="magnifying-glass" />
            </span>
            <For each={command.pages()}>
                {(page) => (
                    <span data-slot="command-page-chip" {...style.attrs(text.xs, styles.page)}>
                        {page.title}
                    </span>
                )}
            </For>
            {renderPart(
                "input",
                "command-input",
                omit(properties, "onKeyDown", "value", "defaultValue", "onValueChange"),
                [text.sm, state.typed, styles.input, isDialog && dialogStyles.input],
                merge(
                    {
                        "aria-label":
                            untrack(() => properties.placeholder) ?? locale.render(t`Search`),
                    },
                    mergeProperties(omit(search, "onKeyDown"), back),
                ),
            )}
        </div>
    );
}

/** Render the list box of a command's options. */
export function CommandList(
    properties: CommandElementProperties<
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown" | "onFocusOut">
    >,
): JSX.Element {
    const isDialog = useContext(CommandDialogContext);

    return (
        <ListBox
            control={useListBox()}
            data-slot="command-list"
            {...properties}
            xstyle={[styles.list, isDialog && dialogStyles.list, properties.xstyle]}
        />
    );
}

/** Render a message while the search matches no option. */
export function CommandEmpty(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return <ListBoxEmpty data-slot="command-empty" {...properties} />;
}

/** Render the state of options the owner loads, announced as it changes. */
export function CommandLoading(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return <ListBoxLoading data-slot="command-loading" {...properties} />;
}

/** Render a group of options under a heading, hidden while the search matches none of them. */
export function CommandGroup(properties: CommandGroupProperties): JSX.Element {
    const isDialog = useContext(CommandDialogContext);
    const rest = omit(properties, "heading", "xstyle", "children");

    return (
        <ListBoxSection
            data-slot="command-group"
            {...rest}
            xstyle={[isDialog && dialogStyles.group, properties.xstyle]}
        >
            <Show when={"heading" in properties}>
                <CommandGroupHeading>{properties.heading}</CommandGroupHeading>
            </Show>
            {properties.children}
        </ListBoxSection>
    );
}

/** Render the heading that labels the nearest command group. */
export function CommandGroupHeading(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const isDialog = useContext(CommandDialogContext);

    return (
        <ListBoxSectionHeading
            data-slot="command-group-heading"
            {...properties}
            xstyle={[isDialog && dialogStyles.heading, properties.xstyle]}
        />
    );
}

/** Render an option that the search filters and a click or Enter chooses. */
export function CommandItem(properties: CommandItemProperties): JSX.Element {
    const isDialog = useContext(CommandDialogContext);
    const rest = omit(properties, "icon", "xstyle", "children");

    return (
        <ListBoxItem
            data-slot="command-item"
            {...rest}
            xstyle={[isDialog && dialogStyles.item, properties.xstyle]}
        >
            {/* The icon, below the search's in a dialog, where every title starts on one edge */}
            <Show when={isDialog || properties.icon !== undefined}>
                <span
                    data-slot="command-item-icon"
                    aria-hidden="true"
                    {...style.attrs(styles.icon, isDialog && dialogStyles.icon)}
                >
                    {properties.icon}
                </span>
            </Show>
            {properties.children}
        </ListBoxItem>
    );
}

/** Render a line between groups of options while the search is empty. */
export function CommandSeparator(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const isDialog = useContext(CommandDialogContext);

    return (
        <ListBoxSeparator
            data-slot="command-separator"
            {...properties}
            xstyle={[styles.separator, isDialog && dialogStyles.separator, properties.xstyle]}
        />
    );
}

/** Render the options of one page while it is shown, the root's without a name. */
export function CommandPage(properties: {
    /** The page's name, which `CommandControl.push` opens, the root when absent. */
    readonly name?: string;
    /** The page's options and groups. */
    readonly children?: JSX.Element;
}): JSX.Element {
    const command = useCommand();

    return <Show when={command.page() === properties.name}>{properties.children}</Show>;
}

/** The properties of an option's keyboard shortcut. */
export type CommandShortcutProperties = CommandElementProperties<
    JSX.HTMLAttributes<HTMLSpanElement>
> & {
    /** The key combination in the keybinding syntax, such as `mod+k`, printed for the platform. */
    readonly keys?: string;
};

/** Render the keyboard shortcut of an option at its end, from its keys or its text. */
export function CommandShortcut(properties: CommandShortcutProperties): JSX.Element {
    // draw the keys as the footer's hints do, else the caller's own text
    const rest = omit(properties, "keys", "xstyle", "style");

    return (
        <span
            data-slot="command-shortcut"
            {...rest}
            {...style.attributes([text.xs, styles.shortcut, properties.xstyle], properties.style)}
        >
            <Show when={properties.keys} fallback={properties.children}>
                {(keys) => <KbdShortcut keys={keys()} />}
            </Show>
        </span>
    );
}

/** Render a command in a modal dialog near the viewport's top, titled for assistive technology. */
export function CommandDialog(properties: CommandDialogProperties): JSX.Element {
    const locale = useLocale();
    const command = omit(
        properties,
        "open",
        "defaultOpen",
        "onOpenChange",
        "modal",
        "size",
        "title",
        "description",
    );

    return (
        <Dialog {...properties}>
            <DialogContent
                data-slot="command-dialog"
                showCloseButton={false}
                xstyle={[
                    styles.dialog,
                    properties.size === "sm" && styles.small,
                    transition.instant,
                ]}
            >
                <DialogTitle xstyle={visuallyHiddenStyle()}>
                    {properties.title ?? locale.render(t`Command palette`)}
                </DialogTitle>
                <DialogDescription xstyle={visuallyHiddenStyle()}>
                    {properties.description ?? locale.render(t`Search for a command to run`)}
                </DialogDescription>
                <CommandDialogContext value={true}>
                    <Command {...command} />
                </CommandDialogContext>
            </DialogContent>
        </Dialog>
    );
}

/** Render a line after an option's title, such as where it lives. */
export function CommandItemDescription(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    return renderPart("span", "command-item-description", properties, [
        text.xs,
        styles.description,
    ]);
}

/** Render the details at an option's end, such as its space and its shortcut. */
export function CommandItemMeta(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    return renderPart("span", "command-item-meta", properties, [text.xs, styles.meta]);
}

/** Render the bar under a command's options that lists the keys of what they do. */
export function CommandFooter(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const isDialog = useContext(CommandDialogContext);

    return renderPart("div", "command-footer", properties, [
        text.xs,
        styles.footer,
        isDialog && dialogStyles.footer,
    ]);
}

/** The properties of a key hint: what the keys do and the keys. */
export type CommandHintProperties = CommandElementProperties<
    JSX.HTMLAttributes<HTMLSpanElement>
> & {
    /** The key combination in the keybinding syntax, such as `mod+enter`. */
    readonly keys: string;
};

/** Render what a key combination does to the highlighted option, such as Open with Enter. */
export function CommandHint(properties: CommandHintProperties): JSX.Element {
    const rest = omit(properties, "keys", "children", "xstyle", "style");

    return (
        <span
            data-slot="command-hint"
            {...rest}
            {...style.attributes([styles.hint, properties.xstyle], properties.style)}
        >
            {properties.children}
            <KbdShortcut keys={properties.keys} />
        </span>
    );
}

/** Render a command's options beside the detail of the highlighted one. */
export function CommandBody(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const isDialog = useContext(CommandDialogContext);

    return renderPart("div", "command-body", properties, [
        styles.body,
        isDialog && dialogStyles.body,
    ]);
}

/** The properties of the detail of a command's highlighted option. */
export interface CommandDetailProperties {
    /** Render the detail of the highlighted option's value, none while no option is highlighted. */
    readonly children: (value: string | undefined) => JSX.Element;
    /** The StyleX styles applied after the detail's styles. */
    readonly xstyle?: style.Styles;
}

/** Render the detail of the highlighted option beside the options. */
export function CommandDetail(properties: CommandDetailProperties): JSX.Element {
    const list = useListBox();
    const isDialog = useContext(CommandDialogContext);

    return (
        <div
            data-slot="command-detail"
            {...style.attrs(styles.detail, isDialog && dialogStyles.detail, properties.xstyle)}
        >
            {properties.children(list.activeValue())}
        </div>
    );
}

/** Render the fields of a detail, label beside value. */
export function CommandDetailMetadata(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDListElement>>,
): JSX.Element {
    return renderPart("dl", "command-detail-metadata", properties, [text.xs, styles.fields]);
}

/** The properties of one field of a detail: its label and its value. */
export type CommandDetailFieldProperties = CommandElementProperties<
    JSX.HTMLAttributes<HTMLElement>
> & {
    /** The field's name. */
    readonly label: string;
};

/** Render one field of a detail: its label, then its value. */
export function CommandDetailField(properties: CommandDetailFieldProperties): JSX.Element {
    const rest = omit(properties, "label", "children", "xstyle", "style");

    return (
        <>
            <dt data-slot="command-detail-label" {...style.attrs(styles.label)}>
                {properties.label}
            </dt>
            <dd
                data-slot="command-detail-value"
                {...rest}
                {...style.attributes([styles.value, properties.xstyle], properties.style)}
            >
                {properties.children}
            </dd>
        </>
    );
}

/** Render where a command stands at the start of its footer, such as the page shown. */
export function CommandFooterLocation(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    return renderPart("span", "command-footer-location", properties, styles.location);
}
