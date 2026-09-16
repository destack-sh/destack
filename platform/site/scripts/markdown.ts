import { escapeHtml, escapeAttribute } from "../src/content/html.ts";
import type { Tokens, Renderer } from "marked";
import { renderListing } from "../src/content/listing.ts";
import { Marked, marked } from "marked";
import { existsSync } from "node:fs";
import { extname, relative, resolve } from "node:path";
import { highlightCode } from "./highlight.ts";
import { parseDirective, parseAttributes } from "./directives.ts";
import { searchTextFor } from "./text.ts";

/// An asset collected while rendering a content page.
export type ContentAsset = { importName: string; path: string; placeholder: string; };
/// Link and asset resolution for one Markdown source.
export type MarkdownContext = {
    assets: ContentAsset[];
    documentDirectory?: string;
    markdownDirectory?: string;
    kind?: string;
    ownHeadings?: Set<string>;
    route?: string;
    slug?: string;
    sourceRoutes?: Map<string, { route: string; headings: Set<string>; }>;
};
/// An authored footnote and its collected backlinks.
type Footnote = { text: string; references: string[]; number?: number; };
/// A heading and the Markdown accumulated beneath it.
type SearchSectionSource = { depth: number; id: string; source: string[]; title: string; };

const codeExtensions: Record<string, string> = {
    bash: "sh",
    bytecode: "dsa",
    javascript: "js",
    plaintext: "txt",
    rust: "rs",
    mir: "dsm",
    shell: "sh",
    typescript: "ts",
};

/// Split one content source into metadata and Markdown.
export function parseFrontmatter(source: string, file: string) {
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

/// Parse the supported frontmatter subset.
function parseMetadata(source: string, file: string) {
    const metadata: Record<string, unknown> = {};
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

    return metadata;
}

/// Parse one scalar or array frontmatter value.
function parseMetadataValue(value: string) {
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

/// Remove matching quotes from one metadata scalar.
function parseQuotedString(value: string) {
    if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
    ) {
        return value.slice(1, -1);
    }

    return value;
}

/// Require one non-empty string metadata field.
export function requireString<K extends string>(metadata: Record<string, unknown>, field: K, file: string): asserts metadata is Record<string, unknown> & Record<K, string> {
    if (typeof metadata[field] !== "string" || metadata[field] === "") {
        throw new Error(`missing ${field} in ${file}`);
    }
}

/// Render trusted Markdown with collection-aware links and assets.
export function renderMarkdown(markdown: string, context: MarkdownContext) {
    const renderer = new marked.Renderer();
    const parser = new Marked({ gfm: true });
    const headingSlugs = new Map();
    const counters = {
        figure: 0,
    };

    renderer.heading = (token) => {
        const id = uniqueSlug(token.text, headingSlugs);
        const content = parser.parseInline(token.text);

        return `<h${token.depth} id="${id}">${content}</h${token.depth}>`;
    };
    renderer.link = (token) => {
        const href = resolveLink(token.href, context);
        const title = token.title == undefined ? "" : ` title="${escapeAttribute(token.title)}"`;
        const rel = isExternalLink(href) ? ' rel="external noopener noreferrer"' : "";
        const target = isExternalLink(href) ? ' target="_blank"' : "";
        const text = parser.parseInline(token.text);

        return `<a href="${escapeAttribute(href)}"${title}${rel}${target}>${text}</a>`;
    };
    renderer.image = (token) => {
        const src = resolveLink(token.href, context);
        const title = token.title == undefined ? "" : ` title="${escapeAttribute(token.title)}"`;

        return `<img alt="${escapeAttribute(token.text)}" src="${escapeAttribute(src)}" loading="lazy" decoding="async"${title}>`;
    };
    renderer.table = (token) => renderTable(token, renderer, parser);
    renderer.code = (token) => renderCode(token, counters);
    renderer.blockquote = (token) => {
        // render GitHub alerts using the existing callout presentation
        const alert = /^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*\n/.exec(token.text);
        if (alert != null) {
            const kind = alert[1].toLowerCase();
            const body = parser.parse(token.text.slice(alert[0].length));

            return `<aside class="markdown-callout" data-kind="${kind}"><strong>${kind}</strong>${body}</aside>`;
        }

        // a final, separate attribution paragraph belongs outside the quoted words
        const attribution = /^([\s\S]+)\n\n—[ \t]+([^\n]+)\n?$/.exec(token.text);
        if (attribution != null) {
            const body = parser.parse(attribution[1]);
            const credit = parser.parseInline(attribution[2]);

            return `<figure class="markdown-quote"><blockquote>\n${body}</blockquote><figcaption>— ${credit}</figcaption></figure>\n`;
        }

        return `<blockquote>\n${parser.parse(token.text)}</blockquote>\n`;
    };

    // register the complete renderer before collecting and rendering notes
    parser.use({
        renderer,
        extensions: [
            {
                name: "directive",
                level: "block",
                start: (source) => source.search(/^:::\w/m),
                tokenizer(source) {
                    const directive = parseDirective(source);

                    return directive == undefined
                        ? undefined
                        : {
                            type: "directive",
                            ...directive,
                            tokens: this.lexer.blockTokens(directive.body),
                        };
                },
                renderer(token) {
                    return renderDirective(
                        token.name,
                        token.attributes,
                        token.body,
                        context,
                        parser,
                        counters,
                    );
                },
            },
        ],
    });
    const footnotes = configureFootnotes(markdown, parser, context);
    const html = parser.parse(markdown);
    const notes = renderFootnotes(footnotes, parser);

    return `${html}${notes}`;
}

/// Render one responsive GFM table.
function renderTable(token: Tokens.Table, renderer: Renderer, parser: Marked) {
    const labels = token.header.map((cell) => searchTextFor(cell.text));
    const header = token.header
        .map((cell) => {
            const alignment = cell.align == null ? "" : ` align="${cell.align}"`;

            return `<th${alignment}>${parser.parseInline(cell.text)}</th>`;
        })
        .join("");
    const head = renderer.tablerow({ text: header });
    const rows = token.rows
        .map((row) => {
            const cells = row
                .map((cell, index) => {
                    const label = escapeAttribute(labels[index] ?? "");

                    const alignment = cell.align == null ? "" : ` align="${cell.align}"`;
                    const content = parser.parseInline(cell.text);

                    return `<td data-label="${label}"${alignment}><div>${content}</div></td>`;
                })
                .join("");

            return renderer.tablerow({ text: cells });
        })
        .join("");
    const body = rows === "" ? "" : `<tbody>${rows}</tbody>`;

    return `<div class="markdown-table" tabindex="0"><table><thead>${head}</thead>${body}</table></div>`;
}

/// Register footnote definitions and references with the page parser.
function configureFootnotes(markdown: string, parser: Marked, context: MarkdownContext) {
    const definitions = new Map<string, Footnote>();
    const notes: Footnote[] = [];
    let isCollecting = true;

    // let Markdown distinguish notes from code, comments, escapes, and link destinations
    parser.use({
        extensions: [
            {
                name: "footnoteDefinition",
                level: "block",
                start: (source) => source.search(/^ {0,3}\[\^[^\]\s]+]:/m),
                tokenizer(source) {
                    const opening = /^ {0,3}\[\^([^\]\s]+)]:[ \t]*([^\n]*)(?:\n|$)/.exec(source);
                    if (opening == null) {
                        return;
                    }

                    // include indented paragraphs, lists, and fenced code in the definition
                    let length = opening[0].length;
                    let body = opening[2];
                    let continuation;
                    while (
                        (continuation = /^(?:[ \t]*\n)*(?: {4}|\t)[^\n]*(?:\n|$)/.exec(
                            source.slice(length),
                        ))
                    ) {
                        body += `\n${continuation[0].replace(/^( {4}|\t)/gm, "").trimEnd()}`;
                        length += continuation[0].length;
                    }

                    // collect each definition once before rendering references
                    const id = opening[1].toLowerCase();
                    if (isCollecting) {
                        if (definitions.has(id)) {
                            throw new Error(
                                `duplicate footnote definition in ${context.slug}: ${id}`,
                            );
                        }
                        definitions.set(id, { text: body, references: [] });
                    }

                    return { type: "footnoteDefinition", raw: source.slice(0, length) };
                },
                renderer: () => "",
            },
            {
                name: "footnoteReference",
                level: "inline",
                start: (source) => source.indexOf("[^"),
                tokenizer(source) {
                    const match = /^\[\^([^\]\s]+)]/.exec(source);

                    return match == null
                        ? undefined
                        : {
                            type: "footnoteReference",
                            raw: match[0],
                            id: match[1].toLowerCase(),
                        };
                },
                renderer(token) {
                    const note = definitions.get(token.id);
                    if (note == undefined) {
                        throw new Error(
                            `missing footnote definition in ${context.slug}: ${token.id}`,
                        );
                    }

                    // number notes by their first citation and link back to every occurrence
                    if (note.references.length === 0) {
                        note.number = notes.length + 1;
                        notes.push(note);
                    }
                    const reference = `fnref-${note.number}-${note.references.length + 1}`;
                    note.references.push(reference);

                    return `<sup class="markdown-footnote-ref" id="${reference}"><a href="#fn-${note.number}" role="doc-noteref" aria-label="Footnote ${note.number}">${note.number}</a></sup>`;
                },
            },
        ],
    });

    // resolve definitions before rendering, including citations in figure captions
    parser.lexer(markdown);
    isCollecting = false;

    return notes;
}

/// Render collected footnotes below the article.
function renderFootnotes(notes: Footnote[], parser: Marked) {
    if (notes.length === 0) {
        return "";
    }

    // render note bodies before backlinks so citations inside notes are included
    const bodies: (string | Promise<string>)[] = [];
    for (let index = 0; index < notes.length; index++) {
        bodies.push(parser.parse(notes[index].text));
    }

    const items = notes
        .map((note, index) => {
            const body = bodies[index];
            const links = note.references
                .map((reference, index) => {
                    const occurrence = note.references.length === 1 ? "" : ` ${index + 1}`;

                    return `<a class="markdown-footnote-back" href="#${reference}" role="doc-backlink" aria-label="Back to reference ${note.number}${occurrence}">↩${occurrence}</a>`;
                })
                .join(" ");

            return `<li id="fn-${note.number}"><span class="markdown-footnote-number">${note.number}</span><div>${body}${links}</div></li>`;
        })
        .join("");

    return `<section class="markdown-footnotes" role="doc-endnotes" aria-label="Footnotes"><h2>Notes</h2><ol>${items}</ol></section>`;
}

/// Render one supported Markdown directive.
function renderDirective(name: string, attributes: Record<string, string>, body: string, context: MarkdownContext, parser: Marked, counters: { figure: number; }) {
    if (name === "callout") {
        const kind = attributes.kind ?? "note";
        const html = parser.parse(body);

        return `<aside class="markdown-callout" data-kind="${escapeAttribute(kind)}"><strong>${escapeHtml(kind)}</strong>${html}</aside>`;
    }

    // constrain authored media widths to positive pixel values
    const width = attributes.width;
    if (width != undefined && !/^[1-9][0-9]*$/.test(width)) {
        throw new Error(`invalid media width in ${context.slug}: ${width}`);
    }
    const sizing = width == undefined ? "" : ` style="--figure-width: ${width}px"`;

    if (name === "figure") {
        const src = requireAttribute(attributes, "src", context.slug, name);
        const alt = requireAttribute(attributes, "alt", context.slug, name);
        const caption = attributes.caption ?? body.trim();
        const url = resolveLink(src, context);
        const label = nextFigureLabel(counters);

        return `<figure class="markdown-figure"${sizing}><div class="markdown-figure__frame"><img alt="${escapeAttribute(alt)}" src="${escapeAttribute(url)}" loading="lazy" decoding="async"></div><figcaption><span>${label}</span><span>${parser.parseInline(caption)}</span></figcaption></figure>`;
    }

    if (name === "video") {
        const src = requireAttribute(attributes, "src", context.slug, name);
        const title = requireAttribute(attributes, "title", context.slug, name);
        const caption = attributes.caption ?? body.trim();
        const url = resolveLink(src, context);
        const youtube = youtubeVideo(src);
        const poster =
            attributes.poster == undefined ? undefined : resolveLink(attributes.poster, context);
        const link = `<a href="${escapeAttribute(url)}">${escapeHtml(title)}</a>`;
        let player;

        // load the YouTube player only when the reader activates its preview
        if (youtube != undefined) {
            const preview = poster ?? `https://i.ytimg.com/vi/${youtube.id}/hqdefault.jpg`;
            player = `<a class="markdown-video__preview" href="${escapeAttribute(url)}" data-video-src="${escapeAttribute(youtube.embed)}" data-video-title="${escapeAttribute(title)}" aria-label="${escapeAttribute(`Play ${title}`)}"><img src="${escapeAttribute(preview)}" alt="" loading="lazy" decoding="async"><span aria-hidden="true"><svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M8 5v14l11-7z"/></svg></span></a>`;
        } else {
            const extension = extname(src.split(/[?#]/, 1)[0]).toLowerCase();
            if (![".mp4", ".webm", ".ogv"].includes(extension)) {
                throw new Error(`unsupported video in ${context.slug}: ${src}`);
            }
            const preview = poster == undefined ? "" : ` poster="${escapeAttribute(poster)}"`;
            player = `<video controls playsinline preload="none" aria-label="${escapeAttribute(title)}" src="${escapeAttribute(url)}"${preview}>${link}</video>`;
        }

        const source = youtube == undefined ? "Open video" : "YouTube";

        return `<figure class="markdown-figure markdown-video"${sizing}><div class="markdown-figure__frame">${player}</div><figcaption><span>${escapeHtml(title)}${caption === "" ? "" : `<br>${parser.parseInline(caption)}`}</span><a href="${escapeAttribute(url)}" aria-label="${escapeAttribute(`Open ${title}${youtube == undefined ? "" : " on YouTube"}`)}">${source} ↗</a></figcaption></figure>`;
    }

    throw new Error(`unknown directive in ${context.slug}: ${name}`);
}

/// Resolve supported YouTube URLs into a privacy-enhanced player URL.
function youtubeVideo(source: string) {
    if (!/^https?:\/\//i.test(source)) {
        return;
    }

    const url = new URL(source);
    const host = url.hostname.replace(/^(www|m)\./, "");
    if (!["youtube.com", "youtube-nocookie.com", "youtu.be"].includes(host)) {
        return;
    }

    const segments = url.pathname.split("/").filter(Boolean);
    const id =
        host === "youtu.be"
            ? segments[0]
            : url.pathname === "/watch"
                ? url.searchParams.get("v")
                : ["embed", "shorts"].includes(segments[0])
                    ? segments[1]
                    : undefined;
    if (id == undefined || !/^[\w-]{11}$/.test(id)) {
        throw new Error(`invalid YouTube video: ${source}`);
    }

    // preserve an optional start time in seconds or YouTube's hour/minute/second form
    const embed = new URL(`https://www.youtube-nocookie.com/embed/${id}`);
    const start = url.searchParams.get("start") ?? url.searchParams.get("t");
    if (start != null) {
        const duration = /^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?$/.exec(start);
        if (!/^\d+$/.test(start) && (duration == null || start === "")) {
            throw new Error(`invalid YouTube start time: ${source}`);
        }
        const seconds = /^\d+$/.test(start)
            ? Number(start)
            : Number(duration![1] ?? 0) * 3600 +
            Number(duration![2] ?? 0) * 60 +
            Number(duration![3] ?? 0);
        embed.searchParams.set("start", String(seconds));
    }
    embed.searchParams.set("autoplay", "1");

    return { id, embed: embed.href };
}

/// Require one non-empty directive attribute.
function requireAttribute(attributes: Record<string, string>, attribute: string, slug: string | undefined, directive: string) {
    const value = attributes[attribute];
    if (value == undefined || value === "") {
        throw new Error(`missing ${attribute} in ${slug} ${directive} directive`);
    }

    return value;
}

/// Render one titled and highlighted code listing.
function renderCode(token: Tokens.Code, counters: { figure: number; }) {
    const fence = parseCodeFence(token.lang ?? "");
    const language = fence.language;

    if (language === "diagram") {
        const label = nextFigureLabel(counters);
        const caption = fence.caption ?? fence.title ?? "diagram";

        return `<figure class="markdown-diagram"><figcaption><span>${label}</span>${escapeHtml(caption)}</figcaption><pre tabindex="0"><code>${escapeHtml(token.text)}</code></pre></figure>`;
    }

    const highlighted = highlightCode(token.text, language);
    const caption = fence.caption ?? fence.title;
    const format = codeFormat(language);
    return renderListing(highlighted, { title: caption, detail: format, label: language || "Code" });
}

/// Convert a fence language into its visible file format.
function codeFormat(language: string) {
    const extension = codeExtensions[language] ?? language;

    return extension === "" || extension === "text" ? "text" : `.${extension}`;
}

/// Allocate the next figure label.
function nextFigureLabel(counters: { figure: number; }) {
    counters.figure += 1;

    return `Figure ${counters.figure}`;
}

/// Parse the language and attributes from a code fence.
function parseCodeFence(language: string) {
    const [head, ...tail] = language.trim().split(/\s+/);
    const attributes = parseAttributes(tail.join(" "));
    const [name, qualifier] = (head ?? "").replace(/^\./, "").split(":", 2);
    const shorthandTitle = qualifier === "unchecked" ? undefined : qualifier;

    return {
        caption: attributes.caption,
        language: name,
        title: attributes.title ?? shorthandTitle,
    };
}

/// Resolve and validate one content link.
function resolveLink(href: string, context: MarkdownContext) {
    if (isExternalLink(href)) {
        return href;
    }

    if (href.startsWith("#")) {
        return resolveHeadingLink(href, context);
    }

    const markdownLink = href.match(/^(.+\.md)(#[a-z0-9-]+)?$/);
    if (markdownLink != undefined) {
        const target = resolve(context.markdownDirectory!, markdownLink[1]);

        const content = context.sourceRoutes!.get(target);
        if (content == undefined) {
            throw new Error(
                `markdown link leaves its content collection in ${context.slug}: ${href}`,
            );
        }

        const fragment = markdownLink[2] ?? "";
        const id = fragment.slice(1);
        if (id !== "" && !content.headings.has(id)) {
            throw new Error(`missing heading in ${context.slug}: ${href}`);
        }

        return `${content.route}${fragment}`;
    }

    if (href.startsWith("./") || href.startsWith("../") || isBareAssetLink(href)) {
        return resolveAsset(href, context);
    }

    return href;
}

/// Validate one local heading link.
function resolveHeadingLink(href: string, context: MarkdownContext) {
    if (!context.ownHeadings!.has(href.slice(1))) {
        throw new Error(`missing local heading in ${context.slug}: ${href}`);
    }

    return href;
}

/// Return whether a link carries an absolute URI scheme.
function isExternalLink(href: string) {
    return /^[a-z][a-z0-9+.-]*:/i.test(href);
}

/// Return whether a relative link looks like an asset path.
function isBareAssetLink(href: string) {
    if (href.startsWith("/") || href.startsWith("#")) {
        return false;
    }

    const path = href.split(/[?#]/, 1)[0];

    return extname(path) !== "";
}

/// Resolve one content asset into a build-time placeholder.
function resolveAsset(href: string, context: MarkdownContext) {
    const target = resolve(context.markdownDirectory!, href);
    const contentDirectory =
        context.kind === "document" ? context.documentDirectory : context.markdownDirectory;
    const relativeTarget = relative(contentDirectory!, target);

    if (relativeTarget.startsWith("..") || relativeTarget === "") {
        throw new Error(`asset escapes content directory in ${context.slug}: ${href}`);
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
        placeholder: `__CONTENT_ASSET_${index}__`,
    };
    context.assets.push(asset);

    return asset.placeholder;
}

/// Extract real headings while ignoring fenced examples.
export function headingsFor(markdown: string) {
    const slugs = new Map();
    const headings = [];

    for (const token of marked.lexer(markdown, { gfm: true })) {
        if (token.type !== "heading") {
            continue;
        }

        headings.push({
            depth: token.depth,
            id: uniqueSlug(token.text, slugs),
            text: token.text.replace(/`/g, ""),
        });
    }

    return headings;
}

/// Allocate one stable GitHub-style heading identifier.
function uniqueSlug(text: string, slugs: Map<string, number>) {
    const base = text
        .toLowerCase()
        .replace(/`([^`]+)`/g, "$1")
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "");
    const count = slugs.get(base) ?? 0;
    slugs.set(base, count + 1);

    return count === 0 ? base : `${base}-${count + 1}`;
}

/// Split Markdown into independently searchable heading sections.
export function searchSectionsFor(markdown: string) {
    const sections = [];
    const slugs = new Map();
    let section: SearchSectionSource | undefined;

    for (const token of marked.lexer(markdown, { gfm: true })) {
        if (token.type === "heading" && token.depth <= 3) {
            if (section != undefined) {
                sections.push(renderSearchSection(section));
            }

            section = {
                depth: token.depth,
                id: uniqueSlug(token.text, slugs),
                source: [],
                title: token.text.replace(/`/g, ""),
            };
            continue;
        }

        section?.source.push(token.raw);
    }

    if (section != undefined) {
        sections.push(renderSearchSection(section));
    }

    return sections;
}

/// Normalize one accumulated search section.
function renderSearchSection(section: SearchSectionSource) {
    return {
        depth: section.depth,
        id: section.id,
        text: searchTextFor(section.source.join("")),
        title: section.title,
    };
}
