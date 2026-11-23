import { DESTACK_FONTS, DESTACK_TEXT, destackTheme } from "./destack.ts";
import { defineTheme } from "./theme.ts";

/** Enceladus: Saturn's bright ice moon, in GitHub's white and near-black with its blue. */
export const enceladusTheme = defineTheme({
    name: "enceladus",
    base: "#8c959f",
    accent: "#0969da",
    status: { destructive: "#cf222e", success: "#1a7f37", warning: "#9a6700", info: "#0969da" },
    series: ["#8250df", "#1a7f37", "#bc4c00", "#bf3989"],
    ground: {
        light: { background: "#ffffff", foreground: "#1f2328", muted: "#eff0f0" },
        dark: { background: "#0d1117", foreground: "#e6edf3", muted: "#21252b" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Umbriel: Uranus's dark moon, in Vercel's true black and near-white with its blue. */
export const umbrielTheme = defineTheme({
    name: "umbriel",
    base: "#8a8a8a",
    accent: "#52a9ff",
    status: { destructive: "#e5484d", success: "#63c174", warning: "#f1a10d", info: "#52a9ff" },
    series: ["#52a9ff", "#63c174", "#bf7af0", "#0ac5b3"],
    ground: {
        light: { background: "#fbfbfa", foreground: "#24292f", muted: "#ececec" },
        dark: { background: "#000000", foreground: "#ededed", muted: "#151515" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Triton: Neptune's pink ice moon, in Ubuntu's aubergine and Horizon's blush with orange. */
export const tritonTheme = defineTheme({
    name: "triton",
    base: "#a18993",
    accent: "#e95420",
    status: { destructive: "#cc0000", success: "#4e9a06", warning: "#c4a000", info: "#3465a4" },
    series: ["#3465a4", "#4e9a06", "#75507b", "#06989a"],
    ground: {
        light: { background: "#fdf0ed", foreground: "#16161d", muted: "#ede1de" },
        dark: { background: "#300a24", foreground: "#eeeeec", muted: "#411f36" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Iapetus: Saturn's moon of one bright and one dark half, in Lumina's parchment and coffee. */
export const iapetusTheme = defineTheme({
    name: "iapetus",
    base: "#9c9282",
    accent: "#c08a2e",
    status: { destructive: "#d47169", success: "#92d3b9", warning: "#e0ba6a", info: "#97bdf4" },
    series: ["#97bdf4", "#92d3b9", "#d4a3dc", "#85cbd3"],
    ground: {
        light: { background: "#f9efda", foreground: "#352516", muted: "#ebe1cc" },
        dark: { background: "#2a2116", foreground: "#efe5d0", muted: "#3c3327" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Pandora: Saturn's shepherd moon of its F ring, in Lumina's canopy greens. */
export const pandoraTheme = defineTheme({
    name: "pandora",
    base: "#8a958b",
    accent: "#337545",
    status: { destructive: "#d57068", success: "#99d0a4", warning: "#d1cca8", info: "#9acaf1" },
    series: ["#9acaf1", "#99d0a4", "#e2acdd", "#86cfd7"],
    ground: {
        light: { background: "#f1f9ef", foreground: "#1e3323", muted: "#e2ebe1" },
        dark: { background: "#0c1b10", foreground: "#d8e3d6", muted: "#1e2d22" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Titan: Saturn's hazy moon of methane seas, in Solarized's cream and deep teal. */
export const titanTheme = defineTheme({
    name: "titan",
    base: "#839496",
    accent: "#b58900",
    status: { destructive: "#dc322f", success: "#859900", warning: "#cb4b16", info: "#268bd2" },
    series: ["#268bd2", "#2aa198", "#d33682", "#6c71c4"],
    ground: {
        light: { background: "#fdf6e3", foreground: "#4f6369", muted: "#f1ecda" },
        dark: { background: "#002b36", foreground: "#eee8d5", muted: "#153c44" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Europa: Jupiter's ice moon over an ocean, in Nord's snow and slate with its frost. */
export const europaTheme = defineTheme({
    name: "europa",
    base: "#4c566a",
    accent: "#88c0d0",
    status: { destructive: "#bf616a", success: "#a3be8c", warning: "#ebcb8b", info: "#81a1c1" },
    series: ["#b48ead", "#a3be8c", "#d08770", "#8fbcbb"],
    ground: {
        light: { background: "#e5e9f0", foreground: "#414858", muted: "#dadee5" },
        dark: { background: "#2e3440", foreground: "#eceff4", muted: "#3d434f" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Charon: Pluto's moon named for the ferryman of the dead, in Dracula's night and Alucard. */
export const charonTheme = defineTheme({
    name: "charon",
    base: "#6272a4",
    accent: "#bd93f9",
    status: { destructive: "#ff5555", success: "#50fa7b", warning: "#ffb86c", info: "#8be9fd" },
    series: ["#ff79c6", "#8be9fd", "#50fa7b", "#ffb86c"],
    ground: {
        light: { background: "#fffbeb", foreground: "#1f1f1f", muted: "#efecdd" },
        dark: { background: "#282a36", foreground: "#f8f8f2", muted: "#3b3d47" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Ariel: Uranus's moon named for an airy spirit, in Catppuccin's latte and mocha. */
export const arielTheme = defineTheme({
    name: "ariel",
    base: "#7f849c",
    accent: "#8839ef",
    status: { destructive: "#d20f39", success: "#40a02b", warning: "#df8e1d", info: "#1e66f5" },
    series: ["#ea76cb", "#1e66f5", "#179299", "#fe640b"],
    ground: {
        light: { background: "#eff1f5", foreground: "#4c4f69", muted: "#e4e6eb" },
        dark: { background: "#1e1e2e", foreground: "#d7def6", muted: "#2e2f40" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Destack's themes: the brand, the neutrals, then the others around the hues of their nights. */
export const THEMES = [
    destackTheme,
    enceladusTheme,
    umbrielTheme,
    tritonTheme,
    iapetusTheme,
    pandoraTheme,
    titanTheme,
    europaTheme,
    charonTheme,
    arielTheme,
] as const;
