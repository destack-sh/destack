import * as style from "@destack/style";
import {
    createEffect,
    createMemo,
    createSignal,
    type JSX,
    omit,
    Show,
    untrack,
} from "@destack/view";
import { createMediaQuery } from "@destack/view/primitives/media";
import { createMutationObserver } from "@destack/view/primitives/mutation-observer";
import { createReducedMotion } from "@destack/view/motion";
import {
    ShaderError,
    type ShaderFailure,
    ShaderMount,
    type ShaderResolution,
} from "../mount/mount.ts";
import { resolveValues, type ShaderValues } from "./value.ts";

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
    /** The GLSL ES 3.0 fragment shader, which reads the vertex shader's coordinates and the uniforms. */
    readonly fragmentShader: string;
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
    /** Receive each mount the shader builds, to read its animation time. */
    readonly onMount?: (mount: ShaderMount) => void;
    /** The StyleX styles applied after the shader's styles. */
    readonly xstyle?: style.Styles;
    /** The inline style applied last. */
    readonly style?: JSX.CSSProperties | string;
}

/** Draw a fragment shader behind its content, colored by theme tokens and held still under the theme's reduced motion. */
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

    // follow the theme: the device preferences it reads and the theme roots above the canvas
    const preferences = THEME_QUERIES.map((query) => createMediaQuery(query));
    createMutationObserver(
        () => ancestorsOf(canvas()),
        { attributes: true, attributeFilter: ["style", "class"] },
        () => setMutations((count) => count + 1),
    );
    const theme = createMemo(() => [mutations(), ...preferences.map((matches) => matches())]);

    // hold still where the theme's motion scale, the person's motion setting, reads zero
    const isStill = createReducedMotion(canvas);

    // build the mount once the canvas exists, and again for a new fragment shader
    createEffect(
        () => ({ element: canvas(), fragment: properties.fragmentShader }),
        ({ element, fragment }) => {
            if (element === undefined) {
                return undefined;
            }
            // build from the uniforms of the moment, which the next effect keeps current
            let isCurrent = true;
            void resolveValues(
                untrack(() => properties.uniforms),
                element,
            ).then((uniforms) => {
                // drop a build a newer one replaced
                if (!isCurrent) {
                    return;
                }
                try {
                    const built = new ShaderMount(element, fragment, uniforms, {
                        speed: isStill() ? 0 : (properties.speed ?? 0),
                        frame: properties.frame ?? 0,
                        minPixelRatio: properties.minPixelRatio,
                        maxPixelCount: properties.maxPixelCount,
                        resolution: properties.resolution,
                        mipmaps: properties.mipmaps,
                    });
                    setMount(built);
                    properties.onMount?.(built);
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

/** List an element's ancestors, the theme roots that may set its colors and motion. */
function ancestorsOf(element: Element | undefined): Element[] {
    const ancestors: Element[] = [];
    for (let root = element?.parentElement ?? null; root !== null; root = root.parentElement) {
        ancestors.push(root);
    }

    return ancestors;
}
