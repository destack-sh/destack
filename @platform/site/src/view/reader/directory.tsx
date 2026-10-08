import { Icon } from "@destack/icon";
import arrowRight from "@destack/icon/phosphor/arrow-right";
import { text } from "@destack/theme/text";
import { color, stroke, weight } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Button } from "@destack/ui/button";
import { Empty, EmptyContent, EmptyHeader, EmptyTitle } from "@destack/ui/empty";
import {
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemGroup,
    ItemHeader,
    ItemSeparator,
    ItemTitle,
    itemStyle,
} from "@destack/ui/item";
import { Select, SelectItem, SelectTrigger, SelectValue } from "@destack/ui/select";
import { ToggleGroup, ToggleGroupItem } from "@destack/ui/toggle-group";
import { type Accessor, createMemo, For, type JSX, Show } from "@destack/view";
import { parseAsInteger, useQueryState } from "@destack/view/router";

import { type ContentEntry, formatDate } from "../content/presentation";
import { PageHeader } from "./header";
import { publicationStyles } from "./publication.stylex";

/** The archive value that stands for every year. */
const ALL_YEARS = "all";

/** Keep collection filters in the URL so Back restores the previous view. */
export function createDirectory(entries: Accessor<readonly ContentEntry[]>) {
    // read the year filter from the URL and list the years on offer, latest first
    const [year, setYear] = useQueryState("year", parseAsInteger);
    const years = createMemo(() =>
        [
            ...new Set(
                entries().flatMap((entry) =>
                    entry.date === undefined || entry.date === ""
                        ? []
                        : [Number(entry.date.slice(0, 4))],
                ),
            ),
        ].toSorted((left, right) => right - left),
    );
    const filtered = createMemo(() => entries().filter((entry) => isIn(entry, year())));

    return { entries, year, years, filtered, setYear };
}

/** The year filter state of a dated collection. */
type Directory = ReturnType<typeof createDirectory>;

/** Browse a dated collection by publication year. */
export function DirectoryArchive(properties: { directory: Directory }) {
    const directory = properties.directory;

    // count the entries published in one year
    const countIn = (year: number) =>
        directory.entries().filter((entry) => isIn(entry, year)).length;

    return (
        <Show when={directory.years().length > 1}>
            <ToggleGroup
                aria-label="Archive"
                orientation="vertical"
                value={optionOf(directory.year())}
                onValueChange={(option) => void directory.setYear(yearOf(option))}
                xstyle={publicationStyles.collectionList}
            >
                <ToggleGroupItem
                    value={ALL_YEARS}
                    xstyle={[
                        publicationStyles.collectionLink,
                        styles.year,
                        directory.year() === null && publicationStyles.active,
                    ]}
                >
                    All time{" "}
                    <span {...style.attrs(styles.count)}>{directory.entries().length}</span>
                </ToggleGroupItem>
                <For each={directory.years()}>
                    {(year) => (
                        <ToggleGroupItem
                            value={optionOf(year)}
                            xstyle={[
                                publicationStyles.collectionLink,
                                styles.year,
                                directory.year() === year && publicationStyles.active,
                            ]}
                        >
                            {year} <span {...style.attrs(styles.count)}>{countIn(year)}</span>
                        </ToggleGroupItem>
                    )}
                </For>
            </ToggleGroup>
        </Show>
    );
}

/** Render a collection title above its content. */
function DirectorySection(properties: {
    title: string;
    description?: string | undefined;
    children: JSX.Element;
}) {
    return (
        <section {...style.attrs(styles.section)}>
            <PageHeader
                title={properties.title}
                variant="chapter"
                description={properties.description}
            />
            {properties.children}
        </section>
    );
}

/** Render collection controls and navigable entries. */
export function DirectoryContent(properties: {
    title: string;
    description?: string | undefined;
    directory: Directory;
    children?: JSX.Element;
}) {
    return (
        <DirectorySection title={properties.title} description={properties.description}>
            {/* offer the year filter inline when the sidebar archive is hidden */}
            <Show
                when={
                    properties.directory.years().length > 1 || properties.directory.year() !== null
                }
            >
                <Select
                    aria-label="Archive year"
                    value={optionOf(properties.directory.year())}
                    onValueChange={(option) => void properties.directory.setYear(yearOf(option))}
                    xstyle={styles.filter}
                >
                    <SelectTrigger>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectItem value={ALL_YEARS}>All time</SelectItem>
                    <For each={properties.directory.years()}>
                        {(year) => <SelectItem value={optionOf(year)}>{year}</SelectItem>}
                    </For>
                </Select>
            </Show>
            {properties.children}
            <DirectoryList label={properties.title} entries={properties.directory.filtered()} />
            <Show when={properties.directory.filtered().length === 0}>
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>No matching entries</EmptyTitle>
                    </EmptyHeader>
                    <EmptyContent>
                        <Button
                            variant="link"
                            onClick={() => void properties.directory.setYear(null)}
                        >
                            Clear filters
                        </Button>
                    </EmptyContent>
                </Empty>
            </Show>
        </DirectorySection>
    );
}

/** Render a collection's entries as a list of links between rules, each with its cover, summary and date. */
function DirectoryList(properties: { label: string; entries: readonly ContentEntry[] }) {
    return (
        <ItemGroup aria-label={properties.label} xstyle={styles.list}>
            <For each={properties.entries}>
                {(entry, index) => (
                    <>
                        <Show when={index() > 0}>
                            <ItemSeparator />
                        </Show>
                        <a
                            href={entry.href}
                            role="listitem"
                            {...style.attrs(style.defaultMarker(), itemStyle({}), styles.entry)}
                        >
                            <Show when={entry.image}>
                                {(image) => (
                                    <ItemHeader>
                                        <img
                                            src={image().source}
                                            alt={image().alt}
                                            loading="lazy"
                                            {...style.attrs(styles.cover)}
                                        />
                                    </ItemHeader>
                                )}
                            </Show>
                            <ItemContent>
                                <ItemTitle xstyle={[text.title3, styles.title]}>
                                    {entry.title}
                                </ItemTitle>
                                <Show when={entry.summary}>
                                    {(summary) => (
                                        <ItemDescription
                                            xstyle={[publicationStyles.reading, styles.summary]}
                                        >
                                            {summary()}
                                        </ItemDescription>
                                    )}
                                </Show>
                                <Show when={entry.date}>
                                    {(date) => (
                                        <time
                                            datetime={date()}
                                            {...style.attrs(text.subheadline, styles.date)}
                                        >
                                            {formatDate(date())}
                                        </time>
                                    )}
                                </Show>
                            </ItemContent>
                            <ItemActions xstyle={styles.arrow}>
                                <Icon icon={arrowRight} />
                            </ItemActions>
                        </a>
                    </>
                )}
            </For>
        </ItemGroup>
    );
}

/** Report whether an entry was published in a year, every entry for no year. */
function isIn(entry: ContentEntry, year: number | null): boolean {
    return year === null || entry.date?.startsWith(String(year)) === true;
}

/** Write a year as its archive option, all years for none. */
function optionOf(year: number | null): string {
    return year === null ? ALL_YEARS : String(year);
}

/** Read the year an archive option names, none for all years. */
function yearOf(option: string | undefined): number | null {
    return option === undefined || option === ALL_YEARS ? null : Number(option);
}

/** The directory styles. */
const styles = style.create({
    section: {
        minWidth: 0,
    },
    year: {
        alignItems: "baseline",
        backgroundColor: "transparent",
        borderWidth: 0,
        cursor: "pointer",
        display: "flex",
        font: "inherit",
        justifyContent: "space-between",
        paddingInline: 0,
        textAlign: "left",
        width: "100%",
    },
    count: {
        color: color.mutedForeground,
        display: "inline-block",
        fontVariantNumeric: "tabular-nums",
    },
    filter: {
        justifySelf: "start",
        marginBottom: "1.5rem",
        "@media (width >= 60rem)": { display: "none" },
    },
    list: {
        borderBlockWidth: stroke.border,
        borderBlockStyle: "solid",
        borderBlockColor: color.border,
    },
    entry: {
        alignItems: "flex-start",
        paddingBlock: { default: "1.5rem", "@media (max-width: 600px)": "1.25rem" },
        paddingInlineStart: 0,
        paddingInlineEnd: { default: "1.5rem", "@media (max-width: 600px)": "1.25rem" },
        borderRadius: 0,
        backgroundColor: "transparent",
        color: color.foreground,
    },
    cover: {
        display: "block",
        width: "100%",
        aspectRatio: "21 / 9",
        objectFit: "cover",
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        marginBottom: "0.5rem",
    },
    title: {
        fontWeight: weight.medium,
        color: {
            default: null,
            [style.when.ancestor(":is(:hover, :focus-visible)")]: color.primary,
        },
        textDecorationLine: {
            default: "none",
            [style.when.ancestor(":is(:hover, :focus-visible)")]: "underline",
        },
        textDecorationThickness: "1px",
        textUnderlineOffset: "0.2em",
    },
    summary: {
        display: "block",
        maxWidth: "72ch",
        color: color.foreground,
        lineHeight: 1.5,
        WebkitLineClamp: "unset",
    },
    date: {
        marginTop: "0.5rem",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    arrow: {
        alignSelf: "flex-start",
        marginTop: "0.25rem",
        color: {
            default: color.mutedForeground,
            [style.when.ancestor(":is(:hover, :focus-visible)")]: color.primary,
        },
    },
});
