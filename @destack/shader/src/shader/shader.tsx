import * as style from "@destack/style";
import { createEffect, createSignal, type JSX, omit, onCleanup, Show } from "@destack/view";
import {
    ShaderError,
    type ShaderFailure,
    ShaderMount,
    type ShaderResolution,
} from "../mount/mount.ts";
import { resolveValues, type ShaderValues } from "./value.ts";

/** The device preferences a theme's colors follow. */
const THEME_QUERIES = ["(prefers-color-scheme: dark)", "(prefers-contrast: more)"];

/** The device preference that holds animation still. */
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

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
    /** The GLSL ES 3.0 fragment shader, which reads the vertex shader's coordinates and the uniforms. */
    readonly fragment: string;
    /** The uniforms by their GLSL names: numbers, vectors, CSS colors and theme tokens, or images. */
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
    /** The StyleX styles applied after the shader's styles. */
    readonly xstyle?: style.Styles;
    /** The inline style applied last. */
    readonly style?: JSX.CSSProperties | string;
}

/** Draw a fragment shader behind its content, colored by theme tokens and held still under reduced motion. */
export function Shader(properties: ShaderProperties): JSX.Element {
    // hold the canvas, the running mount, the failure and the device preferences
    const rest = omit(
        properties,
        "fragment",
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
    const [theme, setTheme] = createSignal(0, { ownedWrite: true });
    const [isStill, setIsStill] = createSignal(false, { ownedWrite: true });

    // build the mount once the canvas exists, and again for a new fragment shader
    createEffect(
        () => ({ element: canvas(), fragment: properties.fragment }),
        ({ element, fragment }) => {
            if (element === undefined) {
                return undefined;
            }
            let isCurrent = true;
            void resolveValues(properties.uniforms, element).then((uniforms) => {
                // drop a build a newer one replaced
                if (!isCurrent) {
                    return;
                }
                try {
                    setMount(
                        new ShaderMount(element, fragment, uniforms, {
                            speed: isStill() ? 0 : (properties.speed ?? 0),
                            frame: properties.frame ?? 0,
                            minPixelRatio: properties.minPixelRatio,
                            maxPixelCount: properties.maxPixelCount,
                            resolution: properties.resolution,
                            mipmaps: properties.mipmaps,
                        }),
                    );
                } catch (error) {
                    // show the fallback where the shader cannot draw, and rethrow anything else
                    if (!(error instanceof ShaderError)) {
                        throw error;
                    }
                    setFailure(error.reason);
                    properties.onFailure?.(error.reason);
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
        ({ running, uniforms }) => {
            if (running === undefined) {
                return undefined;
            }
            let isCurrent = true;
            void resolveValues(uniforms, running.canvas).then((resolved) => {
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
    onCleanup(() => mount()?.dispose());

    // follow reduced motion and the theme: the device's scheme and contrast, and theme roots above the canvas
    createEffect(canvas, (element) => {
        if (element === undefined) {
            return undefined;
        }
        const still = matchMedia(REDUCED_MOTION_QUERY);
        const followMotion = (): void => {
            setIsStill(still.matches);
        };
        const followTheme = (): void => {
            setTheme((count) => count + 1);
        };
        followMotion();
        still.addEventListener("change", followMotion);
        const queries = THEME_QUERIES.map((query) => matchMedia(query));
        for (const query of queries) {
            query.addEventListener("change", followTheme);
        }
        const observer = new MutationObserver(followTheme);
        for (let root = element.parentElement; root !== null; root = root.parentElement) {
            observer.observe(root, { attributes: true, attributeFilter: ["style", "class"] });
        }

        return () => {
            observer.disconnect();
            still.removeEventListener("change", followMotion);
            for (const query of queries) {
                query.removeEventListener("change", followTheme);
            }
        };
    });

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
