/** The vertex stage shared by every full-canvas shader. */
const vertexSource = `
attribute vec2 position;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}
`;

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
    /** The observer that tracks the displayed size. */
    sizes: ResizeObserver;
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
        this.sizes = new ResizeObserver(([entry]) => {
            this.width = entry.contentRect.width;
            this.height = entry.contentRect.height;
        });
        this.sizes.observe(canvas);
        this.frame = undefined;
        this.isVisible = true;
        this.onDraw = onDraw;
        this.pace = 0;
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

    /** Pause while off screen, and pick up again once back. */
    show(isVisible: boolean) {
        this.isVisible = isVisible;
        if (isVisible) {
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

    /** Stop drawing and release the size observer and the drawing context. */
    dispose() {
        this.stop();
        this.sizes.disconnect();
        this.context.getExtension("WEBGL_lose_context")?.loseContext();
    }

    /** Draw one frame with the uniforms `onDraw` uploads, and schedule the next while it keeps animating. */
    render(now: number) {
        // clear the pending frame, and skip this one when the draw asks for a slower pace
        this.frame = undefined;
        const context = this.context;
        if (now - this.drawnAt < this.pace - 2) {
            this.request();
            return;
        }
        this.drawnAt = now;

        // match the backing store to the displayed size
        const scale = Math.min(window.devicePixelRatio, this.density);
        const width = Math.round(this.width * scale);
        const height = Math.round(this.height * scale);
        if (this.canvas.width !== width || this.canvas.height !== height) {
            this.canvas.width = width;
            this.canvas.height = height;
        }
        context.viewport(0, 0, width, height);
        context.uniform2f(this.uniform("resolution"), width, height);
        context.uniform1f(this.uniform("scale"), scale);

        // upload the frame, then clear and draw the quad
        const isMoving = this.onDraw(now);
        context.clearColor(0, 0, 0, 0);
        context.clear(context.COLOR_BUFFER_BIT);
        context.drawArrays(context.TRIANGLE_STRIP, 0, 4);

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
    for (const [kind, source] of [
        [context.VERTEX_SHADER, vertexSource],
        [context.FRAGMENT_SHADER, fragmentSource],
    ] as const) {
        const shader = context.createShader(kind);
        if (shader === null) {
            throw new Error("could not create shader");
        }
        context.shaderSource(shader, source);
        context.compileShader(shader);
        if (!context.getShaderParameter(shader, context.COMPILE_STATUS)) {
            throw new Error(`shader failed: ${context.getShaderInfoLog(shader)}`);
        }
        context.attachShader(program, shader);
    }

    // link, then blend premultiplied output over the page
    context.linkProgram(program);
    if (!context.getProgramParameter(program, context.LINK_STATUS)) {
        throw new Error(`program failed: ${context.getProgramInfoLog(program)}`);
    }
    context.useProgram(program);
    context.enable(context.BLEND);
    context.blendFunc(context.ONE, context.ONE_MINUS_SRC_ALPHA);

    return program;
}

/** Return whether the page is drawn dark, by its chosen theme or else the system's. */
export function isDarkPage() {
    const theme = document.documentElement.dataset.theme;

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
