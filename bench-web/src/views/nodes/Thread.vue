<script lang="ts" setup>
import { supergraph } from "@/globals";
import { toCamelName } from "@/language/core/const";
import { Alignment, NodeType, Orientation, RectangleData, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas, pkgGraph } from "@/system/space";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RootHeader from "@/views/builtins/RootHeader.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Text from "@/views/content/Text.vue";
import Chat from "@/views/helpers/Chat.vue";
import { Ref, ref, toRef } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    isRoot?: boolean;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "focus">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// state
const nodePtr = toRef(props, "nodePtr");
const { node, connection } = supergraph.getLinkRef(nodePtr);

// view
const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div>
    <!-- Root header -->
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus="$props.focus" :graph="pkgGraph" />

    <Chat
      id="chat"
      :alignment="Alignment.END"
      :node-ptr="nodePtr"
      :focus="focus"
      :graph="pkgGraph"
      :size="{
        width: size?.width,
        height: size?.height != null ? size.height - (isRoot ? VIEW_DEFAULT_ROOT_HEADER_HEIGHT : 0) : undefined,
      }"
      v-bind="state.getChildState('chat')"
    >
      <!-- Beginning -->
      <template v-slot:beginning>
        <!-- Title -->
        <NodeReference
          v-if="node"
          ref="nameRef"
          class="w-full px-0.5"
          :orientation="Orientation.VERTICAL"
          size="title"
          :node="node"
          is-input
          :tx="() => connection!.tx"
          @navigate="(direction) => emit('navigate', direction)"
          @click.stop.prevent="nameRef?.focusIdentifier('right')"
        />
        <!-- Beginning -->
        <div class="mt-1.5 px-0.5 text-base text-gray-400">
          <span>This is the beginning of this {{ toCamelName(NodeType, nodePtr!.nodeType) }}.</span>
        </div>
      </template>
    </Chat>
  </div>
</template>
