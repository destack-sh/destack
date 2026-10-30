import { testSimulation } from "@destack/test/world";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { expect, onTestFinished, test, vi } from "@destack/test";
import { render } from "@destack/view/test";
import { ShaderMount } from "../mount/mount.ts";
import { Shader } from "./shader.tsx";

test("show the fallback and report why where the browser draws no WebGL 2", async () => {
    await using simulation = testSimulation();
    const failures: (readonly [string, string, string])[] = [];
    const { container } = render(
        () => (
            <Shader
                fragmentShader="#version 300 es"
                uniforms={{ u_size: 1 }}
                fallback={<img alt="" src="/still.png" />}
                onFailure={(failure) =>
                    failures.push([failure.code, failure.reason, failure.message])
                }
            />
        ),
        { environment: new BrowserSimulation(simulation).environment("browser") },
    );

    // the canvas gives way to the fallback once the mount finds no WebGL 2 context
    await expect
        .poll(() => failures)
        .toEqual([
            ["PRECONDITION_FAILED", "WEBGL_UNSUPPORTED", "this browser does not support WebGL 2"],
        ]);
    expect([
        container.querySelector("[data-slot=shader]")?.getAttribute("data-state"),
        container.querySelector("canvas"),
        container.querySelector("img")?.getAttribute("src"),
    ]).toEqual(["fallback", null, "/still.png"]);
});

/** Stand in for a WebGL 2 context. */
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
    // draw on a stand-in context in a world, recording the speeds the mount takes
    await using simulation = testSimulation();
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
    const { container } = render(
        () => (
            <Shader
                fragmentShader="#version 300 es"
                uniforms={{}}
                speed={1}
                onMount={(mount) => mounts.push(mount)}
            />
        ),
        { environment: new BrowserSimulation(simulation).environment("browser") },
    );
    await expect.poll(() => mounts.length).toBe(1);
    const running = speed.mock.calls.at(-1);

    // turn the person's motion off and change the theme root, the test DOM resolving only the canvas's own custom properties
    container.querySelector("canvas")?.style.setProperty("--destack-motion-scale", "0");
    container.className = "reduced";
    await simulation.until(() => speed.mock.calls.at(-1)?.[0] === 0);

    expect([mounts[0]?.canvas === container.querySelector("canvas"), running]).toEqual([true, [1]]);
});
