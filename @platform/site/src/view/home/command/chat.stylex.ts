import * as style from "@destack/style";

/** The look of the assistant's conversation, set per assistant, Claude's by default. */
export const chat = style.defineVars({
    /** The window's background. */
    canvas: "light-dark(#faf9f5, #262624)",
    /** The text. */
    ink: "light-dark(#141413, #faf9f5)",
    /** Secondary text and the tool lines. */
    muted: "light-dark(#73726c, #9c9a92)",
    /** Rules and frames. */
    border: "light-dark(#e3e1d8, #3d3d3a)",
    /** A tool's opened request and response. */
    field: "light-dark(#f5f4ed, #1f1e1d)",
    /** Your message's bubble. */
    bubble: "light-dark(#f0eee6, #141413)",
    /** Your message bubble's corners. */
    bubbleRadius: "12px",
    /** Your message bubble's top right corner. */
    bubbleCorner: "12px",
    /** The typeface the assistant answers in. */
    replyFont: 'ui-serif, "Iowan Old Style", Georgia, serif',
    /** The size the assistant answers in. */
    replySize: "0.9375rem",
    /** The line height the assistant answers in. */
    replyLine: "1.6",
});

/** The fixed colours of the chat: results, and the assistants' own marks. */
export const assistant = style.defineConsts({
    /** A tool's failed result. */
    failed: "light-dark(#b53333, #e57373)",
    /** A tool's result. */
    passed: "light-dark(#3d7a4f, #7fc493)",
    /** Claude's send button and spark. */
    clay: "#c96442",
    /** White on a filled button. */
    white: "#ffffff",
});

/** ChatGPT's colours, light and dark as it ships. */
export const chatgpt = style.defineConsts({
    /** The window's background. */
    canvas: "light-dark(#ffffff, #212121)",
    /** The text. */
    ink: "light-dark(#0d0d0d, #ececec)",
    /** Secondary text. */
    muted: "light-dark(#5d5d5d, #afafaf)",
    /** Rules and frames. */
    border: "light-dark(#e5e5e5, #3a3a3a)",
    /** A tool's opened request and response. */
    field: "light-dark(#f9f9f9, #171717)",
    /** Your message's bubble. */
    bubble: "light-dark(#f4f4f4, #303030)",
    /** The composer's field. */
    composer: "light-dark(#ffffff, #303030)",
    /** The composer's lift and ring. */
    lift: "0 4px 12px rgba(0, 0, 0, 0.06), 0 0 0 1px light-dark(rgba(0, 0, 0, 0.08), rgba(255, 255, 255, 0.06))",
    /** The send button. */
    send: "light-dark(#000000, #ffffff)",
    /** The send button's arrow. */
    sendInk: "light-dark(#ffffff, #000000)",
});

/** Gemini's colours, light and dark as it ships. */
export const gemini = style.defineConsts({
    /** The window's background. */
    canvas: "light-dark(#ffffff, #131314)",
    /** The text. */
    ink: "light-dark(#1f1f1f, #e3e3e3)",
    /** Secondary text. */
    muted: "light-dark(#444746, #c4c7c5)",
    /** Rules and frames. */
    border: "light-dark(#dde3ea, #3c4043)",
    /** A tool's opened request and response. */
    field: "light-dark(#f8fafd, #1e1f20)",
    /** Your message's bubble. */
    bubble: "light-dark(#e9eef6, #333537)",
    /** The composer's field. */
    composer: "light-dark(#f0f4f9, #1e1f20)",
    /** Your account's avatar. */
    avatar: "#2f6f7e",
    /** The spark that turns while Gemini works. */
    spark: "#4285f4",
    /** The send button. */
    send: "light-dark(#0b57d0, #a8c7fa)",
    /** The send button's arrow. */
    sendInk: "light-dark(#ffffff, #062e6f)",
});

/** Claude's composer colours, light and dark as it ships. */
export const claude = style.defineConsts({
    /** The composer's field. */
    composer: "light-dark(#ffffff, #30302e)",
    /** The composer's and its buttons' edge. */
    edge: "light-dark(#e0ddd2, #45453f)",
    /** The composer's soft lift. */
    lift: "0 1px 6px rgba(0, 0, 0, 0.04)",
});
