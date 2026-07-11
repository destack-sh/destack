import type { JSX } from "solid-js";

import type { PageSource } from "../content/source";
import { Contents, type ContentsEntry } from "./contents";
import { SourceActions } from "./source";

type ReaderProps = {
    /// The rendered article.
    children: JSX.Element;

    /// The current article headings.
    contents: readonly ContentsEntry[];

    /// The collection navigation shown beside the article.
    navigation: JSX.Element;

    /// The current location rendered in the article toolbar.
    location: JSX.Element;

    /// The portable source files for the current page.
    source: PageSource;
};

/// Render the common blog and documentation reading frame.
export function Reader(props: ReaderProps) {
    return (
        <div class="reader">
            <aside class="reader__sidebar">
                {props.navigation}
                <Contents entries={props.contents} />
            </aside>

            <article class="reader__article">
                <header class="reader__toolbar">
                    {props.location}
                    <SourceActions source={props.source} />
                </header>
                {props.children}
            </article>
        </div>
    );
}
