import type { Elevation } from "@destack/access";

/** How recent an authentication sensitive changes ask for: fifteen minutes, as sudo mode lasts. */
export const SUDO_MILLISECONDS = 15 * 60_000;

/** The authentication sensitive changes ask for: a second factor within the sudo window. */
export const sudo: Elevation = { assurance: 2, maxAge: SUDO_MILLISECONDS };
