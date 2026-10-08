import { readFile } from "node:fs/promises";
import { basename } from "node:path";
import { createFontStack } from "@capsizecss/core";
import arial from "@capsizecss/metrics/arial";
import timesNewRoman from "@capsizecss/metrics/timesNewRoman";
import { fromBuffer } from "@capsizecss/unpack";
import type { Plugin } from "@destack/package/build";

/** The local fonts a fallback adjusts to a font's metrics. */
const LOCAL_FONTS = { Arial: arial, "Times New Roman": timesNewRoman } as const;

/** The query an import adds to read a font file with its metrics. */
const FONT_QUERY = "?font";

/** A local font's overrides adjusting it to a font's metrics. */
export interface FallbackMetrics {
    /** The size adjustment. */
    readonly size: string;
    /** The ascent override. */
    readonly ascent: string;
    /** The descent override. */
    readonly descent: string;
    /** The line gap override. */
    readonly lineGap: string;
}

/** A font file a build read: its address, its family and the overrides adjusting each local fallback to it. */
export interface FontFile {
    /** The file's address. */
    readonly url: string;
    /** The family the file declares. */
    readonly family: string;
    /** The overrides of each local fallback. */
    readonly fallbacks: Readonly<Record<keyof typeof LOCAL_FONTS, FallbackMetrics>>;
}

/** Read a font file's family and the overrides adjusting each local fallback to its metrics. */
export async function readFont(bytes: Uint8Array): Promise<Omit<FontFile, "url">> {
    // read the font's metrics
    const metrics = await fromBuffer(Buffer.from(bytes));

    // compute each local fallback's overrides through a font stack of the font and the local font
    const fallbackOf = (local: (typeof LOCAL_FONTS)[keyof typeof LOCAL_FONTS]): FallbackMetrics => {
        // stack the font over the local font, which yields the local font's adjusted face
        const [face] = createFontStack([metrics, local], {
            fontFaceFormat: "styleObject",
        }).fontFaces;
        if (face === undefined) {
            throw new TypeError(
                `no fallback face adjusts ${local.familyName} to ${metrics.familyName}`,
            );
        }
        const rule = face["@font-face"];

        return {
            size: rule.sizeAdjust ?? "100%",
            ascent: String(rule.ascentOverride ?? "normal"),
            descent: String(rule.descentOverride ?? "normal"),
            lineGap: String(rule.lineGapOverride ?? "normal"),
        };
    };
    const fallbacks = {
        Arial: fallbackOf(LOCAL_FONTS.Arial),
        "Times New Roman": fallbackOf(LOCAL_FONTS["Times New Roman"]),
    };

    return { family: metrics.familyName, fallbacks };
}

/** Turn `?font` imports into font files: emitted as build assets with their metrics, served from their source while developing. */
export function fontPlugin(): Plugin {
    // remember whether the plugin builds or serves
    let isBuild = true;

    return {
        name: "destack-font",
        enforce: "pre",
        configResolved(configuration) {
            isBuild = configuration.command === "build";
        },
        async load(id) {
            // leave modules without the font query
            if (!id.endsWith(FONT_QUERY)) {
                return null;
            }

            // read the file and its metrics, emitting it as a hashed asset when building
            const path = id.slice(0, -FONT_QUERY.length);
            const bytes = new Uint8Array(await readFile(path));
            const font = await readFont(bytes);
            const url = isBuild
                ? `import.meta.ROLLUP_FILE_URL_${this.emitFile({ type: "asset", name: basename(path), source: bytes })}`
                : JSON.stringify(`/@fs${path}`);

            return `export default { ...${JSON.stringify(font)}, url: ${url} };\n`;
        },
    };
}
