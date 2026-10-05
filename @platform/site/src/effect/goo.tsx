import * as stylex from "@destack/style";
import { type JSX, onSettled } from "@destack/view";

import { tokens } from "../style/tokens.stylex";
import { ambientPace, isDarkPage, isWeakGraphics, pageScroll, Shader } from "./gl";
import { telemetry } from "@destack/telemetry";
import { log } from "../site/telemetry.ts";

/** The distance the goo field reaches past the site frame, in CSS pixels, so its rim can wobble across the frame rules. */
const spill = 8;
/** How far past its cells the goo can reach, in CSS pixels: its rim, the charge swell and the pointer's pull, with room to spare. */
const clipMargin = 24;
/** The milliseconds the field takes to fade in over the still stars. */
const fadeTime = 500;
/** How far an island's rim lies outside its cell, in CSS pixels. */
const rimOffset = 3;
/** The most goo islands the field draws. */
const islandCapacity = 4;
/** The seconds the goo's bulge takes to catch up with the pointer, so it drags behind like something thick. */
const gooLag = 0.6;
/** The seconds the tail of the bulge takes to catch up with its head, drawing the goo out into a strand while the pointer moves. */
const gooTail = 1.6;
/** The seconds the goo takes to swell toward a pointer that comes near. */
const swellTime = 1;
/** The seconds the goo takes to sag back once the pointer leaves. */
const sagTime = 2.4;
/** The seconds the slow swells from moving the pointer take to build and settle. */
const stirTime = 1.2;
/** The pointer speed that stirs the goo fully, in CSS pixels per second. */
const fullStir = 3000;
/** The seconds a shooting star fired by a click takes to streak across and fade. */
const starLife = 1.2;
/** The milliseconds between measurements of the goo cells, in case the page shifts under them without resizing. */
const remeasureTime = 1000;
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
const fragmentSource = `
precision mediump float;
uniform vec2 resolution;
uniform float scale;
uniform float time;
uniform float wobble;
uniform vec2 pointer;
uniform vec2 trail;
uniform vec2 origin;
uniform float pull;
uniform float stir;
uniform float shower;
uniform vec2 far;
uniform vec2 near;
uniform float charge;
uniform vec3 star;
uniform vec4 islands[${islandCapacity}];
uniform float islandCount;
uniform float outline;

const vec3 space = vec3(0.031, 0.09, 0.137);
const vec3 rimColor = vec3(0.945, 0.918, 0.859);
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
    float alpha = mix(0.35, 0.9, hash(cell + 7.7)) * (0.7 + 0.3 * sin(t * (0.5 + hash(cell + 9.0) * 0.9) + seed * 40.0));

    // tint some stars in soft pastels
    float hue = hash(cell + 3.3);
    vec3 tint = hue < 0.18 ? vec3(0.98, 0.72, 0.84) : (hue < 0.36 ? vec3(0.66, 0.85, 1.0) : (hue < 0.5 ? vec3(1.0, 0.9, 0.62) : (hue < 0.6 ? vec3(0.7, 0.95, 0.84) : rimColor)));
    return mix(rimColor, tint, 0.8) * disc * alpha;
}

// return one shooting star's light at a point, one streak per period on its own track
vec3 streak(vec2 frag, vec2 size, float time, float period, float track) {
    float round = floor(time / period);
    float phase = fract(time / period) / 0.14;
    if (phase >= 1.0) {
        return vec3(0.0);
    }
    vec2 start = vec2(hash(vec2(round, 1.0 + track)), hash(vec2(round, 2.0 + track)) * 0.6) * size;
    vec2 direction = normalize(vec2(1.0, 0.3 + hash(vec2(round, 3.0 + track)) * 0.4));
    vec2 head = start + direction * phase * size.x * 0.7;
    vec2 relative = frag - head;
    float behind = dot(relative, -direction);
    float across = length(relative + direction * behind);
    float tail = step(0.0, behind) * exp(-behind / 45.0) * exp(-across * across / 0.8);
    return vec3(1.0, 0.97, 0.92) * tail * (1.0 - phase) * 0.9;
}

void main() {
    // work in CSS pixels from the top left
    vec2 size = resolution / scale;
    vec2 frag = vec2(gl_FragCoord.x, resolution.y - gl_FragCoord.y) / scale;
    if (islandCount < 0.5) {
        discard;
    }

    // measure the island cells
    float d = 1e5;
    for (int i = 0; i < ${islandCapacity}; i++) {
        if (float(i) < islandCount) {
            vec4 island = islands[i];
            d = min(d, box(frag - island.xy, island.zw + charge * 3.0, 6.0));
        }
    }
    if (d > 140.0) {
        discard;
    }

    // sag and swell the edge very slowly
    vec2 page = frag + origin;
    float slow = noise(page * 0.007 + vec2(time * 0.025, time * 0.018)) - 0.5;
    float swell = noise(page * 0.013 - vec2(time * 0.03, time * 0.012)) - 0.5;
    d -= (slow * 2.0 + swell) * wobble;

    // swell toward the pointer along the strand it drags behind it, thinning toward the tail, and heave in slow broad swells while it moves
    vec2 along = trail - pointer;
    float reach = clamp(dot(frag - pointer, along) / max(dot(along, along), 1.0), 0.0, 1.0);
    float away = length(frag - pointer - along * reach);
    float thickness = 1.0 - reach * 0.55;
    d -= pull * thickness * (2.5 * exp(-away * away / 9000.0) + 5.0 * exp(-away * away / 1400.0));
    d -= stir * 2.5 * sin(away * 0.06 - time * 2.0) * exp(-away / 140.0);

    // clip to the goo with a fine cream rim, outlined in ink on a light page
    float edge = 1.0 / scale;
    float inside = 1.0 - smoothstep(-edge, edge, d);
    float rim = 1.0 - smoothstep(0.4, 1.1, abs(d + 0.6));
    float ink = (1.0 - smoothstep(0.35, 0.9, abs(d - 1.1))) * outline;
    if (inside + rim + ink < 0.001) {
        discard;
    }

    // bend the stars away from the pointer like a lens while it is near
    vec2 toward = frag - pointer;
    vec2 sky = frag + toward / max(length(toward), 1.0) * 22.0 * pull * exp(-dot(toward, toward) / 4200.0);

    // glow in slow page-wide clouds under two depths of sparse flat stars placed across the page
    float falloff = smoothstep(0.25, 0.85, noise(page * 0.0012 + vec2(time * 0.004, 0.0)));
    vec3 nebula = (vec3(0.09, 0.2, 0.27) - space) * falloff * 0.45;
    vec3 distant = starLayer(sky + far + vec2(time * 1.2, time * 0.3), 26.0, ${distantStars}, time);
    vec3 close = starLayer(sky + near + 71.0 + vec2(time * 2.4, time * 0.6), 58.0, ${closeStars}, time * 1.3) * 1.25;

    // shower shooting stars while the pointer is over the goo
    vec3 meteor = vec3(0.0);
    if (shower > 0.01) {
        for (int i = 1; i < 4; i++) {
            float track = float(i);
            meteor += streak(frag, size, time + track * 0.73, 0.9 + track * 0.35, track) * shower;
        }
    }

    // glow faintly inside the rim so the edge reads as a surface
    float sheen = (1.0 - smoothstep(0.0, 14.0, -d)) * 0.05;
    vec3 color = space + nebula * (1.0 + charge) + sheen + (distant * 0.6 + close * 0.8) * (1.0 + charge * 0.6) + meteor;

    // streak a shooting star down and away from where the reader clicked, flashing where it starts
    if (star.z >= 0.0 && star.z < ${starLife.toFixed(1)}) {
        float life = star.z / ${starLife.toFixed(1)};
        vec2 heading = normalize(vec2(-0.82, 0.57));
        vec2 head = star.xy + heading * star.z * 520.0;
        float tail = 120.0 * (1.0 - life);
        vec2 back = frag - head;
        float along = clamp(dot(back, -heading) / max(tail, 1.0), 0.0, 1.0);
        float across = length(back + heading * along * tail);
        float streak = exp(-across * across / 2.2) * (1.0 - along) * (1.0 - life);
        float flash = exp(-dot(frag - star.xy, frag - star.xy) / 40.0) * max(0.0, 1.0 - star.z * 5.0);
        color += vec3(1.0, 0.95, 0.85) * (streak * 1.4 + flash);
    }

    color = mix(color, rimColor, rim);
    color = mix(color, inkColor, ink * (1.0 - max(inside, rim)));

    float alpha = max(max(inside, rim), ink);
    gl_FragColor = vec4(color * alpha, alpha);
}
`;

/** How charged the goo is, from 0 to 1, while the Destack switch is hovered. */
const pageCharge = { target: 0 };

/** Charge the page's goo to a level: 0 at rest, 1 while the switch promises Destack, and more while the switch is held down. */
export function charge(level: number) {
    pageCharge.target = level;
}

/** The pointer anywhere on the page in client pixels. */
const pagePointer = { x: 0, y: 0, isKnown: false };

/** The site's one goo field, which draws every island cell. */
class Field {
    /** The shader that draws the goo. */
    shader: Shader;
    /** The smoothed pointer position in canvas CSS pixels, lagging behind the real one. */
    pointer: { x: number; y: number };
    /** The tail of the strand the goo draws out behind the pointer, lagging further still. */
    trail: { x: number; y: number };
    /** The time of the previous frame in milliseconds, to ease by time rather than by frames. */
    lastAt: number;
    /** The smoothed swell strength from 0 to 1, fading with the pointer's distance outside the goo. */
    pull: number;
    /** The smoothed ripple strength from 0 to 1, rising with the pointer's speed. */
    stir: number;
    /** The pointer's target on the previous frame, to feel how fast it moves. */
    lastTarget: { x: number; y: number };
    /** The smoothed meteor shower strength from 0 to 1, while the pointer is over the goo. */
    shower: number;
    /** Whether the goo moves at all. */
    isMoving: boolean;
    /** Return the island cells of the current frame, in client pixels. */
    islands: () => readonly Rect[];
    /** The smoothed page charge from 0 to 1. */
    charge: number;
    /** Where a click last fired a shooting star, in canvas CSS pixels, and when, in milliseconds. */
    star: { x: number; y: number; at: number };
    /** The fixed canvas's place on screen, measured once and again after the window resizes. */
    bounds: DOMRect | undefined;

    /** Create the field on a canvas, or throw when WebGL is unavailable. */
    constructor(canvas: HTMLCanvasElement, isMoving: boolean, islands: () => readonly Rect[]) {
        // start the shader and rest the goo until the pointer comes
        this.shader = new Shader(canvas, fragmentSource, 1.25, (now) => this.draw(now));
        this.pointer = { x: 0, y: 0 };
        this.trail = { x: 0, y: 0 };
        this.lastAt = 0;
        this.pull = 0;
        this.stir = 0;
        this.lastTarget = { x: 0, y: 0 };
        this.shower = 0;
        this.isMoving = isMoving;
        this.islands = islands;
        this.charge = 0;
        this.star = { x: 0, y: 0, at: -1e9 };
        this.bounds = undefined;
        window.addEventListener("resize", () => {
            this.bounds = undefined;
        });
    }

    /** Upload one frame, and return whether to keep going while the goo moves. */
    draw(now: number) {
        // read the shader, its place, and the islands
        const shader = this.shader;
        const context = shader.context;
        this.bounds ??= shader.canvas.getBoundingClientRect();
        const bounds = this.bounds;
        const cells = this.islands();
        const seconds = now / 1000;

        // ease by the time since the last frame, so the goo is as thick at any frame rate
        const elapsed = this.lastAt === 0 ? 0 : Math.min(0.1, (now - this.lastAt) / 1000);
        this.lastAt = now;
        const follow = (lag: number) => 1 - Math.exp(-elapsed / lag);

        // drag the bulge after the pointer, and the strand's tail after the bulge
        const target = pagePointer.isKnown
            ? { x: pagePointer.x - bounds.left, y: pagePointer.y - bounds.top }
            : this.pointer;
        this.pointer.x += (target.x - this.pointer.x) * follow(gooLag);
        this.pointer.y += (target.y - this.pointer.y) * follow(gooLag);
        this.trail.x += (this.pointer.x - this.trail.x) * follow(gooTail);
        this.trail.y += (this.pointer.y - this.trail.y) * follow(gooTail);

        // swell slowly toward a pointer near or over the goo, sag back more slowly still, and heave with the pointer's speed
        const speed =
            Math.hypot(target.x - this.lastTarget.x, target.y - this.lastTarget.y) /
            Math.max(elapsed, 0.001);
        this.lastTarget = { x: target.x, y: target.y };
        const outside = pagePointer.isKnown
            ? outsideAt(pagePointer.x, pagePointer.y, cells)
            : Number.POSITIVE_INFINITY;
        const swell = outside < 0 ? 1 : Math.exp(-outside / 60) * 0.8;
        this.pull += (swell - this.pull) * follow(swell > this.pull ? swellTime : sagTime);
        this.stir += (Math.min(1, speed / fullStir) * swell - this.stir) * follow(stirTime);
        this.shower += ((outside < 0 ? 1 : 0) - this.shower) * 0.04;
        this.charge += (pageCharge.target - this.charge) * 0.08;

        // place the sky by page position, far stars shifting less than near ones
        const lookX = pagePointer.isKnown ? (pagePointer.x - window.innerWidth / 2) * -0.04 : 0;
        const lookY = pagePointer.isKnown ? (pagePointer.y - window.innerHeight / 2) * -0.04 : 0;

        // upload the motion, the pointer, and the sky
        context.uniform1f(shader.uniform("time"), this.isMoving ? seconds : 0);
        context.uniform2f(
            shader.uniform("origin"),
            bounds.left + pageScroll.x,
            bounds.top + pageScroll.y,
        );
        context.uniform1f(shader.uniform("outline"), isDarkPage() ? 0 : 1);
        context.uniform1f(shader.uniform("charge"), this.isMoving ? this.charge : 0);
        context.uniform3f(
            shader.uniform("star"),
            this.star.x,
            this.star.y,
            this.isMoving ? (now - this.star.at) / 1000 : -1,
        );
        context.uniform1f(shader.uniform("wobble"), this.isMoving ? 2.2 : 0);
        context.uniform2f(shader.uniform("pointer"), this.pointer.x, this.pointer.y);
        context.uniform2f(shader.uniform("trail"), this.trail.x, this.trail.y);
        context.uniform1f(shader.uniform("pull"), this.isMoving ? this.pull : 0);
        context.uniform1f(shader.uniform("stir"), this.isMoving ? this.stir : 0);
        context.uniform1f(shader.uniform("shower"), this.isMoving ? this.shower : 0);
        context.uniform2f(
            shader.uniform("far"),
            bounds.left + pageScroll.x * 0.3 + lookX * 0.4,
            bounds.top + pageScroll.y * 0.3 + lookY * 0.4,
        );
        context.uniform2f(
            shader.uniform("near"),
            bounds.left + pageScroll.x * 0.7 + lookX,
            bounds.top + pageScroll.y * 0.7 + lookY,
        );

        // upload the islands, each with its rim outside its cell
        const islands = new Float32Array(islandCapacity * 4);
        cells.forEach((island, index) =>
            islands.set(
                [
                    island.left + island.width / 2 - bounds.left,
                    island.top + island.height / 2 - bounds.top,
                    island.width / 2 + rimOffset,
                    island.height / 2 + rimOffset,
                ],
                index * 4,
            ),
        );
        context.uniform4fv(shader.uniform("islands"), islands);
        context.uniform1f(shader.uniform("islandCount"), cells.length);

        // shade only around the islands, as far as their rim, swell and pull can reach
        const left = Math.min(...cells.map((cell) => cell.left)) - bounds.left - clipMargin;
        const top = Math.min(...cells.map((cell) => cell.top)) - bounds.top - clipMargin;
        const right = Math.max(...cells.map((cell) => cell.right)) - bounds.left + clipMargin;
        const bottom = Math.max(...cells.map((cell) => cell.bottom)) - bounds.top + clipMargin;
        shader.clip =
            cells.length === 0
                ? { left: 0, top: 0, width: 0, height: 0 }
                : { left, top, width: right - left, height: bottom - top };

        // twinkle at half the frame rate while nothing moves
        const isBusy =
            this.pull > 0.02 ||
            this.stir > 0.02 ||
            now - this.star.at < starLife * 1000 ||
            Math.hypot(this.pointer.x - this.trail.x, this.pointer.y - this.trail.y) > 1 ||
            Math.abs(this.charge - pageCharge.target) > 0.01;
        shader.pace = isBusy ? 0 : ambientPace;

        // announce the first frame to the page, so the cells let the field show through
        if (document.documentElement.dataset["field"] !== "painted") {
            document.documentElement.dataset["field"] = "painted";
        }

        return this.isMoving;
    }
}

/** Mark a lattice cell as an island of goo, drawn by the site's one field behind its content. */
export function Goo(properties: { children?: JSX.Element; style?: stylex.Styles }) {
    return (
        <div data-goo {...stylex.attrs(styles.host, properties.style)}>
            {/* stand in with still stars until the field paints */}
            <span
                aria-hidden="true"
                style={{ "background-image": stillStars }}
                data-goo-backdrop
                {...stylex.attrs(styles.backdrop)}
            />
            <div {...stylex.attrs(styles.content)}>{properties.children}</div>
        </div>
    );
}

/** Draw the site's one goo field behind its content, filling every island cell. */
export function GooField() {
    // hold the frame and the canvas
    let frame: HTMLDivElement | undefined;
    let canvas: HTMLCanvasElement | undefined;

    // run the field, and release it with the page
    onSettled(() => {
        // require the rendered frame and canvas
        if (!frame || !canvas) {
            throw new TypeError("the goo rendered without its frame and canvas");
        }

        // hold the cells and where they sit on the page
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        let cells: HTMLElement[] = [];
        let pageIslands: Rect[] = [];
        let measuredAt = -Infinity;

        // measure again whenever the window or the page itself changes size, as when images load or another page opens
        const remeasure = () => {
            measuredAt = -Infinity;
        };
        window.addEventListener("resize", remeasure);
        const pageSize = new ResizeObserver(remeasure);
        pageSize.observe(document.body);

        // measure the cells in page pixels now and then, and move them with the scroll every frame
        const islands = () => {
            // find the cells again once any of them leaves the page
            const now = performance.now();
            if (cells.length === 0 || cells.some((cell) => !cell.isConnected)) {
                cells = [...document.querySelectorAll<HTMLElement>("[data-goo]")].slice(
                    0,
                    islandCapacity,
                );
                measuredAt = -Infinity;
            }

            // measure them in page pixels once the last measurement is stale
            if (now - measuredAt > remeasureTime) {
                measuredAt = now;
                pageIslands = cells.map((cell) => onPage(cell.getBoundingClientRect()));
            }

            return pageIslands.map(onScreen);
        };

        // follow the pointer across the page
        const point = (event: PointerEvent) => {
            pagePointer.x = event.clientX;
            pagePointer.y = event.clientY;
            pagePointer.isKnown = true;
        };
        window.addEventListener("pointermove", point, { passive: true });

        // start the field, or keep the still stars when WebGL is unavailable
        let field: Field;
        try {
            field = new Field(canvas, !isStill && !isWeakGraphics(), islands);
        } catch (error) {
            log.error("goo.render.failed", telemetry.exceptionAttributes(error));
            return;
        }
        field.shader.request();

        // fire a shooting star from a click on empty goo, away from anything clickable
        const shoot = (event: PointerEvent) => {
            // skip clicks on anything clickable or draggable, and clicks outside the goo
            const target = event.target;
            const isEmpty =
                !(target instanceof Element) ||
                !target.closest("a, button, input, summary, [role=switch], [data-card]");
            if (
                !isEmpty ||
                !field.isMoving ||
                outsideAt(event.clientX, event.clientY, islands()) >= 0
            ) {
                return;
            }

            // start the star where the click landed
            const bounds = field.bounds ?? canvas.getBoundingClientRect();
            field.star = {
                x: event.clientX - bounds.left,
                y: event.clientY - bounds.top,
                at: performance.now(),
            };
            field.shader.request();
        };
        window.addEventListener("pointerdown", shoot, { passive: true });

        return () => {
            // stop following the pointer and the page, and release the shader
            window.removeEventListener("pointermove", point);
            window.removeEventListener("pointerdown", shoot);
            window.removeEventListener("resize", remeasure);
            pageSize.disconnect();
            field.shader.dispose();
        };
    });

    return (
        <div ref={frame} aria-hidden="true" {...stylex.attrs(styles.frame)}>
            <canvas ref={canvas} data-goo-field {...stylex.attrs(styles.canvas)} />
        </div>
    );
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

/** Return a client rectangle in page pixels, so it holds still as the page scrolls. */
function onPage(rect: Rect) {
    return shift(rect, pageScroll.x, pageScroll.y);
}

/** Return a page rectangle in client pixels at the current scroll. */
function onScreen(rect: Rect) {
    return shift(rect, -pageScroll.x, -pageScroll.y);
}

/** The goo styles. */
const styles = stylex.create({
    host: {
        minWidth: 0,
        position: "relative",
    },
    backdrop: {
        backgroundColor: tokens.space,
        inset: 0,
        position: "absolute",
        transition: `opacity ${fadeTime}ms ease`,
    },
    frame: {
        bottom: 0,
        left: `calc(50% - ${tokens.siteWidth} / 2)`,
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        width: tokens.siteWidth,
        zIndex: -1,
    },
    canvas: {
        height: `calc(100svh + ${spill * 2}px)`,
        left: `calc(50% - ${tokens.siteWidth} / 2 - ${spill}px)`,
        opacity: 0,
        position: "fixed",
        top: `-${spill}px`,
        transition: `opacity ${fadeTime}ms ease`,
        width: `calc(${tokens.siteWidth} + ${spill * 2}px)`,
    },
    content: {
        height: "100%",
        position: "relative",
    },
});
