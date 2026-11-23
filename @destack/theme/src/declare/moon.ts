import { DESTACK_FONTS, DESTACK_TEXT, destackTheme } from "./destack.ts";
import { defineTheme } from "./theme.ts";

/** Io: Jupiter's sulfur moon, in Solarized's cream and deep teal with its yellow. */
export const ioTheme = defineTheme({
    name: "io",
    base: "#839496",
    accent: "#b58900",
    status: { destructive: "#dc322f", success: "#859900", warning: "#cb4b16", info: "#268bd2" },
    series: ["#268bd2", "#2aa198", "#d33682", "#6c71c4"],
    ground: {
        light: { background: "#fdf6e3", foreground: "#3d5157", muted: "#eee8d5" },
        dark: { background: "#002b36", foreground: "#eee8d5", muted: "#073642" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Europa: Jupiter's ice moon, in GitHub's cool white and near-black with its blue. */
export const europaTheme = defineTheme({
    name: "europa",
    base: "#8c959f",
    accent: "#0969da",
    status: { destructive: "#cf222e", success: "#1a7f37", warning: "#9a6700", info: "#0969da" },
    series: ["#8250df", "#1a7f37", "#bc4c00", "#bf3989"],
    ground: {
        light: { background: "#ffffff", foreground: "#1f2328", muted: "#f6f8fa" },
        dark: { background: "#0d1117", foreground: "#e6edf3", muted: "#161b22" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Pandora: Saturn's shepherd moon of its F ring, in Everforest's forest greens. */
export const pandoraTheme = defineTheme({
    name: "pandora",
    base: "#859289",
    accent: "#8da101",
    status: { destructive: "#f85552", success: "#35a77c", warning: "#dfa000", info: "#3a94c5" },
    series: ["#3a94c5", "#df69ba", "#f57d26", "#35a77c"],
    ground: {
        light: { background: "#fdf6e3", foreground: "#46535a", muted: "#f4f0d9" },
        dark: { background: "#2d353b", foreground: "#f5efe0", muted: "#343f44" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Enceladus: Saturn's bright ice moon, in Nord's polar night and snow with its frost. */
export const enceladusTheme = defineTheme({
    name: "enceladus",
    base: "#4c566a",
    accent: "#88c0d0",
    status: { destructive: "#bf616a", success: "#a3be8c", warning: "#ebcb8b", info: "#81a1c1" },
    series: ["#b48ead", "#a3be8c", "#d08770", "#8fbcbb"],
    ground: {
        light: { background: "#f3f5f8", foreground: "#2e3440", muted: "#e5e9f0" },
        dark: { background: "#2e3440", foreground: "#eceff4", muted: "#3b4252" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Titan: Saturn's hazy moon, in Gruvbox's warm earth with its orange. */
export const titanTheme = defineTheme({
    name: "titan",
    base: "#928374",
    accent: "#d65d0e",
    status: { destructive: "#cc241d", success: "#98971a", warning: "#d79921", info: "#458588" },
    series: ["#458588", "#98971a", "#b16286", "#689d6a"],
    ground: {
        light: { background: "#f9f5d7", foreground: "#3c3836", muted: "#f2e5bc" },
        dark: { background: "#282828", foreground: "#f2e5bc", muted: "#3c3836" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Ariel: Uranus's bright airy moon, in Catppuccin's latte and mocha with its pink. */
export const arielTheme = defineTheme({
    name: "ariel",
    base: "#7f849c",
    accent: "#ea76cb",
    status: { destructive: "#d20f39", success: "#40a02b", warning: "#df8e1d", info: "#1e66f5" },
    series: ["#8839ef", "#1e66f5", "#179299", "#fe640b"],
    ground: {
        light: { background: "#f5f6f9", foreground: "#4c4f69", muted: "#e6e9ef" },
        dark: { background: "#1e1e2e", foreground: "#e2e7fa", muted: "#313244" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Triton: Neptune's pink ice moon, in Rosé Pine's dawn and night with its rose. */
export const tritonTheme = defineTheme({
    name: "triton",
    base: "#908caa",
    accent: "#d7827e",
    status: { destructive: "#b4637a", success: "#286983", warning: "#ea9d34", info: "#56949f" },
    series: ["#907aa9", "#56949f", "#ea9d34", "#286983"],
    ground: {
        light: { background: "#faf4ed", foreground: "#575279", muted: "#f2e9e1" },
        dark: { background: "#191724", foreground: "#e0def4", muted: "#26233a" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Nyx: Pluto's moon named for the night, in Dracula's purple night and Alucard. */
export const nyxTheme = defineTheme({
    name: "nyx",
    base: "#6272a4",
    accent: "#bd93f9",
    status: { destructive: "#ff5555", success: "#50fa7b", warning: "#ffb86c", info: "#8be9fd" },
    series: ["#ff79c6", "#8be9fd", "#50fa7b", "#ffb86c"],
    ground: {
        light: { background: "#fffbeb", foreground: "#1f1f1f", muted: "#f2ecd6" },
        dark: { background: "#282a36", foreground: "#f8f8f2", muted: "#343746" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Destack's themes: the brand, then its moons by their planets from the sun outward. */
export const THEMES = [
    destackTheme,
    ioTheme,
    europaTheme,
    pandoraTheme,
    enceladusTheme,
    titanTheme,
    arielTheme,
    tritonTheme,
    nyxTheme,
] as const;
