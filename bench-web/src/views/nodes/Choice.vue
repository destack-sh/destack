<script lang="ts" setup>
import { NodeReferenceData, NodeType, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedNodeConnection, useAutoConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import InlinePageHeader from "@/views/builtin/InlinePageHeader.vue";
import { FocusAnchor, NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import { computed, ref, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedNodeConnection;
    isRoot?: boolean;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInline" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

const choicePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.CHOICE>);
const preparedConnection = props.preparedConnection ?? useAutoConnection(choicePtr);
const { graph, connection } = preparedConnection;
const choice = graph.getRef(choicePtr);

const headerRef = ref<InstanceType<typeof InlinePageHeader> | null>(null);

function focus(anchor: FocusAnchor | NodeReferenceData = "bottom") {
  headerRef.value?.focus?.(anchor ?? "top");
}

defineExpose<ViewExpose>({ self, id, focus });
</script>
<template>
  <div>
    <InlinePageHeader
      ref="headerRef"
      :self="self"
      :node="choice"
      :connection="preparedConnection"
      :node-ptr="choicePtr"
      :prepared-connection="preparedConnection"
      :is-root="isRoot"
      :is-inline="isInline"
      :is-minimal="isMinimal"
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    />
    <!-- TODO :Incomplete: Choice view -->
  </div>
</template>
