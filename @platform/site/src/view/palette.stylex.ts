import * as style from "@destack/style";

/** The inks of the site's figures and effects, the same in both appearances. */
export const palette = style.defineConsts({
    /** The brand's signal orange, drawn on figures and the mark. */
    signal: "#ff792e",
    /** The deep teal ink figures draw lines and text in. */
    ink: "#12313c",
    /** The cream of owned plates and of drawings on dark fields. */
    cream: "#f1eadb",
    /** The night field behind dark drawings. */
    night: "#0b2029",
    /** The deep space field behind illustrations. */
    space: "#081723",
    /** The water of the stack figure and the shallows, by appearance. */
    water: "light-dark(#2b6f82, #0a2e3a)",
    /** The water just below the surface, by appearance. */
    waterShallow: "light-dark(#5fb3c4, #1c5a68)",
    /** The water at the bottom of the stack figure, by appearance. */
    waterDeep: "light-dark(#2b6f82, #082a35)",
    /** The light the water nets into caustics and rays, by appearance. */
    waterLight: "light-dark(#d9f3f6, #7cc4d0)",
    /** The sunlit faces of the ice, by appearance. */
    ice: "light-dark(#fbfdfd, #dcebef)",
    /** The shaded faces of the ice, by appearance. */
    iceShadow: "light-dark(#a9cdd6, #7fa9b5)",
    /** The green ink. */
    green: "#3c8f58",
    /** The light green ink. */
    lightGreen: "#4caf6e",
    /** The blue ink. */
    blue: "#3d6fb0",
    /** Your own colour, beside the agent's violet. */
    teal: "#2f7d8c",
    /** The agent's colour. */
    violet: "#6b5ca5",
    /** The indigo ink. */
    indigo: "#5e6ad2",
    /** The ochre ink. */
    ochre: "#b8862b",
    /** The amber ink. */
    amber: "#d9a21b",
    /** The gold ink. */
    gold: "#e0a030",
    /** The umber ink. */
    umber: "#8a6510",
    /** The rust ink. */
    rust: "#a15c07",
    /** The red ink. */
    red: "#c0392b",
    /** The brick red ink. */
    brick: "#b03a2e",
    /** The surface of a generated app's pane. */
    artifactSurface: "#ffffff",
    /** The text of a generated app's pane. */
    artifactText: "#1f2937",
    /** The headings of a generated app's panels. */
    artifactHeading: "#111827",
    /** The ground of a generated app's panel headings. */
    artifactHeadingGround: "#f9fafb",
    /** The borders of a generated app's panels. */
    artifactBorder: "#e5e7eb",
    /** The primary button of a generated app. */
    artifactButton: "#4f46e5",
    /** The close button of a window's title bar. */
    windowClose: "#ff5f57",
    /** The minimize button of a window's title bar. */
    windowMinimize: "#febc2e",
    /** The zoom button of a window's title bar. */
    windowZoom: "#28c840",
});
