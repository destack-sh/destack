import { color } from "@destack/theme/tokens.stylex";
import { For, Match, onSettled, Show, Switch } from "@destack/view";
import * as stylex from "@destack/style";

import { type Document, type DocumentContent } from "../content/document";
import { Breadcrumbs } from "./breadcrumbs";
import { publicationStyles } from "./publication.stylex";
import { Reader } from "./reader";
import { enhanceRuleCatalog } from "./rules";
import "./rules.css";
import { PageHeader } from "./header";
import { createDirectory, DirectoryContent, DirectorySection } from "./directory";

/// Properties for one rendered manual chapter.
type DocumentArticleProps = {
    /// The rendered document body.
    content: DocumentContent;

    /// The current document.
    document: Document;
};

/// Render a document with its collection navigation.
export function DocumentArticle(props: DocumentArticleProps) {
    let body: HTMLDivElement | undefined;

    // activate controls only after the complete static directory is mounted
    onSettled(() => {
        if (body) {
            return enhanceRuleCatalog(body);
        }
    });

    const tokenCount = props.document.kind === "chapter" ? props.document.tokens : undefined;

    return (
        <Reader
            location={() => <DocumentLocation document={props.document} />}
            navigation={() => <DocumentNavigation current={props.document} />}
            pagination={() => <DocumentPagination current={props.document} />}
            publication="manual"
            source={props.document}
            tokenCount={tokenCount}
        >
            <Switch>
                <Match when={props.document.entries}>
                    {(entries) => (
                        <DocumentDirectory
                            document={props.document}
                            entries={entries()}
                            html={props.content.html}
                        />
                    )}
                </Match>
                <Match when={props.document.kind === "catalog"}>
                    <DirectorySection title={props.document.title}>
                        <div ref={body} class="markdown" innerHTML={props.content.html} />
                    </DirectorySection>
                </Match>
                <Match when={true}>
                    <PageHeader
                        title={props.document.title}
                        variant={props.document.kind === "chapter" ? "chapter" : "reference"}
                        description={props.document.lead}
                    />
                    <div ref={body} class="markdown" innerHTML={props.content.html} />
                </Match>
            </Switch>
        </Reader>
    );
}

/// Render an index document: its intro above its filterable entries.
function DocumentDirectory(props: {
    document: Document;
    entries: NonNullable<Document["entries"]>;
    html: string;
}) {
    const directory = createDirectory(() => props.entries);

    return (
        <DirectoryContent
            title={props.document.title}
            description={props.document.lead}
            directory={directory}
        >
            <div class="directory-intro">
                <div class="markdown" innerHTML={props.html} />
            </div>
        </DirectoryContent>
    );
}

/// Properties for the manual chapter navigation.
type DocumentNavigationProps = {
    /// The current document.
    current: Document;
};

/// Render the collection pages independently of the current article outline.
function DocumentNavigation(props: DocumentNavigationProps) {
    const navigation = () => props.current.navigation;
    // reference items highlight their containing page without changing the page list
    const activeRoute = () =>
        [props.current, ...navigation().ancestors.toReversed()].find((page) =>
            navigation().entries.some((entry) => entry.route === page.route),
        )?.route;

    const parent = () => {
        const ancestors = navigation().ancestors;
        const rootIndex = ancestors.findIndex((entry) => entry.route === navigation().root.route);
        return ancestors[rootIndex < 0 ? ancestors.length - 1 : rootIndex - 1];
    };

    return (
        <nav aria-label="Manual contents">
            <div {...stylex.attrs(publicationStyles.context)}>
                <a href={navigation().root.route} {...stylex.attrs(publicationStyles.contextTitle)}>
                    {navigation().root.title}
                </a>
                <Show when={parent()}>
                    {(parent) => (
                        <a href={parent().route} {...stylex.attrs(publicationStyles.contextBack)}>
                            ← {parent().title}
                        </a>
                    )}
                </Show>
            </div>
            <ol {...stylex.attrs(publicationStyles.collectionList)}>
                <For each={navigation().entries}>
                    {(entry, index) => (
                        <li>
                            <a
                                {...stylex.attrs(
                                    publicationStyles.collectionLink,
                                    documentIndent(entry.depth),
                                    entry.depth === 0 && styles.section,
                                    entry.depth === 0 && index() > 0 && styles.sectionGap,
                                    entry.route === activeRoute() && publicationStyles.active,
                                )}
                                href={entry.route}
                            >
                                {entry.title}
                            </a>
                        </li>
                    )}
                </For>
            </ol>
        </nav>
    );
}

/// Return the indentation of one generated navigation entry.
function documentIndent(depth: number) {
    return [styles.depth0, styles.depth1, styles.depth2, styles.depth3][Math.min(depth, 3)];
}

/// Render the generated document ancestors.
function DocumentLocation(props: { document: Document }) {
    return (
        <Breadcrumbs
            items={props.document.navigation.ancestors.map((link) => ({
                href: link.route,
                label: link.title,
            }))}
        />
    );
}

/// Render the generated adjacent chapter links.
function DocumentPagination(props: { current: Document }) {
    const previous = () => props.current.navigation.previous;
    const next = () => props.current.navigation.next;

    return (
        <Show when={props.current.kind !== "catalog" && (previous() || next())}>
            <nav aria-label="Chapter pagination" {...stylex.attrs(publicationStyles.pagination)}>
                <Show when={previous()}>
                    {(link) => (
                        <a {...stylex.attrs(publicationStyles.paginationLink)} href={link().route}>
                            ← {link().title}
                        </a>
                    )}
                </Show>
                <Show when={next()}>
                    {(link) => (
                        <a {...stylex.attrs(publicationStyles.paginationLink)} href={link().route}>
                            {link().title} →
                        </a>
                    )}
                </Show>
            </nav>
        </Show>
    );
}

/// Manual navigation styles.
const styles = stylex.create({
    depth0: { paddingLeft: 0 },
    depth1: { paddingLeft: "1rem" },
    depth2: { paddingLeft: "2rem" },
    depth3: { paddingLeft: "3rem" },
    section: {
        color: color.foreground,
        fontWeight: 500,
    },
    sectionGap: {
        paddingTop: "0.75rem",
    },
});
