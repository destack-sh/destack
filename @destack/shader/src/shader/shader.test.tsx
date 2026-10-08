import { expect, onTestFinished, test, vi } from "@destack/test";
import { render } from "@destack/view/test";
import { type ShaderFailure, ShaderMount } from "../mount/mount.ts";
import { Shader } from "./shader.tsx";

test("show the fallback and report why where the browser draws no WebGL 2", async () => {
    const failures: ShaderFailure[] = [];
    const { container } = render(() => (
        <Shader
            fragmentShader="#version 300 es"
            uniforms={{ u_size: 1 }}
            fallback={<img alt="" src="/still.png" />}
            onFailure={(failure) => failures.push(failure)}
        />
    ));

    // the canvas gives way to the fallback once the mount finds no WebGL 2 context
    await expect.poll(() => failures).toEqual(["unsupported"]);
    expect([
        container.querySelector("[data-slot=shader]")?.getAttribute("data-state"),
        container.querySelector("canvas"),
        container.querySelector("img")?.getAttribute("src"),
    ]).toEqual(["fallback", null, "/still.png"]);
});

/** Stand in for a WebGL 2 context: every call succeeds with another stand-in and every check passes. */
function stubContext(): object {
    return new Proxy(
        {},
        {
            get: (_target, key) =>
                key === "getShaderParameter" || key === "getProgramParameter"
                    ? () => true
                    : stubContext,
        },
    );
}

test("hand each built mount to onMount and hold it still while the theme's motion scale reads zero", async () => {
    // draw on a stand-in context, recording the speeds the mount takes
    const getContext = Object.getOwnPropertyDescriptor(HTMLCanvasElement.prototype, "getContext");
    Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
        configurable: true,
        value: stubContext,
    });
    onTestFinished(() => {
        if (getContext !== undefined) {
            Object.defineProperty(HTMLCanvasElement.prototype, "getContext", getContext);
        }
    });
    vi.stubGlobal("visualViewport", undefined);
    const speed = vi.spyOn(ShaderMount.prototype, "setSpeed");
    const mounts: ShaderMount[] = [];
    const { container } = render(() => (
        <Shader
            fragmentShader="#version 300 es"
            uniforms={{}}
            speed={1}
            onMount={(mount) => mounts.push(mount)}
        />
    ));
    await expect.poll(() => mounts.length).toBe(1);
    const running = speed.mock.calls.at(-1);

    // turn the person's motion off and change the theme root, the test DOM resolving only the canvas's own custom properties
    container.querySelector("canvas")?.style.setProperty("--destack-motion-scale", "0");
    container.className = "reduced";
    await expect.poll(() => speed.mock.calls.at(-1)).toEqual([0]);

    expect([mounts[0]?.canvas === container.querySelector("canvas"), running]).toEqual([true, [1]]);
});
