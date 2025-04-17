<script lang="ts" setup>
import { supergraph } from "@/globals";
import { toCamelName } from "@/language/core/const";
import { Alignment, ChannelData, NodeType, Orientation, RectangleData, ViewData } from "@/proto/wire";
import { isNode, TypedNodeReferenceData } from "@/proto/wiring";
import { benchGraph, canvas } from "@/system/space";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import NodeReference from "@/views/builtin/NodeReference.vue";
import RootHeader from "@/views/builtin/RootHeader.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Chat from "@/views/helpers/Chat.vue";
import { computed, Ref, ref, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    isRoot?: boolean;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "focusPtr" | "alignment">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// state
const nodePtr = toRef(props, "nodePtr");
const { node, connection, graph } = supergraph.getLinkRef(nodePtr);
const channelPtr = computed(() => {
  if (node.value == null) return null;
  if (isNode(node.value, NodeType.THREAD)) return node.value.channelPtr;
  else return null;
});
const channel = supergraph.getRef(channelPtr) as Ref<ChannelData | null>;
const scopePtr = computed(() => {
  if (node.value == null) return null;
  if (isNode(node.value, NodeType.THREAD)) return node.value.scopePtr;
  else return null;
});
const scope = supergraph.getRef(scopePtr);

// view
const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);
const chatRef: Ref<InstanceType<typeof Chat> | null> = ref(null);

function focus() {
  chatRef.value?.focus?.();
}

defineExpose<ViewExpose>({ self, id, focus });
</script>
<template>
  <div>
    <!-- Root header -->
    <!-- NOTE :Incomplete: call/videochat with Thread? -->
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus-ptr="focusPtr" :graph="graph" />

    <!-- Chat -->
    <Chat
      id="chat"
      ref="chatRef"
      :alignment="alignment ?? Alignment.END"
      :node-ptr="nodePtr"
      :focus-ptr="focusPtr"
      :graph="benchGraph"
      :size="{
        width: size?.width,
        height: size?.height != null ? size.height - (isRoot ? VIEW_DEFAULT_ROOT_HEADER_HEIGHT : 0) : undefined,
      }"
      v-bind="state.getChildState('chat')"
    >
      <!-- Beginning -->
      <template v-slot:beginning="{ node }">
        <!-- Title -->
        <NodeReference
          v-if="node"
          ref="nameRef"
          class="mt-3 w-full px-0.5"
          :orientation="Orientation.VERTICAL"
          size="title"
          :node="node"
          is-input
          hide-metadata
          :tx="() => connection!.tx"
          @navigate="(direction) => emit('navigate', direction)"
        />
        <!-- Beginning -->
        <div v-if="node" class="mt-1.5 px-0.5 text-base text-gray-400">
          <span>
            This is the beginning of this
            {{ nodePtr != null ? toCamelName(NodeType, nodePtr.nodeType) : "???"
            }}<span v-if="isNode(node, NodeType.THREAD) && channel != null">
              in #{{ (channel as ChannelData)?.name ?? "???" }}</span
            ><span v-if="(scope as any)?.name != null"> on {{ (scope as any).name }}</span
            >.
          </span>
        </div>
      </template>
    </Chat>
  </div>
</template>
