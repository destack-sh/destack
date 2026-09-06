import { buildNavigation } from "./navigation.mjs";
import { collections } from "../content.ts";
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
import {
    moduleCatalogRoute,
    readLibraryReference,
    renderLibraryDocuments,
} from "./reference.mjs";
import {
    ruleCatalogRoute,
    readLintReference,
    renderLintDocuments,
} from "./lint.mjs";
import { withLock } from "./lock.mjs";

const repositoryDirectory = resolve(
    dirname(fileURLToPath(import.meta.url)),
    "../../..",
);
const siteDirectory = join(repositoryDirectory, "platform/site");
const contentDirectory = join(
    repositoryDirectory,
    collections.find((collection) => collection.route === "/blog/").sources[0]
        .directory,
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
const publishedPageFile = join(
    siteDirectory,
    ".generated/published-pages.json",
);
const publicSearchFile = join(publicDirectory, "search.json");
const generatedPostFile = join(generatedDirectory, "posts.ts");
const generatedRouteFile = join(generatedDirectory, "prerender-routes.ts");
const isCheck = process.argv.includes("--check");
const documentPathPattern =
    /^(?:[a-z0-9]+(?:-[a-z0-9]+)*\/)*[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;
const documentSegmentPattern = /^(\d{2})-([a-z0-9]+(?:-[a-z0-9]+)*)$/;

mkdirSync(generatedDirectory, { recursive: true });
await withLock(join(generatedDirectory, ".content-lock"), async () => {
    const documentSources = readDocumentSources();
    const renderedDocuments = renderDocuments(documentSources);
    const libraryIndex = renderedDocuments.find(
        (document) => document.route === moduleCatalogRoute,
    );
    if (libraryIndex == undefined) {
        throw new Error("missing standard library documentation index");
    }
    const libraryReference = readLibraryReference();
    const library = renderLibraryDocuments(libraryReference, libraryIndex);
    const lintIndex = renderedDocuments.find(
        (document) => document.route === ruleCatalogRoute,
    );
    if (lintIndex == undefined) {
        throw new Error("missing lint rule documentation index");
    }
    const lintReference = readLintReference();
    const lint = renderLintDocuments(lintReference, lintIndex);
    const documents = [
        ...renderedDocuments.filter(
            (document) => document !== libraryIndex && document !== lintIndex,
        ),
        ...library.documents,
        ...lint.documents,
    ].sort(
        (left, right) =>
            left.order - right.order || left.route.localeCompare(right.route),
    );
    const postSources = readPostSources();
    const posts = renderPosts(postSources);
    const references = [...library.items, ...lint.items];
    // resolve navigation once for authored chapters and generated references
    const modules = new Set(
        library.documents
            .filter((page) => page.route !== libraryIndex.route)
            .map((page) => page.route),
    );
    for (const page of [...documents, ...references]) {
        page.kind =
            page.searchKind ?? (modules.has(page.route) ? "module" : "chapter");
    }
    buildNavigation(documents, references);
    appendChapterContents(documents);
    const pages = [...documents, ...references, ...posts];
    const searchEntries = searchEntriesFor(posts, documents, references);
    if (!isCheck) {
        await writePageSources(pages, publicDirectory);
        writePageContent(pages);
        removeObsoletePages(pages);
        writeGeneratedFile(
            publicSearchFile,
            `${JSON.stringify(searchEntries)}\n`,
        );
    }
    generateDocumentMetadata([...documents, ...references]);
    const postSource = renderPostModule(posts);
    const routeSource = renderRouteModule(posts, documents, references);

    if (isCheck) {
        checkGeneratedFile(generatedPostFile, postSource);
        checkGeneratedFile(generatedRouteFile, routeSource);
    } else {
        mkdirSync(generatedDirectory, { recursive: true });
        writeGeneratedFile(generatedPostFile, postSource);
        writeGeneratedFile(generatedRouteFile, routeSource);
    }
});

/// Append immediate child chapters to each authored index.
function appendChapterContents(documents) {
    const chapters = documents.filter((page) => page.kind === "chapter");

    // preserve the resolved navigation order and specialized catalogs
    for (const chapter of chapters) {
        if (
            (chapter.path !== "index.md" &&
                !chapter.path.endsWith("/index.md")) ||
            chapter.route === moduleCatalogRoute ||
            chapter.route === ruleCatalogRoute
        ) {
            continue;
        }

        const children = chapters.filter((page) => page.parent === chapter);
        if (children.length === 0) {
            continue;
        }

        // render the same generated links in each published representation
        const contents = children
            .map((page) => {
                const title = page.title.replace(/[\\`*_[\]<>]/g, "\\$&");

                return `- [${title}](${page.route})`;
            })
            .join("\n");
        chapter.markdown = `${chapter.markdown.trimEnd()}\n\n${contents}\n`;
        chapter.html += renderMarkdown(contents, {
            assets: [],
            documentDirectory: chapter.directory,
            kind: "document",
            markdownDirectory: dirname(chapter.file),
            ownHeadings: new Set(),
            route: chapter.route,
            slug: chapter.path,
            sourceRoutes: new Map(),
        });

        // keep search text and token counts consistent with the published body
        chapter.searchSections = searchSectionsFor(chapter.markdown);
        chapter.searchText = searchTextFor(chapter.markdown);
        chapter.tokens = tokenEstimateFor(plainTextFor(chapter.markdown));
    }
}

/// Write rendered page bodies and their content-addressed assets.
function writePageContent(pages) {
    mkdirSync(publicContentDirectory, { recursive: true });
    const assetRoutes = new Map();

    // resolve every page against one deduplicated asset collection
    for (const page of pages) {
        const assets = Object.fromEntries(
            page.assets.map((asset) => {
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

/// Generate or check the metadata for every documentation page.
function generateDocumentMetadata(documents) {
    for (const document of documents) {
        const metadata = {
            contentRoute: contentRouteFor(document),
            description: document.description,
            lead: document.lead,
            markdownRoute: document.markdownRoute,
            kind: document.kind,
            navigation: document.navigation,
            route: document.route,
            tableOfContents: document.tableOfContents,
            textRoute: document.textRoute,
            title: document.title,
            tokens: document.tokens,
        };
        const file = join(
            publicDirectory,
            documentMetadataRoute(document.route).slice(1),
        );
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
function removeObsoletePages(pages) {
    const routes = pages.flatMap((page) => [
        page.markdownRoute,
        page.textRoute,
        ...(page.route.startsWith("/docs/")
            ? [documentMetadataRoute(page.route)]
            : []),
    ]);
    const previous = existsSync(publishedPageFile)
        ? JSON.parse(readFileSync(publishedPageFile, "utf8"))
        : [];
    const current = new Set(routes);

    // retain immutable bodies for readers that already loaded earlier metadata
    for (const route of previous) {
        if (
            !/^\/(?:docs|blog|_content\/docs)\//.test(route) ||
            route.includes("..")
        ) {
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
function documentMetadataRoute(route) {
    return `/_content${route}index.json`;
}

/// Return the static rendered-body route for one content page.
function contentRouteFor(page) {
    const hash = createHash("sha256").update(page.html);
    for (const asset of page.assets) {
        hash.update(readFileSync(asset.path));
    }

    return `/_content/html/${hash.digest("hex")}.html`;
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

/// Read every documentation source in path order.
function readDocumentSources() {
    const sources = documentDirectories
        .flatMap((collection) => {
            // require each documentation directory
            if (!existsSync(collection.directory)) {
                throw new Error(
                    `missing documentation directory: ${collection.directory}`,
                );
            }

            // map files into the documentation tree
            return markdownFiles(collection.directory).map((file) => {
                const source = readFileSync(file, "utf8");
                const { markdown, metadata } = parseFrontmatter(source, file);
                requireString(metadata, "title", file);
                requireString(metadata, "description", file);

                // reject frontmatter ordering
                if (Object.hasOwn(metadata, "order")) {
                    throw new Error(
                        `documentation order belongs in the source path: ${file}`,
                    );
                }

                const sourcePath = relative(
                    collection.directory,
                    file,
                ).replaceAll("\\", "/");
                const { hierarchy, path: publicPath } = parseDocumentPath(
                    sourcePath,
                    file,
                );
                const path =
                    collection.path === ""
                        ? publicPath
                        : `${collection.path}/${publicPath}`;
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
                    textRoute: `/${join("docs", path.replace(/\.md$/, ".txt")).replaceAll("\\", "/")}`,
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
function parseDocumentPath(path, file) {
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
        const name =
            extension === "" ? segment : segment.slice(0, -extension.length);
        const match = documentSegmentPattern.exec(name);
        if (match == null) {
            throw new Error(
                `documentation path lacks a numeric prefix: ${file}`,
            );
        }

        hierarchy.push(Number(match[1]));
        names.push(`${match[2]}${extension}`);
    }

    return { hierarchy, path: names.join("/") };
}

/// Compare documentation sources by their numeric hierarchy.
function compareDocuments(left, right) {
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
function validateDocuments(documents) {
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

        const titles = document.headings.filter(
            (heading) => heading.depth === 1,
        );
        if (titles.length !== 1 || titles[0].text !== document.title) {
            throw new Error(
                `documentation title does not match its H1: ${document.path}`,
            );
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
    const routePath =
        withoutExtension === "index"
            ? ""
            : withoutExtension.endsWith("/index")
              ? withoutExtension.slice(0, -6)
              : withoutExtension;

    return `/docs/${routePath === "" ? "" : `${routePath}/`}`;
}

/// Render documentation sources against the complete manual graph.
function renderDocuments(sources) {
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
        const context = {
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
            html: renderMarkdown(source.markdown, context),
            searchSections: searchSectionsFor(source.markdown),
            searchText: searchTextFor(source.markdown),
            tableOfContents: source.headings.filter(
                (heading) => heading.depth > 1,
            ),
        };
    });
}

/// Read and order blog posts.
function readPostSources() {
    if (!existsSync(contentDirectory)) {
        return [];
    }

    const directories = readdirSync(contentDirectory)
        .filter((entry) =>
            statSync(join(contentDirectory, entry)).isDirectory(),
        )
        .sort();
    const seen = new Set();

    return directories.map((directory) => {
        // apply the documentation numbering convention to blog source directories
        const postDirectory = join(contentDirectory, directory);
        const { path } = parseDocumentPath(
            `${directory}/index.md`,
            postDirectory,
        );
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
    });
}

/// Render blog sources against the complete post graph.
function renderPosts(sources) {
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
function renderRouteModule(posts, documents, references) {
    const routes = [
        "/",
        "/blog/",
        ...posts.map((post) => post.route),
        ...documents.map((document) => document.route),
        ...references.map((reference) => reference.route),
    ];

    return `/// The complete static browser route set.\nexport const prerenderRoutes = ${JSON.stringify(routes, null, 4)} as const;\n`;
}

/// Return the complete full-text search index.
function searchEntriesFor(posts, documents, references) {
    return [
        ...documents.flatMap((document) => [
            {
                context:
                    document.path === "index.md"
                        ? "docs"
                        : document.path.split("/").slice(0, -1).join(" / "),
                kind: document.kind === "module" ? "module" : "page",
                route: document.route,
                text: `${document.description} ${document.searchSections.find((section) => section.depth === 1)?.text ?? ""}`,
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
        ...references.map((reference) => ({
            context: reference.searchContext,
            kind: reference.searchKind,
            route: reference.route,
            text: reference.searchSections[0].text,
            title: reference.title,
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
        throw new Error(
            "generated content is out of date, run `just platform/site/generate`",
        );
    }
}

/// Atomically replace one generated file.
function writeGeneratedFile(file, source) {
    const temporaryFile = `${file}.${process.pid}.tmp`;

    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}
