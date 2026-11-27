import { ActionInput } from "@destack/object";
import { JsonValue, LocalReference } from "@destack/schema";
import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import type { Component } from "@destack/view/declare";
import { VERTEX_SHADER } from "./mount/vertex.ts";
import { Shader } from "./shader/shader.tsx";

/** The uniforms the mount sets on every program. */
const MOUNT_UNIFORMS = ["u_time", "u_resolution", "u_pixelRatio"];

/** The directories of the package that hold no effect. */
const CORE = new Set(["mount", "shader", "glsl", "error"]);

/** A color no effect defaults to, to change one by. */
const CHANGED_COLOR = "#123456";

/** An effect as its modules give it: its fragment shader, its uniforms and its component. */
interface Effect {
    /** The effect's directory. */
    readonly name: string;
    /** The fragment shader. */
    readonly fragment: string;
    /** Write options as the fragment shader's uniforms. */
    readonly uniforms: (options: object) => Readonly<Record<string, unknown>>;
    /** The component that draws it. */
    readonly component: Component;
}

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

/** Report whether a value writes an effect's options as uniforms. */
function isMapping(
    value: unknown,
): value is (options: object) => Readonly<Record<string, unknown>> {
    return typeof value === "function";
}

/** Report whether a value is a declared component. */
function isComponent(value: unknown): value is Component {
    return typeof value === "function" && "properties" in value && "defaults" in value;
}

/** Read every effect of the package from its modules. */
async function effects(): Promise<Effect[]> {
    // list the effect directories in a world
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const names = (await files.readDirectory(import.meta.dirname))
        .filter((entry) => entry.kind === "directory" && !CORE.has(entry.name))
        .map((entry) => entry.name);

    // read each effect's fragment shader, uniforms and component
    return Promise.all(
        names.map(async (name) => {
            const module: unknown = await import(`./${name}/effect.ts`);
            const drawn: unknown = await import(`./${name}/${name}.tsx`);
            if (!isRecord(module) || !isRecord(drawn)) {
                throw new TypeError(`effect '${name}' has no modules`);
            }
            const fragment = Object.entries(module).find(([key]) => key.endsWith("_FRAGMENT"))?.[1];
            const uniforms = Object.entries(module).find(([key]) => key.endsWith("Uniforms"))?.[1];
            const component = Object.values(drawn).find((each) => isComponent(each));
            if (typeof fragment !== "string" || !isMapping(uniforms) || !component) {
                throw new TypeError(
                    `effect '${name}' lacks a fragment shader, uniforms or component`,
                );
            }

            return { name, fragment, uniforms, component };
        }),
    );
}

/** Read another value a declared property takes than its current one. */
function otherValue(
    name: string,
    field: Component["properties"][string],
    current: unknown,
): JsonValue {
    // take another choice, the other truth, another color, or a number within the limits
    const { value } = ActionInput.of(name, field);
    if (value.kind === "choice") {
        return value.choices.find((choice) => choice.value !== current)?.value ?? "";
    } else if (value.kind === "boolean") {
        return current !== true;
    } else if (value.kind === "json" && LocalReference.root(value.schema) === "style.color") {
        return CHANGED_COLOR;
    } else if (value.kind === "json" && LocalReference.root(value.schema) === "style.palette") {
        return [CHANGED_COLOR];
    } else if (value.kind === "number") {
        const low = value.minimum ?? 0;
        const high = value.maximum ?? low + 1;
        const middle = value.step === 1 ? Math.round((low + high) / 2) : (low + high) / 2;

        return middle === current ? high : middle;
    }

    throw new TypeError(`property '${name}' takes no value to change it by`);
}

test("declare every uniform an effect sets, and set every uniform its fragment shader reads", async () => {
    const mismatches: Record<string, { unset: string[]; undeclared: string[] }> = {};
    const found = await effects();
    for (const { name, fragment, uniforms, component } of found) {
        // compare what the effect sets with what its shaders declare
        const set = Object.keys(uniforms(component.defaults));
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
    expect([found.length, mismatches]).toEqual([20, {}]);
});

test("change an effect's uniforms by each property it declares, the shader's own aside", async () => {
    // change each property alone, noting those that leave the uniforms as they are
    const ignored: string[] = [];
    for (const { name, uniforms, component } of await effects()) {
        const before = JSON.stringify(uniforms(component.defaults));
        for (const [key, field] of Object.entries(component.properties)) {
            if (key in Shader.properties) {
                continue;
            }
            const current = component.defaults[key];
            const changed = { ...component.defaults, [key]: otherValue(key, field, current) };
            if (JSON.stringify(uniforms(changed)) === before) {
                ignored.push(`${name}.${key}`);
            }
        }
    }

    expect(ignored).toEqual([]);
});

test("give every example of an effect only values its declared properties take", async () => {
    // check each declared value of each example against its field
    const refused: string[] = [];
    for (const { name, component } of await effects()) {
        const module: unknown = await import(`./${name}/${name}.example.tsx`);
        const examples = isRecord(module) ? Object.entries(module) : [];
        for (const [exported, example] of examples) {
            const given =
                isRecord(example) && isRecord(example["properties"]) ? example["properties"] : {};
            for (const [key, value] of Object.entries(given)) {
                const field = component.properties[key];
                const isTaken =
                    field === undefined || ActionInput.of(key, field).takes(JsonValue.of(value));
                if (!isTaken) {
                    refused.push(`${exported}.${key}: ${JSON.stringify(value)}`);
                }
            }
        }
    }

    expect(refused).toEqual([]);
});
