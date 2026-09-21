import { type Accessor, createMemo, For, type JSX, Show } from "@destack/view";
import { useSearchParams } from "@destack/view/router";
import { type ContentEntry, renderContentList } from "../content/presentation";
import "./directory.css";

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

    return (
        <Show when={directory.years().length > 0}>
            <section aria-label="Archive">
                <button
                    type="button"
                    aria-pressed={!directory.year() ? "true" : "false"}
                    onClick={() => directory.setParameters({ year: undefined })}
                >
                    All time <span>{directory.entries().length}</span>
                </button>
                <For each={directory.years()}>
                    {(year) => (
                        <button
                            type="button"
                            aria-pressed={directory.year() === year ? "true" : "false"}
                            onClick={() => directory.setParameters({ year })}
                        >
                            {year}
                            <span>
                                {
                                    directory
                                        .entries()
                                        .filter((entry) => entry.date?.startsWith(year)).length
                                }
                            </span>
                        </button>
                    )}
                </For>
            </section>
        </Show>
    );
}

/// Align collection headings, controls, and content.
export function DirectorySection(props: { title: string; children: JSX.Element }) {
    return (
        <section class="directory">
            <header class="directory-heading">
                <h1>{props.title}</h1>
            </header>
            {props.children}
        </section>
    );
}

/// Render collection controls and navigable entries.
export function DirectoryContent(props: {
    title: string;
    directory: Directory;
    children?: JSX.Element;
}) {
    return (
        <DirectorySection title={props.title}>
            <Show when={props.directory.years().length > 1 || props.directory.year()}>
                <div class="collection-mobile-filters">
                    <Show when={props.directory.years().length > 1 || props.directory.year()}>
                        <select
                            aria-label="Archive year"
                            value={props.directory.year()}
                            onChange={(event) =>
                                props.directory.setParameters({
                                    year: event.currentTarget.value || undefined,
                                })
                            }
                        >
                            <option value="">All time</option>
                            <For each={props.directory.years()}>
                                {(year) => <option value={year}>{year}</option>}
                            </For>
                        </select>
                    </Show>
                </div>
            </Show>
            {props.children}
            <div innerHTML={renderContentList(props.directory.filtered())} />
            <Show when={props.directory.filtered().length === 0}>
                <p class="directory-empty">
                    No matching entries.{" "}
                    <button
                        type="button"
                        onClick={() => props.directory.setParameters({ year: undefined })}
                    >
                        Clear filters
                    </button>
                </p>
            </Show>
        </DirectorySection>
    );
}
