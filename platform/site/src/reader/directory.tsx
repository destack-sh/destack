import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { type Accessor, createMemo, For, type JSX, Show } from "@destack/view";
import { useSearchParams } from "@destack/view/router";

import { type ContentEntry, renderContentList } from "../content/presentation";
import { tokens } from "../style/tokens.stylex";
import { PageHeader } from "./header";
import { publicationStyles } from "./publication.stylex";

/// Keep collection filters in the URL so Back restores the previous view.
export function createDirectory(entries: Accessor<readonly ContentEntry[]>) {
    const [parameters, setParameters] = useSearchParams();
    const year = () => String(parameters.year ?? "");
    const years = createMemo(() =>
        [...new Set(entries().flatMap((entry) => (entry.date ? [entry.date.slice(0, 4)] : [])))]
            .sort()
            .reverse(),
    );
    const filtered = createMemo(() =>
        entries().filter((entry) => {
            return !year() || entry.date?.startsWith(year());
        }),
    );

    return { entries, year, years, filtered, setParameters };
}
type Directory = ReturnType<typeof createDirectory>;

/// Browse a dated collection by publication year.
export function DirectoryArchive(props: { directory: Directory }) {
    const directory = props.directory;

    // count the entries published in one year
    const countIn = (year: string) =>
        directory.entries().filter((entry) => entry.date?.startsWith(year)).length;

    return (
        <Show when={directory.years().length > 0}>
            <section aria-label="Archive" {...stylex.attrs(publicationStyles.collectionList)}>
                <button
                    type="button"
                    aria-pressed={!directory.year() ? "true" : "false"}
                    onClick={() => directory.setParameters({ year: undefined })}
                    {...stylex.attrs(
                        publicationStyles.collectionLink,
                        styles.year,
                        !directory.year() && publicationStyles.active,
                    )}
                >
                    All time{" "}
                    <span {...stylex.attrs(styles.count)}>{directory.entries().length}</span>
                </button>
                <For each={directory.years()}>
                    {(year) => (
                        <button
                            type="button"
                            aria-pressed={directory.year() === year ? "true" : "false"}
                            onClick={() => directory.setParameters({ year })}
                            {...stylex.attrs(
                                publicationStyles.collectionLink,
                                styles.year,
                                directory.year() === year && publicationStyles.active,
                            )}
                        >
                            {year} <span {...stylex.attrs(styles.count)}>{countIn(year)}</span>
                        </button>
                    )}
                </For>
            </section>
        </Show>
    );
}

/// Render a collection title above its content.
export function DirectorySection(props: {
    title: string;
    description?: string;
    children: JSX.Element;
}) {
    return (
        <section {...stylex.attrs(styles.section)}>
            <PageHeader title={props.title} variant="chapter" description={props.description} />
            {props.children}
        </section>
    );
}

/// Render collection controls and navigable entries.
export function DirectoryContent(props: {
    title: string;
    description?: string;
    directory: Directory;
    children?: JSX.Element;
}) {
    return (
        <DirectorySection title={props.title} description={props.description}>
            {/* offer the year filter inline when the sidebar archive is hidden */}
            <Show when={props.directory.years().length > 1 || props.directory.year()}>
                <select
                    aria-label="Archive year"
                    value={props.directory.year()}
                    onChange={(event) =>
                        props.directory.setParameters({
                            year: event.currentTarget.value || undefined,
                        })
                    }
                    {...stylex.attrs(styles.filter)}
                >
                    <option value="">All time</option>
                    <For each={props.directory.years()}>
                        {(year) => <option value={year}>{year}</option>}
                    </For>
                </select>
            </Show>
            {props.children}
            <div innerHTML={renderContentList(props.directory.filtered())} />
            <Show when={props.directory.filtered().length === 0}>
                <p {...stylex.attrs(styles.empty)}>
                    No matching entries.{" "}
                    <button
                        type="button"
                        onClick={() => props.directory.setParameters({ year: undefined })}
                        {...stylex.attrs(styles.clear)}
                    >
                        Clear filters
                    </button>
                </p>
            </Show>
        </DirectorySection>
    );
}

const styles = stylex.create({
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
    empty: {
        margin: 0,
        paddingBlock: "1.5rem",
    },
    clear: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.primary,
        cursor: "pointer",
        textDecorationLine: "underline",
    },
});
