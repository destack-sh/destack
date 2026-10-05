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

/** How far the water canvas reaches past the figure on the sides and bottom, in CSS pixels, where the figure clips its rim. */
export const waterSpill = 8;

/** The milliseconds a full drain or fill takes. */
export const travel = 2400;

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
uniform float agitation;
uniform vec3 deep;
uniform vec3 shallow;
uniform vec3 caustic;
uniform vec3 foam;
uniform vec3 ink;
uniform vec4 ripples[${rippleCapacity}];
uniform float rest;

const vec3 signal = vec3(1.0, 0.475, 0.18);

vec2 hash2(vec2 p) {
    p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
    return fract(sin(p) * 43758.5453);
}

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
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

// return how far the ripples running along the surface lift it at a position
float rippleAt(float x) {
    float lift = 0.0;
    for (int i = 0; i < ${rippleCapacity}; i++) {
        vec4 ripple = ripples[i];
        float age = time - ripple.y;
        if (age < 0.0 || age > ${rippleLife.toFixed(1)}) {
            continue;
        }
        float gap = abs(x - ripple.x) - age * ${rippleSpeed.toFixed(1)};
        lift += ripple.z * exp(-age * 1.4) * exp(-gap * gap / 484.0) * sin(gap * 0.25);
    }
    return lift;
}

void main() {
    // work in CSS pixels from the top left
    vec2 size = resolution / scale;
    float x = gl_FragCoord.x / scale;
    float y = size.y - gl_FragCoord.y / scale;

    // ripple the surface gently, rougher while it moves
    float swell = 1.0 + agitation * 3.0;
    float surface = level
        + sin(x * 0.017 + time * 0.7) * 1.8 * swell
        + sin(x * 0.043 - time * 1.0) * 0.8 * swell
        + sin(x * 0.11 + time * 1.4) * 0.3
        + rippleAt(x);
    float below = y - surface;

    // hold the water in a basin whose walls sag and swell slowly
    float wall = basin(vec2(x, y), size)
        - ((noise(vec2(x, y) * 0.007 + vec2(time * 0.025, time * 0.018)) - 0.5) * 2.0
        + noise(vec2(x, y) * 0.013 - vec2(time * 0.03, time * 0.012)) - 0.5) * 2.2;
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

    // brighten softly just under the surface
    color = mix(color, caustic, (1.0 - smoothstep(4.0, 26.0, below)) * 0.14);

    // slant faint shafts of light down from the surface, fading with depth
    float slant = x + below * 0.35;
    float shafts = smoothstep(0.55, 1.0, sin(slant * 0.018 + time * 0.15))
        + smoothstep(0.6, 1.0, sin(slant * 0.031 - time * 0.11 + 1.7)) * 0.7;
    color = mix(color, caustic, shafts * 0.07 * (1.0 - smoothstep(0.0, 0.8, depth)));

    // net the light into faint caustic lines that thin out with depth
    vec2 drift = vec2(x, y + sin(x * 0.02 + time * 0.5) * 6.0) / 70.0;
    float border = cells(drift, time * 0.6);
    float threshold = 0.035 - depth * 0.02;
    color = mix(color, caustic, (1.0 - smoothstep(threshold - 0.01, threshold, border)) * 0.06 * (1.0 - depth));

    // edge the surface with one foam line that thickens while the water moves, and rim the basin in foam
    float crest = 2.0 + agitation * 1.5;
    float froth = 1.0 - smoothstep(crest, crest + 0.8, below);
    color = mix(color, mix(foam, signal, agitation * 0.45), froth);
    color = mix(color, foam, rim);
    color = mix(color, ink, line * (1.0 - max(held, rim)));

    // keep the water nearly opaque so the submerged stack reads only as shapes
    float alpha = mix(0.74, 0.88, smoothstep(0.0, 0.8, depth));
    alpha = max(max(alpha * held, froth), max(rim, line)) * cover;
    gl_FragColor = vec4(color * alpha, alpha);
}
`;

/** Render stylised water below a waterline that drains and fills over time. */
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

        // shape the move: rough while travelling
        const progress = this.progressAt(now);
        const level = this.levelAt(now);
        const motion = Math.sin(progress * Math.PI);

        // upload the frame parameters
        context.uniform1f(shader.uniform("time"), this.isMoving ? now / 1000 : 0);
        context.uniform1f(shader.uniform("level"), level);
        context.uniform1f(shader.uniform("rest"), waterSpill - 3);
        context.uniform1f(shader.uniform("agitation"), motion);
        context.uniform3fv(shader.uniform("deep"), this.palette.deep);
        context.uniform3fv(shader.uniform("shallow"), this.palette.shallow);
        context.uniform3fv(shader.uniform("caustic"), this.palette.caustic);
        context.uniform3fv(shader.uniform("foam"), this.palette.foam);
        context.uniform3fv(shader.uniform("ink"), this.palette.ink);
        const packed = new Float32Array(rippleCapacity * 4).fill(-1000);
        ripples.forEach((ripple, index) =>
            packed.set([ripple.x, ripple.at, ripple.strength, 0], index * 4),
        );
        context.uniform4fv(shader.uniform("ripples"), packed);
        this.onLevel(level);

        // draw every frame while the waterline moves or ripples run, else at the ambient pace
        const isStirred = ripples.some((ripple) => now / 1000 - ripple.at < rippleLife);
        shader.pace = progress < 1 || isStirred ? 0 : ambientPace;

        // keep animating while water shows or the waterline still moves
        const isDrained = level >= shader.height && progress >= 1;

        return (this.isMoving && !isDrained) || progress < 1;
    }
}

/**
 * Return how far the resting waterline rises or falls at a position across the canvas, in CSS pixels.
 *
 * The waves match the shader's waves.
 */
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

/** Ease in and out, slow at both ends. */
function ease(progress: number) {
    return progress < 0.5 ? 4 * progress ** 3 : 1 - (-2 * progress + 2) ** 3 / 2;
}
