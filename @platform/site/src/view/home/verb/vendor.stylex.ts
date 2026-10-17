import * as style from "@destack/style";

/** The colours of the prompt-built waitlist app. */
export const replit = style.defineConsts({
    /** The page background. */
    night: "light-dark(#ffffff, #0b0d17)",
    /** The badge background. */
    navy: "#0e1525",
    /** The violet of the backdrop glow and the button. */
    violet: "#7c3aed",
    /** The lilac of the logo and the headline glow. */
    lilac: "#a78bfa",
    /** The pink of the headline glow. */
    pink: "#f472b6",
    /** The magenta of the button. */
    magenta: "#db2777",
    /** The text. */
    snow: "light-dark(#0e1525, #f5f7fb)",
});

/** The colours of a personal app's plain site. */
export const personal = style.defineConsts({
    /** The page background. */
    surface: "light-dark(#ffffff, #18181b)",
    /** The text. */
    ink: "light-dark(#242428, #ececf0)",
    /** Secondary text. */
    muted: "light-dark(#7b7b85, #a1a1aa)",
    /** The figures beside each row. */
    figure: "light-dark(#4a4a52, #d4d4d8)",
    /** The rules and card borders. */
    line: "light-dark(#ececf0, #2e2e33)",
    /** The avatar's fill. */
    face: "light-dark(#dde3ea, #3f3f46)",
});

/** The colours of the chat workspace. */
export const slack = style.defineConsts({
    /** The page background. */
    surface: "light-dark(#ffffff, #1a1d21)",
    /** The message text. */
    ink: "light-dark(#1d1c1d, #d1d2d3)",
    /** The top bar and the rail. */
    aubergine: "#350d36",
    /** The channel sidebar. */
    sidebar: "#3f0e40",
    /** The selected channel and thread links. */
    link: "light-dark(#1264a3, #1d9bd1)",
    /** Secondary text. */
    muted: "light-dark(#616061, #ababad)",
    /** The border of a mention chip. */
    highlight: "#1d9bd1",
    /** The composer border. */
    border: "light-dark(#bbbbbb, #565856)",
    /** The member list border. */
    line: "light-dark(#dddddd, #35373b)",
    /** Tab rules and app icon fills. */
    rule: "light-dark(#e8e8e8, #35373b)",
    /** The guest badge. */
    notice: "light-dark(#fff8e1, #3b3214)",
    /** The guest badge text. */
    noticeInk: "light-dark(#5c4a00, #f2c744)",
    /** The mention chip. */
    mention: "light-dark(#e8f5fa, #1d3a4a)",
});

/** The colours of the code host in its light and dark themes. */
export const github = style.defineConsts({
    /** The top bar and tab strip. */
    inset: "light-dark(#f6f8fa, #010409)",
    /** The page background. */
    canvas: "light-dark(#ffffff, #0d1117)",
    /** The latest commit row. */
    overlay: "light-dark(#f6f8fa, #151b23)",
    /** The topic tag background. */
    accentMuted: "light-dark(#ddf4ff, #121d2f)",
    /** Button and branch fills. */
    subtle: "light-dark(#f6f8fa, #212830)",
    /** Counter fills. */
    neutral: "light-dark(#e6eaef, #2f3742)",
    /** Borders. */
    border: "light-dark(#d1d9e0, #3d444d)",
    /** The code button. */
    success: "light-dark(#1f883d, #238636)",
    /** Secondary text. */
    muted: "light-dark(#59636e, #9198a1)",
    /** The topic tag text. */
    accent: "light-dark(#0969da, #4493f8)",
    /** The selected tab underline. */
    attention: "light-dark(#fd8c73, #f78166)",
    /** The text. */
    foreground: "light-dark(#1f2328, #e6edf3)",
});

/** The colours of the site builder. */
export const framer = style.defineConsts({
    /** The page background. */
    page: "light-dark(#ffffff, #0a0a0a)",
    /** The text, and the fill of buttons and badges. */
    ink: "light-dark(#0a0a0a, #ffffff)",
    /** The live dot. */
    green: "#4ade80",
});

/** The colours of the scheduling page. */
export const calendly = style.defineConsts({
    /** The card background. */
    surface: "light-dark(#ffffff, #1b1e23)",
    /** The text and card borders. */
    ink: "light-dark(#1a1a1a, #e8e9eb)",
    /** The title and event text. */
    navy: "light-dark(#0a2540, #e6edf5)",
    /** The ribbon. */
    slate: "#505960",
    /** The brand blue of chosen and open days. */
    blue: "light-dark(#0069ff, #3d8bff)",
    /** The text of open days. */
    deepBlue: "light-dark(#0060e6, #7fb0ff)",
    /** The page background. */
    mist: "light-dark(#fafafa, #121418)",
});

/** The colours of the notes workspace. */
export const notion = style.defineConsts({
    /** The page background. */
    surface: "light-dark(#ffffff, #191919)",
    /** The dark button, light on dark pages. */
    charcoal: "light-dark(#2f2f2f, #f0efed)",
    /** The text. */
    ink: "light-dark(#37352f, #d4d4d2)",
    /** The status tag text. */
    navy: "light-dark(#183347, #d3e5ef)",
    /** The status dot. */
    blue: "#337ea9",
    /** Checked boxes. */
    link: "#2383e2",
    /** The status tag. */
    sky: "light-dark(#d3e5ef, #28456c)",
    /** The callout. */
    sand: "light-dark(#f1f1ef, #2a2a2a)",
    /** The cover's light paint. */
    paper: "#e8d5b5",
    /** The cover's middle paint. */
    tan: "#d9a679",
    /** The cover's dark paint. */
    clay: "#b46d4e",
});

/** The colours of the issue tracker. */
export const linear = style.defineConsts({
    /** The page background. */
    surface: "light-dark(#ffffff, #1b1c21)",
    /** The heading and selected text. */
    ink: "light-dark(#282a30, #eeeff1)",
    /** The text. */
    text: "light-dark(#3c4149, #d0d2d6)",
    /** Secondary text. */
    muted: "light-dark(#6b6f76, #8a8f98)",
    /** The team mark and label dot. */
    orange: "#d9622b",
    /** The team badge. */
    peach: "light-dark(#fde4d6, #4a2a1a)",
    /** The composer ring. */
    ring: "light-dark(#e0e0e3, #33353c)",
    /** Borders. */
    border: "light-dark(#e3e3e6, #2e3037)",
    /** The sidebar rule and the selected row. */
    rule: "light-dark(#e9e9eb, #2a2c32)",
    /** The header's rule. */
    divider: "light-dark(#ededef, #26282e)",
    /** Row rules and the selected tab. */
    line: "light-dark(#f0f0f1, #24262b)",
    /** Group rows. */
    hover: "light-dark(#f4f4f5, #212328)",
    /** The sidebar. */
    panel: "light-dark(#f7f7f8, #16171b)",
});

/** The colours of the rented inbox. */
export const gmail = style.defineConsts({
    /** The page background. */
    canvas: "light-dark(#f6f8fc, #1b1b1b)",
    /** The search field. */
    field: "light-dark(#e9eef6, #333333)",
    /** The list and the compose window. */
    surface: "light-dark(#ffffff, #2a2a2a)",
    /** Read rows. */
    read: "light-dark(#f2f6fc, #242424)",
    /** The text. */
    ink: "light-dark(#1f1f1f, #e3e3e3)",
    /** Quiet text. */
    muted: "light-dark(#5e5e5e, #a8a8a8)",
    /** Row rules. */
    rule: "light-dark(#e7eaf0, #333333)",
    /** The selected folder. */
    selected: "light-dark(#d3e3fd, #004a77)",
    /** The compose button. */
    compose: "light-dark(#c2e7ff, #004a77)",
    /** The send button. */
    blue: "#0b57d0",
    /** The compose window's header. */
    header: "light-dark(#f2f6fc, #333333)",
    /** The limit warning. */
    warning: "light-dark(#b3261e, #f2b8b5)",
});
