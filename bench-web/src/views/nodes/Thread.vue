<script lang="ts" setup>
import { supergraph } from "@/globals";
import { toCamelName } from "@/language/core/const";
import { Alignment, NodeType, Orientation, RectangleData, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { benchGraph, canvas } from "@/system/space";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import NodeReference from "@/views/builtin/NodeReference.vue";
import RootHeader from "@/views/builtin/RootHeader.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Chat from "@/views/helpers/Chat.vue";
import { Ref, ref, toRef } from "vue";

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
    <!-- NOTE :Incomplete: call/videochat with Thread members? (would be pretty cool) -->
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
          class="group/title mt-3 w-full px-0.5"
          :orientation="Orientation.VERTICAL"
          size="title"
          :node="node"
          is-input
          hide-metadata
          :tx="() => connection!.tx"
          @navigate="(direction) => emit('navigate', direction)"
        >
          <!-- Meta -->
          <template #right>
            <button
              v-if="!isRoot"
              v-tooltip="{ small: true, text: 'Open in full' }"
              class="ml-2 cursor-pointer rounded-sm px-1 text-2xl text-gray-400 opacity-0 transition-opacity duration-75 group-focus-within/title:opacity-100 group-hover/title:opacity-100 hover:bg-gray-100 hover:text-gray-700"
              @click="() => canvas.goToNode(node!)"
            >
              <i class="fas fa-arrow-up-right" />
            </button>
          </template>
        </NodeReference>
        <!-- Beginning -->
        <div v-if="node" class="mt-1.5 px-0.5 text-base text-gray-400">
          <span>
            This is the beginning of this
            {{ nodePtr != null ? toCamelName(NodeType, nodePtr.nodeType) : "???" }}.
          </span>
        </div>
      </template>
    </Chat>
  </div>
</template>
