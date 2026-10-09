import { Locale, type LocaleTag, type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, size, space } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
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
import { RadioGroup, RadioGroupItem } from "../radio-group/index.ts";

/** The emoji in each row of a picker's grid by default. */
const COLUMNS = 9;

/** The skin tones emoji take, the default first, in the order the emoji data lists an emoji's variants. */
const SKIN_TONES = ["none", "light", "medium-light", "medium", "medium-dark", "dark"] as const;

/** The skin tone after each one, the last wrapping to the default. */
const NEXT_TONES = {
    none: "light",
    light: "medium-light",
    "medium-light": "medium",
    medium: "medium-dark",
    "medium-dark": "dark",
    dark: "none",
} as const;

/** The hand drawn for each skin tone. */
const TONE_HANDS = {
    none: "✋",
    light: "✋🏻",
    "medium-light": "✋🏼",
    medium: "✋🏽",
    "medium-dark": "✋🏾",
    dark: "✋🏿",
} as const;

/** The name of each skin tone. */
const TONE_NAMES = {
    none: t`Default skin tone`,
    light: t`Light skin tone`,
    "medium-light": t`Medium-light skin tone`,
    medium: t`Medium skin tone`,
    "medium-dark": t`Medium-dark skin tone`,
    dark: t`Dark skin tone`,
} as const;

/** The groups of the emoji data people pick from, in order, with their names, the skin tone components left out. */
const CATEGORIES = [
    { group: 0, label: t`Smileys and emotion` },
    { group: 1, label: t`People and body` },
    { group: 3, label: t`Animals and nature` },
    { group: 4, label: t`Food and drink` },
    { group: 5, label: t`Travel and places` },
    { group: 6, label: t`Activities` },
    { group: 7, label: t`Objects` },
    { group: 8, label: t`Symbols` },
    { group: 9, label: t`Flags` },
] as const;

/** The emoji data of each locale it covers, by language tag, each loaded on first use. */
const EMOJI_LOCALES: Readonly<
    Record<string, () => Promise<{ readonly default: readonly EmojiData[] }>>
> = {
    bn: () => import("emojibase-data/bn/compact.json", { with: { type: "json" } }),
    da: () => import("emojibase-data/da/compact.json", { with: { type: "json" } }),
    de: () => import("emojibase-data/de/compact.json", { with: { type: "json" } }),
    en: () => import("emojibase-data/en/compact.json", { with: { type: "json" } }),
    "en-GB": () => import("emojibase-data/en-gb/compact.json", { with: { type: "json" } }),
    es: () => import("emojibase-data/es/compact.json", { with: { type: "json" } }),
    "es-MX": () => import("emojibase-data/es-mx/compact.json", { with: { type: "json" } }),
    et: () => import("emojibase-data/et/compact.json", { with: { type: "json" } }),
    fi: () => import("emojibase-data/fi/compact.json", { with: { type: "json" } }),
    fr: () => import("emojibase-data/fr/compact.json", { with: { type: "json" } }),
    hi: () => import("emojibase-data/hi/compact.json", { with: { type: "json" } }),
    hu: () => import("emojibase-data/hu/compact.json", { with: { type: "json" } }),
    it: () => import("emojibase-data/it/compact.json", { with: { type: "json" } }),
    ja: () => import("emojibase-data/ja/compact.json", { with: { type: "json" } }),
    ko: () => import("emojibase-data/ko/compact.json", { with: { type: "json" } }),
    lt: () => import("emojibase-data/lt/compact.json", { with: { type: "json" } }),
    ms: () => import("emojibase-data/ms/compact.json", { with: { type: "json" } }),
    nb: () => import("emojibase-data/nb/compact.json", { with: { type: "json" } }),
    nl: () => import("emojibase-data/nl/compact.json", { with: { type: "json" } }),
    pl: () => import("emojibase-data/pl/compact.json", { with: { type: "json" } }),
    pt: () => import("emojibase-data/pt/compact.json", { with: { type: "json" } }),
    ru: () => import("emojibase-data/ru/compact.json", { with: { type: "json" } }),
    sv: () => import("emojibase-data/sv/compact.json", { with: { type: "json" } }),
    th: () => import("emojibase-data/th/compact.json", { with: { type: "json" } }),
    uk: () => import("emojibase-data/uk/compact.json", { with: { type: "json" } }),
    vi: () => import("emojibase-data/vi/compact.json", { with: { type: "json" } }),
    zh: () => import("emojibase-data/zh/compact.json", { with: { type: "json" } }),
    "zh-Hant": () => import("emojibase-data/zh-hant/compact.json", { with: { type: "json" } }),
};

/** A skin tone, none for the default yellow. */
export type SkinTone = (typeof SKIN_TONES)[number];

/** An emoji a picker offers, in the chosen skin tone. */
export interface EmojiEntry {
    /** The emoji. */
    readonly emoji: string;
    /** Its name. */
    readonly label: string;
}

/** An emoji as the compact emoji data describes it. */
interface EmojiData {
    /** The emoji. */
    readonly unicode: string;
    /** Its name. */
    readonly label: string;
    /** The words that find it. */
    readonly tags?: readonly string[] | undefined;
    /** Its group, absent for the regional indicators. */
    readonly group?: number | undefined;
    /** Its order within the data. */
    readonly order?: number | undefined;
    /** Its variants in skin tone order, from the lightest. */
    readonly skins?: readonly { readonly unicode: string }[] | undefined;
}

/** An emoji people pick: one in a group, which the data orders. */
type PickableEmoji = EmojiData & { readonly group: number; readonly order: number };

/** The skin tone and grid an emoji picker's parts share. */
interface EmojiPickerControl {
    /** The chosen skin tone. */
    readonly skinTone: Accessor<SkinTone>;
    /** Choose a skin tone. */
    readonly setSkinTone: (skinTone: SkinTone) => void;
    /** The grid of the emoji the search finds. */
    readonly grid: GridListControl<EmojiEntry>;
    /** Render a cell of the grid. */
    readonly cell: (properties: GridListCellProperties<EmojiEntry>) => JSX.Element;
    /** Render the active emoji where the footer shows it. */
    readonly activeCell: (entry: EmojiEntry | undefined) => JSX.Element;
}

/** The nearest emoji picker's skin tone, null outside one. */
const EmojiPickerContext = createContext<EmojiPickerControl | null>(null);

/** The styles of an emoji picker's own parts. */
const styles = style.create({
    emoji: {
        fontSize: `calc(${size[2]} * 0.6)`,
        userSelect: "none",
    },
    active: {
        display: "flex",
        flexGrow: 1,
        alignItems: "center",
        columnGap: space[2],
        minWidth: 0,
    },
    placeholder: {
        color: color.mutedForeground,
    },
    tones: {
        display: "flex",
        columnGap: space[1],
    },
    tone: {
        width: size[2],
        height: size[2],
        borderRadius: radius[1],
        fontSize: `calc(${size[2]} * 0.6)`,
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
            ":is([aria-checked=true])": color.accent,
        },
        boxShadow: "none",
    },
    selector: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: size[2],
        height: size[2],
        borderRadius: radius[1],
        fontSize: `calc(${size[2]} * 0.6)`,
        backgroundColor: {
            default: "transparent",
            ":hover": { default: null, [media.hover]: color.muted },
        },
        cursor: "pointer",
    },
});

/** The properties of an emoji picker: its search, layout, skin tone and cells, the native element's attributes included. */
export interface EmojiPickerProperties extends GridListElementProperties<
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "onChange">
> {
    /** Report the emoji a person picked with a click or Enter. */
    readonly onEmojiSelect: (entry: EmojiEntry) => void;
    /** The search, which makes it controlled. */
    readonly search?: string;
    /** The search at first while uncontrolled, empty by default. */
    readonly defaultSearch?: string;
    /** Handle each change of the search. */
    readonly onSearchChange?: (search: string) => void;
    /** The emoji in each row of the grid, nine by default. */
    readonly columns?: number;
    /** Render an emoji of the grid, the picker's own by default. */
    readonly cell?: (properties: GridListCellProperties<EmojiEntry>) => JSX.Element;
    /** Render the active emoji where the footer shows it, undefined while none is, its emoji and name by default. */
    readonly activeCell?: (entry: EmojiEntry | undefined) => JSX.Element;
    /** The skin tone, which makes it controlled. */
    readonly skinTone?: SkinTone;
    /** The skin tone at first while uncontrolled, the default yellow by default. */
    readonly defaultSkinTone?: SkinTone;
    /** Handle each change of the skin tone. */
    readonly onSkinToneChange?: (skinTone: SkinTone) => void;
}

/** The styles of an emoji picker's layout. */
const layout = style.create({
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
});

/** Render a picker of emoji around its parts: a search driving a grid of the emoji it finds by category, loaded in the reader's locale once it renders. */
export function EmojiPicker(properties: EmojiPickerProperties): JSX.Element {
    // follow the controlled skin tone, else the picker's own
    const locale = useLocale();
    const rest = omit(
        properties,
        "onEmojiSelect",
        "search",
        "defaultSearch",
        "onSearchChange",
        "columns",
        "cell",
        "activeCell",
        "skinTone",
        "defaultSkinTone",
        "onSkinToneChange",
        "xstyle",
        "style",
    );
    const [skinTone, setSkinTone] = createControllableSignal<SkinTone>({
        isControlled: () => properties.skinTone !== undefined,
        value: () => properties.skinTone ?? "none",
        defaultValue: properties.defaultSkinTone ?? "none",
        onChange: (next) => properties.onSkinToneChange?.(next),
    });

    // load the emoji once and lay out the ones the search finds in the chosen tone
    const autocomplete = new AutocompleteControl(properties);
    const emoji = new Load(loadEmoji(untrack(() => locale.tag)));
    const grid = new GridListControl<EmojiEntry>({
        load: emoji,
        sections: () =>
            layOutEmoji(
                emoji.value(),
                autocomplete.search().trim().toLowerCase(),
                skinTone(),
                locale,
            ),
        key: (entry) => entry.emoji,
        text: (entry) => entry.label,
        columns: () => properties.columns ?? COLUMNS,
        autocomplete,
        onAction: (entry) => properties.onEmojiSelect(entry),
    });
    const control: EmojiPickerControl = {
        skinTone,
        setSkinTone,
        grid,
        cell: (cell) => (properties.cell ?? EmojiPickerEmoji)(cell),
        activeCell: (entry) =>
            properties.activeCell === undefined ? (
                <ActiveEmoji entry={entry} />
            ) : (
                properties.activeCell(entry)
            ),
    };

    return (
        <EmojiPickerContext value={control}>
            <AutocompleteContext value={autocomplete}>
                <GridListProvider control={grid}>
                    <div
                        data-slot="emoji-picker"
                        {...rest}
                        {...style.attributes([layout.picker, properties.xstyle], properties.style)}
                    />
                </GridListProvider>
            </AutocompleteContext>
        </EmojiPickerContext>
    );
}

/** Render the search of the nearest emoji picker, whose arrow keys move the active emoji and Enter picks it. */
export function EmojiPickerSearch(properties: AutocompleteInputProperties): JSX.Element {
    const locale = useLocale();

    return (
        <AutocompleteInput
            aria-haspopup="grid"
            data-slot="emoji-picker-search"
            placeholder={locale.render(t`Search emoji`)}
            aria-label={locale.render(t`Search emoji`)}
            {...properties}
        />
    );
}

/** Render the emoji of the nearest emoji picker by category, with a note while they load or while the search finds none. */
export function EmojiPickerContent(
    properties: Omit<GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>, "children">,
): JSX.Element {
    const picker = useEmojiPicker();
    const locale = useLocale();

    return (
        <GridListViewport data-slot="emoji-picker-content" {...properties}>
            <GridListLoading data-slot="emoji-picker-loading">
                {locale.render(t`Loading emoji…`)}
            </GridListLoading>
            <GridListEmpty data-slot="emoji-picker-empty">
                {locale.render(t`No emoji found`)}
            </GridListEmpty>
            <GridList
                control={picker.grid}
                aria-label={locale.render(t`Emoji`)}
                data-slot="emoji-picker-list"
                label={(entry) => entry.label}
                components={{ Cell: picker.cell }}
            />
        </GridListViewport>
    );
}

/** Render the row below the nearest emoji picker's content, showing the active emoji and its name before its own content. */
export function EmojiPickerFooter(
    properties: GridListElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    const picker = useEmojiPicker();
    const rest = omit(properties, "xstyle", "style", "children");

    return (
        <div
            data-slot="emoji-picker-footer"
            {...rest}
            {...style.attributes([layout.footer, properties.xstyle], properties.style)}
        >
            {picker.activeCell(picker.grid.active())}
            {properties.children}
        </div>
    );
}

/** Render one emoji of an emoji picker's grid, marked while active. */
export function EmojiPickerEmoji(properties: GridListCellProperties<EmojiEntry>): JSX.Element {
    const rest = omit(properties, "item", "xstyle", "children");

    return (
        <GridListCell
            data-slot="emoji-picker-emoji"
            {...rest}
            xstyle={[styles.emoji, properties.xstyle]}
        >
            {properties.children ?? properties.item.emoji}
        </GridListCell>
    );
}

/** Render the skin tones of the nearest emoji picker as a radio group, which arrow keys move through. */
export function EmojiPickerSkinTone(
    properties: GridListElementProperties<
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "onKeyDown" | "children">
    >,
): JSX.Element {
    // read the picker and name the group in the person's language
    const picker = useEmojiPicker();
    const locale = useLocale();
    const rest = omit(properties, "xstyle");

    return (
        <RadioGroup
            aria-label={locale.render(t`Skin tone`)}
            data-slot="emoji-picker-skin-tone"
            orientation="horizontal"
            value={picker.skinTone()}
            onValueChange={(value) => picker.setSkinTone(skinToneOf(value))}
            {...rest}
            xstyle={[styles.tones, properties.xstyle]}
        >
            <For each={SKIN_TONES}>
                {(tone) => (
                    <RadioGroupItem
                        value={tone}
                        aria-label={locale.render(TONE_NAMES[tone])}
                        xstyle={styles.tone}
                    >
                        {TONE_HANDS[tone]}
                    </RadioGroupItem>
                )}
            </For>
        </RadioGroup>
    );
}

/** Render a button that moves the nearest emoji picker to the next skin tone, showing the current one. */
export function EmojiPickerSkinToneSelector(
    properties: GridListElementProperties<
        Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "onClick">
    >,
): JSX.Element {
    // read the picker and the tone after the current one
    const picker = useEmojiPicker();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style", "children");

    return (
        <button
            type="button"
            data-slot="emoji-picker-skin-tone-selector"
            aria-label={locale.render(t`Change the skin tone`)}
            {...rest}
            onClick={() => picker.setSkinTone(NEXT_TONES[picker.skinTone()])}
            {...style.attributes([styles.selector, properties.xstyle], properties.style)}
        >
            {properties.children ?? TONE_HANDS[picker.skinTone()]}
        </button>
    );
}

/** Render the active emoji and its name, or a prompt to pick one while none is active. */
function ActiveEmoji(properties: {
    /** The active emoji, undefined while none is. */
    readonly entry: EmojiEntry | undefined;
}): JSX.Element {
    const locale = useLocale();

    return (
        <Show
            when={properties.entry}
            fallback={
                <span {...style.attrs(styles.placeholder)}>{locale.render(t`Pick an emoji…`)}</span>
            }
        >
            {(entry) => (
                <span data-slot="emoji-picker-active-emoji" {...style.attrs(styles.active)}>
                    <span aria-hidden="true" {...style.attrs(styles.emoji)}>
                        {entry().emoji}
                    </span>
                    <span>{entry().label}</span>
                </span>
            )}
        </Show>
    );
}

/** Read the nearest emoji picker, refusing a part outside one. */
function useEmojiPicker(): EmojiPickerControl {
    const picker = useContext(EmojiPickerContext);
    if (picker === null) {
        throw new TypeError("an emoji picker part needs an emoji picker around it");
    }

    return picker;
}

/** Read the skin tone of a radio's value, refusing a value of no tone. */
function skinToneOf(value: string): SkinTone {
    const tone = SKIN_TONES.find((entry) => entry === value);
    if (tone === undefined) {
        throw new TypeError(`no skin tone ${value}`);
    }

    return tone;
}

/** Lay out the emoji whose name or words hold a search by category, each in a skin tone. */
function layOutEmoji(
    all: readonly EmojiData[],
    search: string,
    tone: SkinTone,
    locale: Localization,
): CollectionSection<EmojiEntry>[] {
    // find the emoji whose name or words hold the search
    const found = all.filter(
        (entry) =>
            search === "" ||
            [entry.label, ...(entry.tags ?? [])].some((word) => word.includes(search)),
    );

    // group them by category, taking the tone's variant where an emoji has one
    return CATEGORIES.map((category) => ({
        key: String(category.group),
        label: locale.render(category.label),
        items: found
            .filter((entry) => entry.group === category.group)
            .map((entry) => ({
                emoji:
                    (tone === "none"
                        ? undefined
                        : entry.skins?.[SKIN_TONES.indexOf(tone) - 1]?.unicode) ?? entry.unicode,
                label: entry.label,
            })),
    }));
}

/** Load the emoji people pick in a locale's names and words, without the components and regional indicators, in the data's order. */
async function loadEmoji(tag: LocaleTag): Promise<readonly PickableEmoji[]> {
    // take the nearest locale the data covers, English otherwise
    const available = Object.keys(EMOJI_LOCALES).map((entry) => Locale.parse(entry));
    const chosen = Locale.negotiate([tag], available) ?? "en";
    const load = EMOJI_LOCALES[chosen];
    if (load === undefined) {
        throw new TypeError(`no emoji data in ${chosen}`);
    }
    const all = (await load()).default;

    return all
        .filter((entry): entry is PickableEmoji => entry.group !== undefined)
        .toSorted((left, right) => left.order - right.order);
}
