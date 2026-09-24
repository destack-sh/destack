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
import {
    moduleCatalogRoute,
    parseReadme,
    readLibraryReference,
    renderPackageDocuments,
} from "./reference.ts";
import { readLintReference, renderLintDocuments, ruleCatalogRoute } from "./lint.ts";
import { withLock } from "./lock.ts";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const siteDirectory = join(repositoryDirectory, "platform/site");
const contentDirectory = join(
    repositoryDirectory,
    collections.find((collection) => collection.route === "/blog/")!.sources[0].directory,
);
const documentDirectories = collections
    .filter((collection) => collection.route.startsWith("/docs/"))
    .flatMap((collection) =>
        collection.sources.map((source) => ({
            ...source,
            directory: join(repositoryDirectory, source.directory),
        })),
    );
const generatedDirectory = join(siteDirectory, "src/generated");
const publicDirectory = join(siteDirectory, "public");
const publicContentDirectory = join(publicDirectory, "_content");
const publishedPageFile = join(siteDirectory, ".generated/published-pages.json");
const publicSearchFile = join(publicDirectory, "search.json");
const generatedPostFile = join(generatedDirectory, "posts.ts");
const generatedRouteFile = join(generatedDirectory, "prerender-routes.ts");
const generatedAssetFile = join(generatedDirectory, "assets.ts");
const isCheck = process.argv.includes("--check");
const hasReference = process.argv.includes("--reference");
const documentPathPattern = /^(?:[a-z0-9]+(?:-[a-z0-9]+)*\/)*[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;
const documentSegmentPattern = /^(\d{2})-([a-z0-9]+(?:-[a-z0-9]+)*)$/;

// refresh compiler references when explicitly requested
if (hasReference) {
    await import("./generate-reference.ts");
}

mkdirSync(generatedDirectory, { recursive: true });
await withLock(join(generatedDirectory, ".content-lock"), async () => {
    const documentSources = readDocumentSources();
    const renderedDocuments = renderDocuments(documentSources);
    let documents = renderedDocuments;
    const references: DocumentationPage[] = [];
    const modules = new Set<string>();

    // include compiler references only when explicitly requested
    if (hasReference) {
        const libraryReference = readLibraryReference();
        const libraryIndex = renderedDocuments.find(
            (document) => document.route === moduleCatalogRoute,
        );
        if (libraryIndex == undefined) {
            throw new Error("missing standard library documentation index");
        }
        const library = renderPackageDocuments(libraryReference, libraryIndex, {
            directory: join(repositoryDirectory, "language/library"),
            referenceFile: join(siteDirectory, ".generated/library-reference.json"),
            sourceUrl: "https://github.com/destack-sh/destack/blob/main/language/library/",
        });
        const lintIndex = renderedDocuments.find((document) => document.route === ruleCatalogRoute);
        if (lintIndex == undefined) {
            throw new Error("missing lint rule documentation index");
        }
        const lintReference = readLintReference();
        const lint = renderLintDocuments(lintReference, lintIndex);
        documents = [
            ...renderedDocuments.filter(
                (document) => document !== libraryIndex && document !== lintIndex,
            ),
            ...library.documents,
            ...lint.documents,
        ];
        references.push(...library.items, ...lint.items);
        for (const page of library.documents) {
            if (page.route !== libraryIndex.route) modules.add(page.route);
        }
    }

    documents.sort(
        (left, right) => left.order - right.order || left.route.localeCompare(right.route),
    );
    const postSources = readPostSources();
    const posts = renderPosts(postSources);
    const blogIndex = renderBlogIndex(posts);
    // resolve navigation once for authored chapters and generated references
    for (const page of [...documents, ...references]) {
        page.kind = page.searchKind ?? (modules.has(page.route) ? "module" : "chapter");
    }
    buildNavigation(documents, references);
    appendChapterContents(documents);
    applyWarnings([...documents, ...references]);
    const pages = [...documents, ...references, ...posts, blogIndex];
    const searchEntries = searchEntriesFor(posts, documents, references);
    if (!isCheck) {
        await writePageSources(pages, publicDirectory);
        writePageContent(pages);
        removeObsoletePages(pages);
        writeGeneratedFile(publicSearchFile, `${JSON.stringify(searchEntries)}\n`);
    }
    generateDocumentMetadata([...documents, ...references]);
    const postSource = await formatSource(generatedPostFile, renderPostModule(posts, blogIndex));
    const routeSource = await formatSource(
        generatedRouteFile,
        renderRouteModule(posts, documents, references),
    );
    const assetSource = await formatSource(generatedAssetFile, renderAssetModule(pages));

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
        .sort()
        .map(
            (route) =>
                `    ${JSON.stringify(route)}: () => import(${JSON.stringify(
                    `../../public${route}?raw`,
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

/// Add the nearest inherited warning to HTML and portable page formats.
function applyWarnings(pages: DocumentationPage[]) {
    for (const page of pages) {
        const warning = [page, ...(page.ancestors ?? []).toReversed()].find(
            (ancestor) => ancestor.warning !== undefined,
        )?.warning;
        if (warning === undefined) continue;

        // reuse GitHub alert rendering and keep the notice immediately below the title
        const markdown = `> [!WARNING]\n${warning
            .split("\n")
            .map((line) => `> ${line}`)
            .join("\n")}\n`;
        const html = renderMarkdown(markdown, { assets: [], route: page.route });
        page.html = /<h1\b[^>]*>[\s\S]*?<\/h1>/.test(page.html)
            ? page.html.replace(/<h1\b[^>]*>[\s\S]*?<\/h1>/, (title) => `${title}\n${html}`)
            : html + page.html;
        page.markdown = page.markdown.replace(
            /^(# [^\n]+)(?:\n|$)/m,
            (_match, title) => `${title}\n\n${markdown}\n`,
        );
        page.tokens = tokenEstimateFor(plainTextFor(page.markdown));
    }
}

/// Append immediate child chapters to each authored index.
function appendChapterContents(documents: DocumentationPage[]) {
    const chapters = documents.filter((page) => page.kind === "chapter");

    // preserve the resolved navigation order and specialized catalogs
    for (const chapter of chapters) {
        if (
            (chapter.path !== "index.md" && !chapter.path.endsWith("/index.md")) ||
            chapter.route === moduleCatalogRoute ||
            chapter.route === ruleCatalogRoute
        ) {
            continue;
        }

        const children = documents.filter(
            (page) =>
                (page.kind === "chapter" || page.kind === "catalog") &&
                page.parent === chapter &&
                (page.collection?.isListed !== false || page.collection === chapter.collection),
        );
        if (children.length === 0) {
            continue;
        }

        // render the same generated links in each published representation
        const links = children
            .map((page) => {
                const title = page.title.replace(/[\\`*_[\]<>]/g, "\\$&");

                const summary = page.lead ?? page.description;

                return `- [${title}](${page.route})${
                    summary && summary !== page.title ? ` — ${summary}` : ""
                }`;
            })
            .join("\n");
        const contents = `---\n\n${links}`;
        chapter.markdown = `${chapter.markdown.trimEnd()}\n\n${contents}\n`;
        chapter.entries = children.map((page) => ({
            title: page.title,
            href: page.route,
            summary:
                (page.lead ?? page.description) !== page.title
                    ? (page.lead ?? page.description)
                    : undefined,
        }));

        // generated indexes use directory presentation rather than article introductions
        chapter.kind = "catalog";
        chapter.lead = undefined;

        // keep search text and token counts consistent with the published body
        chapter.searchSections = searchSectionsFor(chapter.markdown);
        chapter.searchText = searchTextFor(chapter.markdown);
        chapter.tokens = tokenEstimateFor(plainTextFor(chapter.markdown));
    }
}

/// Write rendered page bodies and their content-addressed assets.
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
        const html = resolveAssets(page.html, assets);
        const file = join(publicDirectory, contentRouteFor(page).slice(1));
        mkdirSync(dirname(file), { recursive: true });
        writeGeneratedFile(file, html);
    }
}

/// Return the content-addressed route an asset is published under.
function assetRouteFor(path: string) {
    const digest = createHash("sha256").update(readFileSync(path)).digest("hex").slice(0, 16);

    return `/_content/assets/${digest}${extname(path)}`;
}

/// Generate or check the metadata for every documentation page.
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

/// Remove portable sources and metadata for pages that are no longer published.
function removeObsoletePages(pages: RenderedPage[]) {
    const routes = pages.flatMap((page) => [
        page.markdownRoute,
        page.textRoute,
        ...(page.route.startsWith("/docs/") ? [documentMetadataRoute(page.route)] : []),
    ]);
    const previous = existsSync(publishedPageFile)
        ? JSON.parse(readFileSync(publishedPageFile, "utf8"))
        : [];
    const current = new Set(routes);

    // retain immutable bodies for readers that already loaded earlier metadata
    for (const route of previous) {
        if (!/^\/(?:docs|blog|_content\/docs)\//.test(route) || route.includes("..")) {
            throw new Error(`invalid published page route: ${route}`);
        }
        if (!current.has(route)) {
            rmSync(join(publicDirectory, route.slice(1)), { force: true });
        }
    }
    mkdirSync(dirname(publishedPageFile), { recursive: true });
    writeGeneratedFile(publishedPageFile, JSON.stringify(routes));
}

/// Return the static metadata route for one documentation page.
function documentMetadataRoute(route: string) {
    return `/_content${route}index.json`;
}

/// Return the static rendered-body route for one content page.
function contentRouteFor(page: Pick<RenderedPage, "html" | "assets">) {
    const hash = createHash("sha256").update(page.html);
    for (const asset of page.assets) {
        hash.update(readFileSync(asset.path));
    }

    return `/_content/html/${hash.digest("hex")}.html`;
}

/// Replace content asset placeholders with their public routes.
function resolveAssets(html: string, assets: Record<string, string>) {
    let resolved = html;

    // replace every placeholder emitted by the Markdown renderer
    for (const [placeholder, route] of Object.entries(assets)) {
        resolved = resolved.replaceAll(placeholder, route);
    }

    return resolved;
}

/// Read every documentation source in path order.
function readDocumentSources() {
    const sources = documentDirectories
        .flatMap((collection) => {
            // require each documentation directory
            if (!existsSync(collection.directory)) {
                throw new Error(`missing documentation directory: ${collection.directory}`);
            }

            // map files into the documentation tree
            const files = collection.readme
                ? [join(collection.directory, "README.md")]
                : markdownFiles(collection.directory);
            return files.map((file) => {
                const source = readFileSync(file, "utf8");
                const { markdown, metadata } = collection.readme
                    ? parseReadme(source, file)
                    : parseFrontmatter(source, file);
                requireString(metadata, "title", file);
                requireString(metadata, "description", file);
                if (metadata.warning !== undefined) requireString(metadata, "warning", file);

                // reject frontmatter ordering
                if (Object.hasOwn(metadata, "order")) {
                    throw new Error(`documentation order belongs in the source path: ${file}`);
                }

                const sourcePath = relative(collection.directory, file).replaceAll("\\", "/");
                const { hierarchy, path: publicPath } = parseDocumentPath(
                    collection.readme ? "index.md" : sourcePath,
                    file,
                );
                const path =
                    collection.path === "" ? publicPath : `${collection.path}/${publicPath}`;
                const route = documentRoute(path);
                const headings = headingsFor(markdown);

                return {
                    isPackage: collection.readme === true,
                    warning: metadata.warning as string | undefined,
                    description: metadata.description,
                    directory: collection.directory,
                    file,
                    headings,
                    hierarchy: [...collection.hierarchy, ...hierarchy],
                    lead: collection.readme ? undefined : metadata.description,
                    markdownRoute: `/${join("docs", path).replaceAll("\\", "/")}`,
                    markdown,
                    path,
                    route,
                    textRoute: `/${join("docs", path.replace(/\.md$/, ".txt")).replaceAll(
                        "\\",
                        "/",
                    )}`,
                    title: metadata.title,
                    tokens: tokenEstimateFor(plainTextFor(markdown)),
                };
            });
        })
        .sort(compareDocuments);

    validateDocuments(sources);

    return sources.map(({ hierarchy: _, ...source }, order) => ({
        ...source,
        order,
    }));
}

/// Parse a numbered source path into its public path and hierarchy.
function parseDocumentPath(path: string, file: string) {
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
        const match = documentSegmentPattern.exec(name);
        if (match == null) {
            throw new Error(`documentation path lacks a numeric prefix: ${file}`);
        }

        hierarchy.push(Number(match[1]));
        names.push(`${match[2]}${extension}`);
    }

    return { hierarchy, path: names.join("/") };
}

/// Compare documentation sources by their numeric hierarchy.
function compareDocuments(
    left: { hierarchy: number[]; path: string; route: string },
    right: { hierarchy: number[]; path: string; route: string },
) {
    const depthCount = Math.min(left.hierarchy.length, right.hierarchy.length);

    // compare every shared level
    for (let depth = 0; depth < depthCount; depth += 1) {
        const difference = left.hierarchy[depth] - right.hierarchy[depth];
        if (difference !== 0) {
            return difference;
        }
    }

    const depthDifference = left.hierarchy.length - right.hierarchy.length;

    return depthDifference || left.route.localeCompare(right.route);
}

/// Validate the path, hierarchy, and title invariants of the manual.
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
        if (!documentPathPattern.test(document.path)) {
            throw new Error(`invalid documentation path: ${document.path}`);
        }

        const hierarchy = document.hierarchy.join(".");
        if (hierarchies.has(hierarchy)) {
            throw new Error(`duplicate documentation hierarchy: ${hierarchy}`);
        }
        hierarchies.add(hierarchy);

        const titles = document.headings.filter((heading) => heading.depth === 1);
        if (titles.length !== 1 || titles[0].text !== document.title) {
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

/// Recursively collect Markdown files below one directory.
function markdownFiles(directory: string): string[] {
    return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        const path = join(directory, entry.name);

        if (entry.isDirectory()) {
            return markdownFiles(path);
        }

        return entry.isFile() && entry.name.endsWith(".md") ? [path] : [];
    });
}

/// Convert a documentation source path into its public route.
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

/// Render documentation sources against the complete manual graph.
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
            html: source.isPackage ? "" : renderMarkdown(source.markdown, context),
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings.filter((heading) => heading.depth > 1),
        };
    });
}

/// Read blog posts, newest first.
function readPostSources() {
    if (!existsSync(contentDirectory)) {
        return [];
    }

    const directories = readdirSync(contentDirectory)
        .filter((entry) => statSync(join(contentDirectory, entry)).isDirectory())
        .sort();
    const seen = new Set();

    return directories.map((directory) => {
        // apply the documentation numbering convention to blog source directories
        const postDirectory = join(contentDirectory, directory);
        const { path } = parseDocumentPath(`${directory}/index.md`, postDirectory);
        const slug = path.slice(0, -"/index.md".length);

        if (seen.has(slug)) {
            throw new Error(`duplicate blog slug: ${slug}`);
        }
        seen.add(slug);

        const sourceFile = join(postDirectory, "index.md");
        if (!existsSync(sourceFile)) {
            throw new Error(`missing blog post entrypoint: ${sourceFile}`);
        }

        const source = readFileSync(sourceFile, "utf8");
        const { markdown, metadata } = parseFrontmatter(source, sourceFile);
        requireString(metadata, "title", sourceFile);
        requireString(metadata, "subtitle", sourceFile);
        requireString(metadata, "date", sourceFile);
        requireString(metadata, "author", sourceFile);

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
    }).sort(
        (left, right) =>
            right.date.localeCompare(left.date) || left.title.localeCompare(right.title),
    );
}

/// Render blog sources against the complete post graph.
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

        const html = renderMarkdown(source.markdown, context);

        return {
            ...source,
            assets: context.assets,
            cover: coverFor(html, context.assets),
            html,
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings,
        };
    });
}

/// Return the first image of a rendered post at its published route, if it has one.
function coverFor(html: string, assets: MarkdownContext["assets"]) {
    const image = /<img\b[^>]*>/.exec(html)?.[0];
    const src = image && /\ssrc="([^"]+)"/.exec(image)?.[1];
    if (!src) {
        return null;
    }
    const asset = assets.find((asset) => asset.placeholder === src);

    return {
        source: asset ? assetRouteFor(asset.path) : src,
        alt: /\salt="([^"]*)"/.exec(image)?.[1] ?? "",
    };
}

/// Publish the blog directory in the same portable formats as documentation indexes.
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
        html: renderMarkdown(markdown, { assets: [], route: "/blog/", kind: "blog" }),
        assets: [],
        tokens: tokenEstimateFor(plainTextFor(markdown)),
        headings: headingsFor(markdown),
        tableOfContents: [],
        searchSections: searchSectionsFor(markdown),
        searchText: searchTextFor(markdown),
    };
}

/// Generate the blog metadata and rendered-body loader.
function renderPostModule(posts: ReturnType<typeof renderPosts>, index: RenderedPage) {
    const records = posts.map((post) => renderPostRecord(post)).join(",\n");

    return `import { loadContent, type RenderedContent } from "../content/load";

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

/// Generate one blog metadata record.
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

/// Generate the complete prerender route list.
function renderRouteModule(
    posts: RenderedPage[],
    documents: DocumentationPage[],
    references: DocumentationPage[],
) {
    const routes = [
        "/",
        "/blog/",
        ...posts.map((post) => post.route),
        ...documents.map((document) => document.route),
        ...references.map((reference) => reference.route),
    ];

    return `/** The complete static browser route set. */\nexport const prerenderRoutes = ${JSON.stringify(
        routes,
        null,
        4,
    )} as const;\n`;
}

/// Return the complete full-text search index.
function searchEntriesFor(
    posts: ReturnType<typeof renderPosts>,
    documents: DocumentationPage[],
    references: DocumentationPage[],
) {
    return [
        ...documents
            .filter((document) => document.collection?.isListed !== false)
            .flatMap((document) => [
                {
                    context:
                        document.path === "index.md"
                            ? "docs"
                            : document.path.split("/").slice(0, -1).join(" / "),
                    kind: document.kind === "module" ? "module" : "page",
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
        ...references
            .filter((reference) => reference.collection?.isListed !== false)
            .map((reference) => ({
                context: reference.searchContext,
                kind: reference.searchKind,
                route: reference.route,
                text: reference.searchSections[0].text,
                title: reference.title,
            })),
    ];
}

/// Require one generated file to match its expected contents.
function checkGeneratedFile(file: string, source: string) {
    if (!existsSync(file)) {
        throw new Error(`missing generated content file: ${file}`);
    }

    const current = readFileSync(file, "utf8");
    if (current !== source) {
        throw new Error("generated content is out of date, run `just platform/site/generate`");
    }
}

/// Atomically replace changed generated content.
function writeGeneratedFile(file: string, source: string) {
    if (existsSync(file) && readFileSync(file, "utf8") === source) return;

    const temporaryFile = `${file}.${process.pid}.tmp`;

    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}
