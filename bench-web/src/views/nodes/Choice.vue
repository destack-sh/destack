<script lang="ts" setup>
import { FieldType, NodeReferenceData, NodeType, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedGetConnection, useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import InlineHeader from "@/views/builtins/InlineHeader.vue";
import { FocusAnchor, NavigationDirection, type ViewEmits, type ViewExposed } from "@/views/common";
import FieldList from "@/views/objects/FieldList.vue";
import { computed, ref, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
    isRoot?: boolean;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInline" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const choicePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.CHOICE>);
const preparedConnection = props.preparedConnection ?? useExistingConnection(choicePtr);
const { graph, connection } = preparedConnection;
const choice = graph.getRef(choicePtr);

const headerRef = ref<InstanceType<typeof InlineHeader> | null>(null);

function focus(anchor: FocusAnchor | NodeReferenceData = "bottom") {
  headerRef.value?.focus?.(anchor ?? "top");
}

defineExpose<ViewExposed>({ self, id, focus });
</script>
<template>
  <div>
    <InlineHeader
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
    <FieldList
      v-if="choice"
      :id="choice.id"
      :field-type="FieldType.OPTION"
      :prepared-connection="preparedConnection"
      :node-ptr="choicePtr"
      class="px-1 py-1.5"
      :style="{
        minHeight: VIEW_DEFAULT_HEADER_HEIGHT + 'px',
      }"
    />
  </div>
</template>
