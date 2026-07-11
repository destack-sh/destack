import {
    existsSync,
    mkdirSync,
    readFileSync,
    readdirSync,
    renameSync,
    rmSync,
    statSync,
    writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
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

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const siteDirectory = join(repositoryDirectory, "platform/site");
const contentDirectory = join(siteDirectory, "src/content/blog");
const documentDirectory = join(repositoryDirectory, "docs");
const generatedDirectory = join(siteDirectory, "src/generated");
const publicDirectory = join(siteDirectory, "public");
const generatedDocumentDirectory = join(generatedDirectory, "document");
const generatedPostDirectory = join(generatedDirectory, "post");
const generatedDocumentFile = join(generatedDirectory, "documents.ts");
const generatedPostFile = join(generatedDirectory, "posts.ts");
const generatedRouteFile = join(generatedDirectory, "prerender-routes.ts");
const generatedSearchFile = join(generatedDirectory, "search.ts");
const isCheck = process.argv.includes("--check");
const documentPathPattern = /^(?:[a-z0-9]+(?:-[a-z0-9]+)*\/)*[a-z0-9]+(?:-[a-z0-9]+)*\.md$/;

const documentSources = readDocumentSources();
const documents = renderDocuments(documentSources);
const postSources = readPostSources();
const posts = renderPosts(postSources);
if (!isCheck) {
    await writePageSources([...documents, ...posts], publicDirectory);
}
const documentSource = renderDocumentModule(documents);
const documentContentSources = renderDocumentContentModules(documents);
const postSource = renderPostModule(posts);
const postContentSources = renderPostContentModules(posts);
const routeSource = renderRouteModule(posts, documents);
const searchSource = renderSearchModule(posts, documents);

if (isCheck) {
    checkGeneratedFile(generatedDocumentFile, documentSource);
    checkGeneratedModules(generatedDocumentDirectory, documentContentSources, "document");
    checkGeneratedFile(generatedPostFile, postSource);
    checkGeneratedModules(generatedPostDirectory, postContentSources, "post");
    checkGeneratedFile(generatedRouteFile, routeSource);
    checkGeneratedFile(generatedSearchFile, searchSource);
} else {
    mkdirSync(generatedDirectory, { recursive: true });
    writeGeneratedFile(generatedDocumentFile, documentSource);
    writeGeneratedModules(generatedDocumentDirectory, documentContentSources);
    writeGeneratedFile(generatedPostFile, postSource);
    writeGeneratedModules(generatedPostDirectory, postContentSources);
    writeGeneratedFile(generatedRouteFile, routeSource);
    writeGeneratedFile(generatedSearchFile, searchSource);
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

    if (!paths.has("index.md")) {
        throw new Error("documentation root is missing index.md");
    }

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

/// Read and validate every blog source.
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

        if (!Array.isArray(metadata.tags) || !metadata.tags.every((tag) => typeof tag === "string")) {
            throw new Error(`invalid tags in ${sourceFile}`);
        }

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

/// Generate the documentation metadata and lazy-loader module.
function renderDocumentModule(documents) {
    const records = documents.map((document) => `    {
        description: ${JSON.stringify(document.description)},
        markdownRoute: ${JSON.stringify(document.markdownRoute)},
        order: ${document.order},
        path: ${JSON.stringify(document.path)},
        route: ${JSON.stringify(document.route)},
        tableOfContents: ${JSON.stringify(document.tableOfContents)},
        textRoute: ${JSON.stringify(document.textRoute)},
        title: ${JSON.stringify(document.title)},
        tokens: ${document.tokens},
    }`).join(",\n");
    const loaders = documents.map((document) => {
        const name = generatedDocumentName(document);

        return `    ${JSON.stringify(document.route)}: () => import("./document/${name}"),`;
    }).join("\n");

    return `export type Document = {
    description: string;
    markdownRoute: string;
    order: number;
    path: string;
    route: string;
    tableOfContents: readonly TableOfContentsEntry[];
    textRoute: string;
    title: string;
    tokens: number;
};

export type DocumentContent = {
    html: string;
};

export type TableOfContentsEntry = {
    depth: number;
    id: string;
    text: string;
};

export const documents = [
${records}
] as const satisfies readonly Document[];

export const documentByRoute: ReadonlyMap<string, Document> = new Map(
    documents.map((document): [string, Document] => [document.route, document]),
);

const documentLoaders: Record<string, () => Promise<{ default: DocumentContent }>> = {
${loaders}
};

/// Load one rendered document body by canonical route.
export async function loadDocument(route: string): Promise<DocumentContent | undefined> {
    return documentLoaders[route]?.().then((module) => module.default);
}
`;
}

/// Generate one lazy body module per document.
function renderDocumentContentModules(documents) {
    return renderContentModules(
        documents,
        generatedDocumentDirectory,
        "DocumentContent",
        "../documents",
        generatedDocumentName,
    );
}

/// Return the collision-free module name for a validated document path.
function generatedDocumentName(document) {
    return document.path.slice(0, -3).replaceAll("/", "_");
}

/// Generate the blog metadata and lazy-loader module.
function renderPostModule(posts) {
    const records = posts.map((post) => renderPostRecord(post)).join(",\n");
    const loaders = posts
        .map((post) => `    ${JSON.stringify(post.slug)}: () => import("./post/${post.slug}"),`)
        .join("\n");

    return `export type Post = {
    author: string;
    date: string;
    markdownRoute: string;
    route: string;
    slug: string;
    subtitle: string;
    tableOfContents: readonly TableOfContentsEntry[];
    tags: readonly string[];
    textRoute: string;
    title: string;
    tokens: number;
};

export type PostContent = {
    html: string;
};

export type TableOfContentsEntry = {
    depth: number;
    id: string;
    text: string;
};

export const posts = [
${records}
] as const satisfies readonly Post[];

export const postBySlug: ReadonlyMap<string, Post> = new Map(
    posts.map((post): [string, Post] => [post.slug, post]),
);

const postLoaders: Record<string, () => Promise<{ default: PostContent }>> = {
${loaders}
};

/// Load one rendered post body by slug.
export async function loadPost(slug: string): Promise<PostContent | undefined> {
    return postLoaders[slug]?.().then((module) => module.default);
}
`;
}

/// Generate one blog metadata record.
function renderPostRecord(post) {
    return `    {
        author: ${JSON.stringify(post.author)},
        date: ${JSON.stringify(post.date)},
        markdownRoute: ${JSON.stringify(post.markdownRoute)},
        route: ${JSON.stringify(post.route)},
        slug: ${JSON.stringify(post.slug)},
        subtitle: ${JSON.stringify(post.subtitle)},
        tableOfContents: ${JSON.stringify(post.tableOfContents)},
        tags: ${JSON.stringify(post.tags)},
        textRoute: ${JSON.stringify(post.textRoute)},
        title: ${JSON.stringify(post.title)},
        tokens: ${post.tokens},
    }`;
}

/// Generate one lazy body module per post.
function renderPostContentModules(posts) {
    return renderContentModules(
        posts,
        generatedPostDirectory,
        "PostContent",
        "../posts",
        (post) => post.slug,
    );
}

/// Generate independently loadable rendered-content modules.
function renderContentModules(contents, directory, type, typeImport, nameFor) {
    return new Map(contents.map((content) => {
        const imports = content.assets.map((asset) => {
            const path = relative(directory, asset.path).replaceAll("\\", "/");
            const importPath = JSON.stringify(`${path}?url`);

            return `import ${asset.importName} from ${importPath};`;
        });
        const assets = content.assets
            .map((asset) => `${JSON.stringify(asset.placeholder)}: ${asset.importName}`)
            .join(", ");
        const source = `${imports.join("\n")}${imports.length === 0 ? "" : "\n\n"}import type { ${type} } from ${JSON.stringify(typeImport)};

const content = {
    html: resolveAssets(${JSON.stringify(content.html)}, { ${assets} }),
} satisfies ${type};

export default content;

/// Replace build-time asset placeholders with bundled URLs.
function resolveAssets(html: string, assets: Record<string, string>) {
    let resolved = html;

    for (const [placeholder, asset] of Object.entries(assets)) {
        resolved = resolved.replaceAll(placeholder, asset);
    }

    return resolved;
}
`;

        return [nameFor(content), source];
    }));
}

/// Generate the complete prerender route list.
function renderRouteModule(posts, documents) {
    const routes = ["/", "/blog/", ...posts.map((post) => post.route), ...documents.map((document) => document.route)];

    return `export const prerenderRoutes = ${JSON.stringify(routes, null, 4)} as const;\n`;
}

/// Generate the lazy full-text search index.
function renderSearchModule(posts, documents) {
    const entries = [
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
    ];

    return `import type { SearchEntry } from "../content/search";

export const searchEntries = ${JSON.stringify(entries, null, 4)} as const satisfies readonly SearchEntry[];
`;
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

/// Require one generated module directory to match exactly.
function checkGeneratedModules(directory, sources, kind) {
    if (!existsSync(directory)) {
        throw new Error(`missing generated ${kind} directory: ${directory}`);
    }

    const expected = [...sources.keys()].map((name) => `${name}.ts`).sort();
    const actual = readdirSync(directory).sort();
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`generated ${kind} modules are out of date, run \`just platform/site/generate\``);
    }

    for (const [name, source] of sources) {
        checkGeneratedFile(join(directory, `${name}.ts`), source);
    }
}

/// Atomically replace one generated file.
function writeGeneratedFile(file, source) {
    const temporaryFile = `${file}.${process.pid}.tmp`;

    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}

/// Replace one generated module directory.
function writeGeneratedModules(directory, sources) {
    rmSync(directory, { force: true, recursive: true });
    mkdirSync(directory, { recursive: true });

    for (const [name, source] of sources) {
        writeGeneratedFile(join(directory, `${name}.ts`), source);
    }
}
