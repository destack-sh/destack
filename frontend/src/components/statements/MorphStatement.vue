<script lang="ts" setup>
import type { Statement } from "@/state/module";
import { useStatementMorph, type MorphCommand } from "@/state/statement";
import { Combobox, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { ref, toRef } from "vue";

const props = defineProps<{ statement: Statement }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const { commands } = useStatementMorph(toRef(props, "statement"));
const commandOptionsRef = ref<InstanceType<typeof ComboboxOptions> | null>(null);

function selectCommand(command: MorphCommand) {
  command.action();
  emit("close");
}

defineExpose({
  focus: () => {
    commandOptionsRef.value?.$el.focus();
  },
  blur: () => {
    // nothing?
  },
});
</script>
<template>
  <!-- :MorphCommandStyle -->
  <Combobox as="div" class="relative flex w-full flex-col" @update:model-value="selectCommand($event)" by="label">
    <ComboboxOptions
      ref="commandOptionsRef"
      class="absolute z-50 flex h-fit max-h-[360px] w-[340px] flex-col gap-1 overflow-y-auto rounded-sm bg-white p-1 py-1 shadow-md ring-1 ring-orange-900 ring-opacity-20 focus:outline-none"
      static
    >
      <ComboboxOption v-for="(command, i) in commands" :key="command.label" :value="command" v-slot="{ active }">
        <div
          v-if="i == 0 || command.group != commands[i - 1]?.group"
          class="select-none px-2 py-1 text-xs font-semibold tracking-wide text-gray-500"
        >
          {{ command.group?.name }}
        </div>
        <li
          class="flex flex-row items-center justify-between gap-3"
          :class="['cursor-pointer select-none px-2 py-0.5', active ? 'bg-orange-100 text-gray-900' : 'text-gray-900']"
        >
          <div class="py-1">
            <div class="relative h-8 w-8 rounded-md bg-orange-500">
              <component :is="command.icon" class="absolute left-1.5 top-1.5 h-5 w-5 text-white" />
            </div>
          </div>
          <div class="flex flex-1 flex-col">
            <span class="font-semibold text-orange-600">
              {{ command.label }}
            </span>
            <span class="text-xs text-gray-700"> {{ command.description }}. </span>
          </div>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
