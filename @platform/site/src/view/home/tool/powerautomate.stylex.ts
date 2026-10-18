import * as style from "@destack/style";

/** The colours of the office flow designer, light and dark as it ships. */
export const powerAutomate = style.defineConsts({
    /** The canvas, dotted. */
    canvas: "light-dark(#fafafa, #1b1b1b)",
    /** The canvas's dots. */
    dots: "light-dark(#d6d6d6, #3a3a3a)",
    /** The command bar, a card and the configuration pane. */
    panel: "light-dark(#ffffff, #292929)",
    /** The text. */
    ink: "light-dark(#242424, #f0f0f0)",
    /** Secondary text and icons. */
    muted: "light-dark(#616161, #adadad)",
    /** Rules, a field's edge and a card's edge. */
    line: "light-dark(#d1d1d1, #3d3d3d)",
    /** The brand blue of icons, the chosen tab and the chosen card. */
    blue: "#0f6cbd",
    /** The trigger's colour. */
    request: "#009da5",
    /** A dynamic value inside a field. */
    token: "light-dark(#e8f1fb, #0c3b5e)",
    /** A card's lift. */
    lift: "0 1px 2px rgb(0 0 0 / 0.14), 0 0 2px rgb(0 0 0 / 0.12)",
    /** The chosen card's lift. */
    liftChosen: "0 4px 8px rgb(0 0 0 / 0.14), 0 0 2px rgb(0 0 0 / 0.12)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Segoe UI Variable Text", "Segoe UI", -apple-system, BlinkMacSystemFont, Roboto, sans-serif',
});
