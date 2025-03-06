<script lang="ts" setup>
import { makeType, NAME_TYPE } from "@/language/core/type";
import { isTaskActive, isTaskTerminal, toggleTaskStatus } from "@/language/runtime/task";
import {
  BenchType,
  ColorShade,
  NodeReferenceData,
  NodeType,
  PrimitiveType,
  TaskData,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedGetConnection, useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { pushPopover } from "@/ui/popover";
import { getTaskColorHex } from "@/ui/style";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { FocusAnchor, NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import Datetime from "@/views/content/Datetime.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { computed, ref, Ref, toRef, TrackOpTypes } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isInput" | "isMinimal" | "isInline">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// state
const taskPtr = toRef(props, "nodePtr");
const { graph, connection } = props.preparedConnection ?? useExistingConnection(taskPtr);
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
  <div v-if="task" class="group/task flex flex-row items-start gap-x-2">
    <!-- Status -->
    <button
      class="ml-1 mt-0.5 flex h-5 w-5 flex-row items-center justify-center rounded-2xl border border-gray-200 bg-white p-[1px] transition-colors duration-75"
      :style="{
        borderColor: fillState != 'empty' ? getTaskColorHex(task.status, ColorShade.S600) : undefined,
      }"
      @click="toggleTaskStatus(connection.tx, task)"
    >
      <span
        class="inline-block h-full w-full rounded-2xl transition-all duration-75"
        :style="{
          backgroundColor: fillState != 'empty' ? getTaskColorHex(task.status, ColorShade.S600) : undefined,
          clipPath: fillState == 'full' ? undefined : 'inset(0 0 50% 0)',
        }"
      />
    </button>
    <!-- Body -->
    <div class="flex w-full gap-y-0.5" :class="[hasMeta ? 'flex-col' : 'flex-row']">
      <div>
        <!-- Name -->
        <!-- nocheckin: rich 'names'? (for Task, Plan, Page, ...?) -->
        <NativeInput
          id="name"
          ref="nameRef"
          class="text-base font-medium"
          is-minimal
          is-input
          placeholder="Task"
          :value-type="NAME_TYPE"
          :model-value="task?.name"
          @update:model-value="
            (newValue) => connection.tx.update(task!, { name: newValue as string }, { debounce: 'long' })
          "
          @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
        />
      </div>
      <!-- Metadata -->
      <div class="flex flex-row gap-x-1.5" :class="[hasMeta ? '' : 'ml-auto']">
        <!-- Owner -->
        <button
          class="flex flex-row items-center gap-x-1.5 rounded bg-white px-1.5 transition-colors duration-150 hover:bg-gray-100 data-[popover=true]:bg-gray-100"
          :class="[
            hasMeta ? 'opacity-100' : 'opacity-0 group-focus-within/task:opacity-100 group-hover/task:opacity-100',
          ]"
          @click="
            (e: MouseEvent) => {
              const button = (e.target as HTMLElement).closest('button')!;
              pushPopover({
                kind: 'view',
                trigger: button,
                reference: button,
                component: ViewType.PICKER,
                title: 'Owner',
                placement: 'bottom-left',
                offset: 'referenceWidth',
                props: {
                  // NOTE :Incomplete: Task.owner should be a union of Flow|User|Role|...?
                  valueType: makeType({ kind: TypeKind.NODE, benchType: BenchType.FLOW }),
                  modelValue: task?.ownedByPtr,
                },
                onApply: (value: any) => {
                  connection.tx.update(task!, { ownedByPtr: value }, { debounce: 'short' });
                },
              });
            }
          "
        >
          <i v-if="!task.ownedByPtr" class="fas fa-user" :class="task.ownedByPtr ? 'text-gray-700' : 'text-gray-400'" />
          <NodeReference v-if="task.ownedByPtr" :node-ptr="task.ownedByPtr" size="regular" />
          <span v-else class="text-gray-400">owner</span>
        </button>
        <!-- Due -->
        <button
          class="flex flex-row items-center gap-x-1.5 rounded bg-white px-1.5 transition-colors duration-150 hover:bg-gray-100 data-[popover=true]:bg-gray-100"
          :class="[
            hasMeta ? 'opacity-100' : 'opacity-0 group-focus-within/task:opacity-100 group-hover/task:opacity-100',
          ]"
          @click="
            (e: MouseEvent) => {
              const button = (e.target as HTMLElement).closest('button')!;
              pushPopover({
                kind: 'view',
                trigger: button,
                reference: button,
                component: ViewType.DATETIME,
                title: 'Date',
                placement: 'bottom-left',
                offset: 'referenceWidth',
                props: {
                  valueType: makeType({ kind: TypeKind.PRIMITIVE, primitiveType: PrimitiveType.DATETIME }),
                  modelValue: task?.dueAt,
                },
                onApply: (value: any) => {
                  connection.tx.update(task!, { dueAt: value }, { debounce: 'short' });
                },
              });
            }
          "
        >
          <i class="fas fa-calendar-days" :class="task.dueAt ? 'text-gray-700' : 'text-gray-400'" />
          <Datetime v-if="task.dueAt" id="datetime" :model-value="task.dueAt" is-minimal />
          <span v-else class="text-gray-400">due</span>
        </button>
        <!-- Triggers -->
        <!-- ... -->
      </div>
    </div>
  </div>
  <Inaccessible v-else :node="taskPtr" :connection="connection" />
</template>
