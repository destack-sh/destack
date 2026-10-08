import { createContext, useContext } from "solid-js";
import type { JSX } from "@solidjs/web";
import { For, Show } from "../solid/flow.ts";
import { omit } from "../solid/reactive.ts";
import { Head, Link, Meta, Title } from "./head.ts";

/** An image a page shares, as social previews show it. */
export interface MetadataImage {
    /** The image's address, absolute or against the metadata base. */
    readonly url: string;
    /** The width in pixels. */
    readonly width?: number;
    /** The height in pixels. */
    readonly height?: number;
    /** The text standing in for the image. */
    readonly alt?: string;
}

/** An icon a page names, with its sizes and media type. */
export interface MetadataIcon {
    /** The icon's address. */
    readonly url: string;
    /** The sizes, such as `32x32` or `any`. */
    readonly sizes?: string;
    /** The media type, such as `image/svg+xml`. */
    readonly type?: string;
}

/** A page's metadata: its title, description, addresses, previews, icons and indexing. */
export interface Metadata {
    /** The address relative URLs resolve against, such as `https://destack.sh`. */
    readonly metadataBase?: string;
    /** The title: plain, a template inner pages fill such as `%s | Destack` with its default, or one taken as is past enclosing templates. */
    readonly title?:
        | string
        | { readonly template?: string; readonly default: string; readonly absolute?: string };
    /** The description search results and previews show. */
    readonly description?: string;
    /** The application's name. */
    readonly applicationName?: string;
    /** The keywords. */
    readonly keywords?: readonly string[];
    /** The authors, each with a page about them. */
    readonly authors?: readonly { readonly name: string; readonly url?: string }[];
    /** The page's creator. */
    readonly creator?: string;
    /** The page's publisher. */
    readonly publisher?: string;
    /** The canonical path, the alternate representations by media type, and the translations by locale. */
    readonly alternates?: {
        readonly canonical?: string;
        readonly types?: Readonly<Record<string, string>>;
        readonly languages?: Readonly<Record<string, string>>;
    };
    /** The OpenGraph preview. */
    readonly openGraph?: {
        readonly type?: "website" | "article";
        readonly url?: string;
        readonly siteName?: string;
        readonly locale?: string;
        readonly title?: string;
        readonly description?: string;
        readonly images?: readonly (string | MetadataImage)[];
        readonly publishedTime?: string;
        readonly modifiedTime?: string;
        readonly authors?: readonly string[];
        readonly section?: string;
        readonly tags?: readonly string[];
    };
    /** The Twitter card, falling back to the OpenGraph preview. */
    readonly twitter?: {
        readonly card?: "summary" | "summary_large_image" | "app" | "player";
        readonly site?: string;
        readonly creator?: string;
        readonly title?: string;
        readonly description?: string;
        readonly images?: readonly (string | MetadataImage)[];
    };
    /** The icons: the favicons, the shortcut icon, the Apple touch icons and icons of other relations. */
    readonly icons?: {
        readonly icon?: string | readonly MetadataIcon[];
        readonly shortcut?: string;
        readonly apple?: string | readonly MetadataIcon[];
        readonly other?: readonly (MetadataIcon & { readonly rel: string })[];
    };
    /** The web app manifest's address. */
    readonly manifest?: string;
    /** Whether search engines index the page and follow its links. */
    readonly robots?: { readonly index?: boolean; readonly follow?: boolean };
    /** The site ownership tokens of search engines. */
    readonly verification?: {
        readonly google?: string;
        readonly yandex?: string;
        readonly other?: Readonly<Record<string, string>>;
    };
}

/** How a page lays out on devices. */
export interface Viewport {
    /** The layout width, `device-width` by default. */
    readonly width?: string;
    /** The initial zoom, 1 by default. */
    readonly initialScale?: number;
    /** The largest zoom the person may reach. */
    readonly maximumScale?: number;
    /** Whether the person may zoom. */
    readonly userScalable?: boolean;
    /** The browser interface color, plain or per color scheme. */
    readonly themeColor?: string | readonly { readonly media?: string; readonly color: string }[];
    /** The color schemes the page renders in, such as `light dark`. */
    readonly colorScheme?: string;
}

/** A head tag metadata resolves to. */
type HeadTag =
    | { readonly tag: "meta"; readonly attributes: Readonly<Record<string, string>> }
    | { readonly tag: "link"; readonly attributes: Readonly<Record<string, string>> };

/** The metadata enclosing pages give the pages inside them. */
const MetadataContext = createContext<Metadata>({});

/** Declare metadata defaults, such as a site's. */
export function defineMetadata(metadata: Metadata): Metadata {
    return metadata;
}

/** Declare how pages lay out on devices. */
export function defineViewport(viewport: Viewport): Viewport {
    return viewport;
}

/** The properties of a page's metadata: its fields and the pages inside it. */
export type MetadataProperties = Metadata & {
    /** The pages inside, which inherit the merged metadata. */
    readonly children?: JSX.Element;
};

/** Write a page's metadata into the document head, over the metadata of the pages enclosing it. */
export function Metadata(properties: MetadataProperties): JSX.Element {
    // merge each field over the enclosing metadata, keeping the enclosing title template
    const enclosing = useContext(MetadataContext);
    const merged = (): Metadata => ({ ...enclosing, ...fieldsOf(properties) });
    const title = () => titleOf(properties.title, enclosing.title);

    return (
        <MetadataContext value={inherited(merged(), templateOf(properties.title, enclosing.title))}>
            <Head>
                <Show when={title()}>{(text) => <Title>{text()}</Title>}</Show>
                <For each={tagsOf(merged(), title())}>
                    {(tag) =>
                        tag.tag === "meta" ? (
                            <Meta {...tag.attributes} />
                        ) : (
                            <Link {...tag.attributes} />
                        )
                    }
                </For>
            </Head>
            {properties.children}
        </MetadataContext>
    );
}

/** Write how pages lay out on devices into the document head. */
export function Viewport(properties: Viewport): JSX.Element {
    // write the layout and the browser interface's color per scheme
    const colors = () =>
        typeof properties.themeColor === "string"
            ? [{ color: properties.themeColor }]
            : (properties.themeColor ?? []);

    return (
        <Head>
            <Meta name="viewport" content={viewportContent(properties)} />
            <For each={colors()}>
                {(theme) => (
                    <Meta
                        name="theme-color"
                        content={theme.color}
                        {...(theme.media === undefined ? {} : { media: theme.media })}
                    />
                )}
            </For>
            <Show when={properties.colorScheme}>
                {(scheme) => <Meta name="color-scheme" content={scheme()} />}
            </Show>
        </Head>
    );
}

/** Resolve metadata into its head tags, addresses against its base and previews falling back to the page's. */
function tagsOf(metadata: Metadata, title: string | undefined): HeadTag[] {
    // resolve addresses and the previews' shared text
    const absolute = (path: string) =>
        metadata.metadataBase === undefined ? path : new URL(path, metadata.metadataBase).href;
    const graph = metadata.openGraph;
    const card = metadata.twitter;
    const sharedTitle = graph?.title ?? title;
    const sharedDescription = graph?.description ?? metadata.description;
    const canonical = metadata.alternates?.canonical;
    const pageUrl = graph?.url ?? canonical;

    return [
        // describe the page and who made it
        ...named("description", metadata.description),
        ...named("application-name", metadata.applicationName),
        ...named("keywords", metadata.keywords?.join(", ")),
        ...(metadata.authors ?? []).flatMap((author) => [
            ...named("author", author.name),
            ...(author.url === undefined
                ? []
                : [link({ rel: "author", href: absolute(author.url) })]),
        ]),
        ...named("creator", metadata.creator),
        ...named("publisher", metadata.publisher),
        ...named(
            "robots",
            metadata.robots === undefined
                ? undefined
                : `${metadata.robots.index === false ? "noindex" : "index"}, ${metadata.robots.follow === false ? "nofollow" : "follow"}`,
        ),

        // link the canonical address, the alternate representations and the translations
        ...(canonical === undefined ? [] : [link({ rel: "canonical", href: absolute(canonical) })]),
        ...Object.entries(metadata.alternates?.types ?? {}).map(([type, path]) =>
            link({ rel: "alternate", type, href: absolute(path) }),
        ),
        ...Object.entries(metadata.alternates?.languages ?? {}).map(([locale, path]) =>
            link({ rel: "alternate", hreflang: locale, href: absolute(path) }),
        ),

        // describe the OpenGraph preview
        ...(graph === undefined
            ? []
            : [
                  ...property("og:type", graph.type ?? "website"),
                  ...property("og:url", pageUrl === undefined ? undefined : absolute(pageUrl)),
                  ...property("og:site_name", graph.siteName),
                  ...property("og:locale", graph.locale),
                  ...property("og:title", sharedTitle),
                  ...property("og:description", sharedDescription),
                  ...imagesOf(graph.images, absolute).flatMap((image) => [
                      ...property("og:image", image.url),
                      ...property("og:image:width", image.width?.toString()),
                      ...property("og:image:height", image.height?.toString()),
                      ...property("og:image:alt", image.alt),
                  ]),
                  ...property("article:published_time", graph.publishedTime),
                  ...property("article:modified_time", graph.modifiedTime),
                  ...(graph.authors ?? []).flatMap((author) => property("article:author", author)),
                  ...property("article:section", graph.section),
                  ...(graph.tags ?? []).flatMap((tag) => property("article:tag", tag)),
              ]),

        // describe the Twitter card over the OpenGraph preview
        ...(card === undefined
            ? []
            : [
                  ...named("twitter:card", card.card ?? "summary_large_image"),
                  ...named("twitter:site", card.site),
                  ...named("twitter:creator", card.creator),
                  ...named("twitter:title", card.title ?? sharedTitle),
                  ...named("twitter:description", card.description ?? sharedDescription),
                  ...imagesOf(card.images ?? graph?.images, absolute).flatMap((image) => [
                      ...named("twitter:image", image.url),
                      ...named("twitter:image:alt", image.alt),
                  ]),
              ]),

        // link the icons and the manifest
        ...iconsOf(metadata.icons?.icon).map((icon) => link({ rel: "icon", ...icon })),
        ...(metadata.icons?.shortcut === undefined
            ? []
            : [link({ rel: "shortcut icon", href: metadata.icons.shortcut })]),
        ...iconsOf(metadata.icons?.apple).map((icon) => link({ rel: "apple-touch-icon", ...icon })),
        ...(metadata.icons?.other ?? []).map((icon) =>
            link({ ...iconsOf([icon])[0], rel: icon.rel }),
        ),
        ...(metadata.manifest === undefined
            ? []
            : [link({ rel: "manifest", href: metadata.manifest })]),

        // verify the site for search engines
        ...named("google-site-verification", metadata.verification?.google),
        ...named("yandex-verification", metadata.verification?.yandex),
        ...Object.entries(metadata.verification?.other ?? {}).flatMap(([name, token]) =>
            named(name, token),
        ),
    ];
}

/** Write the viewport tag's content: the width and zoom, and how far the person may zoom when limited. */
function viewportContent(viewport: Viewport): string {
    const entries = [
        `width=${viewport.width ?? "device-width"}`,
        `initial-scale=${viewport.initialScale ?? 1}`,
        ...(viewport.maximumScale === undefined ? [] : [`maximum-scale=${viewport.maximumScale}`]),
        ...(viewport.userScalable === undefined
            ? []
            : [`user-scalable=${viewport.userScalable ? "yes" : "no"}`]),
    ];

    return entries.join(", ");
}

/** Make a named meta tag, none without content. */
function named(name: string, content: string | undefined): HeadTag[] {
    return content === undefined ? [] : [{ tag: "meta", attributes: { name, content } }];
}

/** Make a property meta tag, none without content. */
function property(key: string, content: string | undefined): HeadTag[] {
    return content === undefined ? [] : [{ tag: "meta", attributes: { property: key, content } }];
}

/** Make a link tag. */
function link(attributes: Readonly<Record<string, string>>): HeadTag {
    return { tag: "link", attributes };
}

/** Read images as absolute images with their dimensions and descriptions. */
function imagesOf(
    images: readonly (string | MetadataImage)[] | undefined,
    absolute: (path: string) => string,
): MetadataImage[] {
    return (images ?? []).map((image) =>
        typeof image === "string"
            ? { url: absolute(image) }
            : { ...image, url: absolute(image.url) },
    );
}

/** Read icons as link attributes with their sizes and media types. */
function iconsOf(icons: string | readonly MetadataIcon[] | undefined): Record<string, string>[] {
    const list = typeof icons === "string" ? [{ url: icons }] : (icons ?? []);

    return list.map((icon) => ({
        href: icon.url,
        ...(icon.sizes === undefined ? {} : { sizes: icon.sizes }),
        ...(icon.type === undefined ? {} : { type: icon.type }),
    }));
}

/** Read a page's title: its own through the enclosing template, or the enclosing default. */
function titleOf(own: Metadata["title"], enclosing: Metadata["title"]): string | undefined {
    // take an absolute title as is, and fill the enclosing template with a plain one
    const template = typeof enclosing === "object" ? enclosing.template : undefined;
    if (typeof own === "object" && own.absolute !== undefined) {
        return own.absolute;
    } else if (typeof own === "string") {
        return template === undefined ? own : template.replace("%s", own);
    }

    // take an own default, or else the enclosing one
    return own?.default ?? (typeof enclosing === "object" ? enclosing.default : enclosing);
}

/** Read the title template inner pages inherit: an own template, or the enclosing one. */
function templateOf(own: Metadata["title"], enclosing: Metadata["title"]): Metadata["title"] {
    return typeof own === "object" ? own : enclosing;
}

/** Read the metadata inner pages inherit: the merged fields with the inherited title template. */
function inherited(merged: Metadata, title: Metadata["title"]): Metadata {
    const { title: _, ...fields } = merged;

    return title === undefined ? fields : { ...fields, title };
}

/** Drop a metadata component's children without rendering them, keeping its fields. */
function fieldsOf(properties: MetadataProperties): Metadata {
    return omit(properties, "children");
}
