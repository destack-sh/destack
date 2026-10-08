import type { ShaderMount, ShaderValues } from "@destack/shader";

/** The most icebergs the water holds. */
const BERG_CAPACITY = 3;

/** How far the water reaches past the figure on the sides and bottom, in CSS pixels, where the figure clips its rim. */
export const WATER_SPILL = 8;

/** The milliseconds a full drain or fill takes. */
export const TRAVEL = 700;

/** The layers the water fragment draws: the ice behind the cards, and the water over them. */
export type WaterLayer = "ice" | "water";

/** The icebergs under the water: the centre of each across the canvas, the half width of the card it carries, and the depths its top and bottom sit at at rest, in CSS pixels. */
export type Bergs = {
    /** The centre of each iceberg across the canvas. */
    readonly centres: readonly number[];
    /** The half width of the card each iceberg carries. */
    readonly half: number;
    /** The waterline at rest, where each tip meets the water. */
    readonly top: number;
    /** The depth of each keel at rest. */
    readonly bottom: number;
    /** How far each tip rises above the waterline at rest. */
    readonly tip: number;
};

/** The water fragment shader: faceted icebergs riding a waterline that drains and fills, and the water over them. */
export const WATER_FRAGMENT = `#version 300 es
precision highp float;

uniform mediump vec2 u_resolution;
uniform mediump float u_pixelRatio;
uniform float u_time;
uniform float u_layer;
uniform float u_levelFrom;
uniform float u_levelTo;
uniform float u_movedAt;
uniform float u_travel;
uniform float u_rest;
uniform float u_tip;
uniform vec4 u_bergs[${BERG_CAPACITY}];
uniform vec4 u_shallow;
uniform vec4 u_deep;
uniform vec4 u_caustic;
uniform vec4 u_foam;
uniform vec4 u_ice;
uniform vec4 u_iceShadow;
uniform vec4 u_ink;

out vec4 fragColor;

float hash(float n) {
    return fract(sin(n) * 43758.5453);
}

vec2 hash2(vec2 p) {
    p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
    return fract(sin(p) * 43758.5453);
}

// return the waterline's depth: an ease out from the last level to the next
float levelAt() {
    float progress = clamp((u_time - u_movedAt) / u_travel, 0.0, 1.0);
    float eased = 1.0 - pow(1.0 - progress, 3.0);
    return mix(u_levelFrom, u_levelTo, eased);
}

// roll the surface with three slow waves
float waveAt(float x) {
    return sin(x * 0.017 + u_time * 0.7) * 1.8
        + sin(x * 0.043 - u_time * 1.0) * 0.8
        + sin(x * 0.11 + u_time * 1.4) * 0.3;
}

// return the nearest and second nearest distances to drifting cell seeds
vec2 cells(vec2 p, float drift) {
    vec2 cell = floor(p);
    vec2 local = fract(p);
    float nearest = 8.0;
    float second = 8.0;
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            vec2 offset = vec2(float(x), float(y));
            vec2 seed = hash2(cell + offset);
            vec2 point = offset + 0.5 + 0.38 * sin(drift + 6.2831 * seed);
            float distance = length(point - local);
            if (distance < nearest) {
                second = nearest;
                nearest = distance;
            } else if (distance < second) {
                second = distance;
            }
        }
    }
    return vec2(nearest, second);
}

// return the id of the facet a point falls in, flat across the facet
float facet(vec2 p) {
    vec2 cell = floor(p);
    vec2 local = fract(p);
    float nearest = 8.0;
    vec2 owner = cell;
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            vec2 offset = vec2(float(x), float(y));
            vec2 point = offset + 0.15 + 0.7 * hash2(cell + offset);
            float distance = length(point - local);
            if (distance < nearest) {
                nearest = distance;
                owner = cell + offset;
            }
        }
    }
    return hash(dot(owner, vec2(17.3, 91.7)));
}

// return a berg's half width below the water at a share of its depth, in card half widths
float massWidth(float s, float seed) {
    float k = clamp(s, 0.0, 1.0) * 5.0;
    float i = floor(min(k, 4.999));
    float f = k - i;
    float c0 = 0.9 + 0.05 * hash(seed + 1.0);
    float c1 = 1.02 + 0.05 * hash(seed + 2.0);
    float c2 = 1.06 + 0.04 * hash(seed + 3.0);
    float c3 = 0.9 + 0.06 * hash(seed + 4.0);
    float c4 = 0.6 + 0.08 * hash(seed + 5.0);
    float c5 = 0.18 + 0.1 * hash(seed + 6.0);
    float a = i < 1.0 ? c0 : i < 2.0 ? c1 : i < 3.0 ? c2 : i < 4.0 ? c3 : c4;
    float b = i < 1.0 ? c1 : i < 2.0 ? c2 : i < 3.0 ? c3 : i < 4.0 ? c4 : c5;
    return mix(a, b, f);
}

// return one of the seven ridge points of a berg's tip, highest at its apex
float ridge(float i, float seed) {
    if (i < 0.5 || i > 5.5) {
        return 0.0;
    }
    float apex = 2.0 + floor(hash(seed + 9.0) * 3.0);
    float fall = abs(i - apex);
    return (1.0 - fall * 0.3) * (0.7 + 0.3 * hash(seed + i * 7.1));
}

// return a berg's tip height above the waterline across its width, in tip heights
float tipHeight(float u, float seed) {
    float k = (clamp(u, -1.0, 1.0) * 0.5 + 0.5) * 6.0;
    float i = floor(min(k, 5.999));
    return mix(ridge(i, seed), ridge(i + 1.0, seed), k - i);
}

// return the signed distance to the basin: straight walls and a floor with rounded corners, open far above
float basin(vec2 p, vec2 size) {
    vec2 centre = vec2(size.x * 0.5, (size.y - u_rest - 4000.0) * 0.5);
    vec2 extent = vec2(size.x * 0.5 - u_rest, (size.y - u_rest + 4000.0) * 0.5);
    vec2 q = abs(p - centre) - extent + 6.0;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - 6.0;
}

void main() {
    // work in CSS pixels from the top left
    vec2 size = u_resolution / u_pixelRatio;
    vec2 p = vec2(gl_FragCoord.x, u_resolution.y - gl_FragCoord.y) / u_pixelRatio;
    float edge = 1.0 / u_pixelRatio;
    float level = levelAt();
    float surface = level + waveAt(p.x);
    float below = p.y - surface;
    float wall = basin(p, size);
    float held = 1.0 - smoothstep(-edge, edge, wall);
    vec4 color = vec4(0.0);

    // draw each berg riding the waterline: a jagged tip above it and a faceted mass below, sinking as the water drains
    if (u_layer < 0.5) {
        for (int i = 0; i < ${BERG_CAPACITY}; i++) {
            vec4 berg = u_bergs[i];
            float span = berg.w - berg.z;
            float presence = clamp((berg.w - level) / max(span, 1.0), 0.0, 1.0);
            if (berg.y <= 0.0 || presence <= 0.0) {
                continue;
            }
            float seed = float(i) * 13.7 + 3.0;
            float bend = below > 0.0 ? sin(p.y * 0.07 + u_time * 1.3) * 1.4 : 0.0;
            float across = p.x - berg.x + bend;
            float inside;
            float s = below / span;
            if (below < 0.0) {
                float base = massWidth(0.0, seed) * berg.y * 0.9;
                float rise = tipHeight(across / base, seed) * u_tip * presence;
                inside = min(rise + below, base - abs(across));
            } else {
                inside = s > 1.0 ? -1.0 : massWidth(s, seed) * berg.y - abs(across);
            }
            float cover = smoothstep(-edge, edge, inside) * (below > 0.0 ? held : 1.0);
            if (cover <= 0.0) {
                continue;
            }

            // shade flat facets lit from the upper left, their edges catching the light
            vec2 grain = vec2(across, p.y - surface) / 22.0 + seed;
            float shade = 0.72 + 0.28 * facet(grain);
            shade += -0.18 * clamp(across / berg.y, -1.0, 1.0);
            vec2 near = cells(grain, 0.0);
            float ridgeLine = 1.0 - smoothstep(0.02, 0.06, near.y - near.x);
            vec3 ice = mix(u_iceShadow.rgb, u_ice.rgb, clamp(shade + ridgeLine * 0.12, 0.0, 1.0));

            // cool and dim the ice with depth, and outline the berg in ink
            float depth = clamp(s, 0.0, 1.0);
            ice = mix(ice, u_deep.rgb, below > 0.0 ? 0.25 + 0.45 * depth : 0.0);
            float outline = 1.0 - smoothstep(0.4 * edge, 1.4 * edge, abs(inside));
            ice = mix(ice, u_ink.rgb, outline * (below > 0.0 ? 0.18 : 0.7));
            color = vec4(ice * cover, cover);
        }
        fragColor = color;
        return;
    }

    // hold the water in the basin below its rolling surface
    float cover = smoothstep(-edge, edge, below) * held;
    if (cover <= 0.0) {
        fragColor = vec4(0.0);
        return;
    }

    // deepen from a clear shallow to the deep, with faint rays slanting down from the surface
    float depth = clamp(below / max(size.y - level, 60.0), 0.0, 1.0);
    vec3 water = mix(u_shallow.rgb, u_deep.rgb, smoothstep(0.0, 1.0, depth));
    float rays = pow(0.5 + 0.5 * sin(p.x * 0.03 + p.y * 0.012 + sin(p.x * 0.007 + u_time * 0.2) * 2.0), 8.0);
    water = mix(water, u_caustic.rgb, rays * 0.12 * (1.0 - depth));

    // net the light into caustic lines that drift and thin out with depth
    vec2 net = cells(p / 70.0, u_time * 0.3);
    float threshold = 0.05 - depth * 0.03;
    water = mix(water, u_caustic.rgb, (1.0 - smoothstep(threshold - 0.02, threshold, net.y - net.x)) * 0.16 * (1.0 - depth));

    // light the band under the surface, and edge it with foam that gathers around each berg
    float glow = 1.0 - smoothstep(0.0, 10.0, below);
    water = mix(water, u_caustic.rgb, glow * 0.22);
    float froth = 1.0 - smoothstep(0.8, 2.0, below);
    for (int i = 0; i < ${BERG_CAPACITY}; i++) {
        vec4 berg = u_bergs[i];
        if (berg.y <= 0.0) {
            continue;
        }
        float reach = massWidth(0.0, float(i) * 13.7 + 3.0) * berg.y;
        float gap = abs(abs(p.x - berg.x) - reach * 0.92);
        froth = max(froth, (1.0 - smoothstep(0.0, 9.0, gap)) * (1.0 - smoothstep(0.0, 4.0, below)) * 0.8);
    }
    water = mix(water, u_foam.rgb, froth);

    // let the cards show through the shallows, and thicken the water toward the bottom
    float alpha = mix(0.5, 0.82, smoothstep(0.0, 0.9, depth));
    alpha = max(alpha, froth) * cover;
    fragColor = vec4(water * alpha, alpha);
}
`;

/** Ease a waterline from one level to the next on a shader mount's clock, reading the level in between. */
export class Waterline {
    /** The level the current move starts at, in CSS pixels. */
    from: number;
    /** The level the current move ends at. */
    to: number;
    /** The animation time the current move started at, in seconds. */
    movedAt: number;
    /** The mount whose clock the moves run on. */
    mount: ShaderMount | undefined;

    /** Rest the waterline at a level. */
    constructor(level: number) {
        // start and end at the level, with no move under way
        this.from = level;
        this.to = level;
        this.movedAt = -TRAVEL / 1000;
        this.mount = undefined;
    }

    /** The mount's animation time in seconds, which the shader reads as `u_time`. */
    get now(): number {
        return (this.mount?.frame ?? 0) / 1000;
    }

    /** Return how far the current move has come, from 0 to 1. */
    progress(): number {
        return Math.min(1, Math.max(0, (this.now - this.movedAt) / (TRAVEL / 1000)));
    }

    /** Return the level now, as the shader draws it. */
    level(): number {
        const eased = 1 - (1 - this.progress()) ** 3;

        return this.from + (this.to - this.from) * eased;
    }

    /** Start a move from the current level toward another. */
    moveTo(level: number): void {
        this.from = this.level();
        this.to = level;
        this.movedAt = this.now;
    }

    /** Place the waterline at a level at once. */
    place(level: number): void {
        this.from = level;
        this.to = level;
        this.movedAt = this.now - TRAVEL / 1000;
    }

    /** Return the shader values of a layer: the move, the bergs and the colors. */
    values(layer: WaterLayer, bergs: Bergs | undefined, colors: WaterColors): ShaderValues {
        // stand each berg on its column, the empty ones zero wide
        const slots = Array.from({ length: BERG_CAPACITY }, (_, index) => {
            const centre = bergs?.centres[index];

            return bergs === undefined || centre === undefined
                ? [0, 0, 0, 0]
                : [centre, bergs.half, bergs.top, bergs.bottom];
        });

        return {
            u_layer: layer === "ice" ? 0 : 1,
            u_levelFrom: this.from,
            u_levelTo: this.to,
            u_movedAt: this.movedAt,
            u_travel: TRAVEL / 1000,
            u_rest: WATER_SPILL - 3,
            u_tip: bergs?.tip ?? 0,
            u_bergs: slots,
            u_shallow: colors.shallow,
            u_deep: colors.deep,
            u_caustic: colors.caustic,
            u_foam: colors.foam,
            u_ice: colors.ice,
            u_iceShadow: colors.iceShadow,
            u_ink: colors.ink,
        };
    }
}

/** The colors of the water and its ice, as CSS colors that may follow the appearance. */
export type WaterColors = {
    /** The water just below the surface. */
    readonly shallow: string;
    /** The water at the bottom. */
    readonly deep: string;
    /** The light the surface nets into caustics and rays. */
    readonly caustic: string;
    /** The foam along the surface. */
    readonly foam: string;
    /** The sunlit faces of the ice. */
    readonly ice: string;
    /** The shaded faces of the ice. */
    readonly iceShadow: string;
    /** The outline ink. */
    readonly ink: string;
};
