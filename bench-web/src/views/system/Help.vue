<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "subnodePacked"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const children = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });

defineExpose<ViewExposed>({ self });
</script>
<template>
  <div class="h-full w-full">
    <!-- nocheckin: Help -->
    {{ children.length }}
  </div>
</template>
