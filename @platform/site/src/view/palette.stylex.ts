import * as style from "@destack/style";

/** The inks of the site's figures and effects, the same in both appearances unless marked. */
export const palette = style.defineConsts({
    /** The brand's signal orange, drawn on figures and the mark. */
    signal: "#ff792e",
    /** The deeper brand orange, legible on white. */
    ember: "#e9581a",
    /** The white every app tile's glyph draws in on the sky. */
    tileGlyph: "#ffffff",
    /** The sky of the brand's icon as every app tile's field, lit at the top. */
    skyField: "linear-gradient(180deg, #24506a 0%, #143349 50%, #0a1c2b 100%)",
    /** An app's colour in the red of Mars, for calendars and alerts. */
    mars: "#ff5a4e",
    /** An app's colour in the orange of the brand's planet, for doing and hosting. */
    sun: "#ff792e",
    /** An app's colour in the amber of Saturn, for what you keep safe. */
    saturn: "#ffb547",
    /** An app's colour in the yellow of Venus, for mail and folders. */
    venus: "#ffd84d",
    /** An app's colour in the lime of an aurora, for code. */
    aurora: "#9be15d",
    /** An app's colour in the teal of a nebula's oxygen, for tables and charts. */
    oxygen: "#2fd6b5",
    /** An app's colour in the cyan of Uranus, for people's accounts. */
    uranus: "#4fd8f0",
    /** An app's colour in the blue of Neptune, for documents and data. */
    neptune: "#4f8cff",
    /** An app's colour in the violet of a nebula, for agents and search. */
    nebula: "#a77bff",
    /** An app's colour in the magenta of a pulsar, for talk and flows. */
    pulsar: "#ff5fb8",
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
    /** Your own colour, beside the agent's violet. */
    teal: "#2f7d8c",
    /** The agent's colour. */
    violet: "#6b5ca5",
    /** The indigo ink. */
    indigo: "#5e6ad2",
    /** The amber ink. */
    amber: "#d9a21b",
    /** The gold ink. */
    gold: "#e0a030",
    /** The rust ink. */
    rust: "#a15c07",
    /** The white ground vendor logos sit on, as their marks expect. */
    logoGround: "#ffffff",
    /** The close button of a window's title bar. */
    windowClose: "#ff5f57",
    /** The minimize button of a window's title bar. */
    windowMinimize: "#febc2e",
    /** The zoom button of a window's title bar. */
    windowZoom: "#28c840",
});
