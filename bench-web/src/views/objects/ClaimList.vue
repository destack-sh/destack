<script lang="ts" setup>
import { RESOURCE_NODE_TYPES } from "@/language/core/const";
import { makeType, makeTypeConstraint } from "@/language/core/type";
import { createClaim } from "@/language/source/claim";
import {
  ActionData,
  ClaimType,
  NodeMode,
  NodeReferenceData,
  NodeType,
  Orientation,
  TypeBaseNodeData,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { type ActionMapKit } from "@/ui/action";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { makeIcon } from "@/ui/icon";
import { pushPopover } from "@/ui/popover";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Picker from "@/views/content/Picker.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
  } & Partial<Pick<ViewData, "isMinimal" | "orientation" | "nodePtr">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const orientation = computed(() => props.orientation ?? Orientation.HORIZONTAL);
const isHorizontal = computed(
  () => orientation.value == Orientation.HORIZONTAL || orientation.value == Orientation.HORIZONTAL_REVERSED,
);
const state = canvas.registerView(self, id);

const containerRef = ref<HTMLElement | null>(null);

const basePtr = computed(() => props.nodePtr);
const { graph, connection } = props.preparedConnection ?? useExistingConnection(basePtr);
const base = graph.getRef(basePtr);
const claims = graph.getChildrenRef(basePtr, NodeType.CLAIM);

// selecting
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

// actions
const actions: Partial<ActionMapKit<"list">> = {
  // list
};

defineExpose<ViewExpose>({ self, id, actions });
</script>
<template>
  <div ref="containerRef" class="relative">
    <ul class="flex flex-col rounded">
      <!-- Claims -->
      <li
        v-for="claim in claims"
        :key="claim.id"
        class="flex h-[30px] flex-row items-center px-1.5 transition-colors duration-150 hover:bg-gray-100"
        :data-node-id="claim.id"
        :data-node-type="claim.metatype"
      >
        <NodeReference :node-ptr="claim.targetPtr" is-light size="sm" />
      </li>
      <!-- Create -->
      <button
        class="h-[30px] rounded px-2.5 text-left text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
        @click="
          (e: MouseEvent) => {
            const button = (e.target as HTMLElement).closest('button')!;
            pushPopover({
              kind: 'view',
              trigger: button,
              reference: button,
              component: ViewType.PICKER,
              placement: 'bottom-left',
              offset: 'referenceWidth',
              props: {
                valueType: makeType({
                  kind: TypeKind.NODE,
                  constraint: makeTypeConstraint({
                    nodeTypes: [NodeType.BROWSER, NodeType.COMPUTER, NodeType.KIT, NodeType.FLOW],
                  }),
                }),
              },
              onApply: (value?: NodeReferenceData) => {
                if (value == null) return;
                createClaim(connection.tx, graph, {
                  claim: {
                    mode: NodeMode.TEMPLATE,
                    type: ClaimType.SHARED,
                    packagePtr: (base as ActionData)?.packagePtr,
                    parentPtr: basePtr,
                    targetPtr: value,
                  },
                });
              },
            });
          }
        "
      >
        <i class="fas fa-plus mr-1.5" />
        <span>Claim</span>
      </button>
    </ul>
  </div>
</template>
