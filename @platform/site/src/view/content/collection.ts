import type { ElementContent, Root } from "hast";
import type { ImageMetadata } from "@destack/view/media";
import { collectionAt, collections } from "../../../content.ts";
import type { Document } from "./document.ts";
import {
    type ContentAsset,
    headingsFor,
    type MarkdownContext,
    parseFrontmatter,
    renderMarkdown,
    requireString,
    searchSectionsFor,
} from "./markdown/markdown.ts";
import { plainTextFor, searchTextFor, tokenEstimateFor } from "./markdown/text.ts";
import type { ContentEntry } from "./presentation.ts";
import type { SearchEntry } from "./search.ts";

/** A public documentation path: lowercase hyphenated segments ending in a Markdown file. */
const DOCUMENT_PATH_PATTERN = /^(?:[a-z0-9]+(?:-[a-z0-9]+)*\/)*[a-z0-9]+(?:-[a-z0-9]+)*\.md$/u;

/** A numbered source segment: a two-digit order and its public name. */
const DOCUMENT_SEGMENT_PATTERN = /^(\d{2})-([a-z0-9]+(?:-[a-z0-9]+)*)$/u;

/** The Markdown sources of every collection, by path below the package. */
const SOURCES: Readonly<Record<string, string>> = import.meta.glob("/content/**/*.md", {
    query: "?raw",
    import: "default",
    eager: true,
});

/** The assets the sources embed, by path: raster images as images, others as URLs. */
const ASSETS: Readonly<Record<string, ImageMetadata | string>> = import.meta.glob(
    "/content/**/*.{png,jpg,jpeg,webp,avif,gif,svg,mp4,webm,ogv}",
    { import: "default", eager: true },
);

/** A page's rendered body. */
export type RenderedContent = {
    /** The body's tree, as remark and rehype built it from its Markdown. */
    tree: Root;
};

/** One rendered post heading. */
export type TableOfContentsEntry = ReturnType<typeof headingsFor>[number];

/** One blog post. */
export type Post = {
    /** The post author. */
    author: string;
    /** The post's first figure image, shown on its directory entry. */
    cover: { image: ImageMetadata; alt: string } | null;
    /** The publication date. */
    date: string;
    /** The authored Markdown route. */
    markdownRoute: string;
    /** The canonical browser route. */
    route: string;
    /** The canonical post slug. */
    slug: string;
    /** The post subtitle. */
    subtitle: string;
    /** The rendered heading tree. */
    tableOfContents: readonly TableOfContentsEntry[];
    /** The plain text route. */
    textRoute: string;
    /** The post title. */
    title: string;
    /** The approximate token count. */
    tokens: number;
};

/** One of the site's policies, such as the privacy policy. */
export type Policy = {
    /** The line below the title. */
    description: string;
    /** The authored Markdown route. */
    markdownRoute: string;
    /** The canonical browser route. */
    route: string;
    /** The policy's slug, such as privacy. */
    slug: string;
    /** The rendered heading tree. */
    tableOfContents: readonly TableOfContentsEntry[];
    /** The plain text route. */
    textRoute: string;
    /** The title. */
    title: string;
    /** The approximate token count. */
    tokens: number;
};

/** A page's portable sources: its Markdown and its plain text, by route. */
export type PageSource = {
    /** The Markdown route. */
    markdownRoute: string;
    /** The plain text route. */
    textRoute: string;
    /** The Markdown. */
    markdown: string;
};

/** A page while its collection is assembled: its tree, sources, headings and search text. */
type RenderedPage = {
    /** The canonical route. */
    route: string;
    /** The title. */
    title: string;
    /** How a documentation page presents: as a chapter, or a catalog of its chapters. */
    kind?: "chapter" | "catalog";
    /** The documentation page above it. */
    parent?: RenderedPage;
    /** The documentation navigation around it. */
    navigation?: Document["navigation"];
    /** The body's tree, its assets as placeholders. */
    tree: Root;
    /** The Markdown it renders. */
    markdown: string;
    /** The Markdown route. */
    markdownRoute: string;
    /** The plain text route. */
    textRoute: string;
    /** The assets its body embeds. */
    assets: ContentAsset[];
    /** The approximate token count. */
    tokens: number;
    /** The headings. */
    headings: ReturnType<typeof headingsFor>;
    /** The sections search indexes. */
    searchSections: ReturnType<typeof searchSectionsFor>;
    /** The text search indexes. */
    searchText: string;
};

/** A documentation page while its collection is assembled. */
type DocumentationPage = RenderedPage & {
    description: string;
    lead?: string;
    path: string;
    entries?: ContentEntry[];
    tableOfContents: ReturnType<typeof headingsFor>;
};

/** A blog post while its collection is assembled. */
type PostPage = RenderedPage & Omit<Post, "cover">;

/** The files of every collection, which links and assets resolve against. */
const files = new Set([...Object.keys(SOURCES), ...Object.keys(ASSETS)]);

/** The documentation pages, rendered once as the module loads on the server. */
const documentPages = renderDocuments(readDocumentSources());

/** The blog posts, rendered once as the module loads on the server. */
const postPages: PostPage[] = renderArticles(readPostSources(), "blog");

/** The policies, rendered once as the module loads on the server. */
const policyPages = renderArticles(readPolicySources(), "legal");

/** The blog directory page. */
const blogPage = renderBlogIndex(postPages);

// link the documentation's navigation, then append each index's chapters
buildNavigation(documentPages);
appendChapterContents(documentPages);

/** The documentation pages with their bodies, in navigation order. */
export const documents: readonly {
    readonly document: Document;
    readonly content: RenderedContent;
}[] = documentPages.map((page) => ({
    document: {
        ...(page.entries === undefined ? {} : { entries: page.entries }),
        description: page.description,
        ...(page.lead === undefined ? {} : { lead: page.lead }),
        markdownRoute: page.markdownRoute,
        kind: page.kind ?? "chapter",
        navigation: present(page.navigation, page.route),
        route: page.route,
        tableOfContents: page.tableOfContents,
        textRoute: page.textRoute,
        title: page.title,
        tokens: page.tokens,
    },
    content: { tree: publish(page) },
}));

/** The blog posts, newest first. */
export const posts: readonly Post[] = postPages.map((page) => ({
    author: page.author,
    cover: coverOf(page),
    date: page.date,
    markdownRoute: page.markdownRoute,
    route: page.route,
    slug: page.slug,
    subtitle: page.subtitle,
    tableOfContents: page.tableOfContents,
    textRoute: page.textRoute,
    title: page.title,
    tokens: page.tokens,
}));

/** The blog posts' rendered bodies, by slug. */
export const postContent: ReadonlyMap<string, RenderedContent> = new Map(
    postPages.map((page) => [page.slug, { tree: publish(page) }]),
);

/** The policies, in path order. */
export const policies: readonly Policy[] = policyPages.map((page) => ({
    description: page.description,
    markdownRoute: page.markdownRoute,
    route: page.route,
    slug: page.slug,
    tableOfContents: page.tableOfContents,
    textRoute: page.textRoute,
    title: page.title,
    tokens: page.tokens,
}));

/** The policies' rendered bodies, by slug. */
export const policyContent: ReadonlyMap<string, RenderedContent> = new Map(
    policyPages.map((page) => [page.slug, { tree: publish(page) }]),
);

/** The blog directory's portable sources and size. */
export const blogIndex = {
    markdownRoute: blogPage.markdownRoute,
    textRoute: blogPage.textRoute,
    tokens: blogPage.tokens,
};

/** The Markdown and text sources of every page. */
export const pageSources: readonly PageSource[] = [
    ...documentPages,
    ...postPages,
    ...policyPages,
    blogPage,
].map((page) => ({
    markdownRoute: page.markdownRoute,
    textRoute: page.textRoute,
    markdown: page.markdown,
}));

/** The full-text search index of the documentation and the blog. */
export const searchEntries: readonly SearchEntry[] = [
    ...documentPages.flatMap((page) => [
        {
            context:
                page.path === "index.md" ? "docs" : page.path.split("/").slice(0, -1).join(" / "),
            kind: "page" as const,
            route: page.route,
            text: `${page.description} ${page.searchSections.find((section) => section.depth === 1)?.text ?? ""}`,
            title: page.title,
        },
        ...sectionEntries(page, page.title),
    ]),
    ...postPages.flatMap((page) => [
        {
            context: `blog / ${page.date}`,
            kind: "page" as const,
            route: page.route,
            text: `${page.subtitle} ${page.searchText}`,
            title: page.title,
        },
        ...sectionEntries(page, page.title),
    ]),
];

/** List the search entries of a page's sections below its title. */
function sectionEntries(page: RenderedPage, context: string): SearchEntry[] {
    return page.searchSections
        .filter((section) => section.depth > 1)
        .map((section) => ({
            context,
            kind: "section",
            route: `${page.route}#${section.id}`,
            text: section.text,
            title: section.title,
        }));
}

/** Require a value the collection assembled. */
function present<Value>(value: Value | undefined, route: string): Value {
    if (value === undefined) {
        throw new Error(`page '${route}' is invalid: it is incomplete`);
    }

    return value;
}

/** Give a page's tree its assets: each image's files and each other asset's URL. */
function publish(page: RenderedPage): Root {
    // attach each image figure's image, and each animation's poster frame
    const tree = structuredClone(page.tree);
    const images = new Map(
        page.assets.flatMap((asset) => {
            const loaded = ASSETS[asset.path];

            return typeof loaded === "object" ? [[asset.placeholder, loaded] as const] : [];
        }),
    );
    attachImages(tree, images);

    // replace every placeholder with its asset's URL
    const urls = new Map(
        page.assets.map((asset) => {
            const loaded = present(ASSETS[asset.path], asset.path);

            return [asset.placeholder, typeof loaded === "object" ? loaded.src : loaded] as const;
        }),
    );
    replacePlaceholders(tree, urls);

    return tree;
}

/** Replace the asset placeholders in a tree's text and properties with their URLs. */
function replacePlaceholders(node: Root | ElementContent, urls: ReadonlyMap<string, string>): void {
    // replace them in an element's text properties and in text
    const replace = (text: string) =>
        [...urls].reduce((each, [placeholder, url]) => each.replaceAll(placeholder, url), text);
    if (node.type === "element") {
        for (const [key, value] of Object.entries(node.properties)) {
            if (typeof value === "string") {
                node.properties[key] = replace(value);
            }
        }
    } else if (node.type === "text") {
        node.value = replace(node.value);
    }

    // descend into the children
    for (const child of "children" in node ? node.children : []) {
        if (child.type !== "doctype") {
            replacePlaceholders(child, urls);
        }
    }
}

/** Give each image figure its image and each animation its poster frame. */
function attachImages(
    node: Root | ElementContent,
    images: ReadonlyMap<string, ImageMetadata>,
): void {
    // attach an image figure's image, or an animation's poster frame
    if (node.type === "element") {
        const kind = node.properties["dataKind"];
        const key = kind === "image" ? "dataSrc" : kind === "animation" ? "dataPoster" : undefined;
        const image = key === undefined ? undefined : images.get(String(node.properties[key]));
        if (image !== undefined) {
            node.properties["dataImage"] = JSON.stringify(image);
        }
    }

    // descend into the children
    for (const child of "children" in node ? node.children : []) {
        if (child.type !== "doctype") {
            attachImages(child, images);
        }
    }
}

/** Read the documentation sources of every documentation collection, in navigation order. */
function readDocumentSources() {
    const sources = collections
        .filter((collection) => collection.route.startsWith("/docs/"))
        .flatMap((collection) =>
            collection.sources.flatMap((source) => {
                // read each Markdown file below the source's directory
                const directory = `/${source.directory.replace(/^@platform\/site\//u, "")}`;
                const found = Object.keys(SOURCES).filter((path) =>
                    path.startsWith(`${directory}/`),
                );
                if (found.length === 0) {
                    throw new Error(`documentation directory '${directory}' not found`);
                }

                return found.map((file) => {
                    // read the title and description from the frontmatter
                    const { markdown, metadata } = parseFrontmatter(
                        present(SOURCES[file], file),
                        file,
                    );
                    requireString(metadata, "title", file);
                    requireString(metadata, "description", file);
                    if (Object.hasOwn(metadata, "order")) {
                        throw new Error(
                            `documentation '${file}' is invalid: its order must be in its source path`,
                        );
                    }

                    // derive the public path and route from the numbered source path
                    const { hierarchy, path: publicPath } = parseDocumentPath(
                        file.slice(directory.length + 1),
                        file,
                    );
                    const path = source.path === "" ? publicPath : `${source.path}/${publicPath}`;

                    return {
                        description: metadata.description,
                        directory,
                        file,
                        headings: headingsFor(markdown),
                        hierarchy: [...source.hierarchy, ...hierarchy],
                        lead: metadata.description,
                        markdownRoute: `/docs/${path}`,
                        markdown,
                        path,
                        route: documentRoute(path),
                        textRoute: `/docs/${path.replace(/\.md$/u, ".txt")}`,
                        title: metadata.title,
                        tokens: tokenEstimateFor(plainTextFor(markdown)),
                    };
                });
            }),
        )
        .toSorted(compareDocuments);
    validateDocuments(sources);

    return sources;
}

/** Parse a numbered source path into its public path and hierarchy. */
function parseDocumentPath(path: string, file: string) {
    // strip the numeric prefix from each segment but a trailing index
    const segments = path.split("/");
    const names: string[] = [];
    const hierarchy: number[] = [];
    for (const [index, segment] of segments.entries()) {
        const isLast = index === segments.length - 1;
        if (isLast && segment === "index.md") {
            names.push(segment);
        } else {
            const extension = isLast ? ".md" : "";
            const match = DOCUMENT_SEGMENT_PATTERN.exec(isLast ? segment.slice(0, -3) : segment);
            if (match === null) {
                throw new Error(
                    `documentation path '${file}' is invalid: it must start with a number`,
                );
            }
            hierarchy.push(Number(match[1]));
            names.push(`${match[2] ?? ""}${extension}`);
        }
    }

    return { hierarchy, path: names.join("/") };
}

/** Compare documentation sources by their numeric hierarchy, then by route. */
function compareDocuments(
    left: { hierarchy: number[]; route: string },
    right: { hierarchy: number[]; route: string },
): number {
    // compare every shared level
    for (const [depth, level] of left.hierarchy.entries()) {
        const other = right.hierarchy[depth];
        if (other === undefined) {
            break;
        }
        if (level !== other) {
            return level - other;
        }
    }

    return left.hierarchy.length - right.hierarchy.length || left.route.localeCompare(right.route);
}

/** Validate the documentation's paths, hierarchy and titles. */
function validateDocuments(
    pages: readonly {
        headings: ReturnType<typeof headingsFor>;
        path: string;
        title: string;
        hierarchy: number[];
    }[],
): void {
    const hierarchies = new Set<string>();
    const paths = new Set(pages.map((document) => document.path));
    for (const document of pages) {
        // require a public path and a hierarchy of its own
        if (!DOCUMENT_PATH_PATTERN.test(document.path)) {
            throw new Error(`documentation path '${document.path}' is invalid`);
        }
        const hierarchy = document.hierarchy.join(".");
        if (hierarchies.has(hierarchy)) {
            throw new Error(`documentation hierarchy '${hierarchy}' already exists`);
        }
        hierarchies.add(hierarchy);

        // require one H1 matching the title
        const titles = document.headings.filter((heading) => heading.depth === 1);
        if (titles.length !== 1 || titles[0]?.text !== document.title) {
            throw new Error(
                `documentation '${document.path}' is invalid: its title must match its H1`,
            );
        }

        // require an index in each directory above the page
        const segments = document.path.split("/");
        for (let depth = 1; depth < segments.length; depth += 1) {
            const index = `${segments.slice(0, depth).join("/")}/index.md`;
            if (!paths.has(index)) {
                throw new Error(`documentation directory ${index} not found`);
            }
        }
    }
}

/** Convert a documentation path into its public route. */
function documentRoute(path: string): string {
    const stem = path.slice(0, -3);
    const route = stem === "index" ? "" : stem.endsWith("/index") ? stem.slice(0, -6) : stem;

    return `/docs/${route === "" ? "" : `${route}/`}`;
}

/** Render the documentation sources against the whole manual's routes and headings. */
function renderDocuments(sources: ReturnType<typeof readDocumentSources>): DocumentationPage[] {
    const sourceRoutes = routesOf(sources);

    return sources.map((source) => {
        const context: MarkdownContext = {
            assets: [],
            collectionDirectory: source.directory,
            files,
            kind: "document",
            markdownDirectory: directoryOf(source.file),
            ownHeadings: new Set(source.headings.map((heading) => heading.id)),
            route: source.route,
            slug: source.path,
            sourceRoutes,
        };

        return {
            ...source,
            assets: context.assets,
            tree: renderMarkdown(source.markdown, context),
            kind: "chapter" as const,
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings.filter((heading) => heading.depth > 1),
        };
    });
}

/** Read the blog's posts, newest first. */
function readPostSources() {
    // read each post directory's index in path order
    const blog = collections.find((collection) => collection.route === "/blog/")?.sources[0];
    if (blog === undefined) {
        throw new Error("blog collection source not found");
    }
    const directory = `/${blog.directory.replace(/^@platform\/site\//u, "")}`;
    const entries = Object.keys(SOURCES)
        .filter((path) => path.startsWith(`${directory}/`) && path.endsWith("/index.md"))
        .toSorted();
    const seen = new Set<string>();

    return entries
        .map((file) => {
            // apply the documentation numbering to the post's directory, refusing a repeated slug
            const name = file.slice(directory.length + 1, -"/index.md".length);
            const { path } = parseDocumentPath(`${name}/index.md`, file);
            const slug = path.slice(0, -"/index.md".length);
            if (seen.has(slug)) {
                throw new Error(`blog slug '${slug}' already exists`);
            }
            seen.add(slug);

            // require the post metadata
            const { markdown, metadata } = parseFrontmatter(present(SOURCES[file], file), file);
            requireString(metadata, "title", file);
            requireString(metadata, "subtitle", file);
            requireString(metadata, "date", file);
            requireString(metadata, "author", file);

            return {
                author: metadata.author,
                date: metadata.date,
                directory,
                file,
                headings: headingsFor(markdown),
                markdown,
                markdownRoute: `/blog/${slug}.md`,
                route: `/blog/${slug}/`,
                slug,
                subtitle: metadata.subtitle,
                textRoute: `/blog/${slug}.txt`,
                title: metadata.title,
                tokens: tokenEstimateFor(plainTextFor(markdown)),
            };
        })
        .toSorted(
            (left, right) =>
                right.date.localeCompare(left.date) || left.title.localeCompare(right.title),
        );
}

/** Read the policies, one Markdown file each, in path order. */
function readPolicySources() {
    // read each policy below the legal collection's directory
    const legal = collections.find((collection) => collection.route === "/legal/")?.sources[0];
    if (legal === undefined) {
        throw new Error("legal collection source not found");
    }
    const directory = `/${legal.directory.replace(/^@platform\/site\//u, "")}`;
    const entries = Object.keys(SOURCES)
        .filter((path) => path.startsWith(`${directory}/`))
        .toSorted();

    return entries.map((file) => {
        // require the policy's metadata, its slug the file's name
        const slug = file.slice(directory.length + 1, -".md".length);
        const { markdown, metadata } = parseFrontmatter(present(SOURCES[file], file), file);
        requireString(metadata, "title", file);
        requireString(metadata, "description", file);

        return {
            description: metadata.description,
            file,
            headings: headingsFor(markdown),
            markdown,
            markdownRoute: `/legal/${slug}.md`,
            route: `/legal/${slug}/`,
            slug,
            textRoute: `/legal/${slug}.txt`,
            title: metadata.title,
            tokens: tokenEstimateFor(plainTextFor(markdown)),
        };
    });
}

/** Render a collection's articles against all its articles' routes and headings. */
function renderArticles<
    Source extends {
        readonly file: string;
        readonly route: string;
        readonly slug: string;
        readonly markdown: string;
        readonly headings: ReturnType<typeof headingsFor>;
    },
>(sources: readonly Source[], kind: "blog" | "legal") {
    const sourceRoutes = routesOf(sources);

    return sources.map((source) => {
        const context: MarkdownContext = {
            assets: [],
            collectionDirectory: directoryOf(source.file),
            files,
            kind,
            markdownDirectory: directoryOf(source.file),
            ownHeadings: new Set(source.headings.map((heading) => heading.id)),
            route: source.route,
            slug: source.slug,
            sourceRoutes,
        };

        return {
            ...source,
            assets: context.assets,
            tree: renderMarkdown(source.markdown, context),
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings,
        };
    });
}

/** Index sources' routes and heading identifiers by their file. */
function routesOf(
    sources: readonly { file: string; route: string; headings: ReturnType<typeof headingsFor> }[],
): Map<string, { route: string; headings: Set<string> }> {
    return new Map(
        sources.map((source) => [
            source.file,
            {
                route: source.route,
                headings: new Set(source.headings.map((heading) => heading.id)),
            },
        ]),
    );
}

/** Read a file's directory below the package. */
function directoryOf(file: string): string {
    return file.slice(0, file.lastIndexOf("/"));
}

/** Read a post's cover: its first raster figure image, none without one. */
function coverOf(page: PostPage): Post["cover"] {
    // find the first image of the post
    const first = firstImageOf(page.tree);
    const asset = page.assets.find((candidate) => candidate.placeholder === first?.source);
    const image = asset === undefined ? undefined : ASSETS[asset.path];

    return typeof image === "object" ? { image, alt: first?.alt ?? "" } : null;
}

/** Find the first image of a tree, an image element or an image figure, in document order. */
function firstImageOf(node: Root | ElementContent): { source: string; alt: string } | undefined {
    // read an image element or an image figure
    if (node.type === "element") {
        const { properties } = node;
        if (node.tagName === "img" && typeof properties["src"] === "string") {
            const alt = properties["alt"];

            return { source: properties["src"], alt: typeof alt === "string" ? alt : "" };
        } else if (
            properties["dataKind"] === "image" &&
            typeof properties["dataSrc"] === "string"
        ) {
            return { source: properties["dataSrc"], alt: String(properties["dataAlt"] ?? "") };
        }
    }

    // search the children in order
    for (const child of "children" in node ? node.children : []) {
        const image = child.type === "doctype" ? undefined : firstImageOf(child);
        if (image !== undefined) {
            return image;
        }
    }

    return undefined;
}

/** Render the blog directory in the same portable formats as documentation indexes. */
function renderBlogIndex(pages: readonly PostPage[]): RenderedPage {
    const markdown = `# Blog\n\n${pages
        .map((page) => `## [${page.title}](${page.route})\n\n${page.subtitle}\n\n${page.date}\n`)
        .join("\n")}`;

    return {
        route: "/blog/",
        title: "Blog",
        markdown,
        markdownRoute: "/blog/index.md",
        textRoute: "/blog/index.txt",
        tree: renderMarkdown(markdown, { assets: [], route: "/blog/", kind: "blog" }),
        assets: [],
        tokens: tokenEstimateFor(plainTextFor(markdown)),
        headings: headingsFor(markdown),
        searchSections: searchSectionsFor(markdown),
        searchText: searchTextFor(markdown),
    };
}

/** Append each index's immediate chapters, which then presents as a catalog. */
function appendChapterContents(pages: DocumentationPage[]): void {
    for (const chapter of pages) {
        // select indexes with chapters below them
        const isIndex = chapter.path === "index.md" || chapter.path.endsWith("/index.md");
        const children = pages.filter((page) => page.parent === chapter);
        if (!isIndex || children.length === 0) {
            continue;
        }

        // write the same links into each published form
        const links = children.map((page) => {
            const title = page.title.replace(/[\\`*_[\]<>]/gu, "\\$&");
            const summary = page.lead ?? page.description;

            return `- [${title}](${page.route})${summary && summary !== page.title ? ` — ${summary}` : ""}`;
        });
        chapter.markdown = `${chapter.markdown.trimEnd()}\n\n---\n\n${links.join("\n")}\n`;
        chapter.entries = children.map((page) => {
            const summary = page.lead ?? page.description;

            return {
                title: page.title,
                href: page.route,
                ...(summary === page.title ? {} : { summary }),
            };
        });

        // present the index as a catalog, its search text and size following the published body
        chapter.kind = "catalog";
        delete chapter.lead;
        chapter.searchSections = searchSectionsFor(chapter.markdown);
        chapter.searchText = searchTextFor(chapter.markdown);
        chapter.tokens = tokenEstimateFor(plainTextFor(chapter.markdown));
    }
}

/** Give each documentation page its parent, ancestry, visible chapters and neighbours. */
function buildNavigation(pages: DocumentationPage[]): void {
    // resolve each page's parent from its route
    const byRoute = new Map(pages.map((page) => [page.route, page]));
    for (const page of pages) {
        const parentRoute = page.route.replace(/[^/]+\/$/u, "");
        const parent = page.route === "/docs/" ? undefined : byRoute.get(parentRoute);
        if (page.route !== "/docs/" && parent === undefined) {
            throw new Error(`parent '${parentRoute}' of '${page.route}' not found`);
        }
        if (parent !== undefined) {
            page.parent = parent;
        }
    }

    // index each page's children in published order
    const children = new Map<RenderedPage | undefined, DocumentationPage[]>();
    for (const page of pages) {
        children.set(page.parent, [...(children.get(page.parent) ?? []), page]);
    }

    // expand the chapters beneath each page's section, and link its neighbours in its collection
    for (const page of pages) {
        const collection = collectionAt(page.route);
        const root = collection === undefined ? undefined : byRoute.get(collection.route);
        if (root === undefined) {
            throw new Error(`collection of '${page.route}' not found`);
        }
        const ancestors = ancestorsOf(page);
        const chain = [...ancestors, page];
        const section = chain[chain.indexOf(root) + 1];
        const visible: DocumentationPage[] = [];
        const visit = (document: DocumentationPage): void => {
            visible.push(document);
            if (
                document === section ||
                (section !== undefined && ancestorsOf(document).includes(section))
            ) {
                for (const child of children.get(document) ?? []) {
                    visit(child);
                }
            }
        };
        for (const document of children.get(root) ?? []) {
            visit(document);
        }
        const rootDepth = ancestorsOf(root).length;
        const sequence = pages.filter(
            (each) => each.kind === "chapter" && collectionAt(each.route) === collection,
        );
        const index = sequence.indexOf(page);
        const previous = index > 0 ? sequence[index - 1] : undefined;
        const next = index >= 0 ? sequence[index + 1] : undefined;
        page.navigation = {
            root: link(root),
            ancestors: ancestors.map(link),
            entries: visible.map((chapter) => ({
                ...link(chapter),
                depth: ancestorsOf(chapter).length - rootDepth - 1,
            })),
            ...(previous === undefined ? {} : { previous: link(previous) }),
            ...(next === undefined ? {} : { next: link(next) }),
        };
    }
}

/** List a page's ancestors, from the root down. */
function ancestorsOf(page: RenderedPage): RenderedPage[] {
    return page.parent === undefined ? [] : [...ancestorsOf(page.parent), page.parent];
}

/** Link to a page by its title and route. */
function link(page: RenderedPage): { readonly title: string; readonly route: string } {
    return { title: page.title, route: page.route };
}
