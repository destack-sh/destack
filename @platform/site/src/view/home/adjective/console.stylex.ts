import * as style from "@destack/style";

/** The colours the consoles share. */
export const shared = style.defineConsts({
    /** White text and marks on a dark navigation or a filled button. */
    white: "#ffffff",
    /** An icon at rest on a dark navigation. */
    navIcon: "rgba(255, 255, 255, 0.65)",
    /** The field of the open item on a dark navigation. */
    navOn: "rgba(255, 255, 255, 0.14)",
});

/** The colours of the error tracker's issue page. */
export const sentry = style.defineConsts({
    /** The page background. */
    canvas: "light-dark(#ffffff, #1d1127)",
    /** A raised panel's background. */
    panel: "light-dark(#faf9fb, #241630)",
    /** The text. */
    ink: "light-dark(#2b2233, #ebe6ef)",
    /** Secondary text. */
    muted: "light-dark(#80708f, #9f92ad)",
    /** Rules and borders. */
    border: "light-dark(#e0dce5, #3e2c4c)",
    /** The navigation's gradient. */
    nav: "linear-gradient(294deg, #2f1937 35%, #452650 62%, #52305f)",
    /** The brand purple. */
    purple: "#6c5fc7",
    /** An error's red. */
    red: "#f55459",
    /** The organisation's avatar. */
    org: "#f8a14b",
    /** The organisation avatar's letter. */
    orgInk: "#2f1937",
    /** An unhandled error's tag. */
    redField: "rgba(245, 84, 89, 0.12)",
    /** The highlighted line of a stack frame. */
    hit: "light-dark(#fdebeb, #4a1d2a)",
});

/** The colours of the workflow engine's console. */
export const temporal = style.defineConsts({
    /** The page background. */
    canvas: "light-dark(#ffffff, #141414)",
    /** The navigation. */
    nav: "#0a0a0a",
    /** The text. */
    ink: "light-dark(#18181b, #f4f4f5)",
    /** Secondary text. */
    muted: "light-dark(#71717a, #a1a1aa)",
    /** Rules and borders. */
    border: "light-dark(#e4e4e7, #2e2e33)",
    /** A table header's background. */
    head: "light-dark(#f4f4f5, #1c1c1f)",
    /** A failed status's text. */
    failed: "light-dark(#b91c1c, #f87171)",
    /** A failed status's field. */
    failedField: "light-dark(#fee2e2, #3b1416)",
    /** The memo's note field. */
    note: "light-dark(#fef9c3, #3a3510)",
});

/** The colours of the database console, dark as it ships. */
export const supabase = style.defineConsts({
    /** The page background. */
    canvas: "#1c1c1c",
    /** The navigation. */
    nav: "#171717",
    /** The text. */
    ink: "#ededed",
    /** Secondary text. */
    muted: "#8f8f8f",
    /** Rules and borders. */
    border: "#2e2e2e",
    /** The brand green. */
    green: "#3ecf8e",
    /** The open item on the navigation. */
    navOn: "#2a2a2a",
    /** The branch tag's border. */
    branch: "rgba(62, 207, 142, 0.4)",
    /** The run button. */
    button: "#006239",
    /** A line number in the editor. */
    number: "#5a5a5a",
    /** The results' header row. */
    head: "#232323",
    /** A keyword in the editor. */
    keyword: "#bda4ff",
    /** A string in the editor. */
    string: "#ffcda1",
    /** A comment in the editor. */
    comment: "#7e7e7e",
});

/** The colours of the mail service's dashboard, dark as it ships. */
export const resend = style.defineConsts({
    /** The page background. */
    canvas: "#000000",
    /** A raised panel's background. */
    panel: "#0a0a0a",
    /** The text. */
    ink: "#f1f1f1",
    /** Secondary text. */
    muted: "#a1a1a1",
    /** Rules and borders. */
    border: "#1f1f1f",
    /** A successful request's status. */
    ok: "#3ecf8e",
    /** A successful request's field. */
    okField: "rgba(62, 207, 142, 0.12)",
    /** A failed request's status. */
    failed: "#ff6369",
    /** A failed request's field. */
    failedField: "rgba(255, 99, 105, 0.12)",
});
