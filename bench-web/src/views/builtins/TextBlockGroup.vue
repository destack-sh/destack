<script lang="ts" setup>
import { BlockData, NodeType, PageData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedGetConnection } from "@/system/connection";
import { useTextEditor } from "@/ui/prosemirror";
import { viewEmits, ViewExposed } from "@/views/common";
import { ref, toRef } from "vue";

const props = defineProps<{
  self?: TypedNodeReferenceData<NodeType.VIEW>;
  id: string;
  page: PageData;
  blocks: BlockData[];
  beforeBlock: BlockData | undefined;
  afterBlock: BlockData | undefined;
  connection: PreparedGetConnection;
  isInput?: boolean;
  suppressEnter?: boolean;
  suppressDrop?: boolean;
}>();
const { graph, connection } = props.connection;
const self = toRef(props, "self");
const id = toRef(props, "id");
const emit = defineEmits(viewEmits());

const textRef = ref<HTMLElement | null>(null);
const { focus, actions, isInDropZone } = useTextEditor({
  textRef,
  modelValue: toRef(props, "modelValue"),
  isInput: toRef(props, "isInput"),
  suppressEnter: toRef(props, "suppressEnter"),
  suppressDrop: toRef(props, "suppressDrop"),
});

defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <div
    ref="textRef"
    class="pm-text relative rounded hover:cursor-text"
    :class="['stealth', isInDropZone ? 'outline-dotted outline-2 outline-gray-400' : '']"
    data-suppress-actions="space.move.left,space.move.right"
    data-suppress-drag="both"
  />
</template>
