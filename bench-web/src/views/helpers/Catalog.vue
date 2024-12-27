<script lang="ts" setup>
import { ViewData, NodeType, ViewType, ExpressionType, MachineProperty, ResourceStatus } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { ref, toRef } from "vue";
import { propertyReference, TypedNodeReferenceData } from "@/proto/wiring";
import List from "@/views/collections/List.vue";
import { packSubnode } from "@/language/node";
import { toCamelName } from "@/language/const";
import { makeExpression } from "@/language/expression";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_BAR_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const RESOURCE_TYPES = [NodeType.MACHINE, NodeType.BROWSER];

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

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
      <div v-for="resourceType in RESOURCE_TYPES" :key="resourceType">
        <!-- Header -->
        <div
          class="mx-4 mt-1.5 flex flex-row items-center gap-1.5"
          :style="{
            height: `${HEADER_HEIGHT - 6}px`,
          }"
        >
          <!-- Type -->
          <span class="font-medium">{{ toCamelName(NodeType, resourceType) }}</span>
          <!-- Count badge -->
          <span
            v-if="listRefs[resourceType]?.total != null"
            class="rounded-2xl bg-green-100 text-sm font-medium text-green-900 px-1.5"
          >
            {{ listRefs[resourceType]?.total }}
          </span>
        </div>
        <!-- List -->
        <List
          :id="`${id}.${resourceType}`"
          :ref="(ref?: any) => (ref != null ? (listRefs[resourceType] = ref) : delete listRefs[resourceType])"
          :subnode-packed="
            packSubnode(NodeType.VIEW, ViewType.LIST, {
              queryNodeType: resourceType,
              filter: makeExpression({
                type: ExpressionType.NOT_IN,
                propertyPtr: propertyReference(resourceType, MachineProperty.status),
                value: [ResourceStatus.DECOMMISSIONED],
              }),
            })
          "
        />
      </div>

      <!-- Selection -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </div>
  </div>
</template>
