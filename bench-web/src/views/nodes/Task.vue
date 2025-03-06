<script lang="ts" setup>
import { NAME_TYPE } from "@/language/core/type";
import { NodeType, TaskData, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedGetConnection, useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import { NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import { Ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInput" | "isMinimal" | "isInline">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const taskPtr = toRef(props, "nodePtr");
const { graph, connection } = props.preparedConnection ?? useExistingConnection(taskPtr);
const task = graph.getRef(taskPtr, { ignoreAncestors: true }) as Ref<TaskData | null>;

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div v-if="task" class="flex flex-row">
    <!-- nocheckin -->
    <!-- Status -->
    <button class="h-5 w-5 rounded border border-gray-200 bg-white p-[1px] transition-colors duration-75">
      <span class="inline-block h-full w-full rounded bg-gray-700 transition-colors duration-75" />
    </button>
    <!--  -->
    <div class="flex flex-col">
      <!-- Name -->
      <NativeInput
        id="name"
        :value-type="NAME_TYPE"
        is-minimal
        is-input
        :model-value="task?.name"
        @update:model-value="
          (newValue) => task && connection.tx.update(task, { name: newValue as string }, { debounce: 'long' })
        "
        @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
      />
      <!-- Metadata -->
      <NodeMetadata :node="task" size="regular" />
    </div>
  </div>
  <Inaccessible v-else :node="taskPtr" :connection="connection" />
</template>
