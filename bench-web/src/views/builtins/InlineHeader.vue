<script lang="ts" setup>
import { InlineSourceNodeData, NodeReferenceData, NodeType, SelectionData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedGetConnection } from "@/system/connection";
import BlockHeader from "@/views/builtins/BlockHeader.vue";
import PageHeader from "@/views/builtins/PageHeader.vue";
import RootHeader from "@/views/builtins/RootHeader.vue";
import { FocusAnchor, NavigationDirection, ViewEmits, ViewExposed } from "@/views/common";
import { computed, ref, Ref } from "vue";

const props = defineProps<{
  self?: TypedNodeReferenceData<NodeType.VIEW>;
  node: InlineSourceNodeData | undefined | null;
  nodePtr: NodeReferenceData;
  preparedConnection: PreparedGetConnection;
  isRoot?: boolean;
  isInline?: boolean;
  isMinimal?: boolean;
  focus?: SelectionData | undefined;
  width?: number;
}>();
const { graph, connection } = props.preparedConnection;
const emit = defineEmits<ViewEmits>();

const pageHeaderRef: Ref<InstanceType<typeof PageHeader> | null> = ref(null);
const blockHeaderRef: Ref<InstanceType<typeof BlockHeader> | null> = ref(null);

const style = computed(() => {
  if (!props.isMinimal) {
    return "page";
  } else {
    return "block";
  }
});

function focusIdentifier(anchor: FocusAnchor) {
  (pageHeaderRef.value ?? blockHeaderRef.value)?.focusIdentifier(anchor);
}

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  focusIdentifier(typeof anchor == "string" ? anchor : "top");
}

defineExpose<Partial<ViewExposed> & { focusIdentifier: (anchor: FocusAnchor) => void }>({
  focusIdentifier,
  focus,
});
</script>
<template>
  <div class="">
    <!-- Root header -->
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus="props.focus" :graph="graph" />

    <!-- Page header -->
    <div v-if="style == 'page' && node != null" class="flex flex-col">
      <PageHeader
        ref="pageHeaderRef"
        :self="self"
        :width="width"
        :node="node"
        :connection="preparedConnection"
        is-input
        @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      >
        <template #meta>
          <slot name="meta" :style="style" />
        </template>
        <template #footer>
          <div class="flex flex-row flex-wrap">
            <div class="flex flex-row items-center gap-x-2">
              <slot name="left" :style="style" />
            </div>
            <div class="ml-auto flex flex-row items-center gap-x-2">
              <slot name="right" :style="style" />
            </div>
          </div>
        </template>
      </PageHeader>
    </div>

    <!-- Block header -->
    <div v-else-if="style == 'block' && node != null">
      <BlockHeader
        ref="blockHeaderRef"
        :node="node"
        :connection="preparedConnection"
        is-input
        @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      >
        <template #left>
          <slot name="left" :style="style" />
        </template>
        <template #right>
          <slot name="right" :style="style" />
        </template>
      </BlockHeader>
    </div>
  </div>
</template>
