import * as stylex from "@destack/style";
import { type JSX, onSettled } from "@destack/view";

import { tokens } from "../style/tokens.stylex";
import { isDarkPage, pageScroll, Shader } from "./gl";
import { drainAt, pageWater } from "./water";

/** The distance the goo field reaches past the site frame, in CSS pixels, so its rim can wobble across the frame rules. */
const spill = 8;
/** The milliseconds the field takes to fade in over the still stars. */
const fadeTime = 500;
/** How far the site frame's rim lies outside the frame, in CSS pixels. */
const rimOffset = 3;
/** The most goo islands the universe grows out of. */
const islandCapacity = 4;
/** The milliseconds the universe takes to spread across the site or shrink back into its islands. */
const spreadTime = 2600;
/** How much nearer the universe's reach below an island counts, so it pours down faster than it spreads. */
const pourDown = 0.55;
/** How much farther the universe's reach above an island counts, so it climbs slowly. */
const pourUp = 1.4;
/** The scale of the universe's ragged edge, in waves per CSS pixel. */
const raggedScale = 0.0045;
/** How far the universe's ragged edge reaches in and out, in CSS pixels. */
const raggedDepth = 220;
/** How far ahead of the universe's front its ragged edge can reach, in CSS pixels, so no loose blobs form beyond it. */
const raggedReach = 60;
/** How softly the universe's islands merge into one another, in CSS pixels. */
const mergeSoftness = 90;
/** The milliseconds between frames of the goo while nothing moves, which only twinkles its stars. */
const idlePace = 33;
/** The milliseconds between measurements of the goo cells, in case the page shifts under them without resizing. */
const remeasureTime = 1000;
/** How far past the farthest point of the site frame the universe spreads, in CSS pixels, to settle its ragged edge. */
const spreadMargin = 60;
/** How many points across and down the site frame are measured for the universe's farthest reach. */
const reachSamples = 24;
/** The share of far star cells that hold a star. */
const distantStars = "0.14";
/** The share of near star cells that hold a star. */
const closeStars = "0.16";
/** The lattice rule color over space. */
const spaceRule = "rgb(241 234 219 / 16%)";
/** How far inside the universe's edge a section's centre must lie before it is drawn for space, in CSS pixels. */
const coverDepth = 24;

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
uniform vec2 origin;
uniform float pull;
uniform float stir;
uniform float shower;
uniform vec2 far;
uniform vec2 near;
uniform vec4 hole;
uniform float glow;
uniform float flow;
uniform float charge;
uniform vec4 islands[${islandCapacity}];
uniform float islandCount;
uniform float spread;
uniform float fullness;
uniform float waterTop;
uniform float outline;
uniform vec4 frame;

const vec3 space = vec3(0.031, 0.09, 0.137);
const vec3 rimColor = vec3(0.945, 0.918, 0.859);
const vec3 inkColor = vec3(0.071, 0.192, 0.235);
const vec3 signal = vec3(1.0, 0.475, 0.18);

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

// return a few crossing waves for a ragged, organic edge that the page can trace exactly
float ragged(vec2 p) {
    return 0.5
        + 0.22 * sin(p.x * 1.7 + p.y * 0.9)
        + 0.14 * sin(p.x * -1.1 + p.y * 2.3 + 1.3)
        + 0.09 * sin(p.x * 3.1 - p.y * 1.9 + 2.1)
        + 0.05 * sin(p.x * 5.3 + p.y * 4.7 + 0.7);
}

// return the distance to a rounded box, squashed below it and stretched above it outside, so heavy goo pours down faster than it climbs
float pour(vec2 p, vec2 extent) {
    vec2 q = abs(p) - extent + 6.0;
    q.y *= q.y > 0.0 ? (p.y > 0.0 ? ${pourDown} : ${pourUp}) : 1.0;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - 6.0;
}

// blend two distances into one gooey union
float merge(float first, float second, float softness) {
    float blend = max(softness - abs(first - second), 0.0) / softness;
    return min(first, second) - blend * blend * softness * 0.25;
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

    // measure the island cells, and the universe growing out of them while it spreads
    float cells = 1e5;
    float grown = 1e5;
    for (int i = 0; i < ${islandCapacity}; i++) {
        if (float(i) < islandCount) {
            vec4 island = islands[i];
            vec2 extent = island.zw + charge * 3.0;
            cells = min(cells, box(frag - island.xy, extent, 6.0));
            grown = merge(grown, pour(frag - island.xy, extent) - spread, ${mergeSoftness.toFixed(1)});
        }
    }

    // push the universe's front out in slow lobes that settle into the frame as it fills, and keep it above the water
    float front = smoothstep(0.0, 80.0, spread) * (1.0 - smoothstep(0.7, 1.0, fullness));
    vec2 page = frag + origin;
    float near = exp(-max(grown, 0.0) / ${raggedReach.toFixed(1)});
    grown += (ragged(page * ${raggedScale} + vec2(time * 0.06, -time * 0.04)) - 0.5) * ${raggedDepth.toFixed(1)} * front * near;
    grown = max(grown, frag.y - waterTop - 4.0);
    float d = max(box(frag - frame.xy, frame.zw, 6.0), min(cells, grown));
    if (d > 140.0) {
        discard;
    }

    // sag and swell the edge very slowly, and heave it while it spreads
    float spreading = front * (1.0 - smoothstep(0.0, 160.0, -grown));
    float slow = noise(page * 0.007 + vec2(time * 0.025, time * 0.018)) - 0.5;
    float swell = noise(page * 0.013 - vec2(time * 0.03, time * 0.012)) - 0.5;
    d -= (slow * 2.0 + swell) * wobble * (1.0 + spreading * 6.0);

    // swell toward the pointer, bulging hard right under it, and ring the edge with ripples while the pointer moves
    float away = length(frag - pointer);
    d -= pull * (8.0 * exp(-away * away / 9000.0) + 16.0 * exp(-away * away / 1400.0));
    d -= stir * 4.0 * sin(away * 0.22 - time * 7.0) * exp(-away / 70.0);

    // clip to the goo with a fine cream rim, outlined in ink on a light page
    float edge = 1.0 / scale;
    float inside = 1.0 - smoothstep(-edge, edge, d);
    float rim = 1.0 - smoothstep(0.4, 1.1, abs(d + 0.6));
    float ink = (1.0 - smoothstep(0.35, 0.9, abs(d - 1.1))) * outline;
    if (inside + rim + ink < 0.001) {
        discard;
    }

    // bend the starlight around the black hole, if there is one
    vec2 around = frag - hole.xy;
    float distance = length(around);
    vec2 sky = frag;
    if (hole.z > 0.0) {
        sky += around / max(distance, 1.0) * hole.z * hole.z * 1.6 / max(distance, hole.z);
    }

    // bend the stars away from the pointer like a lens while it is near
    vec2 toward = frag - pointer;
    sky += toward / max(length(toward), 1.0) * 22.0 * pull * exp(-dot(toward, toward) / 4200.0);

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

    // swallow the light behind a black hole ringed by a swirling accretion disk seen nearly edge on,
    // faint at rest and blazing while lit
    if (hole.z > 0.0) {
        vec2 tilted = vec2(around.x, around.y * 3.2);
        float spread = length(tilted);
        float band = smoothstep(hole.z * 1.2, hole.z * 1.7, spread) * (1.0 - smoothstep(hole.z * 2.4, hole.z * 5.5, spread));
        float swirl = 0.55 + 0.45 * sin(atan(tilted.y, tilted.x) * 3.0 - time * 2.4 + spread * 0.18);
        vec3 disk = mix(vec3(1.0), signal, 0.12 + glow * 0.18) * band * swirl * mix(0.07, 1.1, glow);
        float ring = exp(-abs(distance - hole.z * 1.08) / 0.8) * mix(0.14, 1.0, glow);
        float horizon = 1.0 - smoothstep(hole.z - 0.8, hole.z + 0.2, distance);

        // pour a stream of water from the cell above into the hole while it blazes, down to drain and up to fill
        float width = hole.z * 0.45 * glow;
        float stream = (1.0 - smoothstep(width - 0.6, width + 0.6, abs(around.x))) * step(around.y, 0.0) * step(-hole.w, around.y);
        float ripple = 0.6 + 0.4 * sin(frag.y * 0.9 - flow * time * 18.0);
        color = mix(color, mix(vec3(0.55, 0.82, 0.9), vec3(1.0), ripple * 0.5), stream * glow);
        float front = step(0.0, around.y);
        color += disk * (1.0 - front);
        color = mix(color, vec3(0.0), horizon);
        color += disk * front + vec3(1.0) * ring * (1.0 - horizon);
    }

    color = mix(color, rimColor, rim);
    color = mix(color, inkColor, ink * (1.0 - max(inside, rim)));

    float alpha = max(max(inside, rim), ink);
    gl_FragColor = vec4(color * alpha, alpha);
}
`;

/** How charged the goo is, from 0 to 1, while the Destack switch is hovered. */
const pageCharge = { target: 0 };

/** Charge the goo so it swells and brightens, or let it settle again. */
export function charge(isCharged: boolean) {
    pageCharge.target = isCharged ? 1 : 0;
}

/** The pointer anywhere on the page in client pixels. */
const pagePointer = { x: 0, y: 0, isKnown: false };

/** The island cells, the black hole, and how far the universe has spread, in client pixels. */
type Terrain = {
    /** The island cells the goo fills. */
    islands: readonly Rect[];
    /** How far the universe has spread past the islands. */
    spread: number;
    /** How far the spread has come toward filling the frame, from 0 to 1. */
    fullness: number;
    /** The site frame the universe fills. */
    frame: Rect;
    /** The cell holding the black hole and the hole's radius, if the page has one. */
    hole: { cell: Rect; radius: number } | undefined;
};

/** The site's one goo field: every island cell, the black hole, and the universe that grows out of them. */
class Field {
    /** The shader that draws the goo. */
    shader: Shader;
    /** The smoothed pointer position in canvas CSS pixels. */
    pointer: { x: number; y: number };
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
    /** Return the terrain of the current frame. */
    terrain: () => Terrain;
    /** Which way water flows through the black hole: 1 draining into it, -1 welling out of it, 0 still. */
    flow: () => number;
    /** The smoothed glow of the disk from 0 to 1. */
    glow: number;
    /** The flow on the previous frame. */
    lastFlow: number;
    /** The flare as the last of the water goes in, from 1 fading to 0. */
    flare: number;
    /** The smoothed page charge from 0 to 1. */
    charge: number;
    /** The fixed canvas's place on screen, measured once and again after the window resizes. */
    bounds: DOMRect | undefined;

    /** Create the field on a canvas, or throw when WebGL is unavailable. */
    constructor(
        canvas: HTMLCanvasElement,
        isMoving: boolean,
        terrain: () => Terrain,
        flow: () => number,
    ) {
        // start the shader and rest the goo until the pointer comes
        this.shader = new Shader(canvas, fragmentSource, 1.25, (now) => this.draw(now));
        this.pointer = { x: 0, y: 0 };
        this.pull = 0;
        this.stir = 0;
        this.lastTarget = { x: 0, y: 0 };
        this.shower = 0;
        this.isMoving = isMoving;
        this.terrain = terrain;
        this.flow = flow;
        this.glow = 0;
        this.lastFlow = 0;
        this.flare = 0;
        this.charge = 0;
        this.bounds = undefined;
        window.addEventListener("resize", () => {
            this.bounds = undefined;
        });
    }

    /** Upload one frame, and return whether to keep going while the goo moves. */
    draw(now: number) {
        // read the shader, its place, and the terrain
        const shader = this.shader;
        const context = shader.context;
        this.bounds ??= shader.canvas.getBoundingClientRect();
        const bounds = this.bounds;
        const terrain = this.terrain();
        const seconds = now / 1000;

        // follow the pointer, lagging heavily behind it, swell toward it near or over the goo, and shower meteors over the cells
        const target = pagePointer.isKnown
            ? { x: pagePointer.x - bounds.left, y: pagePointer.y - bounds.top }
            : this.pointer;
        this.pointer.x += (target.x - this.pointer.x) * 0.12;
        this.pointer.y += (target.y - this.pointer.y) * 0.12;

        // ripple harder the faster the pointer moves near the goo
        const speed = Math.hypot(target.x - this.lastTarget.x, target.y - this.lastTarget.y);
        this.lastTarget = { x: target.x, y: target.y };
        const outside = pagePointer.isKnown
            ? edgeAt(pagePointer.x, pagePointer.y, terrain, seconds)
            : Number.POSITIVE_INFINITY;
        const swell = outside < 0 ? 1 : Math.exp(-outside / 60) * 0.8;
        this.pull += (swell - this.pull) * 0.08;
        this.stir += (Math.min(1, speed / 24) * swell - this.stir) * 0.08;
        const isOverCell =
            pagePointer.isKnown && pouredAt(pagePointer.x, pagePointer.y, terrain.islands, 0) < 0;
        this.shower += ((isOverCell ? 1 : 0) - this.shower) * 0.04;
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
        context.uniform1f(shader.uniform("wobble"), this.isMoving ? 2.2 : 0);
        context.uniform2f(shader.uniform("pointer"), this.pointer.x, this.pointer.y);
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

        // upload the islands, each with its rim outside its cell, and the universe grown out of them
        const islands = new Float32Array(islandCapacity * 4);
        terrain.islands.forEach((island, index) =>
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
        context.uniform1f(shader.uniform("islandCount"), terrain.islands.length);
        context.uniform1f(shader.uniform("spread"), terrain.spread);
        context.uniform1f(shader.uniform("fullness"), terrain.fullness);
        context.uniform1f(
            shader.uniform("waterTop"),
            Math.min(pageWater.top - pageScroll.y - bounds.top, 1e4),
        );
        context.uniform4f(
            shader.uniform("frame"),
            terrain.frame.left + terrain.frame.width / 2 - bounds.left,
            terrain.frame.top + terrain.frame.height / 2 - bounds.top,
            terrain.frame.width / 2 + rimOffset,
            terrain.frame.height / 2 + rimOffset,
        );

        // light the black hole's disk slowly, and flare once as the last of the water goes in
        const flow = this.flow();
        this.glow += (Math.abs(flow) - this.glow) * 0.04;
        if (this.lastFlow === 1 && flow === 0) {
            this.flare = 1;
        }
        this.lastFlow = flow;
        this.flare *= 0.94;
        const hole = terrain.hole;
        context.uniform4f(
            shader.uniform("hole"),
            hole ? hole.cell.left + hole.cell.width * drainAt - bounds.left : 0,
            hole ? hole.cell.top + hole.cell.height / 2 - bounds.top : 0,
            hole?.radius ?? 0,
            hole ? hole.cell.height / 2 + rimOffset : 0,
        );
        context.uniform1f(shader.uniform("glow"), Math.min(1.8, this.glow + this.flare * 1.4));
        context.uniform1f(shader.uniform("flow"), flow === 0 ? 1 : flow);

        // twinkle at half the frame rate while nothing moves: no pointer near, no spread, no black hole at work
        const isBusy =
            this.pull > 0.02 ||
            this.stir > 0.02 ||
            this.glow > 0.02 ||
            this.flare > 0.02 ||
            Math.abs(this.charge - pageCharge.target) > 0.01 ||
            (terrain.fullness > 0 && terrain.fullness < 1);
        shader.pace = isBusy ? 0 : idlePace;

        // announce the first frame to the page, so the cells let the field show through
        if (document.documentElement.dataset.field !== "painted") {
            document.documentElement.dataset.field = "painted";
        }

        return this.isMoving;
    }
}

/** Mark a lattice cell as an island of goo, drawn by the site's one field behind its content. */
export function Goo(properties: { children?: JSX.Element; hole?: number; style?: stylex.Styles }) {
    return (
        <div
            data-goo
            data-hole={properties.hole === undefined ? undefined : String(properties.hole)}
            {...stylex.attrs(styles.host, properties.style)}
        >
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

/**
 * Draw the site's one goo field behind its content.
 *
 * The field holds every island cell and the black hole, and while open a universe that grows out of the islands until it fills the site frame.
 * As the universe passes each section marked `data-universe`, it draws that section for space.
 */
export function Universe(properties: { isOpen: boolean; flow: number }) {
    // hold the frame and the canvas
    let frame!: HTMLDivElement;
    let canvas!: HTMLCanvasElement;

    // run the field, and release it with the page
    onSettled(() => {
        // hold the spread's motion, the frame's farthest reach, and the sections' schemes
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        const motion = { from: 0, to: 0, at: -spreadTime };
        const schemes = new Map<HTMLElement, boolean>();
        let wasOpen = false;
        let reach = 1;
        let sections: HTMLElement[] = [];
        let cells: HTMLElement[] = [];
        let pageIslands: Rect[] = [];
        let pageFrame: Rect = { left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0 };
        let pageSections: Rect[] = [];
        let measuredAt = -Infinity;

        // measure again whenever the window or the page itself changes size, as when images load or another page opens
        const remeasure = () => {
            measuredAt = -Infinity;
        };
        window.addEventListener("resize", remeasure);
        const pageSize = new ResizeObserver(remeasure);
        pageSize.observe(document.body);
        let ruleBlend = 0;

        // measure the islands and the frame, and follow the switch: spread just far enough to fill the frame, or back
        const terrain = (): Terrain => {
            // measure the cells and the frame
            const now = performance.now();
            if (cells.length === 0 || cells.some((cell) => !cell.isConnected)) {
                cells = [...document.querySelectorAll<HTMLElement>("[data-goo]")].slice(
                    0,
                    islandCapacity,
                );
                measuredAt = -Infinity;
            }
            // measure the cells and the frame in page pixels now and then, and move them with the scroll every frame
            if (now - measuredAt > remeasureTime) {
                measuredAt = now;
                pageIslands = cells.map((cell) => onPage(cell.getBoundingClientRect()));
                pageFrame = onPage(frame.getBoundingClientRect());
            }
            const islands = pageIslands.map(onScreen);
            const bounds = onScreen(pageFrame);
            const holderIndex = cells.findIndex((cell) => cell.dataset.hole !== undefined);
            const holder = cells[holderIndex];

            // start a new spread whenever the switch flips
            const progress = isStill ? 1 : Math.min(1, (now - motion.at) / spreadTime);
            const spread = motion.from + (motion.to - motion.from) * ease(progress);
            if (properties.isOpen !== wasOpen) {
                wasOpen = properties.isOpen;
                reach = farthest(bounds, islands) + spreadMargin;
                sections = [
                    ...document.querySelectorAll<HTMLElement>(
                        '[data-universe]:not([data-universe="parts"]), [data-universe="parts"] > :not([data-universe="parts"])',
                    ),
                ];
                pageSections = sections.map((section) => onPage(section.getBoundingClientRect()));
                Object.assign(motion, { from: spread, to: properties.isOpen ? reach : 0, at: now });
            }
            // blend the lattice rules toward faint cream as the universe fills the site, touching the page only on change
            const fullness = Math.min(1, spread / reach);
            const blend = Math.round(fullness * 40) / 40;
            if (blend !== ruleBlend) {
                ruleBlend = blend;
                frame.parentElement?.style.setProperty(
                    "--site-rule",
                    blend > 0
                        ? `color-mix(in srgb, ${spaceRule} ${blend * 100}%, var(--destack-color-border))`
                        : "",
                );
            }
            const result: Terrain = {
                islands,
                spread,
                fullness,
                frame: bounds,
                hole: holder
                    ? { cell: islands[holderIndex], radius: Number(holder.dataset.hole) }
                    : undefined,
            };

            // draw each section for space once the universe's edge has passed its centre, touching only changed ones
            const boxes = pageSections.map(onScreen);
            sections.forEach((section, index) => {
                // measure the section's centre against the universe's edge
                const box = boxes[index];
                const x = (box.left + box.right) / 2;
                const y = (box.top + box.bottom) / 2;
                const isCovered = spread > 0 && grownAt(x, y, result, now / 1000) < -coverDepth;
                if (schemes.get(section) !== isCovered) {
                    schemes.set(section, isCovered);
                    section.style.colorScheme = isCovered ? "dark" : "";
                }
            });

            return result;
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
            field = new Field(canvas, !isStill, terrain, () => properties.flow);
        } catch (error) {
            console.error("goo rendering failed", error);
            return;
        }
        field.shader.request();

        return () => {
            // stop following the pointer and the page, and release the shader
            window.removeEventListener("pointermove", point);
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

/** Return how far a client point lies outside the islands poured out by a distance and merged softly, as the shader measures it. */
function pouredAt(x: number, y: number, islands: readonly Rect[], spread: number) {
    let merged = 1e5;
    for (const island of islands) {
        // measure the rounded box, squashed below it and stretched above it outside
        const offset = y - (island.top + island.bottom) / 2;
        const across =
            Math.abs(x - (island.left + island.right) / 2) - island.width / 2 - rimOffset + 6;
        let down = Math.abs(offset) - island.height / 2 - rimOffset + 6;
        down *= down > 0 ? (offset > 0 ? pourDown : pourUp) : 1;
        const outside = Math.hypot(Math.max(across, 0), Math.max(down, 0));
        const inside = Math.min(Math.max(across, down), 0);
        const distance = outside + inside - 6 - spread;

        // blend it into the islands so far
        const blend = Math.max(mergeSoftness - Math.abs(merged - distance), 0) / mergeSoftness;
        merged = Math.min(merged, distance) - blend * blend * mergeSoftness * 0.25;
    }

    return merged;
}

/** Return how far a client point lies outside the grown universe, exactly as the shader draws it, in CSS pixels. */
function grownAt(x: number, y: number, terrain: Terrain, seconds: number) {
    // push the front out in the same crossing waves the shader uses, and keep it above the water
    const front = smoothstep(0, 80, terrain.spread) * (1 - smoothstep(0.7, 1, terrain.fullness));
    const across = (x + pageScroll.x) * raggedScale + seconds * 0.06;
    const down = (y + pageScroll.y) * raggedScale - seconds * 0.04;
    const ragged =
        0.5 +
        0.22 * Math.sin(across * 1.7 + down * 0.9) +
        0.14 * Math.sin(across * -1.1 + down * 2.3 + 1.3) +
        0.09 * Math.sin(across * 3.1 - down * 1.9 + 2.1) +
        0.05 * Math.sin(across * 5.3 + down * 4.7 + 0.7);
    const poured = pouredAt(x, y, terrain.islands, terrain.spread);
    const near = Math.exp(-Math.max(poured, 0) / raggedReach);
    const grown = poured + (ragged - 0.5) * raggedDepth * front * near;

    return Math.max(grown, y - (pageWater.top - pageScroll.y) - 4);
}

/** Return how far a client point lies outside the goo, cells or grown universe, in CSS pixels. */
function edgeAt(x: number, y: number, terrain: Terrain, seconds: number) {
    return Math.min(pouredAt(x, y, terrain.islands, 0), grownAt(x, y, terrain, seconds));
}

/** Return how far the universe must spread to reach the farthest point of a frame from its islands, in CSS pixels. */
function farthest(frame: Rect, islands: readonly Rect[]) {
    let most = 0;
    for (let row = 0; row <= reachSamples; row++) {
        for (let column = 0; column <= reachSamples; column++) {
            const x = frame.left + (frame.width * column) / reachSamples;
            const y = frame.top + (frame.height * row) / reachSamples;
            most = Math.max(most, pouredAt(x, y, islands, 0));
        }
    }

    return most;
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

/** Return a smooth step from 0 to 1 between two edges, as the shader's smoothstep does. */
function smoothstep(from: number, to: number, value: number) {
    const progress = Math.max(0, Math.min(1, (value - from) / (to - from)));

    return progress * progress * (3 - 2 * progress);
}

/** Ease in and out, slow at both ends. */
function ease(progress: number) {
    return progress < 0.5 ? 4 * progress ** 3 : 1 - (-2 * progress + 2) ** 3 / 2;
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
