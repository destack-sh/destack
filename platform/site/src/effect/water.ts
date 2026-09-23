import { Shader } from "./gl";

/// The water colors for one theme, as RGB triples in 0..1.
export type WaterPalette = {
    /// The color at the bottom of the figure.
    deep: [number, number, number];
    /// The color just below the surface.
    shallow: [number, number, number];
    /// The caustic and light ray color.
    caustic: [number, number, number];
    /// The surface line color.
    foam: [number, number, number];
    /// The outline ink.
    ink: [number, number, number];
};

/// The water palette on the night page.
export const nightWater: WaterPalette = {
    deep: [0.03, 0.12, 0.16],
    shallow: [0.07, 0.28, 0.34],
    caustic: [0.45, 0.78, 0.84],
    foam: [0.945, 0.918, 0.859],
    ink: [0.01, 0.03, 0.04],
};

/// The water palette on the paper page.
export const paperWater: WaterPalette = {
    deep: [0.1, 0.3, 0.38],
    shallow: [0.24, 0.56, 0.64],
    caustic: [0.8, 0.93, 0.95],
    foam: [1, 1, 1],
    ink: [0.07, 0.19, 0.235],
};

/// Where the water drains, as a fraction of the page frame's width: straight down into the footer's black hole.
export const drainAt = 0.875;

/// The milliseconds a full drain or fill takes.
export const travel = 2400;

/// The most ripples the surface carries at once.
const rippleCapacity = 8;
/// How fast ripples run outward along the surface, in CSS pixels per second.
const rippleSpeed = 90;
/// The seconds a ripple takes to die away.
const rippleLife = 3;

/// Recent stirs of the surface: where across the water canvas, when in seconds, and how hard.
const ripples: { x: number; at: number; strength: number }[] = [];

/// Stir the surface at a position across the water canvas, sending ripples outward.
export function stir(x: number, strength: number) {
    ripples.push({ x, at: performance.now() / 1000, strength });
    if (ripples.length > rippleCapacity) {
        ripples.shift();
    }
}

/// The water fragment shader.
const fragmentSource = `
precision mediump float;
uniform vec2 resolution;
uniform float scale;
uniform float time;
uniform float level;
uniform float funnel;
uniform float fill;
uniform float agitation;
uniform vec3 deep;
uniform vec3 shallow;
uniform vec3 caustic;
uniform vec3 foam;
uniform vec3 ink;
uniform vec3 lens;
uniform vec4 ripples[${rippleCapacity}];

const vec3 signal = vec3(1.0, 0.475, 0.18);

vec2 hash2(vec2 p) {
    p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
    return fract(sin(p) * 43758.5453);
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

    // ripple the surface gently, rougher while it moves; pull a whirlpool down to the drain as it drains,
    // and well the water up from the drain as it fills
    float swell = 1.0 + agitation * 3.0;
    float surface = level
        + sin(x * 0.017 + time * 1.1) * 1.8 * swell
        + sin(x * 0.043 - time * 1.6) * 0.8 * swell
        + sin(x * 0.11 + time * 2.3) * 0.3
        + rippleAt(x);
    float drain = size.x * ${drainAt};
    float spread = (x - drain) / (size.x * 0.05);
    float lean = 1.0 - abs(x - drain) / size.x;
    surface += funnel * ((size.y - level) * exp(-spread * spread) + 60.0 * lean);
    surface -= fill * (size.y * 0.24 * exp(-spread * spread * 0.3) + 50.0 * lean);
    float below = y - surface;
    float cover = smoothstep(-0.6, 0.6, below);
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

    // streak the whirlpool over the drain with spiralling foam
    float whirl = (funnel + fill) * exp(-spread * spread * 0.4);
    float spiral = step(0.8, fract(y * 0.06 + (x - drain) * 0.03 - time * 2.5));
    color = mix(color, foam, whirl * spiral * 0.55);

    // net the light into faint caustic lines that thin out with depth
    vec2 drift = vec2(x, y + sin(x * 0.02 + time * 0.5) * 6.0) / 70.0;
    float border = cells(drift, time * 0.6);
    float threshold = 0.035 - depth * 0.02;
    color = mix(color, caustic, (1.0 - smoothstep(threshold - 0.01, threshold, border)) * 0.06 * (1.0 - depth));

    // clear the water inside the searchlight
    float offset = length(vec2(x, y) - lens.xy);
    float clear = (1.0 - smoothstep(lens.z - 1.5, lens.z + 0.5, offset)) * step(1.0, lens.z);
    color = mix(color, caustic, clear * 0.08);

    // edge the surface with one foam line that thickens while the water moves, and outline the sides in ink
    float crest = 2.0 + agitation * 1.5;
    float sides = min(min(x, size.x - x), size.y - y);
    float outline = 1.0 - smoothstep(0.6, 1.4, sides);
    float froth = 1.0 - smoothstep(crest, crest + 0.8, below);
    color = mix(color, mix(foam, signal, agitation * 0.45), froth);
    color = mix(color, ink, outline * 0.55);

    // keep the water nearly opaque so the submerged stack reads only as shapes, except through the searchlight
    float alpha = mix(mix(0.74, 0.88, smoothstep(0.0, 0.8, depth)), 0.04, clear);
    alpha = max(alpha, max(froth, outline)) * cover;
    gl_FragColor = vec4(color * alpha, alpha);
}
`;

/// Render stylised water below a surface that drains and fills over time.
export class Water {
    /// The shader that draws the water.
    shader: Shader;
    /// The current palette.
    palette: WaterPalette;
    /// The surface position at the start of the current move, in CSS pixels.
    from: number;
    /// The surface position the current move ends at.
    to: number;
    /// The start time of the current move in milliseconds.
    moved: number;
    /// The animation start time in milliseconds.
    start: number;
    /// Whether the water moves.
    isMoving: boolean;
    /// Receive the surface position after every frame.
    onLevel: (level: number) => void;
    /// The searchlight centre and radius in canvas CSS pixels, with no radius when off.
    searchlight: { x: number; y: number; radius: number };

    /// Create water on a canvas, or throw when WebGL is unavailable.
    constructor(
        canvas: HTMLCanvasElement,
        palette: WaterPalette,
        level: number,
        isMoving: boolean,
        onLevel: (level: number) => void,
    ) {
        this.shader = new Shader(canvas, fragmentSource, 1.5, (now) => this.draw(now));
        this.palette = palette;
        this.from = level;
        this.to = level;
        this.moved = -travel;
        this.start = performance.now();
        this.isMoving = isMoving;
        this.onLevel = onLevel;
        this.searchlight = { x: 0, y: 0, radius: 0 };
    }

    /// Shine the searchlight at a point in canvas CSS pixels, or switch it off with no radius.
    shine(x: number, y: number, radius: number) {
        this.searchlight = { x, y, radius };
        this.shader.request();
    }

    /// Drain or fill toward a new surface position.
    moveTo(level: number) {
        const now = performance.now();
        this.from = this.levelAt(now);
        this.to = level;
        this.moved = now;
        this.shader.request();
    }

    /// Place the surface immediately, without animating.
    place(level: number) {
        this.from = level;
        this.to = level;
        this.moved = -travel;
        this.shader.request();
    }

    /// Swap the palette and redraw.
    paint(palette: WaterPalette) {
        this.palette = palette;
        this.shader.request();
    }

    /// Return the surface position at a time.
    levelAt(now: number) {
        return this.from + (this.to - this.from) * ease(this.progressAt(now));
    }

    /// Return how far the current move has come, from 0 to 1.
    progressAt(now: number) {
        return this.isMoving ? Math.min(1, (now - this.moved) / travel) : 1;
    }

    /// Upload one frame, and return whether to keep going while any water shows or the surface moves.
    draw(now: number) {
        const shader = this.shader;
        const context = shader.context;

        // shape the move: rough while travelling, funnelled while draining, welling up while filling
        const progress = this.progressAt(now);
        const level = this.levelAt(now);
        const motion = Math.sin(progress * Math.PI);
        const isDraining = this.to > this.from;

        // upload the frame parameters
        context.uniform1f(shader.uniform("time"), this.isMoving ? now / 1000 : 0);
        const light = this.searchlight;
        context.uniform3f(shader.uniform("lens"), light.x, light.y, light.radius);
        context.uniform1f(shader.uniform("level"), level);
        context.uniform1f(shader.uniform("funnel"), isDraining ? motion : 0);
        context.uniform1f(shader.uniform("fill"), isDraining ? 0 : motion);
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

        // keep animating while water shows or the surface still moves
        const isDrained = level >= shader.height && progress >= 1;

        return (this.isMoving && !isDrained) || progress < 1;
    }
}

/// Return how far the resting surface rises or falls at a position across the water canvas, in CSS pixels,
/// matching the shader's waves.
export function waveAt(x: number, seconds: number) {
    const long = Math.sin(x * 0.017 + seconds * 1.1) * 1.8;
    const middle = Math.sin(x * 0.043 - seconds * 1.6) * 0.8;
    const short = Math.sin(x * 0.11 + seconds * 2.3) * 0.3;

    // add the ripples running along the surface, exactly as the shader does
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

/// Ease in and out, slow at both ends.
function ease(t: number) {
    return t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2;
}
