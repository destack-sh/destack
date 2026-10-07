import { ambientPace, Shader } from "./gl";

/** The water colors for one theme, as RGB triples in 0..1. */
export type WaterPalette = {
    /** The color at the bottom of the figure. */
    deep: [number, number, number];
    /** The color just below the waterline. */
    shallow: [number, number, number];
    /** The caustic and light ray color. */
    caustic: [number, number, number];
    /** The waterline color. */
    foam: [number, number, number];
    /** The outline ink. */
    ink: [number, number, number];
};

/** The water palette on the night page. */
export const nightWater: WaterPalette = {
    deep: [0.03, 0.12, 0.16],
    shallow: [0.07, 0.28, 0.34],
    caustic: [0.45, 0.78, 0.84],
    foam: [0.945, 0.918, 0.859],
    ink: [0.01, 0.03, 0.04],
};

/** The water palette on the paper page. */
export const paperWater: WaterPalette = {
    deep: [0.1, 0.3, 0.38],
    shallow: [0.24, 0.56, 0.64],
    caustic: [0.8, 0.93, 0.95],
    foam: [1, 1, 1],
    ink: [0.07, 0.19, 0.235],
};

/** The icebergs under the water: the centre of each across the canvas, the half width of the card it carries, and the depths its top and bottom sit at, in CSS pixels. */
export type Bergs = { centres: readonly number[]; half: number; top: number; bottom: number };

/** The most icebergs the water holds. */
const bergCapacity = 3;

/** How far the water canvas reaches past the figure on the sides and bottom, in CSS pixels, where the figure clips its rim. */
export const waterSpill = 8;

/** The milliseconds a full drain or fill takes. */
export const travel = 700;

/** The most ripples the waterline carries at once. */
const rippleCapacity = 8;
/** How fast ripples run outward along the waterline, in CSS pixels per second. */
const rippleSpeed = 90;
/** The seconds a ripple takes to die away. */
const rippleLife = 3;

/** Recent stirs of the waterline: where across the water canvas, when in seconds, and how hard. */
const ripples: { x: number; at: number; strength: number }[] = [];

/** Stir the waterline at a position across the water canvas, sending ripples outward. */
export function stir(x: number, strength: number) {
    ripples.push({ x, at: performance.now() / 1000, strength });
    if (ripples.length > rippleCapacity) {
        ripples.shift();
    }
}

/** The water fragment shader. */
const fragmentSource = `
precision mediump float;
uniform vec2 resolution;
uniform float scale;
uniform float time;
uniform float level;
uniform vec3 deep;
uniform vec3 shallow;
uniform vec3 caustic;
uniform vec3 foam;
uniform vec3 ink;
uniform float rest;
uniform vec3 bergs[${bergCapacity}];
uniform vec2 span;

vec2 hash2(vec2 p) {
    p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
    return fract(sin(p) * 43758.5453);
}

// return the signed distance to the basin: straight walls and a floor with rounded corners, open far above
float basin(vec2 p, vec2 size) {
    vec2 centre = vec2(size.x * 0.5, (size.y - rest - 4000.0) * 0.5);
    vec2 extent = vec2(size.x * 0.5 - rest, (size.y - rest + 4000.0) * 0.5);
    vec2 q = abs(p - centre) - extent + 6.0;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - 6.0;
}

// return the gap between the nearest two moving cell seeds, small along cell borders
float cells(vec2 p, float t) {
    vec2 cell = floor(p);
    vec2 local = fract(p);
    float nearest = 8.0;
    float second = 8.0;
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            vec2 offset = vec2(float(x), float(y));
            vec2 seed = hash2(cell + offset);
            vec2 point = offset + 0.5 + 0.4 * sin(t + 6.2831 * seed);
            float distance = length(point - local);
            if (distance < nearest) {
                second = nearest;
                nearest = distance;
            } else if (distance < second) {
                second = distance;
            }
        }
    }
    return second - nearest;
}

// return a berg side's half width at a share of its depth, through five facet corners from the waterline down
float side(float t, vec4 upper, float lowest) {
    float step = clamp(t, 0.0, 1.0) * 4.0;
    float a = step < 1.0 ? upper.x : step < 2.0 ? upper.y : step < 3.0 ? upper.z : upper.w;
    float b = step < 1.0 ? upper.y : step < 2.0 ? upper.z : step < 3.0 ? upper.w : lowest;
    return mix(a, b, fract(min(step, 3.999)));
}

void main() {
    // work in CSS pixels from the top left
    vec2 size = resolution / scale;
    float x = gl_FragCoord.x / scale;
    float y = size.y - gl_FragCoord.y / scale;

    // hold the water flat in a basin with rounded corners
    float below = y - level;
    float wall = basin(vec2(x, y), size);
    float edge = 1.0 / scale;
    float held = 1.0 - smoothstep(-edge, edge, wall);
    float rim = 1.0 - smoothstep(0.4, 1.1, abs(wall + 0.6));
    float line = 1.0 - smoothstep(0.35, 0.9, abs(wall - 1.1));
    float cover = smoothstep(-0.6, 0.6, below) * max(max(held, rim), line);
    if (cover <= 0.0) {
        discard;
    }

    // deepen gradually toward the bottom
    float depth = clamp(below / max(size.y - level, 60.0), 0.0, 0.999);
    vec3 color = mix(shallow, deep, smoothstep(0.0, 1.0, depth) * 0.94);

    // lighten each iceberg under the water, lit from the left and edged with a thin line
    float t = (y - span.x) / max(span.y - span.x, 1.0);
    for (int i = 0; i < ${bergCapacity}; i++) {
        vec3 berg = bergs[i];
        if (berg.y <= 0.0 || t < 0.0 || t > 1.0) {
            continue;
        }
        float seed = float(i) * 1.7;
        float across = x - berg.x;
        vec4 corners = across < 0.0
            ? vec4(0.92, 1.08 + 0.04 * sin(seed), 0.98, 0.82 + 0.05 * cos(seed))
            : vec4(0.9, 1.04 + 0.04 * cos(seed), 1.02 + 0.04 * sin(seed), 0.76);
        float reach = berg.y * side(t, corners, 0.5 + 0.08 * sin(seed * 2.0));
        float inside = reach - abs(across);
        if (inside > 0.0) {
            float lit = across < 0.0 ? 0.16 : 0.09;
            color = mix(color, caustic, lit * (1.0 - t * 0.6));
            color = mix(color, caustic, (1.0 - smoothstep(0.6, 1.6, inside)) * 0.35);
        }
    }

    // net the light into faint caustic lines that drift slowly and thin out with depth
    float border = cells(vec2(x, y) / 80.0, time * 0.25);
    float threshold = 0.03 - depth * 0.015;
    color = mix(color, caustic, (1.0 - smoothstep(threshold - 0.01, threshold, border)) * 0.05 * (1.0 - depth));

    // edge the surface with one thin foam line, and rim the basin in foam
    float froth = 1.0 - smoothstep(1.2, 2.0, below);
    color = mix(color, foam, froth * 0.9);
    color = mix(color, foam, rim);
    color = mix(color, ink, line * (1.0 - max(held, rim)));

    // keep the water nearly opaque so the submerged stack reads only as shapes
    float alpha = mix(0.78, 0.9, smoothstep(0.0, 0.8, depth));
    alpha = max(max(alpha * held, froth), max(rim, line)) * cover;
    gl_FragColor = vec4(color * alpha, alpha);
}
`;

/** Render calm water with icebergs under a flat waterline that drains and fills over time. */
export class Water {
    /** The shader that draws the water. */
    shader: Shader;
    /** The current palette. */
    palette: WaterPalette;
    /** The water level at the start of the current move, in CSS pixels. */
    from: number;
    /** The water level the current move ends at. */
    to: number;
    /** The start time of the current move in milliseconds. */
    moved: number;
    /** The animation start time in milliseconds. */
    start: number;
    /** Whether the water moves. */
    isMoving: boolean;
    /** Receive the water level after every frame. */
    onLevel: (level: number) => void;
    /** The icebergs under the water, if any. */
    bergs: Bergs | undefined;

    /** Create water on a canvas, or throw when WebGL is unavailable. */
    constructor(
        canvas: HTMLCanvasElement,
        palette: WaterPalette,
        level: number,
        isMoving: boolean,
        onLevel: (level: number) => void,
    ) {
        // start the shader and rest the water at its level
        this.shader = new Shader(canvas, fragmentSource, 1, (now) => this.draw(now));
        this.palette = palette;
        this.from = level;
        this.to = level;
        this.moved = -travel;
        this.start = performance.now();
        this.isMoving = isMoving;
        this.onLevel = onLevel;
        this.bergs = undefined;
    }

    /** Set the icebergs under the water and redraw. */
    shape(bergs: Bergs) {
        this.bergs = bergs;
        this.shader.request();
    }

    /** Drain or fill toward a new water level. */
    moveTo(level: number) {
        // start a move from the current level
        const now = performance.now();
        this.from = this.levelAt(now);
        this.to = level;
        this.moved = now;
        this.shader.request();
    }

    /** Place the waterline immediately, without animating. */
    place(level: number) {
        // rest the water at the level
        this.from = level;
        this.to = level;
        this.moved = -travel;
        this.shader.request();
    }

    /** Swap the palette and redraw. */
    paint(palette: WaterPalette) {
        this.palette = palette;
        this.shader.request();
    }

    /** Return the water level at a time. */
    levelAt(now: number) {
        return this.from + (this.to - this.from) * ease(this.progressAt(now));
    }

    /** Return how far the current move has come, from 0 to 1. */
    progressAt(now: number) {
        return this.isMoving ? Math.min(1, (now - this.moved) / travel) : 1;
    }

    /** Upload one frame, and return whether to keep going while any water shows or the waterline moves. */
    draw(now: number) {
        // read the shader and its context
        const shader = this.shader;
        const context = shader.context;

        // read the move
        const progress = this.progressAt(now);
        const level = this.levelAt(now);

        // upload the frame parameters
        context.uniform1f(shader.uniform("time"), this.isMoving ? now / 1000 : 0);
        context.uniform1f(shader.uniform("level"), level);
        context.uniform1f(shader.uniform("rest"), waterSpill - 3);
        context.uniform3fv(shader.uniform("deep"), this.palette.deep);
        context.uniform3fv(shader.uniform("shallow"), this.palette.shallow);
        context.uniform3fv(shader.uniform("caustic"), this.palette.caustic);
        context.uniform3fv(shader.uniform("foam"), this.palette.foam);
        context.uniform3fv(shader.uniform("ink"), this.palette.ink);
        const bergs = new Float32Array(bergCapacity * 3);
        this.bergs?.centres
            .slice(0, bergCapacity)
            .forEach((centre, index) => bergs.set([centre, this.bergs?.half ?? 0, 0], index * 3));
        context.uniform3fv(shader.uniform("bergs"), bergs);
        context.uniform2f(shader.uniform("span"), this.bergs?.top ?? 0, this.bergs?.bottom ?? 0);
        this.onLevel(level);

        // draw every frame while the waterline moves, else at the ambient pace
        shader.pace = progress < 1 ? 0 : ambientPace;

        // keep animating while water shows or the waterline still moves
        const isDrained = level >= shader.height && progress >= 1;

        return (this.isMoving && !isDrained) || progress < 1;
    }
}

/** Return how far a wavy waterline rises or falls at a position across the canvas, in CSS pixels. */
export function waveAt(x: number, seconds: number) {
    // sum three rolling waves
    const long = Math.sin(x * 0.017 + seconds * 0.7) * 1.8;
    const middle = Math.sin(x * 0.043 - seconds * 1.0) * 0.8;
    const short = Math.sin(x * 0.11 + seconds * 1.4) * 0.3;

    // add the ripples running along the waterline, exactly as the shader does
    let lift = 0;
    for (const ripple of ripples) {
        const age = seconds - ripple.at;
        if (age < 0 || age > rippleLife) {
            continue;
        }
        const gap = Math.abs(x - ripple.x) - age * rippleSpeed;
        lift +=
            ripple.strength *
            Math.exp(-age * 1.4) *
            Math.exp(-(gap * gap) / 484) *
            Math.sin(gap * 0.25);
    }

    return long + middle + short + lift;
}

/** Ease out, fast at the start and settling at the end. */
function ease(progress: number) {
    return 1 - (1 - progress) ** 3;
}
