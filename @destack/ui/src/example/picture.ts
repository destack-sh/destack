import { nth } from "@destack/schema";

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
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} ${String(height)}" width="${String(width)}" height="${String(height)}" preserveAspectRatio="xMidYMid slice"><defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${colors.top}"/><stop offset="1" stop-color="${colors.bottom}"/></linearGradient></defs><rect width="${String(width)}" height="${String(height)}" fill="url(#sky)"/><g fill="${colors.shape}">${shape}</g></svg>`;

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

/** The colors of a landscape: its sky, its light and its ridges. */
interface Hour {
    /** The sky's colors, from its top to the horizon. */
    readonly sky: readonly [string, string, string];
    /** The color of the light. */
    readonly light: string;
    /** The color of the farthest ridge. */
    readonly far: string;
    /** The color of the nearest ridge. */
    readonly near: string;
}

/** The hours of a day landscapes are drawn at. */
const HOURS: readonly Hour[] = [
    { sky: ["#274b78", "#7d8fb3", "#f3c6a5"], light: "#fff1d0", far: "#8f93ad", near: "#141c2b" },
    { sky: ["#3f7fb8", "#8fc0de", "#e6f1f2"], light: "#ffffff", far: "#9dbccb", near: "#1d3a2f" },
    { sky: ["#2b2350", "#8a4a79", "#f59a6b"], light: "#ffe0a8", far: "#8d5878", near: "#1a1226" },
    { sky: ["#0b1730", "#1d3a5f", "#4f7f97"], light: "#e9f3ff", far: "#34566f", near: "#050b16" },
    { sky: ["#b8c9cf", "#d6dfdf", "#f0ede4"], light: "#fffdf4", far: "#b9c5c4", near: "#37474a" },
    { sky: ["#5a6fa5", "#c99bb0", "#ffd7b0"], light: "#fff4dc", far: "#b394ad", near: "#2c2338" },
    { sky: ["#1f6f78", "#7fb7a6", "#f2e3b3"], light: "#fff7d6", far: "#7fa795", near: "#102a2a" },
];

/** The height a landscape is drawn at. */
const LANDSCAPE_HEIGHT = 1000;

/** Draw a landscape of ridges under a sky as an SVG data URL, the same one for the same number. */
export function landscape(number: number, ratio: number): string {
    // draw numbers from the landscape's own, which repeat
    let seed = (number + 1) * 2654435761;
    const random = (): number => {
        seed = (seed * 1664525 + 1013904223) % 4294967296;

        return seed / 4294967296;
    };
    const hour = nth(HOURS, number % HOURS.length);

    // place the horizon and the sun
    const width = Math.round(LANDSCAPE_HEIGHT * ratio);
    const horizon = 520 + random() * 120;
    const sun = { x: width * (0.2 + random() * 0.6), y: horizon - 90 - random() * 220 };

    // draw each ridge lower, rougher and darker than the one behind it
    const ridges = Array.from({ length: 5 }, (_ridge, depth) => {
        // sum waves, each half as long as the last
        const base = horizon - 30 + depth * 105;
        const height = 190 - depth * 22;
        const waves = Array.from({ length: 5 }, (_wave, octave) => ({
            length: (width * (0.9 + random() * 0.5)) / 2 ** octave,
            phase: random() * Math.PI * 2,
            height: height / 1.9 ** octave,
        }));

        // follow them from side to side
        const points = Array.from({ length: 121 }, (_point, step) => {
            const x = (width * step) / 120;
            const rise = waves.reduce(
                (sum, wave) =>
                    sum +
                    Math.abs(Math.sin((x / wave.length) * Math.PI + wave.phase)) * wave.height,
                0,
            );

            return `${x.toFixed(0)} ${(base - rise + height * 0.8).toFixed(0)}`;
        });
        const share = String(Math.round((depth / 4) * 100));
        const fill = `color-mix(in oklab, ${hour.near} ${share}%, ${hour.far})`;

        return `<path d="M0 ${String(LANDSCAPE_HEIGHT)} L${points.join(" L")} L${String(width)} ${String(LANDSCAPE_HEIGHT)}Z" style="fill:${fill}"/><rect y="${(base - 40).toFixed(0)}" width="100%" height="220" fill="url(#haze)"/>`;
    });

    // put them under the sky, with a little grain over all
    const [top, middle, bottom] = hour.sky;
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${String(width)} ${String(LANDSCAPE_HEIGHT)}" width="${String(width)}" height="${String(LANDSCAPE_HEIGHT)}"><defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop stop-color="${top}"/><stop offset="0.35" stop-color="${middle}"/><stop offset="0.65" stop-color="${bottom}"/></linearGradient><radialGradient id="glow"><stop stop-color="${hour.light}" stop-opacity="0.95"/><stop offset="0.12" stop-color="${hour.light}" stop-opacity="0.55"/><stop offset="1" stop-color="${hour.light}" stop-opacity="0"/></radialGradient><linearGradient id="haze" x1="0" y1="0" x2="0" y2="1"><stop stop-color="${bottom}" stop-opacity="0"/><stop offset="1" stop-color="${bottom}" stop-opacity="0.16"/></linearGradient><filter id="grain"><feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="${String(number)}"/><feColorMatrix values="0 0 0 0 0.5 0 0 0 0 0.5 0 0 0 0 0.5 0 0 0 0.5 0"/></filter></defs><rect width="100%" height="100%" fill="url(#sky)"/><circle cx="${sun.x.toFixed(0)}" cy="${sun.y.toFixed(0)}" r="520" fill="url(#glow)"/><circle cx="${sun.x.toFixed(0)}" cy="${sun.y.toFixed(0)}" r="42" fill="${hour.light}"/>${ridges.join("")}<rect width="100%" height="100%" filter="url(#grain)" opacity="0.14"/></svg>`;

    return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}
