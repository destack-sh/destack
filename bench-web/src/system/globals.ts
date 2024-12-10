import { NodeSuperGraph } from "@/language/graph";
import { SpaceCanvas } from "@/ui/space";
import { createDeferredProxy } from "@/utils/ref";

// hoisted globals for safe importing
export const { proxy: canvas, setRealObject: setCanvas } = createDeferredProxy<SpaceCanvas>();
export const { proxy: supergraph, setRealObject: setSupergraph } = createDeferredProxy<NodeSuperGraph>();