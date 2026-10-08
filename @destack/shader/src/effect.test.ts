import { expect, test } from "@destack/test";
import { readdirSync } from "node:fs";
import { VERTEX_SHADER } from "./mount/vertex.ts";

/** The uniforms the mount sets on every program. */
const MOUNT_UNIFORMS = ["u_time", "u_resolution", "u_pixelRatio"];

/** The directories of the package that hold no effect. */
const CORE = new Set(["mount", "shader", "glsl"]);

/** Read the uniforms a shader declares. */
function declared(source: string): string[] {
    return [...source.matchAll(/uniform\s+(?:(?:lowp|mediump|highp)\s+)?\w+\s+(\w+)/gu)].map(
        (match) => match[1] ?? "",
    );
}

/** Report whether a value is an object of named exports. */
function isRecord(value: unknown): value is Readonly<Record<string, unknown>> {
    return typeof value === "object" && value !== null;
}

/** Report whether a value is an effect's uniform mapping. */
function isMapping(
    value: unknown,
): value is (options: unknown) => Readonly<Record<string, unknown>> {
    return typeof value === "function";
}

test("declare every uniform an effect sets, and set every uniform its fragment shader reads", async () => {
    const effects = readdirSync(import.meta.dirname, { withFileTypes: true })
        .filter((entry) => entry.isDirectory() && !CORE.has(entry.name))
        .map((entry) => entry.name);
    const mismatches: Record<string, { unset: string[]; undeclared: string[] }> = {};
    for (const name of effects) {
        // read the effect's fragment shader, defaults and uniform mapping from its module
        const module: unknown = await import(`./${name}/effect.ts`);
        if (!isRecord(module)) {
            throw new TypeError(`effect ${name} has no module`);
        }
        const fragment = Object.entries(module).find(([key]) => key.endsWith("_FRAGMENT"))?.[1];
        const defaults = Object.entries(module).find(([key]) => key.endsWith("_DEFAULTS"))?.[1];
        const uniforms = Object.entries(module).find(([key]) => key.endsWith("Uniforms"))?.[1];
        if (typeof fragment !== "string" || !isMapping(uniforms)) {
            throw new TypeError(`effect ${name} exports no fragment shader or uniform mapping`);
        }

        // compare what the effect sets with what its shaders declare
        const set = Object.keys(uniforms(defaults));
        const images = set
            .filter((uniform) => uniform === "u_image")
            .map((uniform) => `${uniform}AspectRatio`);
        const provided = new Set([...set, ...images, ...MOUNT_UNIFORMS]);
        const known = new Set([...declared(fragment), ...declared(VERTEX_SHADER)]);
        const unset = declared(fragment).filter((uniform) => !provided.has(uniform));
        const undeclared = set.filter((uniform) => !known.has(uniform));
        if (unset.length > 0 || undeclared.length > 0) {
            mismatches[name] = { unset, undeclared };
        }
    }

    // a mismatch is a uniform the port dropped, renamed or invented
    expect([effects.length, mismatches]).toEqual([20, {}]);
});
