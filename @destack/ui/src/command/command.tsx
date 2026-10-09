import { Icon } from "@destack/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit, Show, useLocale } from "@destack/view";
import {
    AutocompleteContext,
    AutocompleteControl,
    type AutocompleteProperties,
    useAutocomplete,
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
import { renderPart } from "../part/index.ts";

/** The distance of a command dialog from the viewport's top, so it holds still as its results change. */
const DIALOG_OFFSET = "12vh";

/** The widest a command dialog grows, room for a result's title and excerpt. */
const DIALOG_WIDTH = "40rem";

/** The styles of a command and its elements. */
const styles = style.create({
    dialog: {
        width: `min(100% - 2 * ${space[4]}, ${DIALOG_WIDTH})`,
        marginBlockStart: DIALOG_OFFSET,
        marginBlockEnd: "auto",
        padding: 0,
        gap: 0,
        overflow: "hidden",
    },
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
        height: {
            default: size[3],
            [style.when.ancestor('[data-slot="command-dialog"]')]: size[4],
        },
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
    list: {
        maxHeight: {
            default: `calc(8 * ${size[3]})`,
            [style.when.ancestor('[data-slot="command-dialog"]')]:
                `min(28rem, 100dvh - 2 * ${DIALOG_OFFSET})`,
        },
        overflowX: "hidden",
        overflowY: "auto",
        padding: space[1],
    },
    item: {
        paddingBlock: {
            default: null,
            [style.when.ancestor('[data-slot="command-dialog"]')]: space[3],
        },
    },
    shortcut: {
        marginInlineStart: "auto",
        color: color.mutedForeground,
        letterSpacing: "0.1em",
    },
});

/** The properties of a command: its search, filtering and highlight, the native element's attributes included. */
export interface CommandProperties
    extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class">, AutocompleteProperties {
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
export type CommandItemProperties = ListBoxItemProperties;

/** The properties of a command dialog: its open state and its command's search, filtering and highlight. */
export interface CommandDialogProperties
    extends Omit<DialogProperties, "children">, Omit<CommandProperties, "title"> {
    /** The title assistive technology announces, a generic one by default. */
    readonly title?: string;
    /** The description assistive technology announces, a generic one by default. */
    readonly description?: string;
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

/** Read the search of the nearest command, refusing elements outside one. */
export function useCommand(): AutocompleteControl {
    return useAutocomplete();
}

/** Render a searchable list of commands, an autocomplete over a list box whose search filters the options as the person types. */
export function Command(properties: CommandProperties): JSX.Element {
    // drive one list box of options from the search
    const autocomplete = new AutocompleteControl(properties);
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
        "search",
        "defaultSearch",
        "onSearchChange",
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
        <AutocompleteContext value={autocomplete}>
            <ListBoxContext value={list}>
                <div
                    data-slot="command"
                    {...rest}
                    {...style.attributes(
                        [text.footnote, styles.command, properties.xstyle],
                        properties.style,
                    )}
                />
            </ListBoxContext>
        </AutocompleteContext>
    );
}

/** Render the search of a command, which the arrow keys, Home and End move the highlight from and Enter chooses. */
export function CommandInput(
    properties: CommandElementProperties<
        Omit<JSX.InputHTMLAttributes<HTMLInputElement>, "value" | "onInput" | "onKeyDown">
    >,
): JSX.Element {
    // take the search field's attributes from the command's search
    const autocomplete = useAutocomplete();
    const locale = useLocale();

    return (
        <div data-slot="command-input-wrapper" {...style.attrs(styles.search)}>
            <Icon name="magnifying-glass" />
            {renderPart(
                "input",
                "command-input",
                properties,
                [text.callout, styles.input],
                merge(autocomplete.input(() => locale.direction)),
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
    return (
        <ListBox
            control={useListBox()}
            data-slot="command-list"
            {...properties}
            xstyle={[styles.list, properties.xstyle]}
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
    const rest = omit(properties, "heading", "children");

    return (
        <ListBoxSection data-slot="command-group" {...rest}>
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
    return <ListBoxSectionHeading data-slot="command-group-heading" {...properties} />;
}

/** Render an option that the search filters and a click or Enter chooses. */
export function CommandItem(properties: CommandItemProperties): JSX.Element {
    return (
        <ListBoxItem
            data-slot="command-item"
            {...properties}
            xstyle={[styles.item, properties.xstyle]}
        />
    );
}

/** Render a line between groups of options while the search is empty. */
export function CommandSeparator(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    return <ListBoxSeparator data-slot="command-separator" {...properties} />;
}

/** Render the keyboard shortcut of an option at its end. */
export function CommandShortcut(
    properties: CommandElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    return renderPart("span", "command-shortcut", properties, [text.caption, styles.shortcut]);
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
        "title",
        "description",
    );

    return (
        <Dialog {...properties}>
            <DialogContent
                data-slot="command-dialog"
                showCloseButton={false}
                xstyle={[style.defaultMarker(), styles.dialog]}
            >
                <DialogTitle xstyle={visuallyHiddenStyle()}>
                    {properties.title ?? locale.render(t`Command palette`)}
                </DialogTitle>
                <DialogDescription xstyle={visuallyHiddenStyle()}>
                    {properties.description ?? locale.render(t`Search for a command to run`)}
                </DialogDescription>
                <Command {...command} />
            </DialogContent>
        </Dialog>
    );
}
