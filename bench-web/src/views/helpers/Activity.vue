<script lang="ts" setup>
import { ACTIVE_RUN_STATUSES } from "@/language/core/const";
import { makeExpression } from "@/language/core/expression";
import { packSubnode } from "@/language/core/node";
import {
  ExpressionData,
  ExpressionType,
  InterruptionProperty,
  InterruptionStatus,
  NodeType,
  RunProperty,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { propertyReference, TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import List from "@/views/collections/List.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import { computed, Ref, ref, toRef } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_ROOT_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

type ActivitySection = {
  title: string;
  nodeType: NodeType;
  filter?: ExpressionData;
  sort?: ExpressionData[];
};

const sections: Ref<ActivitySection[]> = computed(() => {
  const sections: ActivitySection[] = [];

  // Runs
  sections.push({
    title: "Runs",
    nodeType: NodeType.RUN,
    filter: makeExpression({
      type: ExpressionType.AND,
      clauses: [
        makeExpression({
          type: ExpressionType.IN,
          propertyPtr: propertyReference(NodeType.RUN, RunProperty.status),
          value: ACTIVE_RUN_STATUSES,
        }),
        makeExpression({
          type: ExpressionType.NOT_EXISTS,
          propertyPtr: propertyReference(NodeType.RUN, RunProperty.rootPtr),
        }),
      ],
    }),
    sort: [
      makeExpression({
        type: ExpressionType.DESCENDING,
        propertyPtr: propertyReference(NodeType.RUN, RunProperty.createdAt),
      }),
    ],
  });

  // Interruptions
  sections.push({
    title: "Interruptions",
    nodeType: NodeType.INTERRUPTION,
    filter: makeExpression({
      type: ExpressionType.EQUALS,
      propertyPtr: propertyReference(NodeType.INTERRUPTION, InterruptionProperty.status),
      value: InterruptionStatus.OPEN,
    }),
    sort: [
      makeExpression({
        type: ExpressionType.DESCENDING,
        propertyPtr: propertyReference(NodeType.INTERRUPTION, InterruptionProperty.createdAt),
      }),
    ],
  });

  return sections;
});

// interaction
const containerRef = ref<HTMLElement | null>(null);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const listRefs = ref<Record<number, InstanceType<typeof List> | null>>({});
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div ref="containerRef">
    <!-- Header? -->
    <!-- ... -->
    <!-- Body -->
    <div class="relative flex flex-col gap-2" @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)">
      <!-- Resource List -->
      <div v-for="{ title, nodeType, filter, sort } in sections" :key="nodeType">
        <!-- Header -->
        <div
          class="mx-4 mt-1.5 flex flex-row items-center gap-1.5"
          :style="{
            height: `${HEADER_HEIGHT - 6}px`,
          }"
        >
          <!-- Type -->
          <span class="font-medium">{{ title }}</span>
          <!-- Count badge -->
          <span
            v-if="listRefs[nodeType]?.total != null"
            class="rounded-2xl bg-green-100 px-1.5 text-sm font-medium text-green-900"
          >
            {{ listRefs[nodeType]?.total }}
          </span>
          <div class="ml-auto">
            <!-- ...something? -->
          </div>
        </div>
        <!-- List -->
        <List
          :id="`${id}.${nodeType}`"
          :ref="(ref?: any) => (ref != null ? (listRefs[nodeType] = ref) : delete listRefs[nodeType])"
          :subnode-packed="
            packSubnode(NodeType.VIEW, ViewType.LIST, { queryNodeType: nodeType, filter: filter, sort: sort })
          "
        />
      </div>

      <!-- Selection -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </div>
  </div>
</template>
