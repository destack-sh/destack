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

/** Properties for one rendered manual chapter. */
type DocumentArticleProperties = {
    /** The rendered document body. */
    content: DocumentContent;

    /** The current document. */
    document: Document;
};

/** Render a document with its collection navigation. */
export function DocumentArticle(properties: DocumentArticleProperties) {
    // hold the rendered body for the rule catalog controls
    let body: HTMLDivElement | undefined;

    // activate controls only after the complete static directory is mounted
    onSettled(() => {
        if (body) {
            return enhanceRuleCatalog(body);
        }
    });

    // count the tokens of a chapter
    const tokenCount =
        properties.document.kind === "chapter" ? properties.document.tokens : undefined;

    return (
        <Reader
            location={() => <DocumentLocation document={properties.document} />}
            navigation={() => <DocumentNavigation current={properties.document} />}
            pagination={() => <DocumentPagination current={properties.document} />}
            publication="manual"
            source={properties.document}
            tokenCount={tokenCount}
        >
            <Switch>
                <Match when={properties.document.entries}>
                    {(entries) => (
                        <DocumentDirectory
                            document={properties.document}
                            entries={entries()}
                            html={properties.content.html}
                        />
                    )}
                </Match>
                <Match when={properties.document.kind === "catalog"}>
                    <DirectorySection title={properties.document.title}>
                        <div ref={body} class="markdown" innerHTML={properties.content.html} />
                    </DirectorySection>
                </Match>
                <Match when={true}>
                    <PageHeader
                        title={properties.document.title}
                        variant={properties.document.kind === "chapter" ? "chapter" : "reference"}
                        description={properties.document.lead}
                    />
                    <div ref={body} class="markdown" innerHTML={properties.content.html} />
                </Match>
            </Switch>
        </Reader>
    );
}

/** Render an index document: its intro above its filterable entries. */
function DocumentDirectory(properties: {
    document: Document;
    entries: NonNullable<Document["entries"]>;
    html: string;
}) {
    const directory = createDirectory(() => properties.entries);

    return (
        <DirectoryContent
            title={properties.document.title}
            description={properties.document.lead}
            directory={directory}
        >
            <div class="directory-intro">
                <div class="markdown" innerHTML={properties.html} />
            </div>
        </DirectoryContent>
    );
}

/** Properties for the manual chapter navigation. */
type DocumentNavigationProperties = {
    /** The current document. */
    current: Document;
};

/** Render the collection pages independently of the current article outline. */
function DocumentNavigation(properties: DocumentNavigationProperties) {
    // read the navigation of the current document
    const navigation = () => properties.current.navigation;

    // highlight the containing page of a reference item without changing the page list
    const activeRoute = () =>
        [properties.current, ...navigation().ancestors.toReversed()].find((page) =>
            navigation().entries.some((entry) => entry.route === page.route),
        )?.route;

    // link back to the page above the navigation root
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

/** Return the indentation of one generated navigation entry. */
function documentIndent(depth: number) {
    return [styles.depth0, styles.depth1, styles.depth2, styles.depth3][Math.min(depth, 3)];
}

/** Render the generated document ancestors. */
function DocumentLocation(properties: { document: Document }) {
    return (
        <Breadcrumbs
            items={properties.document.navigation.ancestors.map((link) => ({
                href: link.route,
                label: link.title,
            }))}
        />
    );
}

/** Render the generated adjacent chapter links. */
function DocumentPagination(properties: { current: Document }) {
    const previous = () => properties.current.navigation.previous;
    const next = () => properties.current.navigation.next;

    return (
        <Show when={properties.current.kind !== "catalog" && (previous() || next())}>
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

/** Manual navigation styles. */
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
