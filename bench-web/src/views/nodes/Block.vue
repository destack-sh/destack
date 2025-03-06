<script lang="ts" setup>
import { CANVAS_BLOCK_TYPES, toCamelName } from "@/language/core/const";
import { BlockType, NodeReferenceData, NodeType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import type { ActionMapKit } from "@/ui/action";
import { usePageContext } from "@/ui/prosemirror/page";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { type FocusAnchor, type NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import File from "@/views/content/File.vue";
import Choice from "@/views/nodes/Choice.vue";
import Database from "@/views/nodes/Database.vue";
import Flow from "@/views/nodes/Flow.vue";
import Kit from "@/views/nodes/Kit.vue";
import { computed, getCurrentInstance, onBeforeUnmount, ref, toRef, triggerRef, type Ref } from "vue";

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
const state = canvas.registerView(self, id);

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
const nodeRef: Ref<InstanceType<typeof Choice> | null> = ref(null);
const isPage = computed(() => block.value?.type == BlockType.PAGE);
const hasCanvas = computed(() => CANVAS_BLOCK_TYPES.includes(block.value?.type!));
const isInspected = computed(() => canvas.isInspected(blockPtr.value) || canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(blockPtr.value) || canvas.isHighlighted(nodePtr.value));
const isSelected = computed(() => state.isSelected(blockPtr.value));

//
// Interaction
//

const actions: Partial<ActionMapKit<"space">> & ActionMapKit<"block"> = {};

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  nodeRef.value?.focus?.(anchor);
}

defineExpose<ViewExpose>({ self, id, actions, focus });
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
      v-if="node && block.type == BlockType.PAGE"
      ref="nodeRef"
      size="large"
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
    />
    <File
      v-else-if="block.type == BlockType.FILE"
      id="file"
      ref="nodeRef"
      class="max-h-[400px]"
      :prepared-connection="preparedConnection"
      is-inline
      is-minimal
      :model-value="nodePtr"
      :size="{ height: MAX_INLINE_HEIGHT }"
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    />
    <Choice
      v-else-if="block.type == BlockType.CHOICE"
      id="choice"
      ref="nodeRef"
      class=""
      :prepared-connection="preparedConnection"
      :node-ptr="nodePtr"
      is-minimal
      is-inline
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    />
    <Flow
      v-else-if="block.type == BlockType.FLOW"
      id="flow"
      ref="nodeRef"
      :style="{ height: MAX_INLINE_HEIGHT + 'px' }"
      v-bind="state.getChildState('flow')"
      :node-ptr="nodePtr"
      :prepared-connection="preparedConnection"
      is-minimal
      is-inline
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    />
    <Database
      v-else-if="block.type == BlockType.DATABASE"
      id="database"
      ref="nodeRef"
      v-bind="state.getChildState('database')"
      :node-ptr="nodePtr"
      :container-gutter-width="pageContext.gutterWidth.value"
      is-minimal
      is-inline
      is-input
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    />
    <Kit
      v-else-if="block.type == BlockType.KIT"
      id="kit"
      ref="nodeRef"
      v-bind="state.getChildState('kit')"
      :node-ptr="nodePtr"
      :prepared-connection="preparedConnection"
      is-minimal
      is-inline
      is-input
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    />
    <!-- nocheckin: Task view -->
    <div v-else class="h-[100px] bg-red-100">
      <span>No View for {{ toCamelName(BlockType, block.type) }}</span>
    </div>
    <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="connection" />
  </div>
  <Inaccessible v-else class="h-full w-full" :node="blockPtr" :connection="connection" />
</template>
