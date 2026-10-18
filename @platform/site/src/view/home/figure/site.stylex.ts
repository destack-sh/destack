import * as style from "@destack/style";

/** The shared measures of a rented site drawn in a window. */
export const site = style.defineConsts({
    /** A window narrower than a phone screen, where each site takes its own phone layout. */
    narrow: "@container (max-width: 479px)",
    /** A window narrower than a laptop's, where each site drops its secondary parts. */
    compact: "@container (max-width: 899px)",
    /** Florian's avatar on every rented site. */
    avatar: "#2f7d8c",
    /** Your agent's avatar on every rented site. */
    agent: "#6b5ca5",
    /** The launch space's mark on every rented site. */
    space: "#c64a17",
    /** The system font the rented sites fall back to. */
    system: '-apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif',
});
