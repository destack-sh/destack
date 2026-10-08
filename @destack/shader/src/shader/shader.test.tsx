import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import type { ShaderFailure } from "../mount/mount.ts";
import { Shader } from "./shader.tsx";

test("show the fallback and report why where the browser draws no WebGL 2", async () => {
    const failures: ShaderFailure[] = [];
    const container = draw(() => (
        <Shader
            fragment="#version 300 es"
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
