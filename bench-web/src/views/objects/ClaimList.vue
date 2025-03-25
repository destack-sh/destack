<script lang="ts" setup>
import { supergraph } from "@/globals";
import { isNodeActive } from "@/language/core/const";
import { makeType, makeTypeConstraint } from "@/language/core/type";
import { isRunnable } from "@/language/runtime/run";
import { createClaim } from "@/language/source/claim";
import {
  ActionData,
  ClaimableNodeData,
  ClaimType,
  NodeMode,
  NodeReferenceData,
  NodeType,
  Orientation,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useAutoConnection, type PreparedNodeConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { type CommandMapKit } from "@/ui/command";
import { useSelectionZone } from "@/ui/drag";
import { pushPopover } from "@/ui/popover";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { computed, ref, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedNodeConnection;
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
const { graph, connection } = props.preparedConnection ?? useAutoConnection(basePtr);
const base = graph.getRef(basePtr);
const claims = graph.getChildrenRef(basePtr, NodeType.CLAIM);

// selecting
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

// actions
const commands: Partial<CommandMapKit<"list">> = {
  // list
};

defineExpose<ViewExpose>({ self, id, commands });
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
        @dblclick.stop="
          (claim.targetPtr != null || claim.targetTemplatePtr != null) &&
          canvas.goToNode((claim.targetPtr ?? claim.targetTemplatePtr)!)
        "
      >
        <!-- TODO :Incomplete: show/control? actual Claim status somehow -->
        <NodeReference :node-ptr="claim.targetPtr ?? claim.targetTemplatePtr" is-light size="sm" />
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
                  constraint: makeTypeConstraint({ nodeTypes: [NodeType.COMPUTER] }),
                }),
              },
              onApply: (value?: NodeReferenceData) => {
                if (value == null) return;
                const node = supergraph.getOrError(value) as ClaimableNodeData;
                createClaim(connection.tx, graph, {
                  claim: {
                    mode: isRunnable(node) ? NodeMode.TEMPLATE : NodeMode.MAIN,
                    type: ClaimType.SHARED,
                    packagePtr: (base as ActionData)?.packagePtr,
                    parentPtr: basePtr,
                    targetPtr: isNodeActive(node) ? value : undefined,
                    targetTemplatePtr: isNodeActive(node) ? undefined : value,
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
