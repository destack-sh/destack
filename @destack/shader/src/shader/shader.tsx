import * as style from "@destack/style";
import type { Environment } from "@destack/environment";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    isServer,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useEnvironment,
} from "@destack/view";
import { createMediaQuery } from "@destack/view/primitives/media";
import { createMutationObserver } from "@destack/view/primitives/mutation-observer";
import { createReducedMotion } from "@destack/view/motion";
import { ShaderMount, type ShaderResolution } from "../mount/mount.ts";
import { type ColorCache, resolveValues, type ShaderValues } from "./value.ts";
import { ShaderError, type ShaderFailure } from "../error/index.ts";

/** The device preferences a theme's colors follow. */
const THEME_QUERIES = ["(prefers-color-scheme: dark)", "(prefers-contrast: more)"];

/** The styles of a shader's surface. */
const styles = style.create({
    shader: {
        isolation: "isolate",
        position: "relative",
    },
    canvas: {
        borderRadius: "inherit",
        contain: "strict",
        display: "block",
        height: "100%",
        inset: 0,
        position: "absolute",
        width: "100%",
        zIndex: -1,
    },
});

/** The properties of a shader, the native element's attributes included. */
export interface ShaderProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style"
> {
    /** The GLSL ES 3.0 fragment shader. */
    readonly fragmentShader: string;
    /** The uniforms by GLSL name: numbers, vectors, CSS colors and theme tokens, or images. */
    readonly uniforms: ShaderValues;
    /** The speed of animation time, 0 to hold one frame; held still under reduced motion. */
    readonly speed?: number;
    /** The animation time to draw, in milliseconds. */
    readonly frame?: number;
    /** The least pixel ratio to render at, 2 by default. */
    readonly minPixelRatio?: number;
    /** The most device pixels to render. */
    readonly maxPixelCount?: number;
    /** Whether slow frames lower the resolution, adaptive by default. */
    readonly resolution?: ShaderResolution;
    /** The image uniforms that get mipmaps. */
    readonly mipmaps?: readonly string[];
    /** The content drawn over the shader. */
    readonly children?: JSX.Element;
    /** What shows where the shader cannot draw, such as a still image. */
    readonly fallback?: JSX.Element;
    /** Handle the shader failing to draw. */
    readonly onFailure?: (failure: ShaderFailure) => void;
    /** Receive each mount the shader builds, to read its animation time. */
    readonly onMount?: (mount: ShaderMount) => void;
    /** The StyleX styles applied after the shader's styles. */
    readonly xstyle?: style.Styles;
    /** The inline style applied last. */
    readonly style?: JSX.CSSProperties | string;
}

/** Draw a fragment shader behind its content, colored by theme tokens. */
export function Shader(properties: ShaderProperties): JSX.Element {
    // hold the canvas, the running mount, the failure and the device preferences
    const rest = omit(
        properties,
        "fragmentShader",
        "uniforms",
        "speed",
        "frame",
        "minPixelRatio",
        "maxPixelCount",
        "resolution",
        "mipmaps",
        "children",
        "fallback",
        "onFailure",
        "onMount",
        "xstyle",
        "style",
    );
    const [canvas, setCanvas] = createSignal<HTMLCanvasElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const [mount, setMount] = createSignal<ShaderMount | undefined>(undefined, {
        ownedWrite: true,
    });
    const [failure, setFailure] = createSignal<ShaderFailure | undefined>(undefined, {
        ownedWrite: true,
    });
    const [mutations, setMutations] = createSignal(0, { ownedWrite: true });

    // stop the theme reads with the shader
    const stopping = new AbortController();
    onCleanup(() => stopping.abort());

    // follow the theme in the page alone, where the browser's clock and display run
    const environment = isServer ? undefined : useEnvironment();
    const preferences = THEME_QUERIES.map((query) => createMediaQuery(query));
    if (environment !== undefined) {
        followTheme(canvas, environment, stopping.signal, () => setMutations((count) => count + 1));
    }
    const theme = createMemo(() => [mutations(), ...preferences.map((matches) => matches())]);

    // hold still where the theme's motion scale, the person's motion setting, reads zero
    const isStill = createReducedMotion(canvas);

    // keep the colors computed for the current theme, so animated values skip the style pass
    let colors: ColorCache = new Map();
    let colorTheme: unknown;

    // build the mount once the canvas exists in the page, and again for a new fragment shader
    createEffect(
        () => ({ element: canvas(), fragment: properties.fragmentShader }),
        ({ element, fragment }) => {
            if (element === undefined || environment === undefined) {
                return undefined;
            }
            // build from the uniforms of the moment, which the next effect keeps current
            let isCurrent = true;
            void resolveValues(
                untrack(() => properties.uniforms),
                element,
                colors,
            ).then((uniforms) => {
                // drop a build a newer one replaced
                if (!isCurrent) {
                    return;
                }
                try {
                    const built = new ShaderMount(
                        element,
                        fragment,
                        uniforms,
                        {
                            speed: isStill() ? 0 : (properties.speed ?? 0),
                            frame: properties.frame ?? 0,
                            minPixelRatio: properties.minPixelRatio,
                            maxPixelCount: properties.maxPixelCount,
                            resolution: properties.resolution,
                            mipmaps: properties.mipmaps,
                        },
                        environment.display,
                    );
                    setMount(built);
                    properties.onMount?.(built);
                } catch (error) {
                    // show the fallback where this browser cannot draw the shader
                    if (
                        ShaderError.is(error, "WEBGL_UNSUPPORTED") ||
                        ShaderError.is(error, "COMPILE_FAILED") ||
                        ShaderError.is(error, "LINK_FAILED")
                    ) {
                        setFailure(error);
                        properties.onFailure?.(error);
                    }
                    // rethrow every other failure
                    else {
                        throw error;
                    }
                }
            });

            return () => {
                isCurrent = false;
                mount()?.dispose();
                setMount(undefined);
            };
        },
    );

    // set the uniforms again as they or the theme's colors change, keeping only the newest
    createEffect(
        () => ({ running: mount(), uniforms: properties.uniforms, theme: theme() }),
        ({ running, uniforms, theme: current }) => {
            if (running === undefined) {
                return undefined;
            }

            // compute the colors again only once the theme changes, as the values change every frame
            if (current !== colorTheme) {
                colors = new Map();
                colorTheme = current;
            }
            let isCurrent = true;
            void resolveValues(uniforms, running.canvas, colors).then((resolved) => {
                if (isCurrent) {
                    running.setUniforms(resolved);
                }
            });

            return () => {
                isCurrent = false;
            };
        },
    );

    // follow the speed, the frame and the resolution limits
    createEffect(
        () => ({ running: mount(), speed: isStill() ? 0 : (properties.speed ?? 0) }),
        ({ running, speed }) => running?.setSpeed(speed),
    );
    createEffect(
        () => ({ running: mount(), frame: properties.frame }),
        ({ running, frame }) => {
            if (frame !== undefined) {
                running?.setFrame(frame);
            }
        },
    );
    createEffect(
        () => ({ running: mount(), limit: properties.maxPixelCount }),
        ({ running, limit }) => {
            if (limit !== undefined) {
                running?.setMaxPixelCount(limit);
            }
        },
    );
    createEffect(
        () => ({ running: mount(), ratio: properties.minPixelRatio }),
        ({ running, ratio }) => {
            if (ratio !== undefined) {
                running?.setMinPixelRatio(ratio);
            }
        },
    );

    return (
        <div
            data-slot="shader"
            data-state={failure() === undefined ? "drawing" : "fallback"}
            {...rest}
            {...style.attributes([styles.shader, properties.xstyle], properties.style)}
        >
            <Show when={failure() === undefined} fallback={properties.fallback}>
                <canvas
                    data-slot="shader-canvas"
                    aria-hidden="true"
                    ref={(element) => setCanvas(element)}
                    {...style.attrs(styles.canvas)}
                />
            </Show>
            {properties.children}
        </div>
    );
}

/** Count each settled burst of changes above the canvas, once the browser painted it. */
function followTheme(
    canvas: Accessor<HTMLCanvasElement | undefined>,
    environment: Pick<Environment<"display">, "clock" | "display">,
    signal: AbortSignal,
    onSettle: () => void,
): void {
    // read the theme again once a burst has painted, when styles are settled
    const { clock, display } = environment;
    let isSettling = false;
    const settle = () => {
        if (isSettling) {
            return;
        }
        isSettling = true;
        void display
            .frame(signal)
            .then(() => clock.sleep(0, signal))
            .then(
                () => {
                    isSettling = false;
                    onSettle();
                },
                (error: unknown) => {
                    if (!signal.aborted) {
                        throw error;
                    }
                },
            );
    };

    // watch the theme roots above the canvas
    createMutationObserver(
        () => ancestorsOf(canvas()),
        { attributes: true, attributeFilter: ["style", "class"] },
        settle,
    );
}

/** List an element's ancestors, the theme roots that may set its colors and motion. */
function ancestorsOf(element: Element | undefined): Element[] {
    const ancestors: Element[] = [];
    for (let root = element?.parentElement ?? null; root !== null; root = root.parentElement) {
        ancestors.push(root);
    }

    return ancestors;
}
