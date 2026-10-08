import type { ElementContent, Root } from "hast";
import type { DocumentationPage, RenderedPage } from "./page.ts";
import { formatSource } from "@destack/check";
import type { MarkdownContext } from "./markdown.ts";
import { buildNavigation } from "./navigation.ts";
import { collections } from "../content.ts";
import { createHash } from "node:crypto";
import {
    copyFileSync,
    existsSync,
    mkdirSync,
    readdirSync,
    readFileSync,
    renameSync,
    rmSync,
    statSync,
    writeFileSync,
} from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
    headingsFor,
    parseFrontmatter,
    renderMarkdown,
    requireString,
    searchSectionsFor,
} from "./markdown.ts";
import { plainTextFor, searchTextFor, tokenEstimateFor } from "./text.ts";
import { writePageSources } from "./sources.ts";
import { withLock } from "./lock.ts";

/** A public documentation path: lowercase hyphenated segments ending in a Markdown file. */
const DOCUMENT_PATH_PATTERN = /^(?:[a-z0-9]+(?:-[a-z0-9]+)*\/)*[a-z0-9]+(?:-[a-z0-9]+)*\.md$/u;

/** A numbered documentation source segment: a two-digit order and its public name. */
const DOCUMENT_SEGMENT_PATTERN = /^(\d{2})-([a-z0-9]+(?:-[a-z0-9]+)*)$/u;

/** The repository root. */
const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

/** The site package directory. */
const siteDirectory = join(repositoryDirectory, "@platform/site");

/** The blog's source collection. */
const blogSource = collections.find((collection) => collection.route === "/blog/")?.sources[0];
if (blogSource == undefined) {
    throw new Error("missing blog collection source");
}

/** The blog post directory. */
const contentDirectory = join(repositoryDirectory, blogSource.directory);

/** The documentation source directories with absolute paths. */
const documentDirectories = collections
    .filter((collection) => collection.route.startsWith("/docs/"))
    .flatMap((collection) =>
        collection.sources.map((source) => ({
            ...source,
            directory: join(repositoryDirectory, source.directory),
        })),
    );

/** The generated TypeScript module directory. */
const generatedDirectory = join(siteDirectory, "src/view/content/generated");

/** The public asset directory. */
const publicDirectory = join(siteDirectory, "public");

/** The public rendered-content directory. */
const publicContentDirectory = join(publicDirectory, "_content");

/** The routes published by the previous run. */
const publishedPageFile = join(siteDirectory, ".generated/published-pages.json");

/** The public full-text search index. */
const publicSearchFile = join(publicDirectory, "search.json");

/** The generated blog post module. */
const generatedPostFile = join(generatedDirectory, "posts.ts");

/** The generated prerender route module. */
const generatedRouteFile = join(generatedDirectory, "prerender-routes.ts");

/** The generated rendered-body loader module. */
const generatedAssetFile = join(generatedDirectory, "assets.ts");

/** Whether this run checks the generated files instead of writing them. */
const isCheck = process.argv.includes("--check");

mkdirSync(generatedDirectory, { recursive: true });
await withLock(join(generatedDirectory, ".content-lock"), async () => {
    // render documentation, posts and the blog index
    const documentSources = readDocumentSources();
    const documents = renderDocuments(documentSources);
    const postSources = readPostSources();
    const posts = renderPosts(postSources);
    const blogIndex = renderBlogIndex(posts);
    buildNavigation(documents);
    appendChapterContents(documents);
    const pages = [...documents, ...posts, blogIndex];
    const searchEntries = searchEntriesFor(posts, documents);
    if (!isCheck) {
        await writePageSources(pages, publicDirectory);
        writePageContent(pages);
        removeObsoletePages(pages);
        writeGeneratedFile(publicSearchFile, `${JSON.stringify(searchEntries)}\n`);
    }
    generateDocumentMetadata(documents);
    const postSource = await formatSource(generatedPostFile, renderPostModule(posts, blogIndex));
    const routeSource = await formatSource(generatedRouteFile, renderRouteModule(posts, documents));
    const assetSource = await formatSource(generatedAssetFile, renderAssetModule(pages));

    // check or write the generated modules
    if (isCheck) {
        checkGeneratedFile(generatedPostFile, postSource);
        checkGeneratedFile(generatedRouteFile, routeSource);
        checkGeneratedFile(generatedAssetFile, assetSource);
    } else {
        mkdirSync(generatedDirectory, { recursive: true });
        writeGeneratedFile(generatedPostFile, postSource);
        writeGeneratedFile(generatedRouteFile, routeSource);
        writeGeneratedFile(generatedAssetFile, assetSource);
    }
});

/** List current server content independently of retained public content. */
function renderAssetModule(pages: RenderedPage[]): string {
    // collect current bodies and documentation metadata in route order
    const routes = new Set(
        pages.flatMap((page) => [
            contentRouteFor(page),
            ...(page.route.startsWith("/docs/") ? [documentMetadataRoute(page.route)] : []),
        ]),
    );
    const entries = [...routes]
        .toSorted()
        .map(
            (route) =>
                `    ${JSON.stringify(route)}: () => import(${JSON.stringify(
                    `../../../../public${route}?raw`,
                )}).then((module) => module.default),`,
        );

    return [
        "/** Current rendered bodies and documentation metadata. */",
        "export const assets: Record<string, () => Promise<string>> = {",
        ...entries,
        "};",
        "",
    ].join("\n");
}

/** Append immediate child chapters to each authored index. */
function appendChapterContents(documents: DocumentationPage[]) {
    // preserve the resolved navigation order
    for (const chapter of documents) {
        if (chapter.path !== "index.md" && !chapter.path.endsWith("/index.md")) {
            continue;
        }

        const children = documents.filter((page) => page.parent === chapter);
        if (children.length === 0) {
            continue;
        }

        // render the same generated links in each published representation
        const links = children
            .map((page) => {
                const title = page.title.replace(/[\\`*_[\]<>]/gu, "\\$&");
                const summary = page.lead ?? page.description;

                return `- [${title}](${page.route})${
                    summary && summary !== page.title ? ` — ${summary}` : ""
                }`;
            })
            .join("\n");
        const contents = `---\n\n${links}`;
        chapter.markdown = `${chapter.markdown.trimEnd()}\n\n${contents}\n`;
        chapter.entries = children.map((page) => {
            const summary = page.lead ?? page.description;

            return {
                title: page.title,
                href: page.route,
                ...(summary === page.title ? {} : { summary }),
            };
        });

        // generated indexes use directory presentation rather than article introductions
        chapter.kind = "catalog";
        delete chapter.lead;

        // keep search text and token counts consistent with the published body
        chapter.searchSections = searchSectionsFor(chapter.markdown);
        chapter.searchText = searchTextFor(chapter.markdown);
        chapter.tokens = tokenEstimateFor(plainTextFor(chapter.markdown));
    }
}

/** Write rendered page bodies and their content-addressed assets. */
function writePageContent(pages: RenderedPage[]) {
    mkdirSync(publicContentDirectory, { recursive: true });
    const assetRoutes = new Map<string, string>();

    // resolve every page against one deduplicated asset collection
    for (const page of pages) {
        const assets = Object.fromEntries(
            page.assets.map((asset) => {
                let route = assetRoutes.get(asset.path);

                // copy each source asset once under its content hash
                if (route == undefined) {
                    route = assetRouteFor(asset.path);
                    const file = join(publicDirectory, route.slice(1));
                    mkdirSync(dirname(file), { recursive: true });
                    if (!existsSync(file)) {
                        const temporaryFile = `${file}.${process.pid}.tmp`;
                        copyFileSync(asset.path, temporaryFile);
                        renameSync(temporaryFile, file);
                    }
                    assetRoutes.set(asset.path, route);
                }

                return [asset.placeholder, route];
            }),
        );
        const tree = resolveAssets(JSON.stringify(page.tree), assets);
        const file = join(publicDirectory, contentRouteFor(page).slice(1));
        mkdirSync(dirname(file), { recursive: true });
        writeGeneratedFile(file, tree);
    }
}

/** Return the content-addressed route an asset is published under. */
function assetRouteFor(path: string) {
    const digest = createHash("sha256").update(readFileSync(path)).digest("hex").slice(0, 16);

    return `/_content/assets/${digest}${extname(path)}`;
}

/** Generate or check the metadata for every documentation page. */
function generateDocumentMetadata(documents: DocumentationPage[]) {
    for (const document of documents) {
        const metadata = {
            contentRoute: contentRouteFor(document),
            description: document.description,
            lead: document.lead,
            entries: document.entries,
            markdownRoute: document.markdownRoute,
            kind: document.kind,
            navigation: document.navigation,
            route: document.route,
            tableOfContents: document.tableOfContents,
            textRoute: document.textRoute,
            title: document.title,
            tokens: document.tokens,
        };
        const file = join(publicDirectory, documentMetadataRoute(document.route).slice(1));
        const source = `${JSON.stringify(metadata)}\n`;
        if (isCheck) {
            checkGeneratedFile(file, source);
        } else {
            mkdirSync(dirname(file), { recursive: true });
            writeGeneratedFile(file, source);
        }
    }
}

/** Remove portable sources and metadata for pages that are no longer published. */
function removeObsoletePages(pages: RenderedPage[]) {
    // compare the routes of this run with those of the previous run
    const routes = pages.flatMap((page) => [
        page.markdownRoute,
        page.textRoute,
        ...(page.route.startsWith("/docs/") ? [documentMetadataRoute(page.route)] : []),
    ]);
    const previous: unknown = existsSync(publishedPageFile)
        ? JSON.parse(readFileSync(publishedPageFile, "utf8"))
        : [];
    if (!Array.isArray(previous)) {
        throw new Error(`invalid published page list: ${publishedPageFile}`);
    }
    const published: unknown[] = previous;
    const current = new Set(routes);

    // retain immutable bodies for readers that already loaded earlier metadata
    for (const route of published) {
        if (
            typeof route !== "string" ||
            !/^\/(?:docs|blog|_content\/docs)\//u.test(route) ||
            route.includes("..")
        ) {
            throw new Error(`invalid published page route: ${JSON.stringify(route)}`);
        }
        if (!current.has(route)) {
            rmSync(join(publicDirectory, route.slice(1)), { force: true });
        }
    }
    mkdirSync(dirname(publishedPageFile), { recursive: true });
    writeGeneratedFile(publishedPageFile, JSON.stringify(routes));
}

/** Return the static metadata route for one documentation page. */
function documentMetadataRoute(route: string) {
    return `/_content${route}index.json`;
}

/** Return the static rendered-body route for one content page. */
function contentRouteFor(page: Pick<RenderedPage, "tree" | "assets">) {
    const hash = createHash("sha256").update(JSON.stringify(page.tree));
    for (const asset of page.assets) {
        hash.update(readFileSync(asset.path));
    }

    return `/_content/tree/${hash.digest("hex")}.json`;
}

/** Replace content asset placeholders with their public routes. */
function resolveAssets(tree: string, assets: Record<string, string>) {
    let resolved = tree;

    // replace every placeholder emitted by the Markdown renderer
    for (const [placeholder, route] of Object.entries(assets)) {
        resolved = resolved.replaceAll(placeholder, route);
    }

    return resolved;
}

/** Read every documentation source in path order. */
function readDocumentSources() {
    const sources = documentDirectories
        .flatMap((collection) => {
            // require each documentation directory
            if (!existsSync(collection.directory)) {
                throw new Error(`missing documentation directory: ${collection.directory}`);
            }

            // map files into the documentation tree
            return markdownFiles(collection.directory).map((file) => {
                // read the title and description from the frontmatter
                const source = readFileSync(file, "utf8");
                const { markdown, metadata } = parseFrontmatter(source, file);
                requireString(metadata, "title", file);
                requireString(metadata, "description", file);

                // reject frontmatter ordering
                if (Object.hasOwn(metadata, "order")) {
                    throw new Error(`documentation order belongs in the source path: ${file}`);
                }

                // derive the public path and route from the numbered source path
                const sourcePath = relative(collection.directory, file).replaceAll("\\", "/");
                const { hierarchy, path: publicPath } = parseDocumentPath(sourcePath, file);
                const path =
                    collection.path === "" ? publicPath : `${collection.path}/${publicPath}`;
                const route = documentRoute(path);
                const headings = headingsFor(markdown);

                return {
                    description: metadata.description,
                    directory: collection.directory,
                    file,
                    headings,
                    hierarchy: [...collection.hierarchy, ...hierarchy],
                    lead: metadata.description,
                    markdownRoute: `/${join("docs", path).replaceAll("\\", "/")}`,
                    markdown,
                    path,
                    route,
                    textRoute: `/${join("docs", path.replace(/\.md$/u, ".txt")).replaceAll(
                        "\\",
                        "/",
                    )}`,
                    title: metadata.title,
                    tokens: tokenEstimateFor(plainTextFor(markdown)),
                };
            });
        })
        .toSorted(compareDocuments);

    validateDocuments(sources);

    return sources.map(({ hierarchy: _, ...source }, order) => ({
        ...source,
        order,
    }));
}

/** Parse a numbered source path into its public path and hierarchy. */
function parseDocumentPath(path: string, file: string) {
    // collect public names and order numbers per segment
    const segments = path.split("/");
    const names = [];
    const hierarchy = [];

    // strip the numeric prefix from each public path segment
    for (const [index, segment] of segments.entries()) {
        const isIndex = index === segments.length - 1 && segment === "index.md";
        if (isIndex) {
            names.push(segment);
            continue;
        }

        const extension = index === segments.length - 1 ? ".md" : "";
        const name = extension === "" ? segment : segment.slice(0, -extension.length);
        const match = DOCUMENT_SEGMENT_PATTERN.exec(name);
        if (match == null) {
            throw new Error(`documentation path lacks a numeric prefix: ${file}`);
        }

        hierarchy.push(Number(match[1]));
        names.push(`${match[2]}${extension}`);
    }

    return { hierarchy, path: names.join("/") };
}

/** Compare documentation sources by their numeric hierarchy. */
function compareDocuments(
    left: { hierarchy: number[]; path: string; route: string },
    right: { hierarchy: number[]; path: string; route: string },
) {
    // compare every shared level
    for (const [depth, level] of left.hierarchy.entries()) {
        const other = right.hierarchy[depth];
        if (other == undefined) {
            break;
        }
        const difference = level - other;
        if (difference !== 0) {
            return difference;
        }
    }

    const depthDifference = left.hierarchy.length - right.hierarchy.length;

    return depthDifference || left.route.localeCompare(right.route);
}

/** Validate the path, hierarchy, and title invariants of the manual. */
function validateDocuments(
    documents: {
        route: string;
        headings: ReturnType<typeof headingsFor>;
        file: string;
        path: string;
        title: string;
        hierarchy: number[];
    }[],
) {
    const hierarchies = new Set();
    const paths = new Set(documents.map((document) => document.path));

    for (const document of documents) {
        if (!DOCUMENT_PATH_PATTERN.test(document.path)) {
            throw new Error(`invalid documentation path: ${document.path}`);
        }

        const hierarchy = document.hierarchy.join(".");
        if (hierarchies.has(hierarchy)) {
            throw new Error(`duplicate documentation hierarchy: ${hierarchy}`);
        }
        hierarchies.add(hierarchy);

        const titles = document.headings.filter((heading) => heading.depth === 1);
        const [title] = titles;
        if (titles.length !== 1 || title?.text !== document.title) {
            throw new Error(`documentation title does not match its H1: ${document.path}`);
        }

        const segments = document.path.split("/");
        for (let depth = 1; depth < segments.length; depth += 1) {
            const index = `${segments.slice(0, depth).join("/")}/index.md`;
            if (!paths.has(index)) {
                throw new Error(`documentation directory is missing ${index}`);
            }
        }
    }
}

/** Recursively collect Markdown files below one directory. */
function markdownFiles(directory: string): string[] {
    return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        const path = join(directory, entry.name);

        if (entry.isDirectory()) {
            return markdownFiles(path);
        }

        return entry.isFile() && entry.name.endsWith(".md") ? [path] : [];
    });
}

/** Convert a documentation source path into its public route. */
function documentRoute(path: string) {
    const withoutExtension = path.slice(0, -3);
    const routePath =
        withoutExtension === "index"
            ? ""
            : withoutExtension.endsWith("/index")
              ? withoutExtension.slice(0, -6)
              : withoutExtension;

    return `/docs/${routePath === "" ? "" : `${routePath}/`}`;
}

/** Render documentation sources against the complete manual graph. */
function renderDocuments(sources: ReturnType<typeof readDocumentSources>): DocumentationPage[] {
    const sourceRoutes = new Map(
        sources.map((source) => [
            resolve(source.file),
            {
                headings: new Set(source.headings.map((heading) => heading.id)),
                route: source.route,
            },
        ]),
    );

    return sources.map((source) => {
        const context: MarkdownContext = {
            assets: [],
            documentDirectory: source.directory,
            kind: "document",
            markdownDirectory: dirname(source.file),
            ownHeadings: new Set(source.headings.map((heading) => heading.id)),
            route: source.route,
            slug: source.path,
            sourceRoutes,
        };

        return {
            ...source,
            assets: context.assets,
            tree: renderMarkdown(source.markdown, context),
            kind: "chapter",
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings.filter((heading) => heading.depth > 1),
        };
    });
}

/** Read blog posts, newest first. */
function readPostSources() {
    if (!existsSync(contentDirectory)) {
        return [];
    }

    // read post directories in path order
    const directories = readdirSync(contentDirectory)
        .filter((entry) => statSync(join(contentDirectory, entry)).isDirectory())
        .toSorted();
    const seen = new Set();

    return directories
        .map((directory) => {
            // apply the documentation numbering convention to blog source directories
            const postDirectory = join(contentDirectory, directory);
            const { path } = parseDocumentPath(`${directory}/index.md`, postDirectory);
            const slug = path.slice(0, -"/index.md".length);

            // refuse a repeated slug
            if (seen.has(slug)) {
                throw new Error(`duplicate blog slug: ${slug}`);
            }
            seen.add(slug);

            // require the post entrypoint
            const sourceFile = join(postDirectory, "index.md");
            if (!existsSync(sourceFile)) {
                throw new Error(`missing blog post entrypoint: ${sourceFile}`);
            }

            // require the post metadata
            const source = readFileSync(sourceFile, "utf8");
            const { markdown, metadata } = parseFrontmatter(source, sourceFile);
            requireString(metadata, "title", sourceFile);
            requireString(metadata, "subtitle", sourceFile);
            requireString(metadata, "date", sourceFile);
            requireString(metadata, "author", sourceFile);

            // record the post with its routes and headings
            const headings = headingsFor(markdown);

            return {
                ...metadata,
                file: sourceFile,
                headings,
                markdown,
                markdownRoute: `/blog/${slug}.md`,
                route: `/blog/${slug}/`,
                slug,
                textRoute: `/blog/${slug}.txt`,
                tokens: tokenEstimateFor(plainTextFor(markdown)),
            };
        })
        .toSorted(
            (left, right) =>
                right.date.localeCompare(left.date) || left.title.localeCompare(right.title),
        );
}

/** Render blog sources against the complete post graph. */
function renderPosts(sources: ReturnType<typeof readPostSources>) {
    const sourceRoutes = new Map(
        sources.map((source) => [
            resolve(source.file),
            {
                headings: new Set(source.headings.map((heading) => heading.id)),
                route: source.route,
            },
        ]),
    );

    return sources.map((source) => {
        const context: MarkdownContext = {
            assets: [],
            kind: "blog",
            markdownDirectory: dirname(source.file),
            ownHeadings: new Set(source.headings.map((heading) => heading.id)),
            route: source.route,
            slug: source.slug,
            sourceRoutes,
        };

        const tree = renderMarkdown(source.markdown, context);

        return {
            ...source,
            assets: context.assets,
            cover: coverFor(tree, context.assets),
            tree,
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings,
        };
    });
}

/** Return the first image of a rendered post at its published route, if it has one. */
function coverFor(tree: Root, assets: MarkdownContext["assets"]) {
    // read the first image or image figure's source and description
    const image = firstImageOf(tree);
    if (image === undefined) {
        return null;
    }

    // publish a collected asset under its content-addressed route
    const asset = assets.find((candidate) => candidate.placeholder === image.source);

    return { source: asset ? assetRouteFor(asset.path) : image.source, alt: image.alt };
}

/** Find the first image of a tree, an image element or an image figure, in document order. */
function firstImageOf(node: Root | ElementContent): { source: string; alt: string } | undefined {
    // read an image element or an image figure
    if (node.type === "element") {
        const properties = node.properties;
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

/** Publish the blog directory in the same portable formats as documentation indexes. */
function renderBlogIndex(posts: ReturnType<typeof renderPosts>): RenderedPage {
    const markdown =
        "# Blog\n\n" +
        posts
            .map(
                (post) => `## [${post.title}](${post.route})\n\n${post.subtitle}\n\n${post.date}\n`,
            )
            .join("\n");

    return {
        route: "/blog/",
        title: "Blog",
        file: contentDirectory,
        markdown,
        markdownRoute: "/blog/index.md",
        textRoute: "/blog/index.txt",
        tree: renderMarkdown(markdown, { assets: [], route: "/blog/", kind: "blog" }),
        assets: [],
        tokens: tokenEstimateFor(plainTextFor(markdown)),
        headings: headingsFor(markdown),
        tableOfContents: [],
        searchSections: searchSectionsFor(markdown),
        searchText: searchTextFor(markdown),
    };
}

/** Generate the blog metadata and rendered-body loader. */
function renderPostModule(posts: ReturnType<typeof renderPosts>, index: RenderedPage) {
    const records = posts.map((post) => renderPostRecord(post)).join(",\n");

    return `import { loadContent, type RenderedContent } from "../load";

/** One generated blog post record. */
export type Post = {
    /** The post author. */
    author: string;
    /** The static rendered HTML route. */
    contentRoute: string;
    /** The post's first figure image, shown on its directory entry. */
    cover: { source: string; alt: string } | null;
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

/** One rendered post body. */
export type PostContent = RenderedContent;

/** One rendered post heading. */
export type TableOfContentsEntry = {
    /** The heading depth. */
    depth: number;
    /** The heading fragment identifier. */
    id: string;
    /** The heading text. */
    text: string;
};

/** Portable formats for the blog directory. */
export const blogIndex = ${JSON.stringify({
        markdownRoute: index.markdownRoute,
        textRoute: index.textRoute,
        tokens: index.tokens,
    })};

/** The generated blog posts. */
export const posts = [
${records}
] as const satisfies readonly Post[];

/** Blog posts indexed by slug. */
export const postBySlug: ReadonlyMap<string, Post> = new Map(
    posts.map((post): [string, Post] => [post.slug, post]),
);

/** Load one rendered post body by slug. */
export async function loadPost(slug: string): Promise<PostContent | undefined> {
    const post = postBySlug.get(slug);

    return post == undefined ? undefined : loadContent(post.contentRoute);
}
`;
}

/** Generate one blog metadata record. */
function renderPostRecord(post: ReturnType<typeof renderPosts>[number]) {
    return `    {
        author: ${JSON.stringify(post.author)},
        contentRoute: ${JSON.stringify(contentRouteFor(post))},
        cover: ${JSON.stringify(post.cover)},
        date: ${JSON.stringify(post.date)},
        markdownRoute: ${JSON.stringify(post.markdownRoute)},
        route: ${JSON.stringify(post.route)},
        slug: ${JSON.stringify(post.slug)},
        subtitle: ${JSON.stringify(post.subtitle)},
        tableOfContents: ${JSON.stringify(post.tableOfContents)},
        textRoute: ${JSON.stringify(post.textRoute)},
        title: ${JSON.stringify(post.title)},
        tokens: ${post.tokens},
    }`;
}

/** Generate the complete prerender route list. */
function renderRouteModule(posts: RenderedPage[], documents: DocumentationPage[]) {
    const routes = [
        "/",
        "/blog/",
        ...posts.map((post) => post.route),
        ...documents.map((document) => document.route),
    ];

    return `/** The complete static browser route set. */\nexport const prerenderRoutes = ${JSON.stringify(
        routes,
        null,
        4,
    )} as const;\n`;
}

/** Return the complete full-text search index. */
function searchEntriesFor(posts: ReturnType<typeof renderPosts>, documents: DocumentationPage[]) {
    return [
        ...documents.flatMap((document) => [
            {
                context:
                    document.path === "index.md"
                        ? "docs"
                        : document.path.split("/").slice(0, -1).join(" / "),
                kind: "page",
                route: document.route,
                text: `${document.description} ${
                    document.searchSections.find((section) => section.depth === 1)?.text ?? ""
                }`,
                title: document.title,
            },
            ...document.searchSections
                .filter((section) => section.depth > 1)
                .map((section) => ({
                    context: document.title,
                    kind: "section",
                    route: `${document.route}#${section.id}`,
                    text: section.text,
                    title: section.title,
                })),
        ]),
        ...posts.flatMap((post) => [
            {
                context: `blog / ${post.date}`,
                kind: "page",
                route: post.route,
                text: `${post.subtitle} ${post.searchText}`,
                title: post.title,
            },
            ...post.searchSections
                .filter((section) => section.depth > 1)
                .map((section) => ({
                    context: post.title,
                    kind: "section",
                    route: `${post.route}#${section.id}`,
                    text: section.text,
                    title: section.title,
                })),
        ]),
    ];
}

/** Require one generated file to match its expected contents. */
function checkGeneratedFile(file: string, source: string) {
    if (!existsSync(file)) {
        throw new Error(`missing generated content file: ${file}`);
    }

    const current = readFileSync(file, "utf8");
    if (current !== source) {
        throw new Error("generated content is out of date, run `just @platform/site/generate`");
    }
}

/** Atomically replace changed generated content. */
function writeGeneratedFile(file: string, source: string) {
    if (existsSync(file) && readFileSync(file, "utf8") === source) {
        return;
    }

    // write a temporary file and rename it into place
    const temporaryFile = `${file}.${process.pid}.tmp`;
    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}
