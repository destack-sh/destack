import type { Environment } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium, type Page } from "playwright-core";
import sharp from "sharp";

/** The running site the figures are captured from, as `just dev` serves it. */
const SITE = "http://localhost:3737/";

/** The directory the README's images go in, beside the repository's other GitHub files. */
const OUTPUT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", ".github", "readme");

/** The device pixel ratio of the capture, about twice the README column's density. */
const SCALE = 1.5;

/** The milliseconds each state of a figure holds before it switches. */
const HOLD = 2000;

/** The milliseconds a figure takes to settle after it switches, before its held frame. */
const SETTLE = 1500;

/** The frames a wipe between the two states takes, the held frames apart. */
const WIPE_FRAMES = 40;

/** The milliseconds each wipe frame shows: a 2.4 second sweep. */
const WIPE_GAP = 60;

/** The site's accent, which the wipe's line, handle and labels wear. */
const ACCENT = "#ee7533";

/** The wipe's line, handle and labels, in CSS pixels. */
const SLIDER = {
    /** The line's width. */
    line: 3,
    /** The blur of the line's glow and the handle's shadow. */
    blur: 4,
    /** The handle's radius. */
    handle: 18,
    /** The handle's chevrons, from tip to arm end. */
    chevron: 5,
    /** The labels' font size. */
    font: 13,
    /** The width of a monospace character as a share of the font size. */
    advance: 0.62,
    /** The labels' height. */
    pill: 26,
    /** The labels' inner padding. */
    pad: 10,
    /** The labels' gap to the line. */
    gap: 8,
    /** The labels' distance from the window's top edge, below its title bar. */
    top: 52,
};

/** How far past its target the sweep overshoots before it settles, the back easing's constant. */
const OVERSHOOT = 1.2;

/** The share of the window's width the line rests from each edge, so both states always show. */
const REST = 0.05;

/** The WebP quality of the frames, high enough for crisp text. */
const QUALITY = 82;

/** The pixels of transparent margin around the windows, room for their shadows. */
const MARGIN = 12;

/** The figures the README shows: their file, section entry, viewport, window parts and height. */
const FIGURES = [
    {
        file: "space",
        entry: "verb",
        // a tablet layout, where the browser spans the frame at about 980 pixels
        viewport: { width: 1080, height: 900 },
        windows: ["section:has(#verb) [data-component=Browser]"],
        // taller than the layout's 480 pixels, about as tall for its width as the stack
        height: 690,
    },
    {
        file: "engine",
        entry: "noun",
        // a tablet layout, where the browser spans the frame at about 980 pixels
        viewport: { width: 1080, height: 900 },
        windows: ["section:has(#noun) [data-component=XRay]"],
        // taller than the layout's 544 pixels, about as tall for its width as the stack
        height: 690,
    },
    {
        file: "stack",
        entry: "past-participle",
        // a desktop layout tall enough for the whole stack, where the window is 1150 by 808 pixels
        viewport: { width: 1320, height: 1300 },
        windows: ["[data-component=LayerList]"],
        // the layout's own height
        height: undefined,
    },
] as const;

/** The colour scheme the README's images show, in GitHub's light and dark themes alike. */
const SCHEME = "light";

/** One captured frame of a figure and how long it shows. */
type Frame = { image: Buffer; delay: number };

/** The pixel size both states of a figure share. */
type Size = { width: number; height: number };

/** A figure of the site, by the file it goes to and the entry heading its section. */
type Figure = (typeof FIGURES)[number];

await exportFigures(BunEnvironment.system);

/** Capture every figure from one browser, and write each as a looping image. */
async function exportFigures(environment: Environment<"files" | "process">) {
    // make the output directory and open the installed Chrome
    const { files, process } = environment;
    await files.mkdir(OUTPUT, { recursive: true });
    const browser = await chromium.launch({ channel: "chrome" });

    // capture each figure at its viewport
    const context = await browser.newContext({ colorScheme: SCHEME, deviceScaleFactor: SCALE });
    const page = await context.newPage();
    for (const figure of FIGURES) {
        await page.setViewportSize(figure.viewport);
        await page.goto(SITE, { waitUntil: "load" });
        const frames = await capture(page, figure);
        const file = join(OUTPUT, `${figure.file}.webp`);
        await encode(frames, file);
        await process.stdout.write(`wrote ${file}\n`);
    }
    await browser.close();
}

/** Capture one figure stacked and destacked, rested and wiped between, as frames to loop. */
async function capture(page: Page, figure: Figure): Promise<readonly Frame[]> {
    // bring the figure's label to the top of the browser window, and clear everything behind its windows
    const clip = await page.evaluate(
        ({ entry, windows, height, margin }) => {
            // find the figure's label and scroll it to the top
            const label = document.getElementById(entry)?.closest("section")?.querySelector("[data-figure-label]");
            if (!label) {
                throw new Error(`no figure under the entry ${entry}`);
            }
            label.scrollIntoView({ block: "start" });

            // find each window from a part of it: the nearest element around or inside it that casts a shadow
            const frames = windows.map((part) => {
                // walk up from the part, then look inside it
                const found = document.querySelector(part);
                let around = found;
                while (around && getComputedStyle(around).boxShadow === "none") {
                    around = around.parentElement;
                }
                const inside = found
                    ? [...found.querySelectorAll("*")].find(
                          (element) => getComputedStyle(element).boxShadow !== "none",
                      )
                    : undefined;
                const frame = around ?? inside;
                if (!(frame instanceof HTMLElement)) {
                    throw new Error(`no window around or inside ${part}`);
                }

                return frame;
            });

            // size the windows to the figure's height, where it has one, and let their section grow around them
            for (const frame of frames) {
                if (height !== undefined) {
                    frame.style.height = `${height}px`;
                    for (let element = frame.parentElement; element; element = element.parentElement) {
                        element.style.height = "auto";
                        element.style.gridTemplateRows = "none";
                        if (element.tagName === "SECTION") {
                            break;
                        }
                    }
                }
            }

            // clear the backgrounds behind the windows, up to the page itself
            for (const frame of frames) {
                for (let element = frame.parentElement; element; element = element.parentElement) {
                    element.style.background = "transparent";
                }
            }

            // clip to the windows with a margin for their shadows
            const rectangles = frames.map((frame) => frame.getBoundingClientRect());
            const left = Math.min(...rectangles.map((rectangle) => rectangle.left)) - margin;
            const top = Math.min(...rectangles.map((rectangle) => rectangle.top)) - margin;
            const right = Math.max(...rectangles.map((rectangle) => rectangle.right)) + margin;
            const bottom = Math.max(...rectangles.map((rectangle) => rectangle.bottom)) + margin;

            return { x: left, y: top, width: right - left, height: bottom - top };
        },
        { entry: figure.entry, windows: [...figure.windows], height: figure.height, margin: MARGIN },
    );

    // hold a browser figure on its first tab, which stops it turning its pages on its own
    const tab = page.locator(`section:has(#${figure.entry}) [data-component=Tab]`).first();
    if ((await tab.count()) > 0) {
        await tab.click();
    }
    await page.waitForTimeout(SETTLE);

    // capture the figure stacked and destacked, each once it has settled
    const toggle = page.locator(`section:has(#${figure.entry}) [role=switch]`).first();
    const stacked = await page.screenshot({ clip, omitBackground: true });
    await toggle.click();
    await page.waitForTimeout(SETTLE);
    const destacked = await page.screenshot({ clip, omitBackground: true });

    // rest the line near the left edge, sweep it near the right edge, rest, and sweep it back
    const { width, height } = await sharp(stacked).metadata();
    const size = { width, height };
    const inset = MARGIN * SCALE;
    const start = Math.round(inset + (width - inset * 2) * REST);
    const end = Math.round(inset + (width - inset * 2) * (1 - REST));

    return [
        { image: await split(stacked, destacked, start, size), delay: HOLD },
        ...(await wipe(stacked, destacked, start, end, size)),
        { image: await split(stacked, destacked, end, size), delay: HOLD },
        ...(await wipe(stacked, destacked, end, start, size)),
    ];
}

/** Return the frames of the line sweeping from one place to another, eased in and out. */
async function wipe(
    stacked: Buffer,
    destacked: Buffer,
    from: number,
    to: number,
    size: Size,
): Promise<readonly Frame[]> {
    const frames: Frame[] = [];
    for (let step = 1; step < WIPE_FRAMES; step++) {
        // place the line at this step of the sweep, overshooting a little before it settles
        const progress = easeInOutBack(step / WIPE_FRAMES);
        const position = Math.round(from + (to - from) * progress);
        frames.push({
            image: await split(stacked, destacked, position, size),
            delay: WIPE_GAP,
        });
    }

    return frames;
}

/** Ease in and out with a small overshoot at each end, as the easings.net back curve does. */
function easeInOutBack(time: number) {
    const scaled = OVERSHOOT * 1.525;

    return time < 0.5
        ? ((2 * time) ** 2 * ((scaled + 1) * 2 * time - scaled)) / 2
        : ((2 * time - 2) ** 2 * ((scaled + 1) * (time * 2 - 2) + scaled) + 2) / 2;
}

/** Draw both states split at a line: destacked left of it, stacked right of it, labelled. */
async function split(stacked: Buffer, destacked: Buffer, x: number, size: Size) {
    // cut the two halves at the line
    const { width, height } = size;
    const halves = [
        ...(x > 0
            ? [{ input: await sharp(destacked).extract({ left: 0, top: 0, width: x, height }).toBuffer(), left: 0, top: 0 }]
            : []),
        ...(x < width
            ? [{ input: await sharp(stacked).extract({ left: x, top: 0, width: width - x, height }).toBuffer(), left: x, top: 0 }]
            : []),
    ];

    // draw the line across the window, a handle at its middle, and a label riding on each side
    const line = SLIDER.line * SCALE;
    const blur = SLIDER.blur * SCALE;
    const inset = MARGIN * SCALE;
    const font = SLIDER.font * SCALE;
    const pill = { height: SLIDER.pill * SCALE, pad: SLIDER.pad * SCALE, gap: SLIDER.gap * SCALE, top: inset + SLIDER.top * SCALE };
    const handle = { radius: SLIDER.handle * SCALE, middle: height / 2, chevron: SLIDER.chevron * SCALE };
    const widthOf = (text: string) => text.length * font * SLIDER.advance + pill.pad * 2;
    const left = "DE·STACKED";
    const right = "STACKED";

    // show a side's label only where it fits inside the window
    const hasLeft = x - pill.gap - widthOf(left) >= inset + pill.gap;
    const hasRight = x + pill.gap + widthOf(right) <= width - inset - pill.gap;
    const leftLabel = hasLeft
        ? `<rect x="${x - pill.gap - widthOf(left)}" y="${pill.top}" width="${widthOf(left)}" height="${pill.height}" rx="${pill.height / 2}" fill="${ACCENT}"/>
            <text x="${x - pill.gap - pill.pad}" y="${pill.top + pill.height * 0.68}" text-anchor="end" fill="#ffffff">${left}</text>`
        : "";
    const rightLabel = hasRight
        ? `<rect x="${x + pill.gap}" y="${pill.top}" width="${widthOf(right)}" height="${pill.height}" rx="${pill.height / 2}" fill="${ACCENT}"/>
            <text x="${x + pill.gap + pill.pad}" y="${pill.top + pill.height * 0.68}" fill="#ffffff">${right}</text>`
        : "";
    const overlay = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">
        <defs>
            <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
                <feGaussianBlur stdDeviation="${blur}"/>
            </filter>
        </defs>
        <rect x="${x - line * 2}" y="${inset}" width="${line * 4}" height="${height - inset * 2}" fill="${ACCENT}" opacity="0.45" filter="url(#glow)"/>
        <rect x="${x - line / 2}" y="${inset}" width="${line}" height="${height - inset * 2}" fill="${ACCENT}"/>
        <circle cx="${x}" cy="${handle.middle + blur / 2}" r="${handle.radius + blur / 2}" fill="#000000" opacity="0.28" filter="url(#glow)"/>
        <circle cx="${x}" cy="${handle.middle}" r="${handle.radius}" fill="${ACCENT}" stroke="#ffffff" stroke-width="${line * 0.7}"/>
        <path d="M${x - handle.chevron * 0.4} ${handle.middle - handle.chevron} l-${handle.chevron} ${handle.chevron} l${handle.chevron} ${handle.chevron} M${x + handle.chevron * 0.4} ${handle.middle - handle.chevron} l${handle.chevron} ${handle.chevron} l-${handle.chevron} ${handle.chevron}" fill="none" stroke="#ffffff" stroke-width="${line * 0.85}" stroke-linecap="round" stroke-linejoin="round"/>
        <g font-family="ui-monospace, Menlo, monospace" font-size="${font}" font-weight="600">
            ${leftLabel}
            ${rightLabel}
        </g>
    </svg>`;

    return sharp({ create: { width, height, channels: 4, background: "#00000000" } })
        .composite([...halves, { input: Buffer.from(overlay), left: 0, top: 0 }])
        .png()
        .toBuffer();
}

/** Encode frames as one looping animated WebP. */
async function encode(frames: readonly Frame[], file: string) {
    await sharp(
        frames.map((frame) => frame.image),
        { join: { animated: true } },
    )
        .webp({ delay: frames.map((frame) => frame.delay), loop: 0, quality: QUALITY })
        .toFile(file);
}
