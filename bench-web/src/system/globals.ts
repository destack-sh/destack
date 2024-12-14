import { NodeSuperGraph } from "@/language/graph";
import { NodeAutoloader } from "@/system/autoload";
import { SpaceCanvas } from "@/ui/space";
import { createDeferredProxy } from "@/utils/ref";

// hoisted globals for safe importing
export const { proxy: canvas, setRealObject: setCanvas } = createDeferredProxy<SpaceCanvas>();
export const { proxy: supergraph, setRealObject: setSupergraph } = createDeferredProxy<NodeSuperGraph>();
export const { proxy: autoloader, setRealObject: setAutoloader } = createDeferredProxy<NodeAutoloader>();