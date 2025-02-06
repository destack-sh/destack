import type { NodeSuperGraph } from "@/language/core/graph";
import type { SpaceData } from "@/proto/wire";
import type { NodeAutoloader } from "@/system/autoload";
import type { SpaceCanvas } from "@/ui/space";
import { createDeferredProxy } from "@/utils/ref";
import { Ref } from "vue";

// hoisted globals for safe importing
export const { proxy: canvas, setRealObject: setCanvas } = createDeferredProxy<SpaceCanvas>();
export const { proxy: supergraph, setRealObject: setSupergraph } = createDeferredProxy<NodeSuperGraph>();
export const { proxy: space, setRealObject: setSpace } = createDeferredProxy<Ref<SpaceData | null>>();
export const { proxy: autoloader, setRealObject: setAutoloader } = createDeferredProxy<NodeAutoloader>();