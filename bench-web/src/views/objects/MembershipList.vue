<script lang="ts" setup>
import { makeType, makeTypeConstraint } from "@/language/core/type";
import { createMembership } from "@/language/source/membership";
import {
  ActionData,
  NodeReferenceData,
  NodeType,
  Orientation,
  TypeKind,
  ViewData,
  ViewType
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useAutoConnection, type PreparedNodeConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { type CommandMapKit } from "@/ui/command";
import { startDraggingIfAllowed, useSelectionZone } from "@/ui/drag";
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
const memberships = graph.getChildrenRef(basePtr, NodeType.MEMBERSHIP);

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
      <!-- Memberships -->
      <li
        v-for="membership in memberships"
        :key="membership.id"
        class="flex h-[30px] flex-row items-center px-1.5 transition-colors duration-150 hover:bg-gray-100"
        :data-node-type="membership.metatype"
        :data-node-id="membership.id"
        :data-node-ck="(membership as any).ck"
        :data-node-bench-id="(membership as any).benchPtr?.id"
        data-suppress-drag="select"
        :draggable="true"
        @dblclick.stop="membership.memberPtr != null && canvas.goToNode(membership.memberPtr)"
        @dragstart.stop="(e: DragEvent) => startDraggingIfAllowed(e, membership.memberPtr ?? membership)"
      >
        <NodeReference :node-ptr="membership.memberPtr" is-light size="sm" />
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
                  constraint: makeTypeConstraint({ nodeTypes: [NodeType.AGENT, NodeType.USER, NodeType.TEAM] }),
                }),
              },
              onApply: (value?: NodeReferenceData) => {
                if (value == null) return;
                if (memberships.find((m) => m.memberPtr?.id == value.id)) return;
                createMembership(connection.tx, graph, {
                  membership: {
                    packagePtr: (base as ActionData)?.packagePtr,
                    parentPtr: basePtr,
                    memberPtr: value,
                  },
                });
              },
            });
          }
        "
      >
        <i class="fas fa-plus mr-1.5" />
        <span>Member</span>
      </button>
    </ul>
  </div>
</template>
