<script lang="ts" setup>
import { InlineNodeData, NodeReferenceData, Orientation } from "@/proto/wire";
import { PreparedNodeConnection } from "@/system/connection";
import { PopoverInfoIn } from "@/ui/popover";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { FocusAnchor, ViewEmits, ViewExpose } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import { ref, Ref } from "vue";

const props = defineProps<{
  node: InlineNodeData;
  connection: PreparedNodeConnection;
  width?: number;
  isCompact?: boolean;
  isInput?: boolean;
}>();
const { graph, connection } = props.connection;
const emit = defineEmits<ViewEmits>();
const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);

function focusIdentifier(anchor: FocusAnchor) {
  nameRef.value?.focusIdentifier(anchor);
}

function focusIcon() {
  nameRef.value?.focusIcon();
}

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  focusIdentifier(typeof anchor == "string" ? anchor : "top");
}

defineExpose<Partial<ViewExpose> & { focusIdentifier: (anchor: FocusAnchor) => void }>({
  focusIdentifier,
  focus,
});
</script>
<template>
  <div class="w-full">
    <!-- Cover image? -->
    <!-- ... -->
    <!-- Main header -->
    <div
      class="group/title pt-4"
      :class="[isCompact ? 'pb-2' : 'pb-4', props.width != null ? 'mx-auto' : '']"
      :style="{
        width: props.width != null ? props.width + 'px' : undefined,
      }"
    >
      <!-- Title actions -->
      <div class="flex flex-row items-center gap-x-2 py-1">
        <!-- Add icon -->
        <button
          v-menu="
            (): PopoverInfoIn => ({
              kind: 'view',
              component: Icon,
              placement: 'bottom-right',
              offset: '-referenceWidth',
              props: { modelValue: (node as any)!.icon, isInput: true },
              isEnabled: isInput,
              onApply: (newIcon) => connection.tx.update(node!, { icon: newIcon }),
            })
          "
          class="rounded-full px-1.5 py-0.5 text-gray-400 opacity-0 transition-colors hover:bg-gray-100 group-focus-within/title:opacity-100 group-hover/title:opacity-100"
        >
          <i class="fas fa-face-smile mr-1.5 text-gray-400" />
          <span>{{ node.icon == null ? "Add icon" : "Change icon" }}</span>
        </button>
      </div>
      <!-- Title -->
      <NodeReference
        ref="nameRef"
        class="w-full px-0.5"
        :orientation="isCompact ? Orientation.HORIZONTAL : Orientation.VERTICAL"
        :hide-icon="node.icon == null"
        size="title"
        :node="node"
        is-input
        :tx="() => connection.tx"
        @navigate="(direction) => emit('navigate', direction)"
      />
    </div>
    <!-- Header "footer" -->
    <div
      v-if="$slots.body != null"
      class="mx-auto py-1"
      :style="{
        width: props.width != null ? props.width + 'px' : undefined,
      }"
    >
      <slot name="body" />
    </div>
  </div>
</template>
