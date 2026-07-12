import type { Accessor, JSX } from "solid-js";

import type { PageSource } from "../content/source";
import { Contents, type ContentsEntry, trackActiveHeading } from "./contents";
import {
    createPageSourceCommands,
    type PageSourceCommands,
    SourceActions,
} from "./source";

/// Properties for the shared reading frame.
type ReaderProps = {
    /// The rendered article.
    children: JSX.Element;

    /// The current article headings.
    contents: readonly ContentsEntry[];

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
    class: string;

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
        <div class="reader">
            <aside class="reader__sidebar">
                {props.navigation()}
                <Contents activeId={activeHeading} entries={props.contents} />
            </aside>

            <article
                class="reader__article"
                data-markdown-route={props.source.markdownRoute}
                data-page-source
                data-text-route={props.source.textRoute}
            >
                <ReaderToolbar
                    activeHeading={activeHeading}
                    class="reader__toolbar--top"
                    contents={props.contents}
                    location={props.location}
                    navigation={props.navigation}
                    sourceCommands={sourceCommands}
                />

                {props.children}

                <ReaderToolbar
                    activeHeading={activeHeading}
                    class="reader__toolbar--bottom"
                    contents={props.contents}
                    location={props.location}
                    navigation={props.navigation}
                    sourceCommands={sourceCommands}
                />
            </article>
        </div>
    );
}

/// Render one toolbar at its responsive DOM position.
function ReaderToolbar(props: ReaderToolbarProps) {
    return (
        <header class={`reader__toolbar ${props.class}`}>
            <details class="reader-menu" name="reader-tools">
                <summary>[menu]</summary>
                <div class="reader-menu__body">
                    {props.navigation()}
                    <Contents activeId={props.activeHeading} entries={props.contents} />
                </div>
            </details>

            <div class="reader__location">{props.location()}</div>
            <SourceActions commands={props.sourceCommands} />
        </header>
    );
}
