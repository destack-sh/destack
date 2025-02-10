<script lang="ts" setup>
import { BlockData, NodeType, PageData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { useHighlightPlugin, useTextBlockGroupInterface, useTextEditor } from "@/ui/prosemirror";
import { computedValue } from "@/utils/ref";
import { NavigationDirection, type ViewEmits, ViewExposed } from "@/views/common";
import { ref, toRef, watch, watchEffect } from "vue";

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
const emit = defineEmits<ViewEmits>();
const state = canvas.registerView(self, id);

const textRef = ref<HTMLElement | null>(null);

// highlighting
const selectedBlockIds = computedValue(() => {
  const selectedBlockIds: string[] = [];
  for (const block of props.blocks) {
    if (canvas.isSelected(block) || (block.nodePtr != null && canvas.isSelected(block.nodePtr))) {
      selectedBlockIds.push(block.id!);
    }
  }
  return selectedBlockIds;
});
const highlightPlugin = useHighlightPlugin({ selectedBlockIds });
watch(selectedBlockIds, () => {
  updatePlugin(highlightPlugin);
});

// editor
const textInterface = useTextBlockGroupInterface({
  page: toRef(props, "page"),
  blocks: toRef(props, "blocks"),
  graph,
  txFactory: () => connection.tx,
});
const { focus, actions, isInDropZone, updatePlugin } = useTextEditor({
  textRef,
  text: textInterface,
  isInput: toRef(props, "isInput"),
  suppressEnter: toRef(props, "suppressEnter"),
  suppressDrop: toRef(props, "suppressDrop"),
  navigate: (direction: NavigationDirection) => emit("navigate", direction),
  deleteSelf: () => emit("deleteSelf"),
  plugins: [highlightPlugin],
});

defineExpose<ViewExposed>({ self, id, actions, focus: (anchor) => focus(anchor) });
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
