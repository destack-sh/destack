import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import { marked } from "marked";
import {
    existsSync,
    mkdirSync,
    readFileSync,
    readdirSync,
    renameSync,
    statSync,
    writeFileSync,
} from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const siteDirectory = join(repositoryDirectory, "platform/site");
const contentDirectory = join(siteDirectory, "src/content/blog");
const generatedDirectory = join(siteDirectory, "src/generated");
const generatedPostFile = join(generatedDirectory, "posts.ts");
const generatedRouteFile = join(generatedDirectory, "prerender-routes.ts");
const isCheck = process.argv.includes("--check");

const destackKeywords = new Set([
    "const",
    "declare",
    "enum",
    "extension",
    "function",
    "import",
    "let",
    "match",
    "module",
    "newtype",
    "readonly",
    "return",
    "satisfies",
    "static",
    "struct",
    "type",
    "using",
]);
const destackTypes = new Set([
    "boolean",
    "float32",
    "float64",
    "int32",
    "int64",
    "never",
    "string",
    "uint8",
    "uint64",
    "unknown",
    "usize",
]);

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("js", javascript);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("ts", typescript);
hljs.registerLanguage("tsx", typescript);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("svg", xml);

const posts = readPosts();
const postSource = renderPostModule(posts);
const routeSource = renderRouteModule(posts);

if (isCheck) {
    checkGeneratedFile(generatedPostFile, postSource);
    checkGeneratedFile(generatedRouteFile, routeSource);
} else {
    mkdirSync(generatedDirectory, { recursive: true });
    writeGeneratedFile(generatedPostFile, postSource);
    writeGeneratedFile(generatedRouteFile, routeSource);
}

function readPosts() {
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
        const context = {
            assets: [],
            postDirectory,
            slug,
        };
        const html = renderMarkdown(markdown, context);

        return {
            ...metadata,
            assets: context.assets,
            html,
            route: `/blog/${slug}/`,
            slug,
            tableOfContents: headingsFor(markdown),
        };
    });
}

function parseFrontmatter(source, file) {
    const match = source.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
    if (match == undefined) {
        throw new Error(`missing frontmatter: ${file}`);
    }

    const metadata = parseMetadata(match[1], file);

    return {
        markdown: match[2].trimEnd(),
        metadata,
    };
}

function parseMetadata(source, file) {
    const metadata = {};
    const lines = source.split("\n");

    for (const line of lines) {
        if (line.trim() === "") {
            continue;
        }

        const match = line.match(/^([a-zA-Z][a-zA-Z0-9]*):\s*(.*)$/);
        if (match == undefined) {
            throw new Error(`invalid frontmatter line in ${file}: ${line}`);
        }

        metadata[match[1]] = parseMetadataValue(match[2]);
    }

    requireString(metadata, "title", file);
    requireString(metadata, "summary", file);
    requireString(metadata, "date", file);
    requireString(metadata, "author", file);
    requireString(metadata, "status", file);

    if (metadata.status !== "draft" && metadata.status !== "published") {
        throw new Error(`invalid status in ${file}: ${metadata.status}`);
    }

    if (!Array.isArray(metadata.tags) || !metadata.tags.every((tag) => typeof tag === "string")) {
        throw new Error(`invalid tags in ${file}`);
    }

    return metadata;
}

function parseMetadataValue(value) {
    const trimmed = value.trim();

    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
        const body = trimmed.slice(1, -1).trim();
        if (body === "") {
            return [];
        }

        return body.split(",").map((item) => parseQuotedString(item.trim()));
    }

    return parseQuotedString(trimmed);
}

function parseQuotedString(value) {
    if (
        (value.startsWith("\"") && value.endsWith("\"")) ||
        (value.startsWith("'") && value.endsWith("'"))
    ) {
        return value.slice(1, -1);
    }

    return value;
}

function requireString(metadata, field, file) {
    if (typeof metadata[field] !== "string" || metadata[field] === "") {
        throw new Error(`missing ${field} in ${file}`);
    }
}

function renderMarkdown(markdown, context) {
    const footnotes = extractFootnotes(markdown, context);
    const renderer = new marked.Renderer();
    const headingSlugs = new Map();
    const counters = {
        figure: 0,
    };

    renderer.heading = (token) => {
        const id = uniqueSlug(token.text, headingSlugs);
        const content = marked.parseInline(token.text);

        return `<h${token.depth} id="${id}">${content}</h${token.depth}>`;
    };
    renderer.link = (token) => {
        const href = resolveLink(token.href, context);
        const title = token.title == undefined ? "" : ` title="${escapeAttribute(token.title)}"`;
        const rel = isExternalLink(href) ? " rel=\"external noopener noreferrer\"" : "";
        const target = isExternalLink(href) ? " target=\"_blank\"" : "";
        const text = marked.parseInline(token.text);

        return `<a href="${escapeAttribute(href)}"${title}${rel}${target}>${text}</a>`;
    };
    renderer.image = (token) => {
        const src = resolveLink(token.href, context);
        const title = token.title == undefined ? "" : ` title="${escapeAttribute(token.title)}"`;

        return `<img alt="${escapeAttribute(token.text)}" src="${escapeAttribute(src)}"${title}>`;
    };
    renderer.code = (token) => renderCode(token, counters);

    const withDirectives = renderDirectives(footnotes.markdown, context, renderer, counters);
    const html = marked.parse(withDirectives, { gfm: true, renderer });
    const notes = renderFootnotes(footnotes.notes, renderer);

    return `${html}${notes}`;
}

function extractFootnotes(markdown, context) {
    const notes = [];
    const lines = markdown.split("\n");
    const kept = [];

    for (const line of lines) {
        const match = line.match(/^\[\^([^\]]+)]:\s*(.+)$/);
        if (match == undefined) {
            kept.push(line);
            continue;
        }

        const number = notes.length + 1;
        notes.push({ id: match[1], number, text: match[2] });
    }

    const nextIndex = new Map();
    const rewritten = kept.join("\n").replace(/\[\^([^\]]+)]/g, (_, id) => {
        const note = notes.find((note) => note.id === id);
        if (note == undefined) {
            throw new Error(`missing footnote definition in ${context.slug}: ${id}`);
        }

        const count = (nextIndex.get(id) ?? 0) + 1;
        const referenceId = count === 1 ? id : `${id}-${count}`;
        const noteId = escapeAttribute(id);
        const reference = escapeAttribute(referenceId);
        nextIndex.set(id, count);

        return `<sup class="blog-footnote-ref" id="fnref-${reference}"><a href="#fn-${noteId}">${escapeHtml(String(note.number))}</a></sup><span class="blog-margin-note" aria-hidden="true"><span>${escapeHtml(String(note.number))}</span>${marked.parseInline(note.text)}</span>`;
    });

    return { markdown: rewritten, notes };
}

function renderFootnotes(notes, renderer) {
    if (notes.length === 0) {
        return "";
    }

    const items = notes
        .map((note) => {
            const body = marked.parseInline(note.text, { renderer });
            const id = escapeAttribute(note.id);

            return `<li id="fn-${id}"><span class="blog-footnote-number">${note.number}</span><span>${body} <a class="blog-footnote-back" href="#fnref-${id}">back</a></span></li>`;
        })
        .join("");

    return `<section class="blog-footnotes"><h2>notes</h2><ol>${items}</ol></section>`;
}

function renderDirectives(markdown, context, renderer, counters) {
    const lines = markdown.split("\n");
    const output = [];

    for (let index = 0; index < lines.length; index += 1) {
        const inlineMatch = lines[index].match(/^:::(\w+)(?:\s+(.*?))?\s+:::$/);
        if (inlineMatch != undefined) {
            const name = inlineMatch[1];
            const attributes = parseAttributes(inlineMatch[2] ?? "");
            output.push(renderDirective(name, attributes, "", context, renderer, counters));
            continue;
        }

        const match = lines[index].match(/^:::(\w+)(?:\s+(.*))?$/);
        if (match == undefined) {
            output.push(lines[index]);
            continue;
        }

        const name = match[1];
        const attributes = parseAttributes(match[2] ?? "");
        const body = [];
        index += 1;

        while (index < lines.length && lines[index] !== ":::") {
            body.push(lines[index]);
            index += 1;
        }

        if (index >= lines.length) {
            throw new Error(`unclosed directive in ${context.slug}: ${name}`);
        }

        output.push(renderDirective(name, attributes, body.join("\n"), context, renderer, counters));
    }

    return output.join("\n");
}

function parseAttributes(source) {
    const attributes = {};

    for (const match of source.matchAll(/(\w+)=(?:"([^"]*)"|'([^']*)'|(\S+))/g)) {
        attributes[match[1]] = match[2] ?? match[3] ?? match[4];
    }

    if (!source.includes("=") && source.trim() !== "") {
        attributes.kind = source.trim();
    }

    return attributes;
}

function renderDirective(name, attributes, body, context, renderer, counters) {
    if (name === "callout") {
        const kind = attributes.kind ?? "note";
        const html = marked.parse(body, { gfm: true, renderer });

        return `<aside class="blog-callout" data-kind="${escapeAttribute(kind)}"><strong>${escapeHtml(kind)}</strong>${html}</aside>`;
    }

    if (name === "figure") {
        const src = requireAttribute(attributes, "src", context.slug, name);
        const alt = requireAttribute(attributes, "alt", context.slug, name);
        const caption = attributes.caption ?? body.trim();
        const url = resolveAsset(src, context);
        const label = nextFigureLabel(counters, "figure");

        return `<figure class="blog-figure"><img alt="${escapeAttribute(alt)}" src="${escapeAttribute(url)}"><figcaption><span>${label}</span>${marked.parseInline(caption, { renderer })}</figcaption></figure>`;
    }

    throw new Error(`unknown directive in ${context.slug}: ${name}`);
}

function requireAttribute(attributes, attribute, slug, directive) {
    const value = attributes[attribute];
    if (value == undefined || value === "") {
        throw new Error(`missing ${attribute} in ${slug} ${directive} directive`);
    }

    return value;
}

function renderCode(token, counters) {
    const language = (token.lang ?? "").trim().split(/\s+/)[0];

    if (language === "diagram") {
        const label = nextFigureLabel(counters, "figure");

        return `<figure class="blog-diagram"><figcaption><span>${label}</span>diagram</figcaption><pre><code>${escapeHtml(token.text)}</code></pre></figure>`;
    }

    const highlighted = highlightCode(token.text, language);
    const label = language === "" ? "text" : language;
    const number = nextFigureLabel(counters, "listing");

    return `<figure class="blog-code"><figcaption><span>${number}</span>${escapeHtml(label)}</figcaption><pre><code>${highlighted}</code></pre></figure>`;
}

function nextFigureLabel(counters, kind) {
    counters.figure += 1;

    return `${kind} ${counters.figure}`;
}

function highlightCode(source, language) {
    if (language === "ds" || language === "destack") {
        return highlightDestack(source);
    }

    if (language !== "" && hljs.getLanguage(language) != undefined) {
        return hljs.highlight(source, { language }).value;
    }

    return hljs.highlightAuto(source).value;
}

function highlightDestack(source) {
    const pattern =
        /\/\/[^\n]*|`(?:\\.|[^`\\])*`|"(?:\\.|[^"\\])*"|\b\d+(?:\.\d+)?\b|\b[A-Za-z_][A-Za-z0-9_]*\b|[{}()[\]<>:;,.|?=!&%*+-]/g;
    let html = "";
    let cursor = 0;

    for (const match of source.matchAll(pattern)) {
        const value = match[0];
        const start = match.index;
        const kind = destackKind(value);

        html += escapeHtml(source.slice(cursor, start));
        if (kind == undefined) {
            html += escapeHtml(value);
        } else {
            html += `<span class="${kind}">${escapeHtml(value)}</span>`;
        }
        cursor = start + value.length;
    }

    html += escapeHtml(source.slice(cursor));

    return html;
}

function destackKind(value) {
    if (value.startsWith("//")) {
        return "hljs-comment";
    }

    if (value.startsWith("`") || value.startsWith("\"")) {
        return "hljs-string";
    }

    if (/^\d/.test(value)) {
        return "hljs-number";
    }

    if (destackKeywords.has(value)) {
        return "hljs-keyword";
    }

    if (destackTypes.has(value)) {
        return "hljs-built_in";
    }

    return undefined;
}

function resolveLink(href, context) {
    if (isExternalLink(href) || href.startsWith("#")) {
        return href;
    }

    if (href.endsWith(".md")) {
        const target = resolve(context.postDirectory, href);
        const relativeTarget = relative(contentDirectory, target).replaceAll("\\", "/");
        const [slug] = relativeTarget.split("/");

        if (!existsSync(target)) {
            throw new Error(`missing markdown link in ${context.slug}: ${href}`);
        }

        return `/blog/${slug}/`;
    }

    if (href.startsWith("./") || href.startsWith("../") || isBareAssetLink(href)) {
        return resolveAsset(href, context);
    }

    return href;
}

function isExternalLink(href) {
    return /^[a-z][a-z0-9+.-]*:/i.test(href);
}

function isBareAssetLink(href) {
    if (href.startsWith("/") || href.startsWith("#")) {
        return false;
    }

    const path = href.split(/[?#]/, 1)[0];

    return extname(path) !== "";
}

function resolveAsset(href, context) {
    const target = resolve(context.postDirectory, href);
    const relativeTarget = relative(context.postDirectory, target);

    if (relativeTarget.startsWith("..") || relativeTarget === "") {
        throw new Error(`asset escapes post directory in ${context.slug}: ${href}`);
    }

    if (!existsSync(target)) {
        throw new Error(`missing asset in ${context.slug}: ${href}`);
    }

    if (extname(target) === ".md") {
        throw new Error(`asset points at markdown in ${context.slug}: ${href}`);
    }

    const existing = context.assets.find((asset) => asset.path === target);
    if (existing != undefined) {
        return existing.placeholder;
    }

    const index = context.assets.length;
    const asset = {
        importName: `asset${index}`,
        path: target,
        placeholder: `__BLOG_ASSET_${index}__`,
    };
    context.assets.push(asset);

    return asset.placeholder;
}

function headingsFor(markdown) {
    const slugs = new Map();

    return markdown
        .split("\n")
        .map((line) => line.match(/^(#{2,3})\s+(.+)$/))
        .filter((match) => match != undefined)
        .map((match) => ({
            depth: match[1].length,
            id: uniqueSlug(match[2], slugs),
            text: match[2].replace(/`/g, ""),
        }));
}

function uniqueSlug(text, slugs) {
    const base = text
        .toLowerCase()
        .replace(/`([^`]+)`/g, "$1")
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "");
    const count = slugs.get(base) ?? 0;
    slugs.set(base, count + 1);

    return count === 0 ? base : `${base}-${count + 1}`;
}

function renderPostModule(posts) {
    const imports = posts.flatMap((post) =>
        post.assets.map((asset) => {
            const path = relative(generatedDirectory, asset.path).replaceAll("\\", "/");

            const importPath = JSON.stringify(`${path}?url`);
            const importName = `${post.slug.replaceAll("-", "_")}_${asset.importName}`;

            return `import ${importName} from ${importPath};`;
        }),
    );
    const records = posts.map((post) => renderPostRecord(post)).join(",\n");

    return `${imports.join("\n")}${imports.length === 0 ? "" : "\n\n"}export type PostStatus = "draft" | "published";

export type Post = {
    author: string;
    date: string;
    html: string;
    route: string;
    slug: string;
    status: PostStatus;
    summary: string;
    tableOfContents: readonly TableOfContentsEntry[];
    tags: readonly string[];
    title: string;
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

function resolveAssets(html: string, assets: Record<string, string>) {
    let resolved = html;

    for (const [placeholder, asset] of Object.entries(assets)) {
        resolved = resolved.replaceAll(placeholder, asset);
    }

    return resolved;
}
`;
}

function renderPostRecord(post) {
    const assetMap = Object.fromEntries(
        post.assets.map((asset) => [
            asset.placeholder,
            `${post.slug.replaceAll("-", "_")}_${asset.importName}`,
        ]),
    );
    const assets = Object.entries(assetMap)
        .map(([placeholder, importName]) => `${JSON.stringify(placeholder)}: ${importName}`)
        .join(", ");

    return `    {
        author: ${JSON.stringify(post.author)},
        date: ${JSON.stringify(post.date)},
        html: resolveAssets(${JSON.stringify(post.html)}, { ${assets} }),
        route: ${JSON.stringify(post.route)},
        slug: ${JSON.stringify(post.slug)},
        status: ${JSON.stringify(post.status)},
        summary: ${JSON.stringify(post.summary)},
        tableOfContents: ${JSON.stringify(post.tableOfContents)},
        tags: ${JSON.stringify(post.tags)},
        title: ${JSON.stringify(post.title)},
    }`;
}

function renderRouteModule(posts) {
    const routes = ["/", "/blog/", ...posts.map((post) => post.route)];

    return `export const prerenderRoutes = ${JSON.stringify(routes, null, 4)} as const;\n`;
}

function checkGeneratedFile(file, source) {
    if (!existsSync(file)) {
        throw new Error(`missing generated post file: ${file}`);
    }

    const current = readFileSync(file, "utf8");
    if (current !== source) {
        throw new Error("generated posts are out of date, run `just platform/site/format`");
    }
}

function writeGeneratedFile(file, source) {
    const temporaryFile = `${file}.${process.pid}.tmp`;

    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}

function escapeHtml(value) {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll("\"", "&quot;");
}

function escapeAttribute(value) {
    return escapeHtml(value).replaceAll("'", "&#39;");
}
