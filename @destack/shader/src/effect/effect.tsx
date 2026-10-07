import { createUniqueId, type JSX, omit, useContext } from "@destack/view";
import type { GpuShaderDefinition } from "shaders/core";
import { LayerContext } from "../shader/context.ts";
import { LayerList } from "../shader/layer.ts";

/** The properties every effect takes beside its own: the layers it wraps and how it blends. */
export interface LayerProperties {
    /** The effects this one wraps, such as the layers a blur blurs. */
    readonly children?: JSX.Element;
    /** How the layer blends with the layers below it. */
    readonly blendMode?: string;
    /** The layer's opacity from 0 to 1. */
    readonly opacity?: number;
    /** Whether the layer draws. */
    readonly visible?: boolean;
}

/** The properties of an effect: its own, each optional over its default, and the layer's. */
export type EffectProperties<Own extends object> = Partial<Own> & LayerProperties;

/** Make the component of an effect, of the registry by name or custom from `defineShader`, which joins the nearest shader or wrapping effect as a layer. */
export function defineEffect<Own extends object>(
    effect: string | GpuShaderDefinition<Own>,
): (properties: EffectProperties<Own>) => JSX.Element {
    const type = typeof effect === "string" ? effect : effect.name;
    const definition = typeof effect === "string" ? {} : { definition: effect };

    return (properties) => {
        // join the nearest shader or wrapping effect, refusing an effect outside one
        const parent = useContext(LayerContext);
        if (parent === null) {
            throw new TypeError(`the ${type} effect needs a shader around it`);
        }
        const wrapped = new LayerList();
        const own = omit(properties, "children");
        parent.add({
            id: createUniqueId(),
            type,
            ...definition,
            properties: () => ({ ...own }),
            layers: wrapped.layers,
        });

        return <LayerContext value={wrapped}>{properties.children}</LayerContext>;
    };
}
