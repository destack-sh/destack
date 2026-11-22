/** The colors of a drawn picture: the sky's two ends and the shape in front of it. */
interface Colors {
    /** The top of the sky. */
    readonly top: string;
    /** The bottom of the sky. */
    readonly bottom: string;
    /** The shape in front of the sky. */
    readonly shape: string;
}

/** Draw a picture as an SVG data URL, so an example shows an image without a file. */
function draw(colors: Colors, shape: string, width = 640, height = 360): string {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} ${String(height)}" preserveAspectRatio="xMidYMid slice"><defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${colors.top}"/><stop offset="1" stop-color="${colors.bottom}"/></linearGradient></defs><rect width="${String(width)}" height="${String(height)}" fill="url(#sky)"/><g fill="${colors.shape}">${shape}</g></svg>`;

    return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** The pictures examples show: three photos of a trip to Lisbon and two portraits. */
export const PICTURES = {
    /** Rooftops under a dusk sky. */
    alfama: draw(
        { top: "#3b4a8c", bottom: "#f4a261", shape: "#2b2d42" },
        '<path d="M0 290 L70 240 L130 290 L190 220 L270 290 L340 250 L400 290 L470 210 L560 290 L640 250 L640 360 L0 360Z"/><circle cx="500" cy="120" r="34" fill="#ffe8b6"/>',
    ),
    /** A tram on a hill. */
    tram: draw(
        { top: "#8ecae6", bottom: "#fefae0", shape: "#e9b44c" },
        '<path d="M0 300 L640 220 L640 360 L0 360Z" fill="#6c757d"/><rect x="230" y="170" width="190" height="86" rx="12" transform="rotate(-7 325 213)"/>',
    ),
    /** Pastries on a counter. */
    belem: draw(
        { top: "#fff3d6", bottom: "#f6d7a7", shape: "#c8823a" },
        '<circle cx="200" cy="210" r="62"/><circle cx="330" cy="230" r="62"/><circle cx="460" cy="205" r="62"/><rect x="0" y="290" width="640" height="70" fill="#8d6e63"/>',
    ),
    /** A portrait on indigo. */
    ada: draw(
        { top: "#5c6bc0", bottom: "#3949ab", shape: "#f5e6d3" },
        '<circle cx="160" cy="130" r="56"/><path d="M50 320 a110 110 0 0 1 220 0Z"/>',
        320,
        320,
    ),
    /** A portrait on teal. */
    grace: draw(
        { top: "#26a69a", bottom: "#00796b", shape: "#f1dcc9" },
        '<circle cx="160" cy="130" r="56"/><path d="M50 320 a110 110 0 0 1 220 0Z"/>',
        320,
        320,
    ),
} as const;
