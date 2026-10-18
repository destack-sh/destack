import * as style from "@destack/style";

/** The colours of the word processor in a browser, light and dark as it ships. */
export const googleDocuments = style.defineConsts({
    /** The app's chrome and the desk the page lies on. */
    chrome: "light-dark(#f9fbfd, #1b1b1b)",
    /** The page. */
    paper: "light-dark(#ffffff, #1e1f20)",
    /** The text. */
    ink: "light-dark(#1f1f1f, #e3e3e3)",
    /** Secondary text and the toolbar's glyphs. */
    muted: "light-dark(#444746, #c4c7c5)",
    /** The toolbar's field. */
    bar: "light-dark(#edf2fa, #282a2c)",
    /** Rules and the ruler's ticks. */
    line: "light-dark(#c7c7c7, #444746)",
    /** The share button. */
    share: "light-dark(#c2e7ff, #004a77)",
    /** The share button's text. */
    shareInk: "light-dark(#001d35, #c2e7ff)",
    /** A done checklist item's text. */
    done: "light-dark(#747775, #8e918f)",
    /** The accent of checked boxes and the ruler's markers. */
    accent: "light-dark(#0b57d0, #a8c7fa)",
    /** The in-progress status chip. */
    status: "light-dark(#fff4c2, #4a3c00)",
    /** The in-progress status chip's text. */
    statusInk: "light-dark(#594300, #ffe07a)",
    /** A person or date chip. */
    chip: "light-dark(#f1f3f4, #303134)",
    /** The page's lift off the desk. */
    lift: "0 1px 3px rgb(60 64 67 / 0.15), 0 1px 1px rgb(60 64 67 / 0.08)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Google Sans", Roboto, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    /** The typeface of the page's text. */
    text: 'Arial, "Helvetica Neue", Helvetica, sans-serif',
});
