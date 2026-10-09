import type { IconName } from "@destack/icon";
import { LazyIcon } from "@destack/icon/lazy";
import { type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, size, space } from "@destack/theme/tokens.stylex";
import { type JSX, omit, Show, useContext, useLocale, createContext } from "@destack/view";
import {
    AutocompleteContext,
    AutocompleteControl,
    AutocompleteInput,
    type AutocompleteInputProperties,
} from "../autocomplete/index.ts";
import { type CollectionSection, Load } from "../collection/index.ts";
import {
    GridList,
    GridListCell,
    type GridListCellProperties,
    GridListControl,
    type GridListElementProperties,
    GridListEmpty,
    GridListLoading,
    GridListProvider,
    GridListViewport,
} from "../grid-list/index.ts";

/** The icons in each row of a picker's grid by default. */
const COLUMNS = 9;

/** The categories icons are grouped under, by the first one each belongs to, with their names. */
const CATEGORIES = [
    { key: "arrows", label: t`Arrows` },
    { key: "brands", label: t`Brands` },
    { key: "commerce", label: t`Commerce` },
    { key: "communications", label: t`Communications` },
    { key: "design", label: t`Design` },
    { key: "editor", label: t`Editor` },
    { key: "finances", label: t`Finances` },
    { key: "games", label: t`Games` },
    { key: "health & wellness", label: t`Health and wellness` },
    { key: "maps & travel", label: t`Maps and travel` },
    { key: "media", label: t`Media` },
    { key: "nature", label: t`Nature` },
    { key: "objects", label: t`Objects` },
    { key: "office", label: t`Office` },
    { key: "people", label: t`People` },
    { key: "system", label: t`System` },
    { key: "technology & development", label: t`Technology and development` },
    { key: "weather", label: t`Weather` },
] as const;

/** An icon of the icon set with its categories and the words that find it. */
interface IconData {
    /** The icon's name. */
    readonly name: IconName;
    /** The categories it belongs to, the first one grouping it. */
    readonly categories: readonly string[];
    /** The words that find it. */
    readonly tags: readonly string[];
}

/** The styles of an icon picker's own parts. */
const styles = style.create({
    picker: {
        display: "flex",
        flexDirection: "column",
        rowGap: space[2],
    },
    footer: {
        display: "flex",
        alignItems: "center",
        columnGap: space[2],
        minHeight: size[2],
    },
    icon: {
        color: color.foreground,
    },
    active: {
        display: "flex",
        flexGrow: 1,
        alignItems: "center",
        columnGap: space[2],
        minWidth: 0,
        color: color.foreground,
    },
    placeholder: {
        color: color.mutedForeground,
    },
});

/** The grid an icon picker's parts share. */
interface IconPickerControl {
    /** The grid of the icons the search finds. */
    readonly grid: GridListControl<IconName>;
    /** Render a cell of the grid. */
    readonly cell: (properties: GridListCellProperties<IconName>) => JSX.Element;
    /** Render the active icon where the footer shows it. */
    readonly activeCell: (icon: IconName | undefined) => JSX.Element;
}

/** The nearest icon picker's grid, null outside one. */
const IconPickerContext = createContext<IconPickerControl | null>(null);

/** The properties of an icon picker: its search, layout and cells, the native element's attributes included. */
export interface IconPickerProperties extends GridListElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onChange">
> {
    /** Report the icon a person picked with a click or Enter. */
    readonly onIconSelect: (icon: IconName) => void;
    /** The search, which makes it controlled. */
    readonly search?: string;
    /** The search at first while uncontrolled, empty by default. */
    readonly defaultSearch?: string;
    /** Handle each change of the search. */
    readonly onSearchChange?: (search: string) => void;
    /** The icons in each row of the grid, nine by default. */
    readonly columns?: number;
    /** Render an icon of the grid, the picker's own by default. */
    readonly cell?: (properties: GridListCellProperties<IconName>) => JSX.Element;
    /** Render the active icon where the footer shows it, undefined while none is, the icon and its name by default. */
    readonly activeCell?: (icon: IconName | undefined) => JSX.Element;
}

/** Render a picker of the icon set's icons around its parts: a search driving a grid of the icons it finds by category, loaded once it renders. */
export function IconPicker(properties: IconPickerProperties): JSX.Element {
    // load the icons once and lay out the ones the search finds by category
    const locale = useLocale();
    const rest = omit(
        properties,
        "onIconSelect",
        "search",
        "defaultSearch",
        "onSearchChange",
        "columns",
        "cell",
        "activeCell",
        "xstyle",
        "style",
    );
    const autocomplete = new AutocompleteControl(properties);
    const icons = new Load(loadIcons());
    const grid = new GridListControl<IconName>({
        load: icons,
        sections: () =>
            layOutIcons(icons.value(), autocomplete.search().trim().toLowerCase(), locale),
        key: (icon) => icon,
        text: (icon) => icon,
        columns: () => properties.columns ?? COLUMNS,
        autocomplete,
        onAction: (icon) => properties.onIconSelect(icon),
    });
    const control: IconPickerControl = {
        grid,
        cell: (cell) => (properties.cell ?? IconPickerIcon)(cell),
        activeCell: (icon) =>
            properties.activeCell === undefined ? (
                <ActiveIcon icon={icon} />
            ) : (
                properties.activeCell(icon)
            ),
    };

    return (
        <IconPickerContext value={control}>
            <AutocompleteContext value={autocomplete}>
                <GridListProvider control={grid}>
                    <div
                        data-slot="icon-picker"
                        {...rest}
                        {...style.attributes([styles.picker, properties.xstyle], properties.style)}
                    />
                </GridListProvider>
            </AutocompleteContext>
        </IconPickerContext>
    );
}

/** Render the search of the nearest icon picker, whose arrow keys move the active icon and Enter picks it. */
export function IconPickerSearch(properties: AutocompleteInputProperties): JSX.Element {
    const locale = useLocale();

    return (
        <AutocompleteInput
            aria-haspopup="grid"
            data-slot="icon-picker-search"
            placeholder={locale.render(t`Search icons`)}
            aria-label={locale.render(t`Search icons`)}
            {...properties}
        />
    );
}

/** Render the icons of the nearest icon picker by category, with a note while they load or while the search finds none. */
export function IconPickerContent(
    properties: Omit<GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>, "children">,
): JSX.Element {
    const picker = useIconPicker();
    const locale = useLocale();

    return (
        <GridListViewport data-slot="icon-picker-content" {...properties}>
            <GridListLoading data-slot="icon-picker-loading">
                {locale.render(t`Loading icons…`)}
            </GridListLoading>
            <GridListEmpty data-slot="icon-picker-empty">
                {locale.render(t`No icon found`)}
            </GridListEmpty>
            <GridList
                control={picker.grid}
                aria-label={locale.render(t`Icons`)}
                data-slot="icon-picker-list"
                label={(icon) => icon}
                components={{ Cell: picker.cell }}
            />
        </GridListViewport>
    );
}

/** Render the row below the nearest icon picker's content, showing the active icon and its name before its own content. */
export function IconPickerFooter(
    properties: GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const picker = useIconPicker();
    const rest = omit(properties, "xstyle", "style", "children");

    return (
        <div
            data-slot="icon-picker-footer"
            {...rest}
            {...style.attributes([styles.footer, properties.xstyle], properties.style)}
        >
            {picker.activeCell(picker.grid.active())}
            {properties.children}
        </div>
    );
}

/** Render one icon of an icon picker's grid, marked while active. */
export function IconPickerIcon(properties: GridListCellProperties<IconName>): JSX.Element {
    const rest = omit(properties, "item", "xstyle", "children");

    return (
        <GridListCell
            data-slot="icon-picker-icon"
            {...rest}
            xstyle={[styles.icon, properties.xstyle]}
        >
            {properties.children ?? <LazyIcon name={properties.item} />}
        </GridListCell>
    );
}

/** Render the active icon and its name, or a prompt to pick one while none is active. */
function ActiveIcon(properties: {
    /** The active icon, undefined while none is. */
    readonly icon: IconName | undefined;
}): JSX.Element {
    const locale = useLocale();

    return (
        <Show
            when={properties.icon}
            fallback={
                <span {...style.attrs(styles.placeholder)}>{locale.render(t`Pick an icon…`)}</span>
            }
        >
            {(icon) => (
                <span data-slot="icon-picker-active-icon" {...style.attrs(styles.active)}>
                    <LazyIcon name={icon()} />
                    <span>{icon()}</span>
                </span>
            )}
        </Show>
    );
}

/** Lay out the icons whose name or words hold a search by the first category each belongs to. */
function layOutIcons(
    all: readonly IconData[],
    search: string,
    locale: Localization,
): CollectionSection<IconName>[] {
    // find the icons whose name or words hold the search
    const found = all.filter(
        (icon) => search === "" || [icon.name, ...icon.tags].some((word) => word.includes(search)),
    );

    // group them by their first category
    return CATEGORIES.map((category) => ({
        key: category.key,
        label: locale.render(category.label),
        items: found.filter((icon) => icon.categories[0] === category.key).map((icon) => icon.name),
    }));
}

/** Read the nearest icon picker, refusing a part outside one. */
function useIconPicker(): IconPickerControl {
    const picker = useContext(IconPickerContext);
    if (picker === null) {
        throw new TypeError("an icon picker part needs an icon picker around it");
    }

    return picker;
}

/** Load the icons with their categories and the words that find them, refusing an icon of an unlisted category. */
async function loadIcons(): Promise<readonly IconData[]> {
    // load the icon set
    const { ICONS } = await import("@destack/icon/icons");

    // require each icon's first category among the listed ones, which group it
    const listed = new Set<string>(CATEGORIES.map((category) => category.key));
    const unlisted = ICONS.find((icon) => !listed.has(icon.categories[0] ?? ""));
    if (unlisted !== undefined) {
        throw new TypeError(`icon ${unlisted.name} is in no listed category`);
    }

    return ICONS;
}
