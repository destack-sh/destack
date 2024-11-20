<script lang="ts" setup>
import { NodeType, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import { VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_MAX_WIDTH } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import { viewEmits, type ViewExposed } from "@/views/common";
import { computed, toRef } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = VIEW_DEFAULT_MAX_WIDTH;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "icon" | "size" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr));
const inspectedPtr = computedValue(() => nodePtr.value ?? inspectionPtr.value);

const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(inspectedPtr);
const inspectedNode = pkgGraph.getRef(inspectedPtr);

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="inspectedNode" class="" :class="size == null ? '' : 'h-full w-full'">
    <!-- Inspection content -->
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    <!-- Empty/missing state -->
  </div>
</template>
