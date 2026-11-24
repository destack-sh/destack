import { RowLayout } from "../gallery/layout.ts";
import * as style from "@destack/style";
import { defineExample } from "@destack/package/declare";
import { color, radius, shadow, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { createSignal, omit } from "@destack/view";
import {
    AutocompleteContext,
    AutocompleteControl,
    AutocompleteInput,
} from "../autocomplete/index.ts";
import type { CollectionSection } from "../collection/index.ts";
import { Stack } from "../stack/index.ts";
import {
    GridList,
    GridListCell,
    GridListControl,
    GridListEmpty,
    GridListProvider,
    GridListViewport,
} from "./grid-list.tsx";

/** A symbol a grid list offers. */
interface MathSymbol {
    /** The symbol. */
    readonly symbol: string;
    /** Its name. */
    readonly name: string;
}

/** Math symbols in two sections, five and three of them. */
const SYMBOLS: readonly CollectionSection<MathSymbol>[] = [
    {
        key: "operators",
        label: "Operators",
        items: [
            { symbol: "±", name: "plus-minus" },
            { symbol: "×", name: "multiplication" },
            { symbol: "÷", name: "division" },
            { symbol: "≠", name: "not equal" },
            { symbol: "≈", name: "almost equal" },
        ],
    },
    {
        key: "others",
        label: "Others",
        items: [
            { symbol: "∞", name: "infinity" },
            { symbol: "∑", name: "summation" },
            { symbol: "√", name: "square root" },
        ],
    },
];

/** Render a grid list of symbol sections, three to a row, reporting the symbol it picks. */
function SymbolGrid(properties: {
    readonly sections: readonly CollectionSection<MathSymbol>[];
    readonly unavailable?: readonly string[];
}) {
    // lay the symbols out three to a row, reporting the one picked
    const [picked, setPicked] = createSignal("");
    const grid = new GridListControl<MathSymbol>({
        sections: () => properties.sections,
        key: (entry) => entry.symbol,
        text: (entry) => entry.name,
        columns: () => 3,
        isDisabled: (entry) => properties.unavailable?.includes(entry.name) === true,
        onAction: (entry) => setPicked(entry.name),
    });

    return (
        <GridListProvider control={grid}>
            <GridListViewport>
                <GridListEmpty>No symbols</GridListEmpty>
                <GridList control={grid} aria-label="Symbols" label={(entry) => entry.name}>
                    {(cell) => (
                        <GridListCell {...omit(cell, "item")}>{cell.item.symbol}</GridListCell>
                    )}
                </GridList>
            </GridListViewport>
            <output>{picked()}</output>
        </GridListProvider>
    );
}

/** Math symbols by section in a grid of three columns. */
export const gridListSymbols = defineExample({
    of: GridList,
    name: "symbols",
    description:
        "math symbols by section in a grid of three columns, which the arrow keys move through and Enter picks from",
    render: () => <SymbolGrid sections={SYMBOLS} />,
});

/** The symbol grid with two symbols unavailable, which the arrow keys pass over. */
export const gridListSymbolsDisabled = defineExample({
    of: GridList,
    name: "symbols-disabled",
    description: "the symbol grid with two symbols unavailable, which the arrow keys pass over",
    render: () => <SymbolGrid sections={SYMBOLS} unavailable={["infinity", "summation"]} />,
});

/** A grid list without symbols, showing its empty note. */
export const gridListEmpty = defineExample({
    of: GridList,
    name: "empty",
    description: "a grid list without symbols, showing its empty note",
    render: () => <SymbolGrid sections={[]} />,
});

/** An asset of a launch's press kit. */
interface Asset {
    /** The asset's id. */
    readonly id: string;
    /** The asset's title. */
    readonly title: string;
    /** Where the asset goes. */
    readonly place: string;
    /** The picture that stands for the asset. */
    readonly emoji: string;
}

/** A launch's press kit: the assets and where they go. */
const ASSETS: readonly Asset[] = [
    { id: "hero", title: "Hero shot", place: "Launch post", emoji: "🌅" },
    { id: "logo", title: "Logo, light and dark", place: "Press kit", emoji: "🔷" },
    { id: "pricing", title: "Pricing page", place: "Website", emoji: "🏷️" },
    { id: "demo", title: "Demo video still", place: "Social", emoji: "🎬" },
    { id: "team", title: "Team photo", place: "About page", emoji: "👥" },
    { id: "chart", title: "Growth chart", place: "Investor update", emoji: "📈" },
    { id: "map", title: "Regions map", place: "Website", emoji: "🗺️" },
];

/** The styles of the gallery's cards. */
const cards = style.create({
    gallery: {
        maxHeight: "28rem",
    },
    card: {
        flexDirection: "column",
        alignItems: "stretch",
        overflow: "hidden",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: color.card,
        boxShadow: shadow.raised,
        cursor: "default",
    },
    cover: {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        aspectRatio: "16 / 9",
        backgroundColor: swatch.blueTint,
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
        fontSize: "2rem",
    },
    content: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        paddingBlock: space[2],
        paddingInline: space[3],
    },
    title: {
        overflow: "hidden",
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
        fontWeight: weight.medium,
    },
    place: {
        color: color.mutedForeground,
    },
});

/** A launch's press kit as a gallery of cards, as many to a row as fit. */
export const gridListGallery = defineExample({
    of: GridList,
    name: "gallery",
    layout: "padded",
    description:
        "a press kit as cards with covers, as many to a row as fit, opened by a press and selected with a modifier or Space",
    render: () => {
        // fit cards at least 220 pixels wide, select several, and report the one opened
        const [opened, setOpened] = createSignal("");
        const grid = new GridListControl<Asset>({
            sections: () => [{ key: "assets", items: ASSETS }],
            key: (asset) => asset.id,
            text: (asset) => asset.title,
            minItemWidth: 220,
            gap: 12,
            rowHeight: 200,
            selection: { multiple: true, behavior: "replace" },
            onAction: (asset) => setOpened(asset.title),
        });

        return (
            <GridListProvider control={grid}>
                <GridListViewport xstyle={cards.gallery}>
                    <GridList control={grid} aria-label="Press kit" label={(asset) => asset.title}>
                        {(cell) => (
                            <GridListCell {...omit(cell, "item")} xstyle={cards.card}>
                                <div aria-hidden="true" {...style.attrs(cards.cover)}>
                                    {cell.item.emoji}
                                </div>
                                <div {...style.attrs(text.sm, cards.content)}>
                                    <span {...style.attrs(cards.title)}>{cell.item.title}</span>
                                    <span {...style.attrs(cards.place)}>{cell.item.place}</span>
                                </div>
                            </GridListCell>
                        )}
                    </GridList>
                </GridListViewport>
                <output>{opened()}</output>
            </GridListProvider>
        );
    },
});

/** Five thousand people, named by number. */
const PEOPLE = Array.from({ length: 5000 }, (_, index) => `Person ${String(index + 1)}`);

/** The styles of the list of people. */
const people = style.create({
    list: {
        width: "100%",
        maxWidth: "24rem",
        maxHeight: "18rem",
        marginInline: "auto",
        borderRadius: radius[3],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
    },
    row: {
        justifyContent: "flex-start",
        width: "100%",
        paddingInline: space[3],
        paddingBlock: space[2],
        borderRadius: 0,
    },
});

/** Five thousand people in one column under a search field, with only the rows in view rendered. */
export const gridListPeople = defineExample({
    of: GridList,
    name: "people",
    layout: "padded",
    description:
        "five thousand people in one column under a search field, with only the rows in view in the document",
    render: () => {
        // filter by the search, one person to a row, and report the one picked
        const [picked, setPicked] = createSignal("");
        const autocomplete = new AutocompleteControl({});
        const grid = new GridListControl<string>({
            sections: () => {
                // read the search once, then list the people it is part of
                const search = autocomplete.value().toLowerCase();
                const items = PEOPLE.filter((person) => person.toLowerCase().includes(search));

                return [{ key: "people", items }];
            },
            key: (person) => person,
            text: (person) => person,
            autocomplete,
            onAction: (person) => setPicked(person),
        });

        return (
            <AutocompleteContext value={autocomplete}>
                <Stack space="2">
                    <AutocompleteInput aria-label="Find a person" placeholder="Find a person" />
                    <GridListProvider control={grid}>
                        <GridListViewport xstyle={people.list}>
                            <GridListEmpty>Nobody by that name</GridListEmpty>
                            <GridList control={grid} aria-label="People" label={(person) => person}>
                                {(cell) => (
                                    <GridListCell
                                        {...omit(cell, "item")}
                                        xstyle={[text.sm, people.row]}
                                    >
                                        {cell.item}
                                    </GridListCell>
                                )}
                            </GridList>
                        </GridListViewport>
                    </GridListProvider>
                    <output>{picked()}</output>
                </Stack>
            </AutocompleteContext>
        );
    },
});

/** Ten thousand photos of ten trips, each with its ratio and color. */
const PHOTOS = Array.from({ length: 10_000 }, (_, i) => ({
    id: String(i),
    title: `Photo ${String(i + 1)}`,
    ratio: [1.5, 4 / 3, 0.75, 1, 16 / 9, 9 / 16, 2.4][(i * 5) % 7] ?? 1,
    color: `hsl(${String((i * 47) % 360)} 45% 62%)`,
}));

/** The styles of a library of photos. */
const library = style.create({
    list: {
        maxHeight: "26rem",
    },
    photo: {
        overflow: "hidden",
        borderRadius: radius[3],
    },
    picture: (tint: string) => ({
        width: "100%",
        height: "100%",
        backgroundColor: tint,
    }),
});

/** Ten thousand photos in rows that fill the page, only those in view in the document. */
export const gridListPhotos = defineExample({
    of: GridList,
    name: "photos",
    layout: "padded",
    description:
        "ten thousand photos in rows that fill the page, only those in view in the document",
    render: () => {
        // break the photos of each trip into rows, and report the one opened
        const [opened, setOpened] = createSignal("");
        const grid = new GridListControl<(typeof PHOTOS)[number]>({
            sections: () =>
                Array.from({ length: 10 }, (_, i) => ({
                    key: String(i),
                    label: `Trip ${String(i + 1)}`,
                    items: PHOTOS.slice(i * 1000, (i + 1) * 1000),
                })),
            key: (photo) => photo.id,
            text: (photo) => photo.title,
            layout: new RowLayout({ height: 110, gap: 6 }),
            ratio: (photo) => photo.ratio,
            selection: { multiple: true, behavior: "replace" },
            onAction: (photo) => setOpened(photo.title),
        });

        return (
            <GridListProvider control={grid}>
                <GridListViewport xstyle={library.list}>
                    <GridList control={grid} aria-label="Photos" label={(photo) => photo.title}>
                        {(cell) => (
                            <GridListCell {...omit(cell, "item")} xstyle={library.photo}>
                                <div {...style.attrs(library.picture(cell.item.color))} />
                            </GridListCell>
                        )}
                    </GridList>
                </GridListViewport>
                <output>{opened()}</output>
            </GridListProvider>
        );
    },
});
