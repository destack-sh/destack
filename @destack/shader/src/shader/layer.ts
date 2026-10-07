import { type Accessor, createSignal, onCleanup } from "@destack/view";
import type { GpuShaderDefinition } from "shaders/core";

/** One layer of a shader: an effect, its properties and the layers it wraps. */
export interface Layer {
    /** The layer's id, which updates name. */
    readonly id: string;
    /** The effect's name in the effect registry or its custom definition, such as LinearGradient. */
    readonly type: string;
    /** The custom effect's definition from `defineShader`, absent for a registry effect. */
    readonly definition?: GpuShaderDefinition;
    /** The effect's properties as the caller passes them. */
    readonly properties: Accessor<Readonly<Record<string, unknown>>>;
    /** The layers the effect wraps, in the order they joined. */
    readonly layers: Accessor<readonly Layer[]>;
}

/** The layers a shader or a wrapping effect holds, which the effects inside it join while mounted. */
export class LayerList {
    /** The layers in the order they joined. */
    readonly layers: Accessor<readonly Layer[]>;
    /** Replace the layers. */
    readonly #setLayers: (layers: readonly Layer[]) => void;

    /** Start without layers. */
    constructor() {
        const [layers, setLayers] = createSignal<readonly Layer[]>([], { ownedWrite: true });
        this.layers = layers;
        this.#setLayers = setLayers;
    }

    /** Add a layer until its effect unmounts. */
    add(layer: Layer): void {
        this.#setLayers([...this.layers(), layer]);
        onCleanup(() => this.#setLayers(this.layers().filter((entry) => entry !== layer)));
    }
}

/** Describe a layer tree's structure, which changes when a layer joins, leaves or moves. */
export function structureOf(layers: readonly Layer[]): string {
    return layers
        .map((layer) => `${layer.type}#${layer.id}(${structureOf(layer.layers())})`)
        .join(",");
}

/** List the custom definitions a layer tree draws, each once, which the renderer compiles beside its registry. */
export function definitionsOf(layers: readonly Layer[]): GpuShaderDefinition[] {
    const found = new Set<GpuShaderDefinition>();
    for (const layer of layers) {
        if (layer.definition !== undefined) {
            found.add(layer.definition);
        }
        for (const inner of definitionsOf(layer.layers())) {
            found.add(inner);
        }
    }

    return [...found];
}
