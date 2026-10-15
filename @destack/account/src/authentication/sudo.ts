import type { Elevation } from "@destack/access";

/** The authentication sensitive changes ask for, sudo's fifteen minutes. */
export const sudo: Elevation = { assurance: 2, maxAge: 15 * 60 * 1000 };
