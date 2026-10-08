import type { JSX } from "@solidjs/web";
import { For, Show } from "../solid/flow.ts";
import type { FallbackMetrics, FontFile } from "../build/font.ts";
import { Link, Style } from "./head.ts";

/** The generic families CSS names without quotes. */
const GENERIC_FAMILIES = new Set([
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    "emoji",
    "math",
]);

/** The font-face format and media type of each font file extension. */
const FORMATS: Readonly<Record<string, { readonly format: string; readonly type: string }>> = {
    woff2: { format: "woff2", type: "font/woff2" },
    woff: { format: "woff", type: "font/woff" },
    ttf: { format: "truetype", type: "font/ttf" },
    otf: { format: "opentype", type: "font/otf" },
};

/** One file of a font family with the weights and style it covers. */
export interface FontSource {
    /** The file, as a `?font` import. */
    readonly path: FontFile;
    /** The weight or weight range, such as `400` or `100 900` for a variable font. */
    readonly weight?: string;
    /** The style, normal unless italic. */
    readonly style?: "normal" | "italic";
    /** The characters the file covers, such as a Latin Extended subset, which loads only when the page uses them. */
    readonly unicodeRange?: string;
}

/** A self-hosted font family. */
export interface FontDefinition {
    /** The family's files: one `?font` import, or several with their weights and styles. */
    readonly src: FontFile | readonly FontSource[];
    /** The family name text styles name, the first file's own family by default. */
    readonly family?: string;
    /** How text shows while the font loads, `swap` by default. */
    readonly display?: "auto" | "block" | "swap" | "fallback" | "optional";
    /** Preload the files before the stylesheets ask for them, true by default. */
    readonly preload?: boolean;
    /** The families shown before and instead of this one. */
    readonly fallback?: readonly string[];
    /** The local font a fallback adjusts to the font's metrics so swapping moves no text, Arial by default. */
    readonly adjustFontFallback?: keyof FontFile["fallbacks"] | false;
}

/** A declared font family with the font stack text styles use. */
export interface Font {
    /** The family name. */
    readonly family: string;
    /** The family's files. */
    readonly sources: readonly FontSource[];
    /** How text shows while the font loads. */
    readonly display: NonNullable<FontDefinition["display"]>;
    /** Whether the files preload. */
    readonly preload: boolean;
    /** The adjusted local fallback with its overrides, absent when none adjusts. */
    readonly adjusted?: { readonly local: string; readonly metrics: FallbackMetrics };
    /** The family, its adjusted fallback and its fallbacks, as a `font-family` value. */
    readonly stack: string;
}

/** Declare a self-hosted font family from its `?font` imports. */
export function defineFont(definition: FontDefinition): Font {
    // read the files and the family they declare
    const sources = "url" in definition.src ? [{ path: definition.src }] : definition.src;
    const first = sources[0]?.path;
    const family = definition.family ?? first?.family;
    if (family === undefined || first === undefined) {
        throw new TypeError("a font names no file");
    }

    // adjust the chosen local font to the first file's metrics
    const local = definition.adjustFontFallback ?? "Arial";
    const adjusted = local === false ? undefined : { local, metrics: first.fallbacks[local] };
    const families = [
        family,
        ...(adjusted === undefined ? [] : [fallbackFamily(family)]),
        ...(definition.fallback ?? []),
    ];

    return {
        family,
        sources,
        display: definition.display ?? "swap",
        preload: definition.preload ?? true,
        ...(adjusted === undefined ? {} : { adjusted }),
        stack: families.map(quote).join(", "),
    };
}

/** The properties of a font's head elements. */
export interface FontProperties {
    /** The declared font. */
    readonly font: Font;
}

/** Write a font's faces, its adjusted fallback and its preloads into the document head. */
export function Font(properties: FontProperties): JSX.Element {
    return (
        <>
            <Style>{faceRules(properties.font)}</Style>
            <Show when={properties.font.preload}>
                <For
                    each={properties.font.sources.filter(
                        (source) => source.unicodeRange === undefined,
                    )}
                >
                    {(source) => (
                        <Link
                            rel="preload"
                            as="font"
                            type={formatOf(source.path.url).type}
                            href={source.path.url}
                            crossorigin="anonymous"
                        />
                    )}
                </For>
            </Show>
        </>
    );
}

/** Write the font-face rules of a font's files and of its adjusted fallback. */
function faceRules(font: Font): string {
    // describe each file of the family
    const faces = font.sources.map((source) =>
        rule({
            "font-family": quote(font.family),
            src: `url("${source.path.url}") format("${formatOf(source.path.url).format}")`,
            "font-weight": source.weight ?? "400",
            "font-style": source.style ?? "normal",
            "font-display": font.display,
            ...(source.unicodeRange === undefined ? {} : { "unicode-range": source.unicodeRange }),
        }),
    );

    // describe the local fallback adjusted to the family's metrics
    const adjusted = font.adjusted;
    const fallback =
        adjusted === undefined
            ? []
            : [
                  rule({
                      "font-family": quote(fallbackFamily(font.family)),
                      src: `local("${adjusted.local}")`,
                      "size-adjust": adjusted.metrics.size,
                      "ascent-override": adjusted.metrics.ascent,
                      "descent-override": adjusted.metrics.descent,
                      "line-gap-override": adjusted.metrics.lineGap,
                  }),
              ];

    return [...faces, ...fallback].join("\n");
}

/** Write one font-face rule. */
function rule(descriptors: Readonly<Record<string, string>>): string {
    const lines = Object.entries(descriptors).map(([name, value]) => `${name}: ${value};`);

    return `@font-face { ${lines.join(" ")} }`;
}

/** Name a family's adjusted local fallback. */
function fallbackFamily(family: string): string {
    return `${family} Fallback`;
}

/** Quote a family name that is no generic family. */
function quote(family: string): string {
    return GENERIC_FAMILIES.has(family) ? family : `"${family}"`;
}

/** Read a font file's format and media type from its extension, refusing a format browsers do not load. */
function formatOf(url: string): { readonly format: string; readonly type: string } {
    // look up the extension's format
    const extension = new URL(url, "file:///").pathname.split(".").at(-1) ?? "";
    const format = FORMATS[extension];
    if (format === undefined) {
        throw new TypeError(`a font file of type ${extension} is no woff2, woff, ttf or otf`);
    }

    return format;
}
