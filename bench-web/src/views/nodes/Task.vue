<script lang="ts" setup>
import { isProcessActive, isProcessTerminal } from "@/language/runtime/process";
import { toggleTaskStatus } from "@/language/runtime/task";
import { ColorShade, NodeReferenceData, NodeType, TaskData, TextLineData, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedNodeConnection, useAutoConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { getProcessColorHex } from "@/ui/style";
import Inaccessible from "@/views/builtin/Inaccessible.vue";
import { FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import TextLine from "@/views/content/TextLine.vue";
import { computed, ref, Ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInput" | "isMinimal" | "isInline">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

// state
const taskPtr = toRef(props, "nodePtr");
const { graph, connection } = props.preparedConnection ?? useAutoConnection(taskPtr);
const task = graph.getRef(taskPtr, { ignoreAncestors: true }) as Ref<TaskData | null>;
const isActive = computed(() => task.value != null && isProcessActive(task.value));
const isTerminal = computed(() => task.value != null && isProcessTerminal(task.value));
const hasMeta = computed(() => task.value != null && (task.value.dueAt != null || task.value.ownedByPtr != null));

// view
const nameRef: Ref<InstanceType<typeof NativeInput> | null> = ref(null);

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  nameRef.value?.focus?.(anchor ?? "left");
}

defineExpose<ViewExpose>({ self, id, focus });
</script>
<template>
  <div v-if="task" class="group/task flex flex-row items-baseline gap-x-2">
    <!-- Status -->
    <button
      class="flex h-4 w-4 flex-row items-center justify-center rounded-sm border bg-white p-[1px] transition-colors duration-75 enabled:cursor-pointer"
      :style="{
        borderColor: getProcessColorHex(task.status, ColorShade.S500),
      }"
      @mousedown.stop="toggleTaskStatus(connection.tx, task)"
    >
      <span
        class="inline-block h-full w-full rounded-sm transition-all duration-75"
        :class="[isActive ? 'animate-task-check-progress' : undefined]"
        :style="{
          backgroundColor: isActive || isTerminal ? getProcessColorHex(task.status, ColorShade.S500) : undefined,
        }"
      />
    </button>
    <!-- Body -->
    <div class="relative w-full">
      <!-- Title -->
      <TextLine
        ref="nameRef"
        class="w-full text-base"
        is-minimal
        is-input
        placeholder="Task"
        :model-value="task?.title"
        @update:model-value="
          (newValue) => connection.tx.update(task!, { title: newValue as TextLineData }, { debounce: 'long' })
        "
        @navigate="emit('navigate', $event)"
        @enter="emit('enter')"
        @deleteSelf="emit('deleteSelf')"
      />
      <!-- NOTE :Incomplete: Task.owner/due/triggers/... -->
      <!-- ... -->
    </div>
  </div>
  <Inaccessible v-else :node="taskPtr" :connection="connection" />
</template>

<style>
@keyframes task-check-progress {
  0% {
    clip-path: inset(0 0 30% 0);
  }
  50% {
    clip-path: inset(0 0 80% 0);
  }
  100% {
    clip-path: inset(0 0 30% 0);
  }
}

.animate-task-check-progress {
  animation: task-check-progress 3s ease-in-out infinite;
}
</style>
