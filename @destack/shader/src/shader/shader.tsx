import * as style from "@destack/style";
import {
    createEffect,
    createSignal,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
} from "@destack/view";
import type { ComponentConfig } from "shaders/core";
import {
    createShader,
    isWebGPUSupported,
    type ShaderInstance,
    type ShaderOptions,
} from "shaders/js";
import { LayerContext } from "./context.ts";
import { definitionsOf, type Layer, LayerList, structureOf } from "./layer.ts";

/** The device preferences a theme's colors follow. */
const THEME_QUERIES = ["(prefers-color-scheme: dark)", "(prefers-contrast: more)"];

/** The styles of a shader's surface. */
const styles = style.create({
    shader: {
        position: "relative",
        overflow: "hidden",
    },
    canvas: {
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
    },
});

/** The properties of a shader, the native element's attributes included. */
export interface ShaderProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The effects drawn, as layers from bottom to top. */
    readonly children?: JSX.Element;
    /** What shows where the device draws no GPU effects, such as a gradient image. */
    readonly fallback?: JSX.Element;
    /** The color space the effects blend in, linear Display P3 by default. */
    readonly colorSpace?: ShaderOptions["colorSpace"];
    /** The curve that maps the effects' light onto the screen, linear by default. */
    readonly toneMapping?: ShaderOptions["toneMapping"];
    /** Whether the shader follows its element's size and pauses off screen, true by default. */
    readonly observeElement?: boolean;
    /** Handle the shader drawing its first frame. */
    readonly onReady?: () => void;
    /** Handle the shader giving up for good, with the reason the renderer reports. */
    readonly onFailure?: (reason: string) => void;
    /** The StyleX styles applied after the shader's styles. */
    readonly xstyle?: style.Styles;
}

/** Draw effects as layers on a GPU canvas, coloured by theme tokens, still under reduced motion, and its fallback where the GPU draws nothing. */
export function Shader(properties: ShaderProperties): JSX.Element {
    // collect the layers and keep the canvas, the running shader and whether it failed
    const layers = new LayerList();
    const rest = omit(
        properties,
        "children",
        "fallback",
        "colorSpace",
        "toneMapping",
        "observeElement",
        "onReady",
        "onFailure",
        "xstyle",
        "style",
    );
    const [canvas, setCanvas] = createSignal<HTMLCanvasElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const [failure, setFailure] = createSignal<string | undefined>(undefined, {
        ownedWrite: true,
    });
    const [colors, setColors] = createSignal(0, { ownedWrite: true });
    let running: ShaderInstance | undefined;
    const fail = (reason: string): void => {
        setFailure(reason);
        properties.onFailure?.(reason);
    };

    // build the shader again whenever a layer joins, leaves or moves, or a rendering option changes
    createEffect(
        () => ({
            element: canvas(),
            structure: structureOf(layers.layers()),
            options: optionsOf(properties),
        }),
        ({ element, options }) => {
            if (element === undefined) {
                return undefined;
            }
            if (!isWebGPUSupported()) {
                fail("unsupported");

                return undefined;
            }
            let isCurrent = true;
            const preset = { components: untrack(() => configsOf(layers.layers(), element)) };
            void createShader(element, preset, {
                ...options,
                components: untrack(() => definitionsOf(layers.layers())),
                disableTelemetry: true,
                onReady: () => properties.onReady?.(),
                onError: (reason) => running?.getFailureReason() !== null && fail(reason),
            }).then(
                (created) => {
                    // keep the shader unless a newer build replaced it, still under reduced motion
                    if (!isCurrent) {
                        created.destroy();
                        return;
                    }
                    running = created;
                    if (prefersReducedMotion()) {
                        created.pause();
                    }
                },
                (error: unknown) => fail(error instanceof Error ? error.message : "init-failed"),
            );

            return () => {
                isCurrent = false;
                running?.destroy();
                running = undefined;
            };
        },
    );

    // update a layer in place as its properties or the theme's colors change
    createEffect(
        () => {
            colors();

            return flatten(layers.layers()).map((layer) => ({
                id: layer.id,
                values: layer.properties(),
            }));
        },
        (updates) => {
            const element = canvas();
            for (const update of updates) {
                if (element !== undefined) {
                    running?.update(update.id, resolved(update.values, element));
                }
            }
        },
    );
    onCleanup(() => running?.destroy());

    // resolve the colors again when the device's scheme or contrast or a theme root above the canvas changes
    createEffect(canvas, (element) => {
        if (element === undefined) {
            return undefined;
        }
        const follow = (): void => {
            setColors((count) => count + 1);
        };
        const queries =
            typeof matchMedia === "function" ? THEME_QUERIES.map((query) => matchMedia(query)) : [];
        for (const query of queries) {
            query.addEventListener("change", follow);
        }
        const observer = new MutationObserver(follow);
        for (let root = element.parentElement; root !== null; root = root.parentElement) {
            observer.observe(root, { attributes: true, attributeFilter: ["style", "class"] });
        }

        return () => {
            observer.disconnect();
            for (const query of queries) {
                query.removeEventListener("change", follow);
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
            <LayerContext value={layers}>{properties.children}</LayerContext>
        </div>
    );
}

/** Read the rendering options a shader passes on, leaving out the ones it does not set. */
function optionsOf(properties: ShaderProperties): ShaderOptions {
    return {
        ...(properties.colorSpace === undefined ? {} : { colorSpace: properties.colorSpace }),
        ...(properties.toneMapping === undefined ? {} : { toneMapping: properties.toneMapping }),
        ...(properties.observeElement === undefined
            ? {}
            : { observeElement: properties.observeElement }),
    };
}

/** Write layers as the renderer's components, theme tokens resolved against the canvas. */
export function configsOf(layers: readonly Layer[], element: HTMLElement): ComponentConfig[] {
    return layers.map((layer) => ({
        type: layer.type,
        id: layer.id,
        props: resolved(layer.properties(), element),
        children: configsOf(layer.layers(), element),
    }));
}

/** List a layer tree's layers, each before the layers it wraps. */
function flatten(layers: readonly Layer[]): Layer[] {
    return layers.flatMap((layer) => [layer, ...flatten(layer.layers())]);
}

/** Resolve the theme tokens among properties, such as `var(--destack-color-primary)`, to the hex colors the element computes in its color scheme. */
function resolved(
    values: Readonly<Record<string, unknown>>,
    element: HTMLElement,
): Record<string, unknown> {
    const resolve = (value: unknown): unknown => {
        // compute a token's color, and the tokens inside lists and objects
        if (typeof value === "string") {
            return value.trim().startsWith("var(") ? colorOf(value, element) : value;
        }
        if (Array.isArray(value)) {
            return value.map(resolve);
        }
        if (typeof value === "object" && value !== null) {
            return Object.fromEntries(
                Object.entries(value).map(([key, entry]) => [key, resolve(entry)]),
            );
        }

        return value;
    };

    return Object.fromEntries(Object.entries(values).map(([key, value]) => [key, resolve(value)]));
}

/** Compute a CSS color the way the element would draw it, as six-digit hex, light or dark as its scheme picks. */
function colorOf(value: string, element: HTMLElement): string {
    // let the browser resolve the color on a probe inside the element
    const probe = document.createElement("span");
    probe.style.color = value;
    element.append(probe);
    const computed = getComputedStyle(probe).color;
    probe.remove();

    // read rgb() channels, paint any other color syntax such as oklab() to read its pixel
    const written = /^rgba?\((\d+)[,\s]+(\d+)[,\s]+(\d+)/u.exec(computed);
    const channels = written === null ? paintedOf(computed) : written.slice(1, 4).map(Number);

    return channels === undefined
        ? computed
        : `#${channels.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
}

/** Paint a CSS color on one pixel and read its sRGB channels, absent where no canvas draws. */
function paintedOf(color: string): number[] | undefined {
    // paint the color on a one-pixel canvas
    const context = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
    if (context === null) {
        return undefined;
    }
    context.fillStyle = color;
    context.fillRect(0, 0, 1, 1);

    return Array.from(context.getImageData(0, 0, 1, 1).data.subarray(0, 3));
}

/** Report whether the person asked the system for reduced motion. */
function prefersReducedMotion(): boolean {
    return (
        typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches
    );
}
