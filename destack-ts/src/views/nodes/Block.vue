<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { newChangeId } from "@/language/core/transaction";
import { BlockType, NodeReferenceData, NodeType, ObjectType, ViewData } from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import type { CommandMapKit } from "@/ui/command";
import { usePageContext } from "@/ui/prosemirror/page";
import { generateOrderKey } from "@/utils/fractional";
import Inaccessible from "@/views/internal/Inaccessible.vue";
import NodeReference from "@/views/internal/NodeReference.vue";
import { type FocusAnchor, type NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import File from "@/views/content/File.vue";
import Page from "@/views/nodes/Page.vue";
import Database from "@/views/nodes/Table.vue";
import Task from "@/views/nodes/Task.vue";
import { computed, getCurrentInstance, nextTick, onBeforeUnmount, ref, toRef, triggerRef, type Ref } from "vue";

const MAX_INLINE_HEIGHT = 400;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    pageKey: string;
    id: string;
  } & Partial<Pick<ViewData, "isMinimal" | "nodePtr">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

// page
const instance = getCurrentInstance();
if (instance == null) {
  throw new Error("no instance in Block");
}
if (instance.parent == null) {
  instance.parent = (instance.vnode as any).parent;
}
if (instance.parent == null) {
  throw new Error("no parent in Block");
}
const pageContext = usePageContext();
pageContext.blocksRefById.value[props.id] = instance as any;
triggerRef(pageContext.blocksRefById);
onBeforeUnmount(() => {
  delete pageContext.blocksRefById.value[props.id];
  triggerRef(pageContext.blocksRefById);
});

// block
const preparedConnection = pageContext.preparedConnection;
const blockPtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.BLOCK>);
const { graph, connection } = preparedConnection;
const block = graph.getRef(blockPtr, { ignoreAncestors: true });
const nodePtr = computed(() => block.value?.nodePtr);
const node = graph.getRef(nodePtr, { ignoreAncestors: true });
const fields = graph.getChildrenRef(nodePtr, NodeType.FIELD);

// view
const blockRef = ref<HTMLElement | null>(null);
const nodeRef: Ref<InstanceType<typeof Page> | null> = ref(null);
const isPage = computed(() => block.value?.nodePtr?.nodeType == NodeType.PAGE);

//
// Interaction
//

const commands: Partial<CommandMapKit<"space">> & CommandMapKit<"block"> = {};

function onEnter() {
  if (block.value == null || pageContext.page.value == null) return;
  // create new block below
  const page = pageContext.page.value;
  const tx = connection.tx.with({ change: { title: "Morph", key: newChangeId() } });
  const blockIdx = pageContext.blocks.value.findIndex((b) => b.id == block.value!.id);
  const nextBlock = pageContext.blocks.value[blockIdx + 1];
  const orderKey = generateOrderKey(block.value.orderKey, nextBlock?.orderKey);
  const newBlock = tx.create({
    metatype: NodeType.BLOCK,
    type: BlockType.PARAGRAPH,
    parentPtr: toNodeRef(page),
    orderKey,
    packagePtr: page.packagePtr,
  });
  nextTick(() => pageContext.focus(toNodeRef(newBlock)));
}

function onDeleteSelf() {
  if (block.value == null || pageContext.page.value == null) return;
  // replace self with empty block
  const oldBlock = block.value;
  const page = pageContext.page.value;
  const tx = connection.tx.with({ change: { title: "Morph", key: newChangeId() } });
  tx.delete(oldBlock);
  const newBlock = tx.create({
    metatype: NodeType.BLOCK,
    type: BlockType.PARAGRAPH,
    parentPtr: toNodeRef(page),
    orderKey: oldBlock.orderKey,
    packagePtr: page.packagePtr,
  });
  nextTick(() => pageContext.focus(toNodeRef(newBlock)));
}

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  nodeRef.value?.focus?.(anchor);
}

defineExpose<ViewExpose>({ self, id, commands, focus });
</script>
<template>
  <div
    v-if="block"
    ref="blockRef"
    class="group/block relative select-none"
    :data-suppress-drag="isPage ? 'select' : undefined"
    data-contextmenu-items="space.navigate.open"
    :contenteditable="false"
    :draggable="false"
    @click="() => isPage && canvas.goToNode(node!)"
  >
    <NodeReference
      v-if="node && node.metatype == ObjectType.PAGE"
      ref="nodeRef"
      class="cursor-pointer"
      size="base"
      is-underline
      is-light
      :node="node"
      :tx="() => connection.tx"
      @navigate="
        (direction: NavigationDirection) => {
          if (direction == 'enter') {
            canvas.goToNode(node!);
          } else {
            emit('navigate', direction);
          }
        }
      "
      @enter="onEnter"
      @deleteSelf="onDeleteSelf"
    />
    <File
      v-else-if="block.nodePtr?.nodeType == NodeType.FILE"
      id="file"
      ref="nodeRef"
      class="max-h-[400px]"
      :prepared-connection="preparedConnection"
      is-inline
      :model-value="nodePtr"
      :size="{ height: MAX_INLINE_HEIGHT }"
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      @enter="onEnter"
      @deleteSelf="onDeleteSelf"
    />
    <Database
      v-else-if="block.nodePtr?.nodeType == NodeType.TABLE"
      id="database"
      ref="nodeRef"
      :node-ptr="nodePtr"
      :container-gutter-width="pageContext.gutterWidth.value"
      is-minimal
      is-inline
      is-input
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      @enter="onEnter"
      @deleteSelf="onDeleteSelf"
    />
    <Task
      v-else-if="block.nodePtr?.nodeType == NodeType.TASK"
      id="task"
      ref="nodeRef"
      :node-ptr="nodePtr"
      :prepared-connection="preparedConnection"
      is-minimal
      is-inline
      is-input
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      @enter="onEnter"
      @deleteSelf="onDeleteSelf"
    />
    <div v-else class="rounded-sm border border-red-500 bg-red-100 text-center font-semibold text-red-900">
      <span>No View for {{ toCamelName(BlockType, block.type) }} Block</span>
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full" :node="blockPtr" :connection="connection" />
</template>
