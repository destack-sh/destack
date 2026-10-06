/** The vertex stage shared by every full-canvas shader. */
const vertexSource = `
attribute vec2 position;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}
`;

/** The milliseconds between drawn frames for ambient motion: thirty a second. */
export const ambientPace = 33;

/** The pixel density factors the governor steps through, from full to coarsest. */
const densitySteps = [1, 0.75, 0.5, 0.35];
/** The frame intervals the governor judges at once. */
const judgedFrames = 20;
/** The median frame interval above which the governor coarsens, in milliseconds. */
const slowInterval = 24;
/** The median frame interval below which a judged window counts as smooth, in milliseconds. */
const smoothInterval = 18;
/** The smooth windows in a row the governor waits for before it sharpens again. */
const smoothWindows = 4;

/** Coarsen every shader's pixel density while the page's frames run long, and sharpen it again once they run smooth. */
class Governor {
    /** The index into `densitySteps`. */
    step: number;
    /** The frame time last observed, in milliseconds. */
    observedAt: number;
    /** The frame intervals of the window being judged. */
    intervals: number[];
    /** The smooth windows judged in a row. */
    smooth: number;

    /** Start at full density. */
    constructor() {
        // begin sharp, with no frame observed
        this.step = 0;
        this.observedAt = Number.NEGATIVE_INFINITY;
        this.intervals = [];
        this.smooth = 0;
    }

    /** The density factor every shader draws at. */
    get density() {
        return densitySteps[this.step] ?? 1;
    }

    /** Start at the coarsest density. */
    coarsen() {
        this.step = densitySteps.length - 1;
    }

    /** Record one animation frame, once per frame however many shaders draw it, and judge each full window. */
    observe(now: number) {
        // skip repeats within a frame, and gaps from hidden tabs or a first frame
        if (now === this.observedAt) {
            return;
        }
        const interval = now - this.observedAt;
        this.observedAt = now;
        if (interval > 250) {
            return;
        }

        // judge the window by its median interval
        this.intervals.push(interval);
        if (this.intervals.length < judgedFrames) {
            return;
        }
        const median =
            this.intervals.toSorted((left, right) => left - right)[judgedFrames >> 1] ?? interval;
        this.intervals = [];
        if (median > slowInterval) {
            this.step = Math.min(this.step + 1, densitySteps.length - 1);
            this.smooth = 0;
        } else if (median < smoothInterval && this.step > 0) {
            this.smooth += 1;
            if (this.smooth >= smoothWindows) {
                this.step -= 1;
                this.smooth = 0;
            }
        }
    }
}

/** The one governor every shader on the page shares. */
const governor = new Governor();

/** The renderers that draw WebGL on the processor. */
const softwareRenderers = /swiftshader|basic render|llvmpipe|softpipe/iu;

/** Whether WebGL draws slowly here, decided once: undefined until asked. */
let isWeak: boolean | undefined;

/** Return whether this browser draws WebGL slowly: a software renderer, or one the browser itself warns about. */
export function isWeakGraphics() {
    if (isWeak === undefined) {
        // ask for a context the browser refuses when it would be slow, then read the renderer's name
        const probe = document.createElement("canvas");
        const context = probe.getContext("webgl", { failIfMajorPerformanceCaveat: true });
        const info = context?.getExtension("WEBGL_debug_renderer_info");
        const renderer = info ? String(context?.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
        isWeak = context === null || softwareRenderers.test(renderer);
        context?.getExtension("WEBGL_lose_context")?.loseContext();
        if (isWeak) {
            governor.coarsen();
        }
    }

    return isWeak;
}

/** A fragment shader drawn over its whole canvas with premultiplied alpha, one animation frame at a time. */
export class Shader {
    /** The drawing context. */
    context: WebGLRenderingContext;
    /** The compiled shader program. */
    program: WebGLProgram;
    /** The canvas being drawn. */
    canvas: HTMLCanvasElement;
    /** The uniform locations by name. */
    uniforms: Map<string, WebGLUniformLocation | null>;
    /** The displayed width in CSS pixels, tracked by `sizes`. */
    width: number;
    /** The displayed height in CSS pixels. */
    height: number;
    /** The most device pixels drawn per CSS pixel. */
    density: number;
    /** The part of the canvas worth shading, in CSS pixels from its top left, or the whole canvas. */
    clip: { left: number; top: number; width: number; height: number } | undefined;
    /** The observer that tracks the displayed size. */
    sizes: ResizeObserver;
    /** The canvas's top in CSS pixels: from the viewport's top when fixed, else from the page's top. */
    offset: number;
    /** Whether the canvas stays put in the viewport as the page scrolls. */
    isFixed: boolean;
    /** Whether the last frame shaded only the part of the clip on screen. */
    isPartial: boolean;
    /** Draw the rest of a partly shaded frame as the page scrolls it into view. */
    onScroll: () => void;
    /** The pending animation frame, if any. */
    frame: number | undefined;
    /** Whether the canvas is on screen. */
    isVisible: boolean;
    /** Upload the uniforms of one frame, and return whether to keep animating. */
    onDraw: (now: number) => boolean;
    /** The fewest milliseconds between drawn frames: 0 draws every frame. */
    pace: number;
    /** When the last frame was drawn, in milliseconds. */
    drawnAt: number;

    /** Compile a fragment shader for a canvas at a capped pixel density, or throw when WebGL is unavailable. */
    constructor(
        canvas: HTMLCanvasElement,
        fragmentSource: string,
        density: number,
        onDraw: (now: number) => boolean,
    ) {
        // get the WebGL context
        const context = canvas.getContext("webgl", { premultipliedAlpha: true, alpha: true });
        if (context === null) {
            throw new Error("webgl is unavailable");
        }

        // compile the program and track the displayed size
        this.canvas = canvas;
        this.context = context;
        this.program = createProgram(context, fragmentSource);
        this.uniforms = new Map();
        this.density = density;
        this.width = canvas.clientWidth;
        this.height = canvas.clientHeight;
        this.offset = 0;
        this.isFixed = false;
        this.isPartial = false;
        this.sizes = new ResizeObserver((entries) => {
            for (const entry of entries) {
                this.width = entry.contentRect.width;
                this.height = entry.contentRect.height;
            }
            this.measure();
        });
        this.sizes.observe(canvas);
        this.onScroll = () => {
            if (this.isPartial) {
                this.request();
            }
        };
        window.addEventListener("scroll", this.onScroll, { passive: true });
        this.frame = undefined;
        this.isVisible = true;
        this.onDraw = onDraw;
        this.pace = 0;
        this.clip = undefined;
        this.drawnAt = -Infinity;

        // cover the canvas with one quad
        const buffer = context.createBuffer();
        context.bindBuffer(context.ARRAY_BUFFER, buffer);
        context.bufferData(
            context.ARRAY_BUFFER,
            new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]),
            context.STATIC_DRAW,
        );
        const position = context.getAttribLocation(this.program, "position");
        context.enableVertexAttribArray(position);
        context.vertexAttribPointer(position, 2, context.FLOAT, false, 0, 0);
    }

    /** Return the uniform location for a name, looking it up once. */
    uniform(name: string) {
        if (!this.uniforms.has(name)) {
            this.uniforms.set(name, this.context.getUniformLocation(this.program, name));
        }

        return this.uniforms.get(name) ?? null;
    }

    /** Schedule the next frame once, while on screen. */
    request() {
        if (this.frame === undefined && this.isVisible) {
            this.frame = requestAnimationFrame((now) => this.render(now));
        }
    }

    /** Measure where the canvas sits so frames find its part on screen without a layout. */
    measure() {
        this.isFixed = getComputedStyle(this.canvas).position === "fixed";
        this.offset = this.canvas.getBoundingClientRect().top + (this.isFixed ? 0 : window.scrollY);
    }

    /** Pause while off screen, and pick up again once back. */
    show(isVisible: boolean) {
        this.isVisible = isVisible;
        if (isVisible) {
            this.measure();
            this.request();
        } else {
            this.stop();
        }
    }

    /** Cancel the pending frame. */
    stop() {
        if (this.frame !== undefined) {
            cancelAnimationFrame(this.frame);
            this.frame = undefined;
        }
    }

    /** Stop drawing and release the size observer, the scroll listener and the drawing context. */
    dispose() {
        // stop drawing and following the page before dropping the context
        this.stop();
        this.sizes.disconnect();
        window.removeEventListener("scroll", this.onScroll);
        this.context.getExtension("WEBGL_lose_context")?.loseContext();
    }

    /** Draw one frame with the uniforms `onDraw` uploads, and schedule the next while it keeps animating. */
    render(now: number) {
        // clear the pending frame, record it for the governor, and skip it off the shared grid of a slower pace
        this.frame = undefined;
        const context = this.context;
        governor.observe(now);
        if (this.pace > 0 && Math.floor(now / this.pace) === Math.floor(this.drawnAt / this.pace)) {
            this.request();
            return;
        }
        this.drawnAt = now;

        // match the backing store to the displayed size at the governed density
        const scale = Math.min(window.devicePixelRatio, this.density) * governor.density;
        const width = Math.round(this.width * scale);
        const height = Math.round(this.height * scale);
        if (this.canvas.width !== width || this.canvas.height !== height) {
            this.canvas.width = width;
            this.canvas.height = height;
        }
        context.viewport(0, 0, width, height);
        context.uniform2f(this.uniform("resolution"), width, height);
        context.uniform1f(this.uniform("scale"), scale);

        // upload the frame and clear the canvas before shading the part of its clip on screen
        const isMoving = this.onDraw(now);
        context.clearColor(0, 0, 0, 0);
        context.clear(context.COLOR_BUFFER_BIT);
        const clip = this.clip ?? { left: 0, top: 0, width: this.width, height: this.height };
        const onScreen = this.isFixed ? this.offset : this.offset - pageScroll.y;
        const top = Math.max(clip.top, -onScreen);
        const bottom = Math.min(clip.top + clip.height, window.innerHeight - onScreen);
        this.isPartial = top > clip.top || bottom < clip.top + clip.height;
        if (bottom > top) {
            context.enable(context.SCISSOR_TEST);
            context.scissor(
                Math.floor(clip.left * scale),
                Math.floor(height - bottom * scale),
                Math.ceil(clip.width * scale),
                Math.ceil((bottom - top) * scale),
            );
            context.drawArrays(context.TRIANGLE_STRIP, 0, 4);
            context.disable(context.SCISSOR_TEST);
        }

        // keep animating while the draw asks for more
        if (isMoving) {
            this.request();
        }
    }
}

/** Compile and link a fragment shader with the shared vertex stage. */
function createProgram(context: WebGLRenderingContext, fragmentSource: string): WebGLProgram {
    // create the empty program
    const program = context.createProgram();

    // compile both stages, failing loudly on driver errors
    const stages: readonly (readonly [kind: number, source: string])[] = [
        [context.VERTEX_SHADER, vertexSource],
        [context.FRAGMENT_SHADER, fragmentSource],
    ];
    for (const [kind, source] of stages) {
        const shader = context.createShader(kind);
        if (shader === null) {
            throw new Error("could not create shader");
        }
        context.shaderSource(shader, source);
        context.compileShader(shader);
        if (context.getShaderParameter(shader, context.COMPILE_STATUS) !== true) {
            throw new Error(`shader failed: ${context.getShaderInfoLog(shader)}`);
        }
        context.attachShader(program, shader);
    }

    // link, then blend premultiplied output over the page
    context.linkProgram(program);
    if (context.getProgramParameter(program, context.LINK_STATUS) !== true) {
        throw new Error(`program failed: ${context.getProgramInfoLog(program)}`);
    }
    context.useProgram(program);
    context.enable(context.BLEND);
    context.blendFunc(context.ONE, context.ONE_MINUS_SRC_ALPHA);

    return program;
}

/** Return whether the page is drawn dark, by its chosen theme or else the system's. */
export function isDarkPage() {
    const theme = document.documentElement.dataset["theme"];

    return (
        theme === "dark" ||
        (theme === undefined && window.matchMedia("(prefers-color-scheme: dark)").matches)
    );
}

/** The page's scroll offset in CSS pixels, kept up to date by scroll events, so frame loops never ask the page for it and force a layout. */
export const pageScroll = { x: 0, y: 0 };

// follow the page's scroll once for every frame loop
if (typeof window !== "undefined") {
    const track = () => {
        pageScroll.x = window.scrollX;
        pageScroll.y = window.scrollY;
    };
    track();
    window.addEventListener("scroll", track, { passive: true });
}
