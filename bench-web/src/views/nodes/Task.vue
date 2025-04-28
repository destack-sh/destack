<script lang="ts" setup>
import { makeType, makeTypeConstraint } from "@/language/core/type";
import { isTaskActive, isTaskTerminal, toggleTaskStatus } from "@/language/runtime/task";
import {
  ColorShade,
  ColorType,
  NodeReferenceData,
  NodeType,
  TaskData,
  TextLineData,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedNodeConnection, useAutoConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { pushPopover } from "@/ui/popover";
import { getColorHex, getProcessColorHex } from "@/ui/style";
import Inaccessible from "@/views/builtin/Inaccessible.vue";
import NodeReference from "@/views/builtin/NodeReference.vue";
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
const isActive = computed(() => task.value != null && isTaskActive(task.value));
const isTerminal = computed(() => task.value != null && isTaskTerminal(task.value));
const hasMeta = computed(() => task.value != null && (task.value.dueAt != null || task.value.ownedByPtr != null));

// view
const nameRef: Ref<InstanceType<typeof NativeInput> | null> = ref(null);
const fillState = computed(() => {
  if (isActive.value) {
    return "half";
  } else if (isTerminal.value) {
    return "full";
  } else {
    return "empty";
  }
});

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  nameRef.value?.focus?.(anchor ?? "left");
}

defineExpose<ViewExpose>({ self, id, focus });
</script>
<template>
  <div v-if="task" class="group/task flex flex-row items-baseline gap-x-2">
    <!-- Status -->
    <button
      class="flex h-4 w-4 cursor-pointer flex-row items-center justify-center rounded-sm border border-gray-500 bg-white p-[1px] transition-colors duration-75"
      :style="{
        borderColor: fillState != 'empty' ? getProcessColorHex(task.status, ColorShade.S500) : undefined,
      }"
      @mousedown.stop="toggleTaskStatus(connection.tx, task)"
    >
      <span
        class="inline-block h-full w-full rounded-sm transition-all duration-75"
        :style="{
          backgroundColor: fillState != 'empty' ? getProcessColorHex(task.status, ColorShade.S500) : undefined,
          clipPath: fillState == 'full' ? undefined : 'inset(0 0 50% 0)',
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
          (newValue) => connection.tx.update(task!, { title: newValue as TextLineData }, { debounce: 'short' })
        "
        @navigate="emit('navigate', $event)"
      />
      <!-- nocheckin: Tasks -->
      <!-- NOTE :Incomplete: Task.owner/due/triggers/... -->
      <!-- ... -->
    </div>
  </div>
  <Inaccessible v-else :node="taskPtr" :connection="connection" />
</template>
