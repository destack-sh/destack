import { DESTACK_FONTS, DESTACK_TEXT, destackTheme } from "./destack.ts";
import { defineTheme } from "./theme.ts";

/** Enceladus: Saturn's bright ice moon, in GitHub's white and near-black with its blue. */
export const enceladusTheme = defineTheme({
    name: "enceladus",
    base: "#8c959f",
    accent: "#0969da",
    hues: {
        brown: "#856d4c",
        red: "#cf222e",
        orange: "#bc4c00",
        yellow: "#9a6700",
        green: "#1a7f37",
        cyan: "#1b7c83",
        blue: "#0969da",
        purple: "#8250df",
        pink: "#bf3989",
    },
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
    hues: {
        brown: "#b07a4f",
        red: "#e5484d",
        orange: "#f76b15",
        yellow: "#f1a10d",
        green: "#63c174",
        cyan: "#0ac5b3",
        blue: "#52a9ff",
        purple: "#bf7af0",
        pink: "#e93d82",
    },
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
    hues: {
        brown: "#8f5902",
        red: "#cc0000",
        orange: "#f57900",
        yellow: "#c4a000",
        green: "#4e9a06",
        cyan: "#06989a",
        blue: "#3465a4",
        purple: "#75507b",
        pink: "#c2527e",
    },
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
    accent: "#8a5a2b",
    hues: {
        brown: "#8a5a2b",
        red: "#912918",
        orange: "#c2601f",
        yellow: "#c9a227",
        green: "#2d7341",
        cyan: "#006e81",
        blue: "#2e5fb0",
        purple: "#87418e",
        pink: "#b8487a",
    },
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
    hues: {
        brown: "#7a5534",
        red: "#860c2c",
        orange: "#bf5a1f",
        yellow: "#b59a1c",
        green: "#337545",
        cyan: "#007184",
        blue: "#046aa7",
        purple: "#864c82",
        pink: "#b04a78",
    },
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
    accent: "#2aa198",
    hues: {
        brown: "#8a6a3f",
        red: "#dc322f",
        orange: "#cb4b16",
        yellow: "#b58900",
        green: "#859900",
        cyan: "#2aa198",
        blue: "#268bd2",
        purple: "#6c71c4",
        pink: "#d33682",
    },
    ground: {
        light: { background: "#fdf6e3", foreground: "#4f6369", muted: "#f1ecda" },
        dark: { background: "#002b36", foreground: "#eee8d5", muted: "#153c44" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Europa: Jupiter's ice moon over an ocean, in Nord's snow, slate and aurora. */
export const europaTheme = defineTheme({
    name: "europa",
    base: "#4c566a",
    accent: "#88c0d0",
    hues: {
        brown: "#9c7355",
        red: "#bf616a",
        orange: "#d08770",
        yellow: "#ebcb8b",
        green: "#a3be8c",
        cyan: "#8fbcbb",
        blue: "#5e81ac",
        purple: "#b48ead",
        pink: "#c9829b",
    },
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
    hues: {
        gray: "#82858f",
        brown: "#8f7560",
        red: "#ff5555",
        orange: "#ffb86c",
        yellow: "#f1fa8c",
        green: "#50fa7b",
        cyan: "#8be9fd",
        blue: "#6f9cff",
        purple: "#bd93f9",
        pink: "#ff79c6",
    },
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
    hues: {
        brown: "#a6694a",
        red: "#d20f39",
        orange: "#fe640b",
        yellow: "#df8e1d",
        green: "#40a02b",
        cyan: "#179299",
        blue: "#1e66f5",
        purple: "#8839ef",
        pink: "#ea76cb",
    },
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
