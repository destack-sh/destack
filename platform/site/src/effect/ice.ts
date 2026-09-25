import { Shader } from "./gl";

/** The milliseconds the ice takes to shatter or reassemble. */
const breakTime = 1200;

/** How one berg floats at a moment: its rise below the waterline and slide across in CSS pixels, and its lean clockwise in degrees. */
export type Bob = { lift: number; sway: number; tilt: number };

/** The ice fragment shader. */
const fragmentSource = `
precision mediump float;
uniform vec2 resolution;
uniform float scale;
uniform float time;
uniform float waterline;
uniform vec3 centres;
uniform float column;
uniform float bulk;
uniform float shatter;
uniform vec3 bobs;
uniform vec3 sways;
uniform vec3 tilts;
uniform float offset;

// the facet size, in cells per CSS pixel; shards break along the same facets
const float grain = 0.032;

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

vec2 hash2(vec2 p) {
    return vec2(hash(p), hash(p + 17.3));
}

// fold one polygon edge into a running distance and inside sign
void edge(vec2 p, vec2 a, vec2 b, inout float nearest, inout float sign) {
    vec2 along = b - a;
    vec2 from = p - a;
    vec2 gap = from - along * clamp(dot(from, along) / dot(along, along), 0.0, 1.0);
    nearest = min(nearest, dot(gap, gap));
    bool isAbove = p.y >= a.y;
    bool isBelow = p.y < b.y;
    bool isLeft = along.x * from.y > along.y * from.x;
    if ((isAbove && isBelow && isLeft) || (!isAbove && !isBelow && !isLeft)) {
        sign = -sign;
    }
}

// return the signed distance to one berg: a low ridge of hard-planed peaks above water, a lean faceted mass below that keeps clear of its neighbours
float berg(vec2 p, float index) {
    // vary the bergs: mirror the middle one and narrow the last
    p.x *= index == 1.0 ? -1.0 : 1.0;
    p.x /= index == 2.0 ? 0.9 : 1.0;
    float w = column;
    float h = min(column * 0.42, waterline * 0.62);
    float b = bulk;

    // trace the ridge above the waterline in a few hard planes up to two peaks, slightly overlapping the mass below
    float nearest = 1e9;
    float sign = 1.0;
    vec2 a0 = vec2(-0.4 * w, 6.0);
    vec2 a1 = vec2(-0.33 * w, -0.36 * h);
    vec2 a2 = vec2(-0.17 * w, -0.7 * h);
    vec2 a3 = vec2(-0.05 * w, -h);
    vec2 a4 = vec2(0.09 * w, -0.6 * h);
    vec2 a5 = vec2(0.23 * w, -0.76 * h);
    vec2 a6 = vec2(0.35 * w, -0.3 * h);
    vec2 a7 = vec2(0.42 * w, 6.0);
    edge(p, a0, a1, nearest, sign);
    edge(p, a1, a2, nearest, sign);
    edge(p, a2, a3, nearest, sign);
    edge(p, a3, a4, nearest, sign);
    edge(p, a4, a5, nearest, sign);
    edge(p, a5, a6, nearest, sign);
    edge(p, a6, a7, nearest, sign);
    edge(p, a7, a0, nearest, sign);
    float ridge = sign * sqrt(nearest);

    // trace the mass below in straight cuts: steep shouldered walls stepping in to a chiselled, off-centre keel
    nearest = 1e9;
    sign = 1.0;
    vec2 b0 = vec2(-0.4 * w, -2.0);
    vec2 b1 = vec2(-0.43 * w, 0.1 * b);
    vec2 b2 = vec2(-0.42 * w, 0.38 * b);
    vec2 b3 = vec2(-0.33 * w, 0.6 * b);
    vec2 b4 = vec2(-0.27 * w, 0.84 * b);
    vec2 b5 = vec2(-0.06 * w, 0.99 * b);
    vec2 b6 = vec2(0.17 * w, 0.9 * b);
    vec2 b7 = vec2(0.36 * w, 0.57 * b);
    vec2 b8 = vec2(0.41 * w, 0.3 * b);
    vec2 b9 = vec2(0.43 * w, 0.08 * b);
    vec2 b10 = vec2(0.41 * w, -2.0);
    edge(p, b0, b1, nearest, sign);
    edge(p, b1, b2, nearest, sign);
    edge(p, b2, b3, nearest, sign);
    edge(p, b3, b4, nearest, sign);
    edge(p, b4, b5, nearest, sign);
    edge(p, b5, b6, nearest, sign);
    edge(p, b6, b7, nearest, sign);
    edge(p, b7, b8, nearest, sign);
    edge(p, b8, b9, nearest, sign);
    edge(p, b9, b10, nearest, sign);
    edge(p, b10, b0, nearest, sign);
    float mass = sign * sqrt(nearest);
    return min(ridge, mass) - 1.0;
}

// return the facet cell nearest a point in facet space, and the distance to its border
vec3 facet(vec2 f) {
    vec2 cell = floor(f);
    vec2 local = fract(f);
    float nearest = 8.0;
    float second = 8.0;
    vec2 owner = vec2(0.0);
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            vec2 offset = vec2(float(x), float(y));
            float distance = length(offset + hash2(cell + offset) - local);
            if (distance < nearest) {
                second = nearest;
                nearest = distance;
                owner = cell + offset;
            } else if (distance < second) {
                second = distance;
            }
        }
    }
    return vec3(owner, second - nearest);
}

// return how far one shard has flown at the current shatter
vec2 flight(vec2 shard, float index) {
    vec2 centre = (shard + hash2(shard) - index * 11.0) / grain;
    vec2 outward = normalize(centre - vec2(0.0, bulk * 0.2) + 0.001);
    float speed = 18.0 + hash(shard + 5.0) * 26.0;
    return outward * speed * shatter + vec2(0.0, 90.0 * shatter * shatter);
}

// return the water surface at a canvas position, matching the water shader's resting waves
float surfaceAt(float x) {
    float figureX = x + offset;
    return waterline
        + sin(figureX * 0.017 + time * 0.7) * 1.8
        + sin(figureX * 0.043 - time * 1.0) * 0.8
        + sin(figureX * 0.11 + time * 1.4) * 0.3;
}

// the outline ink
const vec3 ink = vec3(0.07, 0.19, 0.235);

// shade a facet in flat cel tones lit from the upper left: white and a faint blue above water, two close blues below
vec3 shade(vec2 local, vec2 cell, bool isAbove) {
    float light = 0.55 + 0.45 * hash(cell) - local.x / column * 0.5;
    if (isAbove) {
        return light > 0.62 ? vec3(1.0) : vec3(0.9, 0.955, 0.98);
    }
    return light > 0.6 ? vec3(0.5, 0.76, 0.85) : vec3(0.43, 0.7, 0.8);
}

void main() {
    vec2 frag = vec2(gl_FragCoord.x, resolution.y - gl_FragCoord.y) / scale;
    float edge = 1.0 / scale;

    for (int i = 0; i < 3; i++) {
        float index = float(i);
        float x = i == 0 ? centres.x : (i == 1 ? centres.y : centres.z);
        float lift = i == 0 ? bobs.x : (i == 1 ? bobs.y : bobs.z);
        float sway = i == 0 ? sways.x : (i == 1 ? sways.y : sways.z);
        float tilt = i == 0 ? tilts.x : (i == 1 ? tilts.y : tilts.z);
        vec2 p = frag - vec2(x + sway, waterline + lift);
        p = mat2(cos(tilt), sin(tilt), -sin(tilt), cos(tilt)) * p;

        // skip pixels far outside this berg and its flying shards
        float extent = column * 0.7 + shatter * 70.0;
        if (abs(p.x) > extent || p.y < -column * 0.5 - shatter * 40.0 || p.y > bulk + 20.0 + shatter * 90.0) {
            continue;
        }

        // whole ice: shade the berg directly, with a white rim
        if (shatter <= 0.0) {
            float d = berg(p, index);
            float inside = 1.0 - smoothstep(-edge, edge, d);
            if (inside > 0.001) {
                // split the berg at the moving water surface, then pencil the creases above it and ink the outline
                bool isAbove = frag.y < surfaceAt(frag.x) + 1.0;
                vec3 cell = facet(p * grain + index * 11.0);
                vec3 color = shade(p, cell.xy, isAbove);
                float crease = (1.0 - smoothstep(0.0, 0.04, cell.z)) * (isAbove ? 1.0 : 0.0);
                color = mix(color, ink, crease * 0.08);
                color = mix(color, ink, smoothstep(-1.6, -0.8, d) * 0.6);
                gl_FragColor = vec4(color * inside, inside);
                return;
            }
            continue;
        }

        // shattered ice: find the shard that has flown over this pixel
        vec2 home = floor((p - vec2(0.0, 90.0 * shatter * shatter)) * grain + index * 11.0);
        for (int y = -1; y <= 1; y++) {
            for (int x = -1; x <= 1; x++) {
                vec2 shard = home + vec2(float(x), float(y));
                vec2 centre = (shard + hash2(shard) - index * 11.0) / grain;
                vec2 origin = centre + (p - flight(shard, index) - centre) / (1.0 - 0.45 * shatter);
                vec3 cell = facet(origin * grain + index * 11.0);
                float d = berg(origin, index);
                if (cell.x == shard.x && cell.y == shard.y && d < 0.0) {
                    // ink the cracks as the ice breaks, easing back to whole-ice creases and outline as it settles
                    float apart = smoothstep(0.0, 0.25, shatter);
                    float crack = 1.0 - smoothstep(0.012, 0.03, cell.z);
                    float crease = (1.0 - smoothstep(0.0, 0.04, cell.z)) * step(origin.y, 0.0) * 0.08;
                    vec3 color = mix(shade(origin, shard, origin.y < 0.0), ink, mix(crease, crack * 0.6, apart));
                    color = mix(color, ink, smoothstep(-1.6, -0.8, d) * 0.6);
                    float alpha = 1.0 - smoothstep(0.08, 0.4, shatter);
                    gl_FragColor = vec4(color * alpha, alpha);
                    return;
                }
            }
        }
    }

    discard;
}
`;

/** Render faceted icebergs centred on three columns, floating at a waterline, able to shatter. */
export class Ice {
    /** The shader that draws the ice. */
    shader: Shader;
    /** The waterline in canvas CSS pixels. */
    waterline: number;
    /** The submerged depth of each berg in CSS pixels, fixed at its resting waterline. */
    bulk: number;
    /** The shatter progress at the start of the current break, from 0 whole to 1 gone. */
    from: number;
    /** The shatter progress the current break ends at. */
    to: number;
    /** The start time of the current break in milliseconds. */
    broke: number;
    /** Whether the ice moves. */
    isMoving: boolean;
    /** The drawing's left edge within the water canvas, so both share one set of waves. */
    offset: number;
    /** The centre of each berg as a fraction of the canvas width. */
    centres: readonly number[];
    /** Return how a berg floats at a time in seconds, shared with everything riding on it. */
    bobOf: (berg: number, seconds: number) => Bob;
    /** How each berg floats in the current frame. */
    bobs: Bob[];
    /** Run after every drawn frame, so things floating beside the ice move in step with it. */
    onFrame: () => void;

    /** Create ice on a canvas, or throw when WebGL is unavailable. */
    constructor(
        canvas: HTMLCanvasElement,
        centres: readonly number[],
        isMoving: boolean,
        bobOf: (berg: number, seconds: number) => Bob,
        onFrame: () => void,
    ) {
        // start the shader and rest the ice whole
        this.shader = new Shader(canvas, fragmentSource, 1, (now) => this.draw(now));
        this.waterline = 0;
        this.bulk = 0;
        this.from = 0;
        this.to = 0;
        this.broke = -breakTime;
        this.isMoving = isMoving;
        this.bobOf = bobOf;
        this.bobs = centres.map(() => ({ lift: 0, sway: 0, tilt: 0 }));
        this.onFrame = onFrame;
        this.offset = 0;
        this.centres = centres;
    }

    /** Float the ice at a waterline with a submerged depth, at an offset within the water canvas, and redraw. */
    place(waterline: number, bulk: number, offset: number) {
        // store the placement and redraw
        this.waterline = waterline;
        this.bulk = bulk;
        this.offset = offset;
        this.shader.request();
    }

    /** Shatter the ice, or reassemble it, after a delay in milliseconds. */
    breakTo(shatter: number, delay: number) {
        // start a break from the current shatter
        const now = performance.now();
        this.from = this.shatterAt(now);
        this.to = shatter;
        this.broke = this.isMoving ? now + delay : -breakTime;
        this.shader.request();
    }

    /** Return the shatter progress at a time, easing out as the shards slow or settle. */
    shatterAt(now: number) {
        const progress = Math.max(0, Math.min(1, (now - this.broke) / breakTime));
        const eased = 1 - (1 - progress) ** 2;

        return this.from + (this.to - this.from) * eased;
    }

    /** Upload one frame, and return whether to keep going while the ice bobs or breaks. */
    draw(now: number) {
        // read the shader and its context
        const shader = this.shader;
        const context = shader.context;

        // centre one berg under each board column, sized to a third of the canvas
        const width = shader.width;
        const column = width / 3;
        const shatter = this.shatterAt(now);

        // float each berg as one body with everything riding on it, or hold it still
        const seconds = now / 1000;
        this.bobs = this.centres.map((_, index) =>
            this.isMoving ? this.bobOf(index, seconds) : { lift: 0, sway: 0, tilt: 0 },
        );
        const [first, second, third] = this.bobs;
        context.uniform1f(shader.uniform("time"), this.isMoving ? now / 1000 : 0);
        context.uniform1f(shader.uniform("offset"), this.offset);
        context.uniform1f(shader.uniform("waterline"), this.waterline);
        const [left, middle, right] = this.centres.map((centre) => centre * width);
        context.uniform3f(shader.uniform("centres"), left, middle, right);
        context.uniform1f(shader.uniform("column"), column);
        context.uniform1f(shader.uniform("bulk"), this.bulk);
        context.uniform1f(shader.uniform("shatter"), Math.min(shatter, 0.999));
        context.uniform3f(shader.uniform("bobs"), first.lift, second.lift, third.lift);
        context.uniform3f(shader.uniform("sways"), first.sway, second.sway, third.sway);
        context.uniform3f(
            shader.uniform("tilts"),
            (-first.tilt * Math.PI) / 180,
            (-second.tilt * Math.PI) / 180,
            (-third.tilt * Math.PI) / 180,
        );
        this.onFrame();

        // stop once the ice is fully gone; keep bobbing while it stands
        const isBreaking = now - this.broke < breakTime;

        return isBreaking || (this.isMoving && shatter < 1);
    }
}
