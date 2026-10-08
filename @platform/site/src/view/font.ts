import { defineFont } from "@destack/view/document";
import mono400 from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-400-normal.woff2?font";
import mono600 from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-600-normal.woff2?font";
import mono700 from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-700-normal.woff2?font";
import mono400Extended from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-ext-400-normal.woff2?font";
import mono600Extended from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-ext-600-normal.woff2?font";
import mono700Extended from "@fontsource/ibm-plex-mono/files/ibm-plex-mono-latin-ext-700-normal.woff2?font";
import sansItalic from "@fontsource-variable/ibm-plex-sans/files/ibm-plex-sans-latin-wght-italic.woff2?font";
import sansItalicExtended from "@fontsource-variable/ibm-plex-sans/files/ibm-plex-sans-latin-ext-wght-italic.woff2?font";
import sansUpright from "@fontsource-variable/ibm-plex-sans/files/ibm-plex-sans-latin-wght-normal.woff2?font";
import sansUprightExtended from "@fontsource-variable/ibm-plex-sans/files/ibm-plex-sans-latin-ext-wght-normal.woff2?font";

/** The characters of Fontsource's Latin Extended subset, such as the IPA marks of the pronunciations. */
const LATIN_EXTENDED =
    "U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,U+A720-A7FF";

/** The interface and reading face, upright, preloaded with the page. */
export const sans = defineFont({
    src: [
        { path: sansUpright, weight: "100 700" },
        { path: sansUprightExtended, weight: "100 700", unicodeRange: LATIN_EXTENDED },
    ],
    family: "IBM Plex Sans",
    fallback: ["Helvetica", "Arial", "sans-serif"],
    adjustFontFallback: false,
});

/** The italic of the interface face, loaded when prose asks for it. */
export const sansItalics = defineFont({
    src: [
        { path: sansItalic, weight: "100 700", style: "italic" },
        {
            path: sansItalicExtended,
            weight: "100 700",
            style: "italic",
            unicodeRange: LATIN_EXTENDED,
        },
    ],
    family: "IBM Plex Sans",
    preload: false,
    fallback: ["Helvetica", "Arial", "sans-serif"],
    adjustFontFallback: false,
});

/** The face of commands, labels and code. */
export const mono = defineFont({
    src: [
        { path: mono400, weight: "400" },
        { path: mono600, weight: "600" },
        { path: mono700, weight: "700" },
        { path: mono400Extended, weight: "400", unicodeRange: LATIN_EXTENDED },
        { path: mono600Extended, weight: "600", unicodeRange: LATIN_EXTENDED },
        { path: mono700Extended, weight: "700", unicodeRange: LATIN_EXTENDED },
    ],
    family: "IBM Plex Mono",
    fallback: ["ui-monospace", "SFMono-Regular", "Menlo", "monospace"],
    adjustFontFallback: false,
});
