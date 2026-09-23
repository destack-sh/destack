import * as stylex from "@destack/style";
import { createSignal, type JSX, onSettled } from "@destack/view";

import { tokens } from "../style/tokens.stylex";
import { Shader } from "./gl";
import { drainAt } from "./water";

/// The distance the goo may spill past its cell, in CSS pixels.
const spill = 8;
/// The milliseconds the shader takes to fade in over the still stars.
const fadeTime = 500;

/// A still tile of stars that shows before the shader paints, seeded so server and browser agree.
const stillStars = (() => {
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

/// The goo fragment shader.
const fragmentSource = `
precision mediump float;
uniform vec2 resolution;
uniform float scale;
uniform float time;
uniform float rest;
uniform float wobble;
uniform vec2 pointer;
uniform float pull;
uniform vec2 drift;
uniform vec3 hole;
uniform float glow;
uniform float flow;

const vec3 space = vec3(0.051, 0.133, 0.2);
const vec3 rimColor = vec3(0.945, 0.918, 0.859);
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
    float alpha = mix(0.35, 0.9, hash(cell + 7.7)) * (0.85 + 0.15 * sin(t * (0.4 + hash(cell + 9.0) * 0.6) + seed * 40.0));
    return rimColor * disc * alpha;
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

    // rest as a rounded box whose edge sags and swells very slowly
    float d = box(frag - size * 0.5, size * 0.5 - rest, 6.0);
    float slow = noise(frag * 0.007 + vec2(time * 0.025, time * 0.018)) - 0.5;
    float swell = noise(frag * 0.013 - vec2(time * 0.03, time * 0.012)) - 0.5;
    d -= (slow * 2.0 + swell) * wobble;

    // swell heavily toward the pointer
    float away = length(frag - pointer);
    d -= pull * 10.0 * exp(-away * away / 9000.0);

    // clip to the goo with a fine cream rim
    float edge = 1.0 / scale;
    float inside = 1.0 - smoothstep(-edge, edge, d);
    float rim = 1.0 - smoothstep(0.4, 1.1, abs(d + 0.6));
    if (inside + rim < 0.001) {
        discard;
    }

    // bend the starlight around the black hole, if there is one
    vec2 around = frag - hole.xy;
    float distance = length(around);
    vec2 sky = frag;
    if (hole.z > 0.0) {
        sky += around / max(distance, 1.0) * hole.z * hole.z * 1.6 / max(distance, hole.z);
    }

    // glow softly from the upper left, under two sparse depths of flat stars that drift against the pointer
    float falloff = 1.0 - smoothstep(0.0, 1.0, length(frag - size * vec2(0.3, 0.2)) / max(size.x, size.y) * 1.8);
    vec3 nebula = (vec3(0.118, 0.259, 0.341) - space) * falloff * 0.55;
    vec3 far = starLayer(sky + drift * 0.4 + vec2(time * 1.2, time * 0.3), 26.0, 0.32, time);
    vec3 near = starLayer(sky + drift + 71.0 + vec2(time * 2.4, time * 0.6), 58.0, 0.35, time * 1.3) * 1.25;

    // shower shooting stars while the pointer is over the goo
    vec3 meteor = vec3(0.0);
    if (pull > 0.01) {
        for (int i = 1; i < 4; i++) {
            float track = float(i);
            meteor += streak(frag, size, time + track * 0.73, 0.9 + track * 0.35, track) * pull;
        }
    }

    // glow faintly inside the rim so the edge reads as a surface
    float sheen = (1.0 - smoothstep(0.0, 14.0, -d)) * 0.05;
    vec3 color = space + nebula + sheen + far * 0.6 + near * 0.8 + meteor;

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

        // pour a stream of water between the top edge and the hole while it blazes, down to drain and up to fill
        float width = hole.z * 0.45 * glow;
        float stream = (1.0 - smoothstep(width - 0.6, width + 0.6, abs(around.x))) * step(around.y, 0.0);
        float ripple = 0.6 + 0.4 * sin(frag.y * 0.9 - flow * time * 18.0);
        color = mix(color, mix(vec3(0.55, 0.82, 0.9), vec3(1.0), ripple * 0.5), stream * glow);
        float front = step(0.0, around.y);
        color += disk * (1.0 - front);
        color = mix(color, vec3(0.0), horizon);
        color += disk * front + vec3(1.0) * ring * (1.0 - horizon);
    }

    color = mix(color, rimColor, rim);

    float alpha = max(inside, rim);
    gl_FragColor = vec4(color * alpha, alpha);
}
`;

/// A heavy goo field of quiet stars that swells toward the pointer.
class Starfield {
    /// The shader that draws the goo.
    shader: Shader;
    /// The pointer position the goo follows, in canvas CSS pixels.
    target: { x: number; y: number };
    /// The smoothed pointer position.
    pointer: { x: number; y: number };
    /// The smoothed swell strength from 0 to 1.
    pull: number;
    /// Whether the pointer is over the goo.
    isHovered: boolean;
    /// Whether the goo moves at all.
    isMoving: boolean;
    /// Whether the first frame has been drawn.
    isPainted: boolean;
    /// The animation start time in milliseconds.
    start: number;
    /// Receive the first drawn frame.
    onPaint: () => void;
    /// The black hole's radius in CSS pixels, below the water's drain, or zero for none.
    hole: number;
    /// Which way water flows through the black hole: 1 draining into it, -1 welling out of it, 0 still.
    flow: () => number;
    /// The smoothed glow of the disk from 0 to 1.
    glow: number;
    /// The flow on the previous frame.
    lastFlow: number;
    /// The flare as the last of the water goes in, from 1 fading to 0.
    flare: number;

    /// Create goo on a canvas, or throw when WebGL is unavailable.
    constructor(
        canvas: HTMLCanvasElement,
        isMoving: boolean,
        hole: number,
        flow: () => number,
        onPaint: () => void,
    ) {
        this.shader = new Shader(canvas, fragmentSource, 1.5, (now) => this.draw(now));
        this.target = { x: 0, y: 0 };
        this.pointer = { x: 0, y: 0 };
        this.pull = 0;
        this.isHovered = false;
        this.isMoving = isMoving;
        this.isPainted = false;
        this.start = performance.now();
        this.onPaint = onPaint;
        this.hole = hole;
        this.flow = flow;
        this.glow = 0;
        this.lastFlow = 0;
        this.flare = 0;
    }

    /// Follow the pointer, in client coordinates.
    follow(clientX: number, clientY: number) {
        const bounds = this.shader.canvas.getBoundingClientRect();
        this.target = { x: clientX - bounds.left, y: clientY - bounds.top };
        if (!this.isHovered) {
            this.pointer = { ...this.target };
        }
        this.isHovered = true;
        this.shader.request();
    }

    /// Let the swell settle after the pointer leaves.
    release() {
        this.isHovered = false;
        this.shader.request();
    }

    /// Upload one frame, and return whether to keep going while the goo moves.
    draw(now: number) {
        const shader = this.shader;
        const context = shader.context;

        // move heavily: the pointer and swell lag far behind their targets
        this.pointer.x += (this.target.x - this.pointer.x) * 0.06;
        this.pointer.y += (this.target.y - this.pointer.y) * 0.06;
        this.pull += ((this.isHovered ? 1 : 0) - this.pull) * 0.04;

        // drift the stars against the pointer
        const driftX = (this.pointer.x - shader.width / 2) * -0.03 * this.pull;
        const driftY = (this.pointer.y - shader.height / 2) * -0.03 * this.pull;

        // upload the frame parameters
        context.uniform1f(shader.uniform("time"), this.isMoving ? (now - this.start) / 1000 : 0);
        context.uniform1f(shader.uniform("rest"), spill - 3);
        context.uniform1f(shader.uniform("wobble"), this.isMoving ? 1.6 : 0);
        context.uniform2f(shader.uniform("pointer"), this.pointer.x, this.pointer.y);
        context.uniform1f(shader.uniform("pull"), this.isMoving ? this.pull : 0);
        context.uniform2f(shader.uniform("drift"), driftX, driftY);

        // light the black hole's disk slowly
        const flow = this.flow();
        this.glow += (Math.abs(flow) - this.glow) * 0.04;

        // flare once as the last of the water goes in
        if (this.lastFlow === 1 && flow === 0) {
            this.flare = 1;
        }
        this.lastFlow = flow;
        this.flare *= 0.94;
        context.uniform3f(
            shader.uniform("hole"),
            shader.width * drainAt,
            shader.height / 2,
            this.hole,
        );
        context.uniform1f(shader.uniform("glow"), Math.min(1.8, this.glow + this.flare * 1.4));
        context.uniform1f(shader.uniform("flow"), flow === 0 ? 1 : flow);

        // announce the first frame
        if (!this.isPainted) {
            this.isPainted = true;
            this.onPaint();
        }

        return this.isMoving;
    }
}

/// Fill a lattice cell with goo that spills slightly past its edges, with content on top.
export function Goo(props: {
    children?: JSX.Element;
    hole?: number;
    flow?: number;
    style?: stylex.Styles;
}) {
    const [isPainted, setIsPainted] = createSignal(false);
    const [isCovered, setIsCovered] = createSignal(false);
    let host!: HTMLDivElement;
    let canvas!: HTMLCanvasElement;

    // run the goo only while it is on screen, and release it with the page
    onSettled(() => {
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        let field: Starfield;
        try {
            field = new Starfield(
                canvas,
                !isStill,
                props.hole ?? 0,
                () => props.flow ?? 0,
                () => {
                    setIsPainted(true);
                    setTimeout(() => setIsCovered(true), fadeTime);
                },
            );
        } catch (error) {
            console.error("goo rendering failed", error);
            return;
        }

        // follow the pointer over the cell
        const follow = (event: PointerEvent) => field.follow(event.clientX, event.clientY);
        const release = () => field.release();
        host.addEventListener("pointermove", follow);
        host.addEventListener("pointerleave", release);

        // pause while off screen
        const view = new IntersectionObserver(([entry]) => field.shader.show(entry.isIntersecting));
        view.observe(host);

        return () => {
            view.disconnect();
            host.removeEventListener("pointermove", follow);
            host.removeEventListener("pointerleave", release);
            field.shader.dispose();
        };
    });

    return (
        <div
            ref={host}
            style={isCovered() ? undefined : { "background-image": stillStars }}
            {...stylex.attrs(styles.host, props.style)}
        >
            <canvas
                ref={canvas}
                aria-hidden="true"
                {...stylex.attrs(styles.canvas, isPainted() && styles.canvasPainted)}
            />
            <div {...stylex.attrs(styles.content)}>{props.children}</div>
        </div>
    );
}

const styles = stylex.create({
    host: {
        backgroundColor: tokens.space,
        minWidth: 0,
        position: "relative",
    },
    canvas: {
        height: `calc(100% + ${spill * 2}px)`,
        left: `-${spill}px`,
        opacity: 0,
        pointerEvents: "none",
        position: "absolute",
        top: `-${spill}px`,
        transition: `opacity ${fadeTime}ms ease`,
        width: `calc(100% + ${spill * 2}px)`,
        zIndex: 3,
    },
    canvasPainted: {
        opacity: 1,
    },
    content: {
        height: "100%",
        position: "relative",
        zIndex: 4,
    },
});
