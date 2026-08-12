import type { Accessor, JSX } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import type { PageSource } from "../content/source";
import { Contents, type ContentsEntry, trackActiveHeading } from "./contents";
import {
    createPageSourceCommands,
    type PageSourceCommands,
    SourceActions,
} from "./source";
import { readerStyles } from "./publication.stylex";

/// Properties for the shared reading frame.
type ReaderProps = {
    /// The rendered article.
    children: JSX.Element;

    /// The current article headings.
    contents: readonly ContentsEntry[];

    /// The publication treatment applied to the reader.
    publication: "journal" | "manual";

    /// The collection navigation shown beside the article.
    navigation: () => JSX.Element;

    /// The current location rendered in the article toolbar.
    location: () => JSX.Element;

    /// The portable source files for the current page.
    source: PageSource;
};

/// Properties for one responsive reader toolbar.
type ReaderToolbarProps = {
    /// The currently active heading identifier.
    activeHeading: Accessor<string>;

    /// The toolbar placement.
    placement: "bottom" | "top";

    /// The current article headings.
    contents: readonly ContentsEntry[];

    /// The current location rendered in the article toolbar.
    location: () => JSX.Element;

    /// The collection navigation shown beside the article.
    navigation: () => JSX.Element;

    /// The shared page source commands.
    sourceCommands: PageSourceCommands;
};

/// Render the common blog and documentation reading frame.
export function Reader(props: ReaderProps) {
    const activeHeading = trackActiveHeading(props.contents);
    const sourceCommands = createPageSourceCommands(props.source);

    return (
        <div
            {...stylex.attrs(readerStyles.reader, readerStyles.readerPublication)}
            data-publication={props.publication}
        >
            <aside {...stylex.attrs(readerStyles.sidebar)}>
                {props.navigation()}
                <Contents activeId={activeHeading} entries={props.contents} />
            </aside>

            <article
                {...stylex.attrs(readerStyles.article, readerStyles.articlePublication)}
                data-markdown-route={props.source.markdownRoute}
                data-page-source
                data-text-route={props.source.textRoute}
            >
                <ReaderToolbar
                    activeHeading={activeHeading}
                    contents={props.contents}
                    location={props.location}
                    navigation={props.navigation}
                    placement="top"
                    sourceCommands={sourceCommands}
                />

                {props.children}

                <ReaderToolbar
                    activeHeading={activeHeading}
                    contents={props.contents}
                    location={props.location}
                    navigation={props.navigation}
                    placement="bottom"
                    sourceCommands={sourceCommands}
                />
            </article>
        </div>
    );
}

/// Render one toolbar at its responsive DOM position.
function ReaderToolbar(props: ReaderToolbarProps) {
    return (
        <header
            {...stylex.attrs(
                readerStyles.toolbar,
                readerStyles.toolbarPublication,
                props.placement === "top" ? readerStyles.toolbarTop : readerStyles.toolbarBottom,
            )}
        >
            <details {...stylex.attrs(readerStyles.menu)} name="reader-tools">
                <summary {...stylex.attrs(readerStyles.menuSummary)}>menu</summary>
                <div {...stylex.attrs(readerStyles.menuBody)}>
                    {props.navigation()}
                    <Contents activeId={props.activeHeading} entries={props.contents} isMenu />
                </div>
            </details>

            <div
                {...stylex.attrs(
                    readerStyles.location,
                    props.placement === "bottom" && readerStyles.locationBottom,
                )}
            >
                {props.location()}
            </div>
            <SourceActions commands={props.sourceCommands} />
        </header>
    );
}
