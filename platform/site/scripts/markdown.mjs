import { marked } from "marked";
import { existsSync } from "node:fs";
import { extname, relative, resolve } from "node:path";

import { highlightCode } from "./highlight.mjs";
import { searchTextFor } from "./text.mjs";

const codeExtensions = {
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
export function parseFrontmatter(source, file) {
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

    return metadata;
}

/// Parse one scalar or array frontmatter value.
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

/// Remove matching quotes from one metadata scalar.
function parseQuotedString(value) {
    if (
        (value.startsWith("\"") && value.endsWith("\"")) ||
        (value.startsWith("'") && value.endsWith("'"))
    ) {
        return value.slice(1, -1);
    }

    return value;
}

/// Require one non-empty string metadata field.
export function requireString(metadata, field, file) {
    if (typeof metadata[field] !== "string" || metadata[field] === "") {
        throw new Error(`missing ${field} in ${file}`);
    }
}

/// Render trusted Markdown with collection-aware links and assets.
export function renderMarkdown(markdown, context) {
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
    renderer.table = (token) => renderTable(token, renderer);
    renderer.code = (token) => renderCode(token, counters);
    renderer.blockquote = (token) => {
        // render GitHub alerts using the existing callout presentation
        const alert = /^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*\n/.exec(token.text);
        if (alert != null) {
            const kind = alert[1].toLowerCase();
            const body = marked.parse(token.text.slice(alert[0].length), { gfm: true, renderer });

            return `<aside class="markdown-callout" data-kind="${kind}"><strong>${kind}</strong>${body}</aside>`;
        }

        return `<blockquote>\n${marked.parse(token.text, { gfm: true, renderer })}</blockquote>\n`;
    };

    const withDirectives = renderDirectives(footnotes.markdown, context, renderer, counters);
    const html = marked.parse(withDirectives, { gfm: true, renderer });
    const notes = renderFootnotes(footnotes.notes, renderer);

    return `${html}${notes}`;
}

/// Render one responsive GFM table.
function renderTable(token, renderer) {
    const labels = token.header.map((cell) => searchTextFor(cell.text));
    const header = token.header.map((cell) => renderer.tablecell(cell)).join("");
    const head = renderer.tablerow({ text: header });
    const rows = token.rows.map((row) => {
        const cells = row.map((cell, index) => {
            const label = escapeAttribute(labels[index] ?? "");

            const alignment = cell.align == null ? "" : ` align="${cell.align}"`;
            const content = marked.parseInline(cell.text, { renderer });

            return `<td data-label="${label}"${alignment}><div>${content}</div></td>`;
        }).join("");

        return renderer.tablerow({ text: cells });
    }).join("");
    const body = rows === "" ? "" : `<tbody>${rows}</tbody>`;

    return `<div class="markdown-table" tabindex="0"><table><thead>${head}</thead>${body}</table></div>`;
}

/// Extract footnote definitions and replace their references.
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

        return `<sup class="markdown-footnote-ref" id="fnref-${reference}"><a href="#fn-${noteId}">${escapeHtml(String(note.number))}</a></sup><span class="markdown-margin-note" aria-hidden="true"><span>${escapeHtml(String(note.number))}</span>${marked.parseInline(note.text)}</span>`;
    });

    return { markdown: rewritten, notes };
}

/// Render collected footnotes below the article.
function renderFootnotes(notes, renderer) {
    if (notes.length === 0) {
        return "";
    }

    const items = notes
        .map((note) => {
            const body = marked.parseInline(note.text, { renderer });
            const id = escapeAttribute(note.id);

            return `<li id="fn-${id}"><span class="markdown-footnote-number">${note.number}</span><span>${body} <a class="markdown-footnote-back" href="#fnref-${id}">back</a></span></li>`;
        })
        .join("");

    return `<section class="markdown-footnotes"><h2>notes</h2><ol>${items}</ol></section>`;
}

/// Expand supported block directives before Markdown parsing.
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

/// Parse quoted and unquoted directive or fence attributes.
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

/// Render one supported Markdown directive.
function renderDirective(name, attributes, body, context, renderer, counters) {
    if (name === "callout") {
        const kind = attributes.kind ?? "note";
        const html = marked.parse(body, { gfm: true, renderer });

        return `<aside class="markdown-callout" data-kind="${escapeAttribute(kind)}"><strong>${escapeHtml(kind)}</strong>${html}</aside>`;
    }

    if (name === "figure") {
        const src = requireAttribute(attributes, "src", context.slug, name);
        const alt = requireAttribute(attributes, "alt", context.slug, name);
        const caption = attributes.caption ?? body.trim();
        const url = resolveAsset(src, context);
        const label = nextFigureLabel(counters, "figure");

        return `<figure class="markdown-figure"><img alt="${escapeAttribute(alt)}" src="${escapeAttribute(url)}"><figcaption><span>${label}</span>${marked.parseInline(caption, { renderer })}</figcaption></figure>`;
    }

    throw new Error(`unknown directive in ${context.slug}: ${name}`);
}

/// Require one non-empty directive attribute.
function requireAttribute(attributes, attribute, slug, directive) {
    const value = attributes[attribute];
    if (value == undefined || value === "") {
        throw new Error(`missing ${attribute} in ${slug} ${directive} directive`);
    }

    return value;
}

/// Render one titled and highlighted code listing.
function renderCode(token, counters) {
    const fence = parseCodeFence(token.lang ?? "");
    const language = fence.language;

    if (language === "diagram") {
        const label = nextFigureLabel(counters, "figure");
        const caption = fence.caption ?? fence.title ?? "diagram";

        return `<figure class="markdown-diagram"><figcaption><span>${label}</span>${escapeHtml(caption)}</figcaption><pre tabindex="0"><code>${escapeHtml(token.text)}</code></pre></figure>`;
    }

    const highlighted = highlightCode(token.text, language);
    const caption = fence.caption ?? fence.title;
    const format = codeFormat(language);
    const code = renderCodeBody(highlighted);

    const heading = caption == undefined ? "" : `<figcaption data-publication-caption><span class="markdown-code__title" data-publication-caption-title>${escapeHtml(caption)}</span><span class="markdown-code__format">${escapeHtml(format)}</span></figcaption>`;

    return `<figure class="markdown-code" data-publication-listing>${heading}<pre data-publication-body tabindex="0" aria-label="${escapeAttribute(caption ?? language ?? "Code")}">${code}</pre></figure>`;
}

/// Convert a fence language into its visible file format.
function codeFormat(language) {
    const extension = codeExtensions[language] ?? language;

    return extension === "" || extension === "text" ? "text" : `.${extension}`;
}

/// Allocate the next figure label.
function nextFigureLabel(counters, kind) {
    counters.figure += 1;

    return `${kind} ${counters.figure}`;
}

/// Parse the language and attributes from a code fence.
function parseCodeFence(language) {
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

/// Add stable line structure and gutters to highlighted code.
function renderCodeBody(highlighted) {
    const lines = highlighted.split("\n");
    const rows = lines
        .map((line, index) => {
            const text = line === "" ? " " : line;

            return `<span class="markdown-code-line" data-publication-line><span class="markdown-code-gutter" data-publication-gutter>${index + 1}</span><span class="markdown-code-text" data-publication-code>${text}</span></span>`;
        })
        .join("");

    return `<code class="markdown-code-lines" data-publication-lines>${rows}</code>`;
}

/// Resolve and validate one content link.
function resolveLink(href, context) {
    if (isExternalLink(href)) {
        return href;
    }

    if (href.startsWith("#")) {
        return resolveHeadingLink(href, context);
    }

    const markdownLink = href.match(/^(.+\.md)(#[a-z0-9-]+)?$/);
    if (markdownLink != undefined) {
        const target = resolve(context.markdownDirectory, markdownLink[1]);

        if (!existsSync(target)) {
            throw new Error(`missing markdown link in ${context.slug}: ${href}`);
        }

        const content = context.sourceRoutes.get(target);
        if (content == undefined) {
            throw new Error(`markdown link leaves its content collection in ${context.slug}: ${href}`);
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
function resolveHeadingLink(href, context) {
    if (!context.ownHeadings.has(href.slice(1))) {
        throw new Error(`missing local heading in ${context.slug}: ${href}`);
    }

    return href;
}

/// Return whether a link carries an absolute URI scheme.
function isExternalLink(href) {
    return /^[a-z][a-z0-9+.-]*:/i.test(href);
}

/// Return whether a relative link looks like an asset path.
function isBareAssetLink(href) {
    if (href.startsWith("/") || href.startsWith("#")) {
        return false;
    }

    const path = href.split(/[?#]/, 1)[0];

    return extname(path) !== "";
}

/// Resolve one content asset into a build-time placeholder.
function resolveAsset(href, context) {
    const target = resolve(context.markdownDirectory, href);
    const contentDirectory = context.kind === "document"
        ? context.documentDirectory
        : context.markdownDirectory;
    const relativeTarget = relative(contentDirectory, target);

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
export function headingsFor(markdown) {
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

/// Split Markdown into independently searchable heading sections.
export function searchSectionsFor(markdown) {
    const sections = [];
    const slugs = new Map();
    let section;

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
function renderSearchSection(section) {
    return {
        depth: section.depth,
        id: section.id,
        text: searchTextFor(section.source.join("")),
        title: section.title,
    };
}

/// Escape text for an HTML text node.
function escapeHtml(value) {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll("\"", "&quot;");
}

/// Escape text for an HTML attribute.
function escapeAttribute(value) {
    return escapeHtml(value).replaceAll("'", "&#39;");
}
