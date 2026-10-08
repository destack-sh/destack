import { VERTEX_SHADER } from "./vertex.ts";

/** The most device pixels a shader renders: a 4K screen at twice the density on each side. */
const MAX_PIXEL_COUNT = 1920 * 1080 * 4;

/** The least pixel ratio a shader renders at, twice the CSS pixels so edges stay smooth on 1x screens. */
const MIN_PIXEL_RATIO = 2;

/** The density factors adaptive resolution steps through, from full to coarsest. */
const DENSITY_STEPS = [1, 0.75, 0.5, 0.35];

/** The frame intervals the density judges at once. */
const JUDGED_FRAMES = 20;

/** The median frame interval above which the density coarsens, in milliseconds: below 42 frames a second. */
const SLOW_INTERVAL = 24;

/** The median frame interval below which a judged window counts as smooth, in milliseconds: 55 frames a second. */
const SMOOTH_INTERVAL = 18;

/** The smooth windows in a row the density waits for before it sharpens again. */
const SMOOTH_WINDOWS = 4;

/** The longest gap between frames that counts as a frame interval, in milliseconds; longer gaps are pauses. */
const PAUSE_INTERVAL = 250;

/** The corners of the two triangles that cover the canvas, in clip space. */
const SQUARE = new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]);

/** The bits of floating-point precision the effects need; devices whose medium precision is lower get high precision. */
const FLOAT_PRECISION = 23;

/** A value a shader uniform takes: a float, a boolean, a vector or matrix, a list of vectors, or an image texture. */
export type ShaderUniform =
    | number
    | boolean
    | readonly number[]
    | readonly (readonly number[])[]
    | HTMLImageElement;

/** The uniforms a fragment shader reads, by their GLSL names. */
export type ShaderUniforms = Readonly<Record<string, ShaderUniform>>;

/** Why a shader cannot draw: no WebGL 2 context, or a shader that does not compile or link. */
export type ShaderFailure = "unsupported" | "compile" | "link";

/** How a shader picks its resolution: lower under slow frames, or fixed at its pixel ratio. */
export type ShaderResolution = "adaptive" | "fixed";

/** The motion and resolution a shader starts with. */
export interface ShaderMountOptions {
    /** The speed of animation time, 0 to hold one frame, negative to play backwards; 0 by default. */
    readonly speed?: number;
    /** The animation time to start at, in milliseconds, for a repeatable picture; 0 by default. */
    readonly frame?: number;
    /** The least pixel ratio to render at, 2 by default. */
    readonly minPixelRatio?: number | undefined;
    /** The most device pixels to render, a 4K screen at 2x by default. */
    readonly maxPixelCount?: number | undefined;
    /** Whether slow frames lower the resolution, adaptive by default. */
    readonly resolution?: ShaderResolution | undefined;
    /** The image uniforms that get mipmaps, for images drawn smaller than their size. */
    readonly mipmaps?: readonly string[] | undefined;
    /** The attributes of the WebGL 2 context. */
    readonly contextAttributes?: WebGLContextAttributes;
}

/** A shader that cannot draw, and why. */
export class ShaderError extends Error {
    /** Why the shader cannot draw. */
    readonly reason: ShaderFailure;

    /** Hold the reason and the driver's log. */
    constructor(reason: ShaderFailure, message: string) {
        super(message);
        this.name = "ShaderError";
        this.reason = reason;
    }
}

/** The density every shader on the page renders at, lowered while frames run long and raised once they run smooth. */
export class Density {
    /** The index into the density steps. */
    #step = 0;
    /** The frame time last observed, in milliseconds. */
    #observedAt = Number.NEGATIVE_INFINITY;
    /** The frame intervals of the window being judged. */
    #intervals: number[] = [];
    /** The smooth windows judged in a row. */
    #smooth = 0;

    /** The factor every adaptive shader multiplies its pixel count by. */
    get factor(): number {
        return DENSITY_STEPS[this.#step] ?? 1;
    }

    /** Start at the coarsest density, where the browser warns that WebGL draws slowly. */
    coarsen(): void {
        this.#step = DENSITY_STEPS.length - 1;
    }

    /** Record one animation frame, once per frame however many shaders draw it, and judge each full window. */
    observe(now: number): void {
        // skip repeats within a frame and the gaps of paused or hidden pages
        if (now === this.#observedAt) {
            return;
        }
        const interval = now - this.#observedAt;
        this.#observedAt = now;
        if (interval > PAUSE_INTERVAL) {
            return;
        }

        // collect a full window
        this.#intervals.push(interval);
        if (this.#intervals.length < JUDGED_FRAMES) {
            return;
        }
        const sorted = this.#intervals.toSorted((left, right) => left - right);
        const median = sorted[JUDGED_FRAMES >> 1] ?? interval;
        this.#intervals = [];

        // coarsen at once on a slow window
        if (median > SLOW_INTERVAL) {
            this.#step = Math.min(this.#step + 1, DENSITY_STEPS.length - 1);
            this.#smooth = 0;
        }
        // sharpen after enough smooth windows in a row
        else if (median < SMOOTH_INTERVAL && this.#step > 0) {
            this.#smooth += 1;
            if (this.#smooth >= SMOOTH_WINDOWS) {
                this.#step -= 1;
                this.#smooth = 0;
            }
        }
    }
}

/** The one density the page's shaders share, since they draw on one GPU. */
export const density = new Density();

/** Draw a fragment shader over a canvas at its device pixels, animating its time while the canvas shows. */
export class ShaderMount {
    /** The canvas drawn on. */
    readonly canvas: HTMLCanvasElement;
    /** The WebGL 2 context. */
    readonly #context: WebGL2RenderingContext;
    /** The linked program. */
    readonly #program: WebGLProgram;
    /** The buffer of the two triangles covering the canvas. */
    readonly #buffer: WebGLBuffer;
    /** The location of each uniform the program reads, null for uniforms the compiler removed. */
    readonly #locations = new Map<string, WebGLUniformLocation | null>();
    /** The last value set on each uniform, so unchanged values skip the upload. */
    readonly #values = new Map<string, unknown>();
    /** The texture of each image uniform. */
    readonly #textures = new Map<string, WebGLTexture>();
    /** The texture unit of each image uniform. */
    readonly #units = new Map<string, number>();
    /** The image uniforms that get mipmaps. */
    readonly #mipmaps: ReadonlySet<string>;
    /** Whether slow frames lower the resolution. */
    readonly #resolution: ShaderResolution;

    /** The pending animation frame, or null when no frame is scheduled. */
    #frameRequest: number | null = null;
    /** The time of the last drawn frame, in milliseconds. */
    #renderedAt = 0;
    /** The animation time played so far, in milliseconds, which the shader reads as seconds. */
    #frame: number;
    /** The speed of animation time as set. */
    #speed = 0;
    /** The speed of animation time now, 0 while the page is hidden or the canvas is off screen. */
    #currentSpeed = 0;
    /** The least pixel ratio to render at. */
    #minPixelRatio: number;
    /** The most device pixels to render. */
    #maxPixelCount: number;
    /** The density factor the canvas was last sized at. */
    #density = 1;
    /** Device pixels per CSS pixel of the drawing buffer. */
    #renderScale = 1;
    /** Whether the drawing buffer changed size since the last frame. */
    #isResized = true;
    /** Whether the canvas shows in the viewport. */
    #isInViewport = true;
    /** Whether the mount was disposed. */
    #isDisposed = false;

    /** The canvas's size in CSS pixels. */
    #size = { width: 0, height: 0 };
    /** The canvas's size in device pixels, where the browser reports it. */
    #devicePixels: { width: number; height: number } | undefined;
    /** The observer of the canvas's size. */
    #resizeObserver: ResizeObserver | undefined;
    /** The observer of the canvas entering and leaving the viewport. */
    #intersectionObserver: IntersectionObserver | undefined;

    /** Compile the fragment shader for the canvas and start drawing, or throw a ShaderError. */
    constructor(
        canvas: HTMLCanvasElement,
        fragment: string,
        uniforms: ShaderUniforms,
        options: ShaderMountOptions = {},
    ) {
        // hold the canvas and options
        this.canvas = canvas;
        this.#frame = options.frame ?? 0;
        this.#minPixelRatio = options.minPixelRatio ?? MIN_PIXEL_RATIO;
        this.#maxPixelCount = options.maxPixelCount ?? MAX_PIXEL_COUNT;
        this.#resolution = options.resolution ?? "adaptive";
        this.#mipmaps = new Set(options.mipmaps);

        // open a WebGL 2 context, refusing a browser without one
        const context = canvas.getContext("webgl2", options.contextAttributes);
        if (context === null) {
            throw new ShaderError("unsupported", "this browser draws no WebGL 2");
        }
        this.#context = context;
        if (this.#resolution === "adaptive" && isSlowGraphics()) {
            density.coarsen();
        }

        // link the program and cover the canvas with two triangles
        this.#program = linkProgram(context, VERTEX_SHADER, fragment);
        context.useProgram(this.#program);
        this.#buffer = context.createBuffer();
        context.bindBuffer(context.ARRAY_BUFFER, this.#buffer);
        context.bufferData(context.ARRAY_BUFFER, SQUARE, context.STATIC_DRAW);
        const position = context.getAttribLocation(this.#program, "a_position");
        context.enableVertexAttribArray(position);
        context.vertexAttribPointer(position, 2, context.FLOAT, false, 0, 0);

        // set the uniforms and follow the canvas's size, zoom and visibility
        this.#upload(uniforms);
        this.#observeSize();
        this.#observeVisibility();
        visualViewport?.addEventListener("resize", this.#restartSizeObserver);
        canvas.ownerDocument.addEventListener("visibilitychange", this.#updateSpeed);

        // start the animation
        this.setSpeed(options.speed ?? 0);
    }

    /** The animation time played so far, in milliseconds. */
    get frame(): number {
        return this.#frame;
    }

    /** Set uniforms that changed and draw a frame. */
    setUniforms(uniforms: ShaderUniforms): void {
        this.#upload(uniforms);
        this.#draw(performance.now());
    }

    /** Set the speed of animation time, 0 to hold the current frame. */
    setSpeed(speed: number): void {
        this.#speed = speed;
        this.#updateSpeed();
    }

    /** Set the animation time in milliseconds and draw it. */
    setFrame(frame: number): void {
        this.#frame = frame;
        this.#renderedAt = performance.now();
        this.#draw(this.#renderedAt);
    }

    /** Set the least pixel ratio and size the canvas again. */
    setMinPixelRatio(minPixelRatio: number): void {
        this.#minPixelRatio = minPixelRatio;
        this.#resize();
    }

    /** Set the most device pixels and size the canvas again. */
    setMaxPixelCount(maxPixelCount: number): void {
        this.#maxPixelCount = maxPixelCount;
        this.#resize();
    }

    /** Stop drawing and release the program, textures and observers. */
    dispose(): void {
        // stop the animation before releasing anything it draws with, once
        if (this.#isDisposed) {
            return;
        }
        this.#isDisposed = true;
        if (this.#frameRequest !== null) {
            cancelAnimationFrame(this.#frameRequest);
            this.#frameRequest = null;
        }

        // release the GPU resources
        for (const texture of this.#textures.values()) {
            this.#context.deleteTexture(texture);
        }
        this.#textures.clear();
        this.#context.deleteBuffer(this.#buffer);
        this.#context.deleteProgram(this.#program);

        // stop following the canvas
        this.#resizeObserver?.disconnect();
        this.#intersectionObserver?.disconnect();
        visualViewport?.removeEventListener("resize", this.#restartSizeObserver);
        this.canvas.ownerDocument.removeEventListener("visibilitychange", this.#updateSpeed);
    }

    /** Follow the canvas's size in CSS pixels and, where the browser reports them, device pixels. */
    #observeSize(): void {
        this.#resizeObserver = new ResizeObserver(([entry]) => {
            // read both sizes of the canvas's box
            const box = entry?.borderBoxSize[0];
            const device = entry?.devicePixelContentBoxSize?.[0];
            if (box !== undefined) {
                this.#size = { width: box.inlineSize, height: box.blockSize };
            }
            if (device !== undefined) {
                this.#devicePixels = { width: device.inlineSize, height: device.blockSize };
            }
            this.#resize();
        });
        this.#resizeObserver.observe(this.canvas);
    }

    /** Observe the size afresh after a zoom, which the resize observer alone may not report. */
    readonly #restartSizeObserver = (): void => {
        this.#resizeObserver?.disconnect();
        this.#observeSize();
    };

    /** Pause the animation while the canvas is off screen. */
    #observeVisibility(): void {
        // follow the canvas's own window, which differs inside frames and picture-in-picture windows
        const view = this.canvas.ownerDocument.defaultView;
        if (view === null) {
            throw new TypeError("the shader's canvas belongs to no window");
        }
        this.#intersectionObserver = new view.IntersectionObserver(([entry]) => {
            this.#isInViewport = entry?.isIntersecting ?? true;
            this.#updateSpeed();
        });
        this.#intersectionObserver.observe(this.canvas);
    }

    /** Size the drawing buffer to the canvas's device pixels, within the pixel ratio, pixel count and density. */
    #resize(): void {
        // aim for at least the physical pixels, plus pinch zoom
        const pixelRatio = Math.max(1, devicePixelRatio);
        const pinch = visualViewport?.scale ?? 1;
        const target = this.#targetPixels(pixelRatio, pinch);

        // cap the pixel count, scaled down by the page's density when adaptive
        this.#density = this.#resolution === "adaptive" ? density.factor : 1;
        const limit = this.#maxPixelCount * this.#density * this.#density;
        const headroom = Math.min(1, Math.sqrt(limit / (target.width * target.height)));
        const width = Math.round(target.width * headroom);
        const height = Math.round(target.height * headroom);
        const renderScale = width / Math.max(1, Math.round(this.#size.width));

        // resize the buffer and draw at once so resizing never flashes
        if (
            this.canvas.width !== width ||
            this.canvas.height !== height ||
            this.#renderScale !== renderScale
        ) {
            this.#renderScale = renderScale;
            this.canvas.width = width;
            this.canvas.height = height;
            this.#isResized = true;
            this.#context.viewport(0, 0, width, height);
            this.#draw(performance.now());
        }
    }

    /** Return the device pixels to render before the pixel count cap. */
    #targetPixels(pixelRatio: number, pinch: number): { width: number; height: number } {
        // use the reported device pixels, raised to the least pixel ratio
        if (this.#devicePixels !== undefined) {
            const raise = Math.max(1, this.#minPixelRatio / pixelRatio) * pinch;

            return {
                width: this.#devicePixels.width * raise,
                height: this.#devicePixels.height * raise,
            };
        }

        // approximate device pixels from the pixel ratio and the page zoom it leaves out
        const zoom = Math.max(1, browserZoom(this.canvas.ownerDocument));
        const scale = Math.max(pixelRatio, this.#minPixelRatio) * pinch * zoom;

        return {
            width: Math.round(this.#size.width) * scale,
            height: Math.round(this.#size.height) * scale,
        };
    }

    /** Apply the set speed, or pause while the page is hidden or the canvas is off screen. */
    readonly #updateSpeed = (): void => {
        // pause while nothing shows
        const isHidden = this.canvas.ownerDocument.hidden || !this.#isInViewport;
        this.#currentSpeed = isHidden ? 0 : this.#speed;

        // start the loop when animation begins
        if (this.#frameRequest === null && this.#currentSpeed !== 0) {
            this.#renderedAt = performance.now();
            this.#frameRequest = requestAnimationFrame(this.#tick);
        }
        // stop the loop when it ends
        else if (this.#frameRequest !== null && this.#currentSpeed === 0) {
            cancelAnimationFrame(this.#frameRequest);
            this.#frameRequest = null;
        }
    };

    /** Run one animation frame: judge it for the page's density, draw it, and schedule the next. */
    readonly #tick = (now: number): void => {
        // let the page's density judge the frame, resizing when it changed
        this.#frameRequest = null;
        if (this.#resolution === "adaptive") {
            density.observe(now);
        }
        if (this.#resolution === "adaptive" && density.factor !== this.#density) {
            this.#resize();
        }

        // draw, and schedule the next frame while animating
        this.#draw(now);
        if (this.#currentSpeed !== 0 && !this.#isDisposed) {
            this.#frameRequest = requestAnimationFrame(this.#tick);
        }
    };

    /** Draw the frame at a time, advancing animation time by the speed. */
    #draw(now: number): void {
        // skip drawing after disposal
        if (this.#isDisposed) {
            return;
        }

        // advance animation time
        this.#frame += (now - this.#renderedAt) * this.#currentSpeed;
        this.#renderedAt = now;

        // set time, and resolution after a resize
        const context = this.#context;
        context.useProgram(this.#program);
        context.uniform1f(this.#location("u_time"), this.#frame * 0.001);
        if (this.#isResized) {
            context.uniform2f(
                this.#location("u_resolution"),
                this.canvas.width,
                this.canvas.height,
            );
            context.uniform1f(this.#location("u_pixelRatio"), this.#renderScale);
            this.#isResized = false;
        }

        // draw the two triangles
        context.clear(context.COLOR_BUFFER_BIT);
        context.drawArrays(context.TRIANGLES, 0, 6);
    }

    /** Return a uniform's location, null where the compiler removed an unused uniform. */
    #location(name: string): WebGLUniformLocation | null {
        // look a location up once
        if (!this.#locations.has(name)) {
            this.#locations.set(name, this.#context.getUniformLocation(this.#program, name));
        }

        return this.#locations.get(name) ?? null;
    }

    /** Upload the uniforms whose values changed. */
    #upload(uniforms: ShaderUniforms): void {
        this.#context.useProgram(this.#program);
        for (const [name, value] of Object.entries(uniforms)) {
            // skip a value set before
            const key = value instanceof HTMLImageElement ? imageKey(value) : value;
            if (isSameValue(this.#values.get(name), key)) {
                continue;
            }
            this.#values.set(name, key);

            // set the uniform by its kind
            const location = this.#location(name);
            if (value instanceof HTMLImageElement) {
                this.#uploadImage(name, value, location);
            } else if (typeof value === "number") {
                this.#context.uniform1f(location, value);
            } else if (typeof value === "boolean") {
                this.#context.uniform1i(location, value ? 1 : 0);
            } else {
                this.#uploadVectors(name, value, location);
            }
        }
    }

    /** Upload a vector, a matrix or a list of same-sized vectors. */
    #uploadVectors(
        name: string,
        value: readonly number[] | readonly (readonly number[])[],
        location: WebGLUniformLocation | null,
    ): void {
        // flatten a list of vectors, all the same size
        const first = value[0];
        const size = Array.isArray(first) ? first.length : value.length;
        const flat = value.flat();
        if (Array.isArray(first) && flat.length !== size * value.length) {
            throw new RangeError(`the vectors of uniform ${name} differ in size`);
        }

        // set it by its vector or matrix size
        const context = this.#context;
        switch (size) {
            case 2:
                return context.uniform2fv(location, flat);
            case 3:
                return context.uniform3fv(location, flat);
            case 4:
                return context.uniform4fv(location, flat);
            case 9:
                return context.uniformMatrix3fv(location, false, flat);
            case 16:
                return context.uniformMatrix4fv(location, false, flat);
            default:
                throw new RangeError(
                    `uniform ${name} has ${size} components, not 2, 3, 4, 9 or 16`,
                );
        }
    }

    /** Upload a loaded image as a texture on its own unit, with its aspect ratio beside it. */
    #uploadImage(
        name: string,
        image: HTMLImageElement,
        location: WebGLUniformLocation | null,
    ): void {
        // refuse an image that has not loaded
        if (!image.complete || image.naturalWidth === 0) {
            throw new TypeError(`the image of uniform ${name} has not loaded`);
        }

        // replace the texture on the uniform's unit
        const context = this.#context;
        const existing = this.#textures.get(name);
        if (existing !== undefined) {
            context.deleteTexture(existing);
        }
        const unit = this.#units.get(name) ?? this.#units.size;
        this.#units.set(name, unit);
        context.activeTexture(context.TEXTURE0 + unit);
        const texture = context.createTexture();
        context.bindTexture(context.TEXTURE_2D, texture);

        // clamp at the edges, filter linearly, and upload
        context.texParameteri(context.TEXTURE_2D, context.TEXTURE_WRAP_S, context.CLAMP_TO_EDGE);
        context.texParameteri(context.TEXTURE_2D, context.TEXTURE_WRAP_T, context.CLAMP_TO_EDGE);
        context.texParameteri(context.TEXTURE_2D, context.TEXTURE_MIN_FILTER, context.LINEAR);
        context.texParameteri(context.TEXTURE_2D, context.TEXTURE_MAG_FILTER, context.LINEAR);
        context.texImage2D(
            context.TEXTURE_2D,
            0,
            context.RGBA,
            context.RGBA,
            context.UNSIGNED_BYTE,
            image,
        );
        if (this.#mipmaps.has(name)) {
            context.generateMipmap(context.TEXTURE_2D);
            context.texParameteri(
                context.TEXTURE_2D,
                context.TEXTURE_MIN_FILTER,
                context.LINEAR_MIPMAP_LINEAR,
            );
        }
        this.#textures.set(name, texture);

        // point the sampler at the unit and set the image's aspect ratio
        context.uniform1i(location, unit);
        context.uniform1f(
            this.#location(`${name}AspectRatio`),
            image.naturalWidth / image.naturalHeight,
        );
    }
}

/** Compile and link a program, raising medium precision to high where the device's medium precision is too low. */
function linkProgram(
    context: WebGL2RenderingContext,
    vertex: string,
    fragment: string,
): WebGLProgram {
    // force high precision where medium precision has fewer bits than the effects need
    const format = context.getShaderPrecisionFormat(context.FRAGMENT_SHADER, context.MEDIUM_FLOAT);
    const isLow = format !== null && format.precision < FLOAT_PRECISION;
    const vertexSource = isLow ? highPrecision(vertex) : vertex;
    const fragmentSource = isLow ? highPrecision(fragment) : fragment;

    // compile both stages and link them
    const program = context.createProgram();
    const stages = [
        compileShader(context, context.VERTEX_SHADER, vertexSource),
        compileShader(context, context.FRAGMENT_SHADER, fragmentSource),
    ];
    for (const stage of stages) {
        context.attachShader(program, stage);
    }
    context.linkProgram(program);

    // refuse a program that does not link
    if (context.getProgramParameter(program, context.LINK_STATUS) !== true) {
        const log = context.getProgramInfoLog(program) ?? "";
        context.deleteProgram(program);
        throw new ShaderError("link", `the shader program does not link: ${log}`);
    }

    // release the stages, which the linked program no longer needs
    for (const stage of stages) {
        context.detachShader(program, stage);
        context.deleteShader(stage);
    }

    return program;
}

/** Compile one shader stage, or throw with the driver's log. */
function compileShader(context: WebGL2RenderingContext, type: number, source: string): WebGLShader {
    // compile the source
    const shader = context.createShader(type);
    if (shader === null) {
        throw new ShaderError("compile", "the context created no shader");
    }
    context.shaderSource(shader, source);
    context.compileShader(shader);

    // refuse a stage that does not compile
    if (context.getShaderParameter(shader, context.COMPILE_STATUS) !== true) {
        const log = context.getShaderInfoLog(shader) ?? "";
        context.deleteShader(shader);
        throw new ShaderError("compile", `the shader does not compile: ${log}`);
    }

    return shader;
}

/** Rewrite a shader's low and medium precision declarations as high precision. */
function highPrecision(source: string): string {
    return source
        .replace(/precision\s+(?:lowp|mediump)\s+float/gu, "precision highp float")
        .replace(/\b(uniform|varying|attribute)\s+(?:lowp|mediump)\s+(\w+)/gu, "$1 highp $2");
}

/** Key an image by its source and size, so the same image is not uploaded twice. */
function imageKey(image: HTMLImageElement): string {
    return `${image.src.slice(0, 200)}|${image.naturalWidth}x${image.naturalHeight}`;
}

/** Compare uniform values, lists element by element. */
function isSameValue(left: unknown, right: unknown): boolean {
    // compare scalars and the same list by identity
    if (left === right) {
        return true;
    }

    // compare lists of equal length element by element
    return (
        Array.isArray(left) &&
        Array.isArray(right) &&
        left.length === right.length &&
        left.every((value: unknown, index) => isSameValue(value, right[index]))
    );
}

/** Whether the browser warns that WebGL draws slowly here, probed once per page. */
let isSlow: boolean | undefined;

/** Report whether the browser warns that WebGL draws slowly here, such as on a software renderer. */
function isSlowGraphics(): boolean {
    // ask once for a context the browser refuses when drawing would be slow, and release it
    if (isSlow === undefined) {
        const probe = document.createElement("canvas");
        const context = probe.getContext("webgl2", { failIfMajorPerformanceCaveat: true });
        context?.getExtension("WEBGL_lose_context")?.loseContext();
        isSlow = context === null;
    }

    return isSlow;
}

/** Estimate the browser zoom from the window's outer width and the viewport, rounded to the browser's zoom steps. */
function browserZoom(document: Document): number {
    // measure the viewport, adding the scrollbar the visual viewport leaves out
    const pinch = visualViewport?.scale ?? 1;
    const viewportWidth = visualViewport?.width ?? innerWidth;
    const scrollbar = innerWidth - document.documentElement.clientWidth;
    const ratio = outerWidth / (pinch * viewportWidth + scrollbar);

    // snap to a zoom step: every 5%, and the thirds
    const percent = Math.round(100 * ratio);
    const thirds: Readonly<Record<number, number>> = { 33: 1 / 3, 67: 2 / 3, 133: 4 / 3 };

    return percent % 5 === 0 ? percent / 100 : (thirds[percent] ?? ratio);
}
