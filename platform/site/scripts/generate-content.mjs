import { createHash } from "node:crypto";
import {
    copyFileSync,
    existsSync,
    mkdirSync,
    readFileSync,
    readdirSync,
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
} from "./markdown.mjs";
import { plainTextFor, searchTextFor, tokenEstimateFor } from "./text.mjs";
import { writePageSources } from "./sources.mjs";
import { highlightCode } from "./highlight.mjs";
import { readLibraryReference, renderLibraryDocuments } from "./reference.mjs";
import { homeExamples } from "../src/content/home.ts";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const siteDirectory = join(repositoryDirectory, "platform/site");
const contentDirectory = join(siteDirectory, "src/content/blog");
const documentDirectory = join(repositoryDirectory, "docs");
const generatedDirectory = join(siteDirectory, "src/generated");
const publicDirectory = join(siteDirectory, "public");
const publicContentDirectory = join(publicDirectory, "_content");
const publicSearchFile = join(publicDirectory, "search.json");
const generatedDocumentDirectory = join(generatedDirectory, "document");
const generatedPostDirectory = join(generatedDirectory, "post");
const generatedDocumentFile = join(generatedDirectory, "documents.ts");
const generatedHomeHighlightFile = join(generatedDirectory, "home-highlights.ts");
const generatedPostFile = join(generatedDirectory, "posts.ts");
const generatedRouteFile = join(generatedDirectory, "prerender-routes.ts");
const generatedSearchFile = join(generatedDirectory, "search.ts");
const isCheck = process.argv.includes("--check");
const documentPathPattern = /^(?:[a-z0-9]+(?:-[a-z0-9]+)*\/)*[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;

const documentSources = readDocumentSources();
const libraryReference = readLibraryReference();
const library = renderLibraryDocuments(libraryReference);
const documents = [
    ...renderDocuments(documentSources),
    ...library.documents,
].sort((left, right) => left.order - right.order || left.route.localeCompare(right.route));
const postSources = readPostSources();
const posts = renderPosts(postSources);
const pages = [...documents, ...library.items, ...posts];
const searchEntries = searchEntriesFor(posts, documents, library.items);
if (!isCheck) {
    await writePageSources(pages, publicDirectory);
    writePageContent(pages);
    writeLibraryItemMetadata(library.items);
    writeGeneratedFile(publicSearchFile, `${JSON.stringify(searchEntries)}\n`);
}
const documentSource = renderDocumentModule(documents);
const homeHighlightSource = renderHomeHighlightModule();
const postSource = renderPostModule(posts);
const routeSource = renderRouteModule(posts, documents, library.items);

if (isCheck) {
    checkGeneratedFile(generatedDocumentFile, documentSource);
    checkGeneratedFile(generatedHomeHighlightFile, homeHighlightSource);
    checkGeneratedFile(generatedPostFile, postSource);
    checkGeneratedFile(generatedRouteFile, routeSource);
    checkMissingGeneratedPath(generatedDocumentDirectory);
    checkMissingGeneratedPath(generatedPostDirectory);
    checkMissingGeneratedPath(generatedSearchFile);
} else {
    mkdirSync(generatedDirectory, { recursive: true });
    writeGeneratedFile(generatedDocumentFile, documentSource);
    writeGeneratedFile(generatedHomeHighlightFile, homeHighlightSource);
    writeGeneratedFile(generatedPostFile, postSource);
    writeGeneratedFile(generatedRouteFile, routeSource);
    rmSync(generatedDocumentDirectory, { force: true, recursive: true });
    rmSync(generatedPostDirectory, { force: true, recursive: true });
    rmSync(generatedSearchFile, { force: true });
}

/// Write rendered page bodies and their content-addressed assets.
function writePageContent(pages) {
    rmSync(publicContentDirectory, { force: true, recursive: true });
    mkdirSync(publicContentDirectory, { recursive: true });
    const assetRoutes = new Map();

    // resolve every page against one deduplicated asset collection
    for (const page of pages) {
        const assets = Object.fromEntries(page.assets.map((asset) => {
            let route = assetRoutes.get(asset.path);

            // copy each source asset once under its content hash
            if (route == undefined) {
                const digest = createHash("sha256")
                    .update(readFileSync(asset.path))
                    .digest("hex")
                    .slice(0, 16);
                route = `/_content/assets/${digest}${extname(asset.path)}`;
                const file = join(publicDirectory, route.slice(1));
                mkdirSync(dirname(file), { recursive: true });
                copyFileSync(asset.path, file);
                assetRoutes.set(asset.path, route);
            }

            return [asset.placeholder, route];
        }));
        const html = resolveAssets(page.html, assets);
        const file = join(publicDirectory, contentRouteFor(page).slice(1));
        mkdirSync(dirname(file), { recursive: true });
        writeFileSync(file, html);
    }
}

/// Write independently loaded metadata for every generated library item.
function writeLibraryItemMetadata(items) {
    for (const item of items) {
        const metadata = {
            contentRoute: contentRouteFor(item),
            description: item.description,
            lead: item.lead,
            markdownRoute: item.markdownRoute,
            moduleRoute: item.moduleRoute,
            moduleTitle: item.moduleTitle,
            order: item.order,
            path: item.path,
            route: item.route,
            tableOfContents: item.tableOfContents,
            textRoute: item.textRoute,
            title: item.title,
            tokens: item.tokens,
        };
        const file = join(publicDirectory, libraryItemMetadataRoute(item.route).slice(1));
        mkdirSync(dirname(file), { recursive: true });
        writeFileSync(file, `${JSON.stringify(metadata)}\n`);
    }
}

/// Return the static metadata route for one generated library item.
function libraryItemMetadataRoute(route) {
    return `/_content${route}index.json`;
}

/// Return the static rendered-body route for one content page.
function contentRouteFor(page) {
    return `/_content${page.route}index.html`;
}

/// Replace content asset placeholders with their public routes.
function resolveAssets(html, assets) {
    let resolved = html;

    // replace every placeholder emitted by the Markdown renderer
    for (const [placeholder, route] of Object.entries(assets)) {
        resolved = resolved.replaceAll(placeholder, route);
    }

    return resolved;
}

/// Generate parser-backed syntax spans for every homepage technical listing.
function renderHomeHighlightModule() {
    const entries = homeExamples.map((example) => {
        const editors = example.editors.map((listing) =>
            highlightCode(listing.text, listing.language).split("\n")
        );
        const output = example.output === undefined
            ? null
            : highlightCode(example.output.text, example.output.language).split("\n");
        const highlighted = { editors, output };

        return `    ${JSON.stringify(example.action)}: ${JSON.stringify(highlighted)},`;
    }).join("\n");

    return `export const homeHighlights = {
${entries}
} as const;
`;
}

/// Read and order every documentation source.
function readDocumentSources() {
    if (!existsSync(documentDirectory)) {
        return [];
    }

    const sources = markdownFiles(documentDirectory)
        .map((file) => {
            const source = readFileSync(file, "utf8");
            const { markdown, metadata } = parseFrontmatter(source, file);
            requireString(metadata, "title", file);
            requireString(metadata, "description", file);

            const order = Number(metadata.order);
            if (!Number.isSafeInteger(order) || order < 0) {
                throw new Error(`invalid order in ${file}`);
            }

            const path = relative(documentDirectory, file).replaceAll("\\", "/");
            const route = documentRoute(path);
            const headings = headingsFor(markdown);

            return {
                description: metadata.description,
                file,
                headings,
                lead: metadata.description,
                markdownRoute: `/${join("docs", path).replaceAll("\\", "/")}`,
                markdown,
                order,
                path,
                route,
                textRoute: `/${join("docs", path.replace(/\.md$/, ".txt")).replaceAll("\\", "/")}`,
                title: metadata.title,
                tokens: tokenEstimateFor(plainTextFor(markdown)),
            };
        })
        .sort((left, right) => left.order - right.order || left.route.localeCompare(right.route));

    validateDocuments(sources);

    return sources;
}

/// Validate the path, order, and title invariants of the manual.
function validateDocuments(documents) {
    const orders = new Set();
    const paths = new Set(documents.map((document) => document.path));

    for (const document of documents) {
        if (!documentPathPattern.test(document.path)) {
            throw new Error(`invalid documentation path: ${document.path}`);
        }

        if (orders.has(document.order)) {
            throw new Error(`duplicate documentation order: ${document.order}`);
        }
        orders.add(document.order);

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
function markdownFiles(directory) {
    return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        const path = join(directory, entry.name);

        if (entry.isDirectory()) {
            return markdownFiles(path);
        }

        return entry.isFile() && entry.name.endsWith(".md") ? [path] : [];
    });
}

/// Convert a documentation source path into its public route.
function documentRoute(path) {
    const withoutExtension = path.slice(0, -3);
    const routePath = withoutExtension === "index"
        ? ""
        : withoutExtension.endsWith("/index")
          ? withoutExtension.slice(0, -6)
          : withoutExtension;

    return `/docs/${routePath === "" ? "" : `${routePath}/`}`;
}

/// Render documentation sources against the complete manual graph.
function renderDocuments(sources) {
    const sourceRoutes = new Map(sources.map((source) => [resolve(source.file), {
        headings: new Set(source.headings.map((heading) => heading.id)),
        route: source.route,
    }]));

    return sources.map((source) => {
        const context = {
            assets: [],
            documentDirectory,
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
            html: renderMarkdown(source.markdown, context),
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings.filter((heading) => heading.depth > 1),
        };
    });
}

/// Read the checked standard library package reference.
function readPostSources() {
    if (!existsSync(contentDirectory)) {
        return [];
    }

    const slugs = readdirSync(contentDirectory)
        .filter((entry) => statSync(join(contentDirectory, entry)).isDirectory())
        .sort();
    const seen = new Set();

    return slugs.map((slug) => {
        if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(slug)) {
            throw new Error(`invalid blog slug: ${slug}`);
        }

        if (seen.has(slug)) {
            throw new Error(`duplicate blog slug: ${slug}`);
        }
        seen.add(slug);

        const postDirectory = join(contentDirectory, slug);
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
    });
}

/// Render blog sources against the complete post graph.
function renderPosts(sources) {
    const sourceRoutes = new Map(sources.map((source) => [resolve(source.file), {
        headings: new Set(source.headings.map((heading) => heading.id)),
        route: source.route,
    }]));

    return sources.map((source) => {
        const context = {
            assets: [],
            kind: "blog",
            markdownDirectory: dirname(source.file),
            ownHeadings: new Set(source.headings.map((heading) => heading.id)),
            route: source.route,
            slug: source.slug,
            sourceRoutes,
        };

        return {
            ...source,
            assets: context.assets,
            html: renderMarkdown(source.markdown, context),
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings,
        };
    });
}

/// Generate the documentation metadata and rendered-body loader.
function renderDocumentModule(documents) {
    const records = documents.map((document) => `    {
        contentRoute: ${JSON.stringify(contentRouteFor(document))},
        description: ${JSON.stringify(document.description)},
        lead: ${JSON.stringify(document.lead)},
        markdownRoute: ${JSON.stringify(document.markdownRoute)},
        order: ${document.order},
        path: ${JSON.stringify(document.path)},
        route: ${JSON.stringify(document.route)},
        tableOfContents: ${JSON.stringify(document.tableOfContents)},
        textRoute: ${JSON.stringify(document.textRoute)},
        title: ${JSON.stringify(document.title)},
        tokens: ${document.tokens},
    }`).join(",\n");

    return `import { loadContent, type RenderedContent } from "../content/load";

export type Document = {
    /// The static rendered HTML route.
    contentRoute: string;
    /// The concise chapter description.
    description: string;
    /// The optional introductory sentence.
    lead?: string;
    /// The authored Markdown route.
    markdownRoute: string;
    /// The containing generated module route.
    moduleRoute?: string;
    /// The containing generated module title.
    moduleTitle?: string;
    /// The manual sort order.
    order: number;
    /// The repository-relative authored path.
    path: string;
    /// The canonical browser route.
    route: string;
    /// The rendered heading tree.
    tableOfContents: readonly TableOfContentsEntry[];
    /// The plain text route.
    textRoute: string;
    /// The chapter title.
    title: string;
    /// The approximate token count.
    tokens: number;
};

/// One rendered document body.
export type DocumentContent = RenderedContent;

/// One rendered document heading.
export type TableOfContentsEntry = {
    /// The heading depth.
    depth: number;
    /// The heading fragment identifier.
    id: string;
    /// The heading text.
    text: string;
};

/// The generated manual chapters.
export const documents = [
${records}
] as const satisfies readonly Document[];

/// Manual chapters indexed by canonical route.
export const documentByRoute: ReadonlyMap<string, Document> = new Map(
    documents.map((document): [string, Document] => [document.route, document]),
);

/// Load one rendered document body by canonical route.
export async function loadDocument(route: string): Promise<DocumentContent | undefined> {
    const document = documentByRoute.get(route);

    return document == undefined ? undefined : loadContent(document.contentRoute);
}
`;
}

/// Generate the blog metadata and rendered-body loader.
function renderPostModule(posts) {
    const records = posts.map((post) => renderPostRecord(post)).join(",\n");

    return `import { loadContent, type RenderedContent } from "../content/load";

export type Post = {
    /// The post author.
    author: string;
    /// The static rendered HTML route.
    contentRoute: string;
    /// The publication date.
    date: string;
    /// The authored Markdown route.
    markdownRoute: string;
    /// The canonical browser route.
    route: string;
    /// The canonical post slug.
    slug: string;
    /// The post subtitle.
    subtitle: string;
    /// The rendered heading tree.
    tableOfContents: readonly TableOfContentsEntry[];
    /// The plain text route.
    textRoute: string;
    /// The post title.
    title: string;
    /// The approximate token count.
    tokens: number;
};

/// One rendered post body.
export type PostContent = RenderedContent;

/// One rendered post heading.
export type TableOfContentsEntry = {
    /// The heading depth.
    depth: number;
    /// The heading fragment identifier.
    id: string;
    /// The heading text.
    text: string;
};

/// The generated blog posts.
export const posts = [
${records}
] as const satisfies readonly Post[];

/// Blog posts indexed by slug.
export const postBySlug: ReadonlyMap<string, Post> = new Map(
    posts.map((post): [string, Post] => [post.slug, post]),
);

/// Load one rendered post body by slug.
export async function loadPost(slug: string): Promise<PostContent | undefined> {
    const post = postBySlug.get(slug);

    return post == undefined ? undefined : loadContent(post.contentRoute);
}
`;
}

/// Generate one blog metadata record.
function renderPostRecord(post) {
    return `    {
        author: ${JSON.stringify(post.author)},
        contentRoute: ${JSON.stringify(contentRouteFor(post))},
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
function renderRouteModule(posts, documents, libraryItems) {
    const routes = [
        "/",
        "/blog/",
        "/docs/",
        ...posts.map((post) => post.route),
        ...documents.map((document) => document.route),
        ...libraryItems.map((item) => item.route),
    ];

    return `/// The complete static browser route set.\nexport const prerenderRoutes = ${JSON.stringify(routes, null, 4)} as const;\n`;
}

/// Return the complete full-text search index.
function searchEntriesFor(posts, documents, libraryItems) {
    return [
        ...documents.flatMap((document) => [
            {
                context: "docs",
                route: document.route,
                text: `${document.description} ${document.searchSections.find((section) => section.depth === 1)?.text ?? ""}`,
                title: document.title,
            },
            ...document.searchSections
                .filter((section) => section.depth > 1)
                .map((section) => ({
                    context: `docs / ${document.title}`,
                    route: `${document.route}#${section.id}`,
                    text: section.text,
                    title: section.title,
                })),
        ]),
        ...posts.flatMap((post) => [
            {
                context: `blog / ${post.date}`,
                route: post.route,
                text: `${post.subtitle} ${post.searchText}`,
                title: post.title,
            },
            ...post.searchSections
                .filter((section) => section.depth > 1)
                .map((section) => ({
                    context: `blog / ${post.title}`,
                    route: `${post.route}#${section.id}`,
                    text: section.text,
                    title: section.title,
                })),
        ]),
        ...libraryItems.map((item) => ({
            context: `docs / ${item.module.specifier}`,
            route: item.route,
            text: item.searchSections[0].text,
            title: item.title,
        })),
    ];
}

/// Require one generated file to match its expected contents.
function checkGeneratedFile(file, source) {
    if (!existsSync(file)) {
        throw new Error(`missing generated content file: ${file}`);
    }

    const current = readFileSync(file, "utf8");
    if (current !== source) {
        throw new Error("generated content is out of date, run `just platform/site/generate`");
    }
}

/// Require one obsolete generated path to remain absent.
function checkMissingGeneratedPath(path) {
    if (existsSync(path)) {
        throw new Error(`obsolete generated content remains: ${path}`);
    }
}

/// Atomically replace one generated file.
function writeGeneratedFile(file, source) {
    const temporaryFile = `${file}.${process.pid}.tmp`;

    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}
