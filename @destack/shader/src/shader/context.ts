import { createContext } from "@destack/view";
import type { LayerList } from "./layer.ts";

/** The layers the effects inside a shader or a wrapping effect join, null outside a shader. */
export const LayerContext = createContext<LayerList | null>(null);
