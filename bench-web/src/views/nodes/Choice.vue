<script lang="ts" setup>
import { ViewData, NodeType, FieldType } from "@/proto/wire";
import { type ViewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";
import FieldList from "@/views/objects/FieldList.vue";
import { PreparedGetConnection, useExistingConnection } from "@/system/connection";
import BlockHeader from "@/views/builtins/BlockHeader.vue";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);
const choicePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.CHOICE>);
const preparedConnection = props.preparedConnection ?? useExistingConnection(choicePtr);
const { graph, connection } = preparedConnection;
const choice = graph.getRef(choicePtr);

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <BlockHeader v-if="choice" :node="choice" :connection="preparedConnection" />
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
