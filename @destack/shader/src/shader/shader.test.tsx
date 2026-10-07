import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { draw } from "@destack/view/test";
import {
    Blur,
    Circle,
    defineEffect,
    defineShader,
    LinearGradient,
    transformColor,
    wgsl,
} from "../effect/index.ts";
import { LayerContext } from "./context.ts";
import { definitionsOf, LayerList } from "./layer.ts";
import { configsOf, Shader } from "./shader.tsx";

test("show the fallback and report why where the device draws no GPU effects", () => {
    const failures: string[] = [];
    const container = draw(() => (
        <Shader
            fallback={<img alt="" src="/gradient.png" />}
            onFailure={(reason) => failures.push(reason)}
        >
            <LinearGradient colorA="#0f172a" colorB="#7c3aed" />
        </Shader>
    ));
    flush();
    expect([
        container.querySelector("[data-slot=shader]")?.getAttribute("data-state"),
        container.querySelector("canvas"),
        container.querySelector("img")?.getAttribute("src"),
        failures,
    ]).toEqual(["fallback", null, "/gradient.png", ["unsupported"]]);
});

test("write the effects as the renderer's layers in order, wrapped layers nested and theme tokens resolved", () => {
    const layers = new LayerList();
    const container = draw(() => (
        <LayerContext value={layers}>
            <LinearGradient colorA="var(--accent)" colorB="#7c3aed" />
            <Blur intensity={20}>
                <Circle color="#ffffff" />
            </Blur>
        </LayerContext>
    ));
    container.style.setProperty("--accent", "rgb(62, 99, 221)");
    const written = configsOf(layers.layers(), container).map((layer) => ({
        type: layer.type,
        props: layer.props,
        children: layer.children?.map((child) => [child.type, child.props]),
    }));
    expect(written).toEqual([
        { type: "LinearGradient", props: { colorA: "#3e63dd", colorB: "#7c3aed" }, children: [] },
        { type: "Blur", props: { intensity: 20 }, children: [["Circle", { color: "#ffffff" }]] },
    ]);
});

test("refuse an effect outside a shader", () => {
    expect(() => draw(() => <Circle />)).toThrow("the Circle effect needs a shader around it");
});

test("draw a custom effect under its definition's name, handing each definition to the renderer once", () => {
    // define a flat fill and draw it twice, once inside a blur
    const fill = defineShader({
        name: "Fill",
        props: { tint: { default: "#000000", transform: transformColor } },
        paint: wgsl`return tint;`,
    });
    const Fill = defineEffect(fill);
    const layers = new LayerList();
    const container = draw(() => (
        <LayerContext value={layers}>
            <Fill tint="#ff0000" />
            <Blur intensity={4}>
                <Fill tint="#00ff00" />
            </Blur>
        </LayerContext>
    ));
    const written = configsOf(layers.layers(), container).map((layer) => [
        layer.type,
        layer.children?.map((child) => child.type),
    ]);
    expect([written, definitionsOf(layers.layers())]).toEqual([
        [
            ["Fill", []],
            ["Blur", ["Fill"]],
        ],
        [fill],
    ]);
});
