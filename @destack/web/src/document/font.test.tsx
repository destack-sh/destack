import { expect, onTestFinished, test } from "@destack/test";
import { render } from "@solidjs/web";
import type { FontFile } from "../build/font.ts";
import { defineFont, Font } from "./font.tsx";

/** A variable sans file as a `?font` import reads it, with each local fallback's overrides. */
const plexSans: FontFile = {
    url: "/_assets/plex-sans.woff2",
    family: "IBM Plex Sans",
    fallbacks: {
        Arial: { size: "104.2%", ascent: "98.1%", descent: "26.3%", lineGap: "0%" },
        "Times New Roman": { size: "116.4%", ascent: "87.8%", descent: "23.6%", lineGap: "0%" },
    },
};

/** The same family's italic file. */
const plexSansItalic: FontFile = { ...plexSans, url: "/_assets/plex-sans-italic.woff2" };

/** The family declared from both files, falling back to the system's sans. */
const plex = defineFont({
    src: [
        { path: plexSans, weight: "100 700" },
        { path: plexSansItalic, weight: "100 700", style: "italic" },
    ],
    fallback: ["system-ui", "sans-serif"],
});

/** Render a font's head elements, removing them after the test. */
async function mount(): Promise<void> {
    const host = document.createElement("main");
    document.body.append(host);
    const dispose = render(() => <Font font={plex} />, host);
    onTestFinished(() => {
        dispose();
        host.remove();
        document.head.replaceChildren();
    });

    // wait for the head registry, which applies registered tags after the current task
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
}

test("name the files' family and stack its Arial fallback before the generic families", () => {
    expect([
        plex.family,
        plex.stack,
        defineFont({ src: plexSans, adjustFontFallback: false }).stack,
    ]).toEqual([
        "IBM Plex Sans",
        '"IBM Plex Sans", "IBM Plex Sans Fallback", system-ui, sans-serif',
        '"IBM Plex Sans"',
    ]);
});

test("write a font's faces, its fallback adjusted by the file's metrics and a preload per file", async () => {
    // render the font
    await mount();

    // describe both files with swap display, Arial adjusted to the metrics, and preload the WOFF2 files
    expect({
        rules: document.head.querySelector("style")?.textContent?.split("\n"),
        preloads: [...document.head.querySelectorAll('link[rel="preload"]')].map((link) => [
            link.getAttribute("href"),
            link.getAttribute("type"),
        ]),
    }).toEqual({
        rules: [
            '@font-face { font-family: "IBM Plex Sans"; src: url("/_assets/plex-sans.woff2") format("woff2"); font-weight: 100 700; font-style: normal; font-display: swap; }',
            '@font-face { font-family: "IBM Plex Sans"; src: url("/_assets/plex-sans-italic.woff2") format("woff2"); font-weight: 100 700; font-style: italic; font-display: swap; }',
            '@font-face { font-family: "IBM Plex Sans Fallback"; src: local("Arial"); size-adjust: 104.2%; ascent-override: 98.1%; descent-override: 26.3%; line-gap-override: 0%; }',
        ],
        preloads: [
            ["/_assets/plex-sans.woff2", "font/woff2"],
            ["/_assets/plex-sans-italic.woff2", "font/woff2"],
        ],
    });
});
