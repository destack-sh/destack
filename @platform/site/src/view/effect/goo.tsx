import { frame } from "../layout/frame.stylex";
import * as style from "@destack/style";
import { Shader, type ShaderFailure, type ShaderMount } from "@destack/shader";
import { color } from "@destack/theme/tokens.stylex";
import { createSignal, type JSX, onCleanup } from "@destack/view";

import { night } from "../theme.ts";
import { captureException } from "../telemetry.ts";
import { isMotionReduced } from "@destack/view/motion";
import { makeEventListener } from "@destack/view/primitives/event-listener";
import { makeResizeObserver } from "@destack/view/primitives/resize-observer";

/** The distance the goo field reaches past the site frame, in CSS pixels, so its rim can wobble across the frame rules. */
const spill = 8;
/** How far past its cells the goo can reach, in CSS pixels: its rim, the charge swell and the pointer's pull, with room to spare. */
const clipMargin = 24;
/** The deep space behind the goo's stars, the same in both appearances. */
const SPACE = "#081723";
/** The milliseconds the field takes to fade in over the still stars. */
const fadeTime = 500;
/** How far an island's rim lies outside its cell, in CSS pixels. */
const rimOffset = 3;
/** The most goo islands the field draws. */
const islandCapacity = 4;
/** The seconds the goo's bulge takes to catch up with the pointer, so it follows like something thick. */
const gooLag = 0.6;
/** The seconds the goo takes to swell toward a pointer that comes near. */
const swellTime = 1;
/** The seconds the goo takes to sag back once the pointer leaves. */
const sagTime = 2.4;
/** The milliseconds between measurements of the goo cells, in case the page shifts under them without resizing. */
const remeasureTime = 1000;
/** The milliseconds between frames while the goo only twinkles: thirty a second. */
const twinklePace = 33;
/** The share of far star cells that hold a star. */
const distantStars = "0.14";
/** The share of near star cells that hold a star. */
const closeStars = "0.16";

/** A still tile of stars that shows before the shader paints, seeded so server and browser agree. */
export const stillStars = (() => {
    // draw a dozen stars from a fixed seed
    let seed = 11;
    const random = () => {
        seed = (seed * 16807) % 2147483647;
        return seed / 2147483647;
    };
    let stars = "";
    for (let index = 0; index < 12; index++) {
        const x = (random() * 240).toFixed(1);
        const y = (random() * 120).toFixed(1);
        const size = random();
        const radius = size < 0.7 ? 0.7 : size < 0.93 ? 1.1 : 1.6;
        const alpha = (0.35 + random() * 0.55).toFixed(2);
        stars += `<circle cx='${x}' cy='${y}' r='${radius}' fill='#f1eadb' fill-opacity='${alpha}'/>`;
    }
    const tile = `<svg xmlns='http://www.w3.org/2000/svg' width='240' height='120'>${stars}</svg>`;

    return `url("data:image/svg+xml,${encodeURIComponent(tile)}")`;
})();

/** The goo fragment shader. */
export const GOO_FRAGMENT = `#version 300 es
precision highp float;
uniform mediump vec2 u_resolution;
uniform mediump float u_pixelRatio;
uniform float u_clock;
uniform float u_wobble;
uniform vec2 u_pointer;
uniform vec2 u_origin;
uniform float u_pull;
uniform vec2 u_far;
uniform vec2 u_near;
uniform float u_charge;
uniform vec4 u_islands[${islandCapacity}];
uniform float u_islandCount;
uniform vec4 u_clip;
uniform vec4 u_page;

out vec4 fragColor;

const vec3 space = vec3(0.031, 0.09, 0.137);
const vec3 starColor = vec3(0.945, 0.918, 0.859);
const vec3 inkColor = vec3(0.071, 0.192, 0.235);

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float box(vec2 p, vec2 extent, float radius) {
    vec2 q = abs(p) - extent + radius;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

// return a sparse field of flat, crisp stars in a few sizes
vec3 starLayer(vec2 p, float cellSize, float density, float t) {
    vec2 cell = floor(p / cellSize);
    vec2 local = fract(p / cellSize) * cellSize;
    float seed = hash(cell);
    if (seed > density) {
        return vec3(0.0);
    }
    vec2 centre = (vec2(hash(cell + 1.3), hash(cell + 2.7)) * 0.7 + 0.15) * cellSize;
    float size = hash(cell + 5.1);
    float radius = size < 0.7 ? 0.7 : (size < 0.93 ? 1.1 : 1.6);
    float disc = 1.0 - smoothstep(radius - 0.5, radius + 0.5, length(local - centre));
    float alpha = mix(0.35, 0.9, hash(cell + 7.7)) * (0.8 + 0.2 * sin(t * (0.3 + hash(cell + 9.0) * 0.5) + seed * 40.0));
    return starColor * disc * alpha;
}

void main() {
    // work in CSS pixels from the top left
    vec2 size = u_resolution / u_pixelRatio;
    vec2 frag = vec2(gl_FragCoord.x, u_resolution.y - gl_FragCoord.y) / u_pixelRatio;
    if (u_islandCount < 0.5 || frag.x < u_clip.x || frag.y < u_clip.y || frag.x > u_clip.z || frag.y > u_clip.w) {
        discard;
    }
    float outline = step(0.5, dot(u_page.rgb, vec3(0.2126, 0.7152, 0.0722)));

    // measure the island cells
    float d = 1e5;
    for (int i = 0; i < ${islandCapacity}; i++) {
        if (float(i) < u_islandCount) {
            vec4 island = u_islands[i];
            d = min(d, box(frag - island.xy, island.zw + u_charge * 3.0, 6.0));
        }
    }
    if (d > 140.0) {
        discard;
    }

    // sag and swell the edge very slowly
    vec2 page = frag + u_origin;
    float slow = noise(page * 0.007 + vec2(u_clock * 0.025, u_clock * 0.018)) - 0.5;
    float swell = noise(page * 0.013 - vec2(u_clock * 0.03, u_clock * 0.012)) - 0.5;
    d -= (slow * 2.0 + swell) * u_wobble;

    // swell gently toward a u_pointer that comes u_near
    float away = length(frag - u_pointer);
    d -= u_pull * 4.0 * exp(-away * away / 2400.0);

    // clip to the goo and trace its edge with one faint line: ink on a light page, light on a dark one
    float edge = 1.0 / u_pixelRatio;
    float inside = 1.0 - smoothstep(-edge, edge, d);
    float line = 1.0 - smoothstep(0.3, 0.9, abs(d - 0.5));
    if (inside + line < 0.001) {
        discard;
    }
    vec2 sky = frag;

    // glow in slow page-wide clouds under two depths of sparse flat stars placed across the page
    float falloff = smoothstep(0.25, 0.85, noise(page * 0.0012 + vec2(u_clock * 0.004, 0.0)));
    vec3 nebula = (vec3(0.09, 0.2, 0.27) - space) * falloff * 0.45;
    vec3 distant = starLayer(sky + u_far + vec2(u_clock * 1.2, u_clock * 0.3), 26.0, ${distantStars}, u_clock);
    vec3 close = starLayer(sky + u_near + 71.0 + vec2(u_clock * 2.4, u_clock * 0.6), 58.0, ${closeStars}, u_clock * 1.3) * 1.25;

    // glow faintly inside the rim so the edge reads as a surface
    float sheen = (1.0 - smoothstep(0.0, 14.0, -d)) * 0.05;
    vec3 color = space + nebula * (1.0 + u_charge) + sheen + (distant * 0.6 + close * 0.8) * (1.0 + u_charge * 0.6);

    vec3 tone = mix(starColor, inkColor, outline);
    float stroke = line * (1.0 - inside) * mix(0.28, 0.4, outline);
    color = mix(color, tone, stroke / max(inside + stroke, 0.001));

    float alpha = max(inside, stroke);
    fragColor = vec4(color * alpha, alpha);
}
`;

/** How charged the goo is, from 0 to 1, while the Destack switch is hovered. */
const pageCharge = { target: 0 };

/** Charge the page's goo to a level: 0 at rest, 1 while the switch promises Destack, and more while the switch is held down. */
export function charge(level: number) {
    pageCharge.target = level;
}

/** Whether the goo field has painted, so its cells let it show through instead of their still stars. */
const [isFieldPainted, setIsFieldPainted] = createSignal(false);

/** Mark a lattice cell as an island of goo, drawn by the site's one field behind its content. */
export function Goo(properties: { children?: JSX.Element; xstyle?: style.Styles }) {
    return (
        <div data-goo {...style.attributes([styles.host, properties.xstyle], night)}>
            {/* stand in with still stars until the field paints */}
            <span
                aria-hidden="true"
                style={{ "background-image": stillStars }}
                {...style.attrs(styles.backdrop, isFieldPainted() && styles.hidden)}
            />
            <div {...style.attrs(styles.content)}>{properties.children}</div>
        </div>
    );
}

/** Draw the site's one goo field behind its content, filling every island cell. */
export function GooField() {
    // hold the frame loop, ending it with the field
    let frameRequest = 0;
    let stop: (() => void) | undefined;
    onCleanup(() => stop?.());

    // drive the mount frame by frame: ease the pointer, the swell and the charge, and follow the cells as the page scrolls
    const start = (mount: ShaderMount) => {
        // hold the motion preference, the pointer, the eased motion and the measured cells
        const isStill = isMotionReduced(mount.canvas);
        const pointer = { x: 0, y: 0, isKnown: false };
        const eased = { x: 0, y: 0, pull: 0, charge: 0 };
        let cells: HTMLElement[] = [];
        let pageIslands: Rect[] = [];
        let measuredAt = Number.NEGATIVE_INFINITY;
        let lastAt = 0;
        let drawnAt = Number.NEGATIVE_INFINITY;

        // measure again whenever the window or the page changes size
        const remeasure = () => {
            measuredAt = Number.NEGATIVE_INFINITY;
        };
        const pageSize = makeResizeObserver(remeasure);
        pageSize.observe(document.body);
        const stopResizing = makeEventListener(window, "resize", remeasure);

        // follow the pointer across the page
        const point = (event: PointerEvent) => {
            pointer.x = event.clientX;
            pointer.y = event.clientY;
            pointer.isKnown = true;
        };
        const stopPointing = makeEventListener(window, "pointermove", point, { passive: true });

        // draw one frame, and keep going while the goo moves
        const draw = (now: number) => {
            // find the cells again once any leaves the page, and measure them in page pixels now and then
            if (cells.length === 0 || cells.some((cell) => !cell.isConnected)) {
                cells = [...document.querySelectorAll<HTMLElement>("[data-goo]")].slice(
                    0,
                    islandCapacity,
                );
                measuredAt = Number.NEGATIVE_INFINITY;
            }
            const scroll = { x: window.scrollX, y: window.scrollY };
            if (now - measuredAt > remeasureTime) {
                measuredAt = now;
                pageIslands = cells.map((cell) =>
                    shift(cell.getBoundingClientRect(), scroll.x, scroll.y),
                );
            }
            const islands = pageIslands.map((island) => shift(island, -scroll.x, -scroll.y));
            const bounds = mount.canvas.getBoundingClientRect();

            // ease by the time since the last frame, so the goo is as thick at any frame rate
            const elapsed = lastAt === 0 ? 0 : Math.min(0.1, (now - lastAt) / 1000);
            lastAt = now;
            const follow = (lag: number) => 1 - Math.exp(-elapsed / lag);
            const target = pointer.isKnown
                ? { x: pointer.x - bounds.left, y: pointer.y - bounds.top }
                : eased;
            eased.x += (target.x - eased.x) * follow(gooLag);
            eased.y += (target.y - eased.y) * follow(gooLag);
            const outside = pointer.isKnown
                ? outsideAt(pointer.x, pointer.y, islands)
                : Number.POSITIVE_INFINITY;
            const swell = Math.exp(-Math.max(outside, 0) / 60) * 0.8;
            eased.pull += (swell - eased.pull) * follow(swell > eased.pull ? swellTime : sagTime);
            eased.charge += (pageCharge.target - eased.charge) * 0.08;

            // twinkle at the slower pace while nothing moves
            const isBusy =
                Math.abs(eased.pull - swell) > 0.02 ||
                Math.hypot(eased.x - target.x, eased.y - target.y) > 1 ||
                Math.abs(eased.charge - pageCharge.target) > 0.01;
            if (isBusy || now - drawnAt >= twinklePace) {
                drawnAt = now;

                // shade only around the islands, as far as their rim, swell and pull can reach
                const slots = Array.from({ length: islandCapacity }, (_, index) => {
                    const island = islands[index];

                    return island === undefined
                        ? [0, 0, 0, 0]
                        : [
                              island.left + island.width / 2 - bounds.left,
                              island.top + island.height / 2 - bounds.top,
                              island.width / 2 + rimOffset,
                              island.height / 2 + rimOffset,
                          ];
                });
                const clip = [
                    Math.min(...islands.map((island) => island.left)) - bounds.left - clipMargin,
                    Math.min(...islands.map((island) => island.top)) - bounds.top - clipMargin,
                    Math.max(...islands.map((island) => island.right)) - bounds.left + clipMargin,
                    Math.max(...islands.map((island) => island.bottom)) - bounds.top + clipMargin,
                ];
                mount.setUniforms({
                    u_clock: isStill ? 0 : now / 1000,
                    u_wobble: isStill ? 0 : 1.2,
                    u_pointer: [eased.x, eased.y],
                    u_pull: isStill ? 0 : eased.pull,
                    u_charge: isStill ? 0 : eased.charge,
                    u_origin: [bounds.left + scroll.x, bounds.top + scroll.y],
                    u_far: [bounds.left + scroll.x * 0.3, bounds.top + scroll.y * 0.3],
                    u_near: [bounds.left + scroll.x * 0.7, bounds.top + scroll.y * 0.7],
                    u_islands: slots,
                    u_islandCount: islands.length,
                    u_clip: clip,
                });
                setIsFieldPainted(true);
            }
            frameRequest = requestAnimationFrame(draw);
        };
        frameRequest = requestAnimationFrame(draw);

        // release the frames and listeners with the field
        stop = () => {
            // stop the frames, the pointer and the page observers
            cancelAnimationFrame(frameRequest);
            stopPointing();
            stopResizing();
            pageSize.unobserve(document.body);
        };
    };

    return (
        <div aria-hidden="true" {...style.attrs(styles.frame)}>
            <Shader
                fragmentShader={GOO_FRAGMENT}
                uniforms={{ u_page: color.background }}
                xstyle={[styles.canvas, isFieldPainted() && styles.shown]}
                onMount={start}
                onFailure={reportFailure}
            />
        </div>
    );
}

/** Report the goo failing to draw, which leaves the still stars in its cells. */
function reportFailure(failure: ShaderFailure): void {
    captureException(new Error(`the goo shader failed: ${failure}`), { tags: { feature: "goo" } });
}

/** Return how far a client point lies outside the nearest island cell and its rim, in CSS pixels. */
function outsideAt(x: number, y: number, islands: readonly Rect[]) {
    let nearest = Number.POSITIVE_INFINITY;
    for (const island of islands) {
        const across =
            Math.abs(x - (island.left + island.right) / 2) - island.width / 2 - rimOffset;
        const down = Math.abs(y - (island.top + island.bottom) / 2) - island.height / 2 - rimOffset;
        const outside = Math.hypot(Math.max(across, 0), Math.max(down, 0));
        nearest = Math.min(nearest, outside + Math.min(Math.max(across, down), 0));
    }

    return nearest;
}

/** A rectangle's edges and size, in page or client pixels. */
type Rect = {
    left: number;
    top: number;
    right: number;
    bottom: number;
    width: number;
    height: number;
};

/** Return a rectangle moved by an offset. */
function shift(rect: Rect, x: number, y: number): Rect {
    return {
        left: rect.left + x,
        top: rect.top + y,
        right: rect.right + x,
        bottom: rect.bottom + y,
        width: rect.width,
        height: rect.height,
    };
}

/** The goo styles. */
const styles = style.create({
    host: {
        minWidth: 0,
        position: "relative",
    },
    backdrop: {
        backgroundColor: SPACE,
        inset: 0,
        position: "absolute",
        transition: `opacity ${fadeTime}ms ease`,
    },
    hidden: {
        opacity: 0,
    },
    frame: {
        bottom: 0,
        left: `calc(50% - ${frame.width} / 2)`,
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        width: frame.width,
        zIndex: -1,
    },
    canvas: {
        height: `calc(100svh + ${spill * 2}px)`,
        left: `calc(50% - ${frame.width} / 2 - ${spill}px)`,
        opacity: 0,
        position: "fixed",
        top: `-${spill}px`,
        transition: `opacity ${fadeTime}ms ease`,
        width: `calc(${frame.width} + ${spill * 2}px)`,
    },
    shown: {
        opacity: 1,
    },
    content: {
        height: "100%",
        position: "relative",
    },
});
