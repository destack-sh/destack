import type { Element, ElementContent, Root, RootContent } from "hast";
import type * as mdast from "mdast";
import { existsSync } from "node:fs";
import { extname, relative, resolve } from "node:path";
import remarkDirective from "remark-directive";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";
import { captured, parseAttributes } from "./directives.ts";
import { highlightCode } from "./highlight.ts";
import { searchTextFor } from "./text.ts";

/** File extensions shown for fence languages whose name differs from their extension. */
const CODE_EXTENSIONS: Record<string, string> = {
    bash: "sh",
    javascript: "js",
    plaintext: "txt",
    rust: "rs",
    shell: "sh",
    typescript: "ts",
};

/** The kinds of GitHub alert a quotation can open with. */
const ALERT = /^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*/u;

/** The video files a video directive may play directly. */
const VIDEO_EXTENSIONS = new Set([".mp4", ".webm", ".ogv"]);

/** An asset collected while rendering a content page. */
export type ContentAsset = { importName: string; path: string; placeholder: string };
/** Link and asset resolution for one Markdown source. */
export type MarkdownContext = {
    assets: ContentAsset[];
    documentDirectory?: string;
    markdownDirectory?: string;
    kind?: string;
    ownHeadings?: Set<string>;
    route?: string;
    slug?: string;
    sourceRoutes?: Map<string, { route: string; headings: Set<string> }>;
};

/** The parser of the site's Markdown: GitHub's flavour with block directives. */
const parser = unified().use(remarkParse).use(remarkGfm).use(remarkDirective);

/** The converter of a Markdown tree to an HTML tree, writing footnotes as GitHub does under a visible heading. */
const converter = unified().use(remarkRehype, {
    clobberPrefix: "",
    footnoteLabel: "Notes",
    footnoteLabelProperties: {},
});

/** Split one content source into metadata and Markdown. */
export function parseFrontmatter(source: string, file: string) {
    // split the delimited metadata block from the Markdown body
    const match = /^---\n([\s\S]*?)\n---\n([\s\S]*)$/u.exec(source);
    if (match == undefined) {
        throw new Error(`missing frontmatter: ${file}`);
    }

    // parse the metadata block
    const metadata = parseMetadata(captured(match, 1), file);

    return {
        markdown: captured(match, 2).trimEnd(),
        metadata,
    };
}

/** Parse the supported frontmatter subset. */
function parseMetadata(source: string, file: string) {
    // read one field per line, skipping blank lines
    const metadata: Record<string, unknown> = {};
    for (const line of source.split("\n")) {
        if (line.trim() === "") {
            continue;
        }

        const match = /^([a-zA-Z][a-zA-Z0-9]*):\s*(.*)$/u.exec(line);
        if (match == undefined) {
            throw new Error(`invalid frontmatter line in ${file}: ${line}`);
        }

        metadata[captured(match, 1)] = parseMetadataValue(captured(match, 2));
    }

    return metadata;
}

/** Parse one scalar or array frontmatter value. */
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

/** Remove matching quotes from one metadata scalar. */
function parseQuotedString(value: string) {
    if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
    ) {
        return value.slice(1, -1);
    }

    return value;
}

/** Require one non-empty string metadata field. */
export function requireString<Field extends string>(
    metadata: Record<string, unknown>,
    field: Field,
    file: string,
): asserts metadata is Record<string, unknown> & Record<Field, string> {
    if (typeof metadata[field] !== "string" || metadata[field] === "") {
        throw new Error(`missing ${field} in ${file}`);
    }
}

/** Render trusted Markdown into its HTML tree, with collection-aware links and assets. */
export function renderMarkdown(markdown: string, context: MarkdownContext): Root {
    // parse the page and shape its Markdown tree
    const tree = parseMarkdown(markdown);
    const counters = { figure: 0 };
    shapeMarkdown(tree, context, new Map());

    // convert it to HTML and shape the elements Markdown has no words for
    const html = converter.runSync(tree);
    shapeHtml(html, context, counters);

    return withoutParserData(html);
}

/** Parse Markdown into its tree, accepting directives written with bare attributes. */
function parseMarkdown(markdown: string): mdast.Root {
    // brace a directive's bare attributes, `:::figure width="600"` as `:::figure{width="600"}`
    const braced = markdown.replace(
        /^:::(\w+)[ \t]+(\S[^\n]*?)[ \t]*$/gmu,
        (_line, name: string, attributes: string) =>
            attributes.startsWith("{") ? `:::${name}${attributes}` : `:::${name}{${attributes}}`,
    );

    return parser.parse(braced);
}

/** Shape a Markdown tree in place: resolve links and assets, name headings, and mark alerts, credits, directives and fences. */
function shapeMarkdown(
    parent: { children: mdast.RootContent[] },
    context: MarkdownContext,
    slugs: Map<string, number>,
): void {
    for (let index = 0; index < parent.children.length; index += 1) {
        const node = parent.children[index];
        if (node === undefined) {
            throw new Error(`missing Markdown node ${index}`);
        }

        // name a heading after its text, leaving out a document's title, which its page header shows
        if (node.type === "heading") {
            const id = uniqueSlug(textOf(node), slugs);
            if (context.kind === "document" && node.depth === 1) {
                parent.children.splice(index, 1);
                index -= 1;
                continue;
            }
            node.data = { ...node.data, hProperties: { id } };
        }
        // resolve links, opening external ones in a new tab, and load images lazily
        else if (node.type === "link" || node.type === "definition") {
            node.url = resolveLink(node.url, context);
            if (node.type === "link" && isExternalLink(node.url)) {
                node.data = {
                    ...node.data,
                    hProperties: { rel: ["external", "noopener", "noreferrer"], target: "_blank" },
                };
            }
        } else if (node.type === "image") {
            node.url = resolveLink(node.url, context);
            node.data = { ...node.data, hProperties: { loading: "lazy", decoding: "async" } };
        }
        // write a quotation as a GitHub alert, or as a figure crediting its source
        else if (node.type === "blockquote") {
            shapeQuotation(node);
        }
        // mark a fence with its title, format and language for its listing
        else if (node.type === "code") {
            const fence = parseCodeFence(`${node.lang ?? ""} ${node.meta ?? ""}`);
            node.data = {
                ...node.data,
                hProperties: {
                    dataLanguage: fence.language,
                    dataTitle: fence.caption ?? fence.title,
                    dataFormat:
                        fence.language === "mermaid" || fence.language === "diagram"
                            ? undefined
                            : codeFormat(fence.language),
                },
            };
        }
        // mark a directive as a figure of its kind, carrying its attributes
        else if (node.type === "containerDirective") {
            shapeDirective(node, context);
        }
        // refuse a leaf directive, which the site has none of
        else if (node.type === "leafDirective") {
            throw new Error(`unknown directive in ${context.slug}: ${node.name}`);
        }
        // write back the text an inline colon made a directive, such as the `:draft` of `Note:draft`
        else if (node.type === "textDirective") {
            parent.children.splice(
                index,
                1,
                { type: "text", value: `:${node.name}` },
                ...node.children,
            );
            continue;
        }

        // shape the node's children
        if ("children" in node) {
            shapeMarkdown(node, context, slugs);
        }
    }
}

/** Write a quotation that opens with `[!NOTE]` as a GitHub alert, or one that ends with `— source` as a figure crediting it. */
function shapeQuotation(quotation: mdast.Blockquote): void {
    // open an alert with its kind as title
    const first = quotation.children[0];
    const opening = first?.type === "paragraph" ? first.children[0] : undefined;
    const alert = opening?.type === "text" ? ALERT.exec(opening.value) : null;
    if (first?.type === "paragraph" && opening?.type === "text" && alert !== null) {
        const kind = captured(alert, 1).toLowerCase();
        opening.value = opening.value.slice(alert[0].length);
        const title: mdast.Paragraph = {
            type: "paragraph",
            data: { hProperties: { className: ["markdown-alert-title"] } },
            children: [{ type: "text", value: `${kind.charAt(0).toUpperCase()}${kind.slice(1)}` }],
        };
        quotation.children.unshift(title);
        quotation.data = {
            hName: "div",
            hProperties: { className: ["markdown-alert", `markdown-alert-${kind}`] },
        };

        return;
    }

    // set a final paragraph that opens with a dash outside the quoted words, as the figure's caption
    const last = quotation.children.at(-1);
    const credit = last?.type === "paragraph" ? last.children[0] : undefined;
    if (
        quotation.children.length > 1 &&
        last?.type === "paragraph" &&
        credit?.type === "text" &&
        credit.value.startsWith("— ")
    ) {
        const quoted: mdast.Blockquote = {
            type: "blockquote",
            children: quotation.children.slice(0, -1),
        };
        const caption: mdast.Paragraph = {
            type: "paragraph",
            data: { hName: "figcaption" },
            children: last.children,
        };
        quotation.children = [quoted, caption];
        quotation.data = { hName: "figure", hProperties: { dataKind: "quote" } };
    }
}

/** Mark a figure or video directive as a figure of its kind, its caption as the figure's caption and its attributes as data. */
function shapeDirective(
    node: Extract<mdast.RootContent, { type: "containerDirective" }>,
    context: MarkdownContext,
): void {
    // read the attributes, constraining an authored width to positive pixels
    const attributes = Object.fromEntries(
        Object.entries(node.attributes ?? {}).flatMap(([key, value]) =>
            typeof value === "string" ? [[key, value]] : [],
        ),
    );
    const width = attributes["width"];
    if (width !== undefined && !/^[1-9][0-9]*$/u.test(width)) {
        throw new Error(`invalid media width in ${context.slug}: ${width}`);
    }

    // take the caption from its attribute or the directive's body
    const caption: mdast.PhrasingContent[] =
        attributes["caption"] === undefined
            ? node.children.flatMap((child) => (child.type === "paragraph" ? child.children : []))
            : [{ type: "text", value: attributes["caption"] }];
    const figcaption: mdast.Paragraph = {
        type: "paragraph",
        data: { hName: "figcaption" },
        children: caption,
    };

    // carry an image's source and description, resolved
    if (node.name === "figure") {
        const source = resolveLink(
            requireAttribute(attributes, "src", context.slug, node.name),
            context,
        );
        node.data = {
            hName: "figure",
            hProperties: {
                dataKind: "image",
                dataWidth: width,
                dataSrc: source,
                dataAlt: requireAttribute(attributes, "alt", context.slug, node.name),
            },
        };
        node.children = [figcaption];
    }
    // carry a video's file or YouTube player, its title and its poster, resolved
    else if (node.name === "video") {
        const source = requireAttribute(attributes, "src", context.slug, node.name);
        const youtube = youtubeVideo(source);
        if (youtube === undefined && !VIDEO_EXTENSIONS.has(extname(pathOf(source)).toLowerCase())) {
            throw new Error(`unsupported video in ${context.slug}: ${source}`);
        }
        const poster =
            attributes["poster"] === undefined
                ? youtube && `https://i.ytimg.com/vi/${youtube.id}/hqdefault.jpg`
                : resolveLink(attributes["poster"], context);
        node.data = {
            hName: "figure",
            hProperties: {
                dataKind: "video",
                dataWidth: width,
                dataSrc: resolveLink(source, context),
                dataEmbed: youtube?.embed,
                dataPoster: poster,
                dataTitle: requireAttribute(attributes, "title", context.slug, node.name),
            },
        };
        node.children = [figcaption];
    }
    // refuse an unknown directive
    else {
        throw new Error(`unknown directive in ${context.slug}: ${node.name}`);
    }
}

/** Shape the HTML tree in place: highlight listings, number figures and plain diagrams, and label table cells by their column. */
function shapeHtml(
    parent: { children: ElementContent[] } | Root,
    context: MarkdownContext,
    counters: { figure: number },
): void {
    for (let index = 0; index < parent.children.length; index += 1) {
        const node = parent.children[index];
        if (node?.type !== "element") {
            continue;
        }

        // number image and video figures
        if (
            node.tagName === "figure" &&
            (node.properties["dataKind"] === "image" || node.properties["dataKind"] === "video")
        ) {
            counters.figure += 1;
            node.properties["dataLabel"] = `Figure ${counters.figure}`;
        }
        // highlight a listing, keep a Mermaid diagram's source, and set a plain diagram in a numbered figure
        else if (node.tagName === "pre") {
            const code = node.children[0];
            if (code?.type !== "element") {
                throw new Error(`a listing in ${context.slug} has no code`);
            }
            const {
                dataLanguage: language,
                dataTitle,
                dataFormat,
                ...properties
            } = code.properties;
            code.properties = properties;
            node.properties = { ...node.properties, dataLanguage: language, dataTitle, dataFormat };
            if (typeof language !== "string") {
                throw new Error(`a listing in ${context.slug} has no language`);
            }

            // highlight the code, or keep a diagram's source
            const source = textOf(code);
            if (language === "diagram") {
                counters.figure += 1;
                const caption = node.properties["dataTitle"] ?? "diagram";
                parent.children[index] = {
                    type: "element",
                    tagName: "figure",
                    properties: { dataKind: "diagram", dataLabel: `Figure ${counters.figure}` },
                    children: [
                        {
                            type: "element",
                            tagName: "figcaption",
                            properties: {},
                            children: [{ type: "text", value: String(caption) }],
                        },
                        node,
                    ],
                };
            } else if (language !== "mermaid") {
                code.children = splitLines(highlightCode(source.replace(/\n$/u, ""), language)).map(
                    (line): Element => ({
                        type: "element",
                        tagName: "span",
                        properties: {},
                        children: line,
                    }),
                );
            }
            continue;
        }
        // label each body cell with its column's header, for rows stacked on narrow screens
        else if (node.tagName === "table") {
            labelCells(node);
            continue;
        }

        shapeHtml(node, context, counters);
    }
}

/** Split highlighted code into its lines, carrying each span that crosses a line break into both lines. */
function splitLines(nodes: readonly ElementContent[]): ElementContent[][] {
    // start the first line, and add each piece to the line being written
    const lines: ElementContent[][] = [[]];
    const append = (node: ElementContent) => lines.at(-1)?.push(node);
    for (const node of nodes) {
        // break text at its line breaks
        if (node.type === "text") {
            node.value.split("\n").forEach((part, index) => {
                if (index > 0) {
                    lines.push([]);
                }
                if (part !== "") {
                    append({ type: "text", value: part });
                }
            });
        }
        // carry a span into every line its children reach
        else if (node.type === "element") {
            splitLines(node.children).forEach((part, index) => {
                if (index > 0) {
                    lines.push([]);
                }
                if (part.length > 0) {
                    append({ ...node, children: part });
                }
            });
        }
    }

    return lines;
}

/** Label each body cell of a table with the text of its column's header. */
function labelCells(table: Element): void {
    // read the header texts, then label the cells of every body row
    const sections = table.children.filter((child): child is Element => child.type === "element");
    const head = sections.find((section) => section.tagName === "thead");
    const labels = (
        head?.children.find((row): row is Element => row.type === "element")?.children ?? []
    )
        .filter((cell): cell is Element => cell.type === "element")
        .map((cell) => textOf(cell));
    for (const body of sections.filter((section) => section.tagName === "tbody")) {
        for (const row of body.children) {
            if (row.type !== "element") {
                continue;
            }
            row.children
                .filter((cell): cell is Element => cell.type === "element")
                .forEach((cell, column) => {
                    cell.properties["dataLabel"] = labels[column] ?? "";
                });
        }
    }
}

/** Return a tree without the source positions and node data the parser recorded, which the page never reads. */
function withoutParserData(tree: Root): Root {
    const copy = structuredClone(tree);
    stripParserData(copy);

    return copy;
}

/** Remove the source position and node data of a node and its descendants. */
function stripParserData(node: Root | RootContent): void {
    delete node.position;
    delete node.data;
    if ("children" in node) {
        node.children.forEach(stripParserData);
    }
}

/** A Markdown or HTML node with text or children. */
type TextNode = {
    readonly type: string;
    readonly value?: unknown;
    readonly children?: readonly TextNode[];
};

/** Read the text of a Markdown or HTML node, its code included. */
function textOf(node: TextNode): string {
    if (typeof node.value === "string") {
        return node.value;
    }

    return (node.children ?? []).map(textOf).join("");
}

/** Resolve supported YouTube URLs into a privacy-enhanced player URL. */
function youtubeVideo(source: string) {
    if (!/^https?:\/\//iu.test(source)) {
        return;
    }

    // accept only YouTube hosts
    const url = new URL(source);
    const host = url.hostname.replace(/^(www|m)\./u, "");
    if (!["youtube.com", "youtube-nocookie.com", "youtu.be"].includes(host)) {
        return;
    }

    // read the video identifier from the watch, short or embed path
    const [first, second] = url.pathname.split("/").filter(Boolean);
    const id =
        host === "youtu.be"
            ? first
            : url.pathname === "/watch"
              ? url.searchParams.get("v")
              : first === "embed" || first === "shorts"
                ? second
                : undefined;
    if (id == undefined || !/^[\w-]{11}$/u.test(id)) {
        throw new Error(`invalid YouTube video: ${source}`);
    }

    // preserve an optional start time in seconds or YouTube's hour/minute/second form
    const embed = new URL(`https://www.youtube-nocookie.com/embed/${id}`);
    const start = url.searchParams.get("start") ?? url.searchParams.get("t");
    if (start != null) {
        const duration = /^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?$/u.exec(start);
        let seconds = Number(start);
        if (!/^\d+$/u.test(start)) {
            if (duration == null || start === "") {
                throw new Error(`invalid YouTube start time: ${source}`);
            }
            seconds =
                Number(duration[1] ?? 0) * 3600 +
                Number(duration[2] ?? 0) * 60 +
                Number(duration[3] ?? 0);
        }
        embed.searchParams.set("start", String(seconds));
    }
    embed.searchParams.set("autoplay", "1");

    return { id, embed: embed.href };
}

/** Require one non-empty directive attribute. */
function requireAttribute(
    attributes: Record<string, string>,
    attribute: string,
    slug: string | undefined,
    directive: string,
) {
    const value = attributes[attribute];
    if (value == undefined || value === "") {
        throw new Error(`missing ${attribute} in ${slug} ${directive} directive`);
    }

    return value;
}

/** Require one link-resolution field of a Markdown context. */
function requireContext<Value>(value: Value | undefined, field: string, context: MarkdownContext) {
    if (value == undefined) {
        throw new Error(`missing ${field} for links in ${context.slug}`);
    }

    return value;
}

/** Convert a fence language into its visible file format. */
function codeFormat(language: string) {
    const extension = CODE_EXTENSIONS[language] ?? language;

    return extension === "" || extension === "text" ? "text" : `.${extension}`;
}

/** Parse the language and attributes from a code fence. */
function parseCodeFence(language: string) {
    // split the language and its title qualifier from the attributes
    const [head, ...tail] = language.trim().split(/\s+/u);
    const attributes = parseAttributes(tail.join(" "));
    const [name, qualifier] = (head ?? "").replace(/^\./u, "").split(":", 2);

    return {
        caption: attributes["caption"],
        language: name ?? "",
        title: attributes["title"] ?? qualifier,
    };
}

/** Resolve and validate one content link. */
function resolveLink(href: string, context: MarkdownContext) {
    if (isExternalLink(href)) {
        return href;
    }

    // check a link to a heading on this page
    if (href.startsWith("#")) {
        return resolveHeadingLink(href, context);
    }

    // route a link to another Markdown source and check its heading
    const markdownLink = /^(.+\.md)(#[a-z0-9-]+)?$/u.exec(href);
    if (markdownLink != undefined) {
        const target = resolve(
            requireContext(context.markdownDirectory, "markdown directory", context),
            captured(markdownLink, 1),
        );

        const content = requireContext(context.sourceRoutes, "source routes", context).get(target);
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

    // collect a relative asset
    if (href.startsWith("./") || href.startsWith("../") || isBareAssetLink(href)) {
        return resolveAsset(href, context);
    }

    return href;
}

/** Validate one local heading link. */
function resolveHeadingLink(href: string, context: MarkdownContext) {
    if (!requireContext(context.ownHeadings, "own headings", context).has(href.slice(1))) {
        throw new Error(`missing local heading in ${context.slug}: ${href}`);
    }

    return href;
}

/** Return whether a link carries an absolute URI scheme. */
function isExternalLink(href: string) {
    return /^[a-z][a-z0-9+.-]*:/iu.test(href);
}

/** Return whether a relative link looks like an asset path. */
function isBareAssetLink(href: string) {
    if (href.startsWith("/") || href.startsWith("#")) {
        return false;
    }

    return extname(pathOf(href)) !== "";
}

/** Strip the query and fragment from a link. */
function pathOf(href: string) {
    return href.split(/[?#]/u, 1)[0] ?? "";
}

/** Resolve one content asset into a build-time placeholder. */
function resolveAsset(href: string, context: MarkdownContext) {
    // resolve the asset against its content directory
    const target = resolve(
        requireContext(context.markdownDirectory, "markdown directory", context),
        href,
    );
    const contentDirectory = requireContext(
        context.kind === "document" ? context.documentDirectory : context.markdownDirectory,
        "content directory",
        context,
    );
    const relativeTarget = relative(contentDirectory, target);

    // require an existing non-Markdown file inside the content directory
    if (relativeTarget.startsWith("..") || relativeTarget === "") {
        throw new Error(`asset escapes content directory in ${context.slug}: ${href}`);
    } else if (!existsSync(target)) {
        throw new Error(`missing asset in ${context.slug}: ${href}`);
    } else if (extname(target) === ".md") {
        throw new Error(`asset points at markdown in ${context.slug}: ${href}`);
    }

    // reuse the placeholder of an asset collected before
    const existing = context.assets.find((asset) => asset.path === target);
    if (existing != undefined) {
        return existing.placeholder;
    }

    // collect a new asset under the next placeholder
    const index = context.assets.length;
    const asset = {
        importName: `asset${index}`,
        path: target,
        placeholder: `__CONTENT_ASSET_${index}__`,
    };
    context.assets.push(asset);

    return asset.placeholder;
}

/** Extract real headings while ignoring fenced examples. */
export function headingsFor(markdown: string) {
    // collect the headings with their unique slugs
    const slugs = new Map<string, number>();

    return parseMarkdown(markdown)
        .children.filter((node): node is mdast.Heading => node.type === "heading")
        .map((heading) => ({
            depth: heading.depth,
            id: uniqueSlug(textOf(heading), slugs),
            text: textOf(heading),
        }));
}

/** Split Markdown into independently searchable heading sections. */
export function searchSectionsFor(markdown: string) {
    // open a section at each heading up to level three and take the source beneath it
    const slugs = new Map<string, number>();
    const nodes = parseMarkdown(markdown).children;
    const sections = [];
    for (const [index, node] of nodes.entries()) {
        if (node.type !== "heading" || node.depth > 3) {
            continue;
        }
        const next = nodes
            .slice(index + 1)
            .find((later) => later.type === "heading" && later.depth <= 3);
        const start = node.position?.end.offset;
        const end = next?.position?.start.offset ?? markdown.length;
        if (start === undefined) {
            throw new Error("a heading has no source position");
        }
        sections.push({
            depth: node.depth,
            id: uniqueSlug(textOf(node), slugs),
            text: searchTextFor(markdown.slice(start, end)),
            title: textOf(node),
        });
    }

    return sections;
}

/** Allocate one stable GitHub-style heading identifier. */
function uniqueSlug(text: string, slugs: Map<string, number>) {
    // slugify the text, then number repeated slugs
    const base = text
        .toLowerCase()
        .replace(/`([^`]+)`/gu, "$1")
        .replace(/[^a-z0-9]+/gu, "-")
        .replace(/^-|-$/gu, "");
    const count = slugs.get(base) ?? 0;
    slugs.set(base, count + 1);

    return count === 0 ? base : `${base}-${count + 1}`;
}
