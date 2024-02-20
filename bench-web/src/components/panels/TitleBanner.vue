<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { useAppearance } from "@/state/appearance";
import type { Action } from "@/state/bench";
import { computed, ref } from "vue";

const props = defineProps<{
  modelValue: string;
  readonly: boolean;
  actions: Action<any>[];
  thing: unknown;
  fatActions?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "enter"): void;
  (e: "navigateDown"): void;
}>();

const appearance = useAppearance();
const nameRef = ref<InstanceType<typeof EditableSpan> | null>(null);

defineExpose({
  focus: (f: "first" | "last" = "first") => nameRef.value?.focus(f),
  selectAll: () => nameRef.value?.selectAll(),
  blur: () => nameRef.value?.blur(),
  editing: computed(() => nameRef.value?.focused ?? false),
});
</script>
<template>
  <div
    class="group/meta relative flex cursor-text flex-row items-center justify-between"
    @click="nameRef?.focus('last')"
  >
    <!-- Name & actions -->
    <span class="flex flex-row items-center">
      <!-- Name -->
      <span>
        <!-- Note the :EditableSyncDance on the name update -->
        <EditableSpan
          ref="nameRef"
          class="text-3xl font-bold text-gray-900"
          :class="appearance.baseClassUnsized"
          :readonly="readonly"
          regex="name"
          :model-value="modelValue"
          @update:model-value="emit('update:modelValue', $event)"
          @enter="emit('enter')"
          @enter-left="emit('enter')"
          @enter-right="emit('enter')"
          @click.stop
          @keyup.up.prevent="() => ({}) /* noop */"
          @keydown.down.prevent.stop="() => emit('navigateDown')"
        />
        <span
          class="cursor-text select-none text-3xl font-bold text-gray-400"
          v-if="modelValue?.trim().length == 0"
          @click="nameRef?.focus()"
        >
          Unnamed
        </span>
      </span>
      <!-- Actions -->
      <span
        class="ml-4 mt-1 flex flex-row gap-1"
        :class="[
          fatActions ? '' : ' opacity-0 transition group-focus-within/meta:opacity-100 group-hover/meta:opacity-100',
        ]"
      >
        <button
          v-for="action in actions.filter((a) => !a.disabled)"
          :key="action.label"
          class="group relative flex flex-row rounded-sm p-1"
          :class="[
            !action.disabled ? '' : 'opacity-50 hover:cursor-not-allowed',
            fatActions
              ? 'bg-orange-600 px-2 text-orange-50 hover:bg-orange-500 focus:bg-orange-500'
              : 'text-gray-300 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/meta:text-gray-500 group-hover/meta:text-gray-500',
          ]"
          @click="action.action(thing)"
          :disabled="action.disabled"
        >
          <component
            :is="action.active ? BusySpinnerIcon : action.icon"
            class="h-5 w-5"
            :class="action.active ? 'animate-spin' : ''"
          />
          <span v-if="fatActions" class="ml-1">{{ action.label }}</span>
          <!-- Label popover -->
          <span
            v-if="!action.active && !fatActions"
            class="pointer-events-none absolute -left-3 top-7 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-700 opacity-0 transition duration-150 group-hover:opacity-100"
          >
            {{ action.label }}
          </span>
        </button>
      </span>
    </span>
  </div>
</template>
