import * as style from "@destack/style";

/** The colours of the game store's community profile. */
export const steam = style.defineConsts({
    /** The global header. */
    header: "#171d25",
    /** The page behind the profile. */
    backdrop: "radial-gradient(120% 70% at 50% 0%, #1e3a4c 0%, #121a24 55%, #0e141b 100%)",
    /** The profile's header band. */
    band: "linear-gradient(180deg, rgb(32 58 76 / 0.9) 0%, rgb(22 40 54 / 0.9) 100%)",
    /** The content column. */
    column: "rgb(19 26 35 / 0.92)",
    /** A section's title bar. */
    bar: "linear-gradient(90deg, rgb(46 74 102 / 0.75) 0%, rgb(46 74 102 / 0.2) 100%)",
    /** A section's body. */
    panel: "rgb(0 0 0 / 0.22)",
    /** The text. */
    ink: "#acb2b8",
    /** Bright text. */
    bright: "#ffffff",
    /** Quiet text. */
    muted: "#8f98a0",
    /** The blue of the chosen tab and links. */
    blue: "#1a9fff",
    /** The cyan of the online status. */
    online: "#57cbde",
    /** The ring around a level of 20 to 29. */
    level: "#d9b93f",
    /** An achievement bar's track. */
    track: "#3a4552",
    /** The typeface the site sets, or the nearest one a machine has. */
    face: '"Motiva Sans", Arial, Helvetica, sans-serif',
});
