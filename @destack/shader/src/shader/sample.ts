/** A ringed planet on transparent ground, the image the image effects' examples draw. */
const PLANET = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><circle cx="32" cy="32" r="16" fill="#ff792e"/><ellipse cx="32" cy="32" rx="28" ry="9" fill="none" stroke="#ff792e" stroke-width="4" transform="rotate(-24 32 32)"/></svg>`;

/** The planet as an image address. */
export const SAMPLE_IMAGE = `data:image/svg+xml,${encodeURIComponent(PLANET)}`;
