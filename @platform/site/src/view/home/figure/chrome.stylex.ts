import * as style from "@destack/style";

/** The window chrome a drawn browser and a ledger share, so their edges line up side by side. */
export const chrome = style.defineConsts({
    /** The corner radius of a window. */
    radius: "10px",
    /** The height of a window's top strip: a browser's tabs, or a ledger's window lights. */
    strip: "2.5rem",
    /** The height of a window's toolbar: a browser's address bar, or a ledger's column heads. */
    toolbar: "2.75rem",
});
