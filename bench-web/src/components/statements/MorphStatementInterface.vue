<script lang="ts" setup>
import type { Statement } from "@/state/module";
import { useStatementMorph, type MorphCommand, getMorphIdentity } from "@/state/statement";
import { Combobox, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { computed, ref, toRef } from "vue";

const props = defineProps<{ statement: Statement }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const morphIdentity = computed(() => getMorphIdentity(props.statement));
const { filteredCommands: commands, doMorph } = useStatementMorph(toRef(props, "statement"), ref(true));
const commandOptionsRef = ref<InstanceType<typeof ComboboxOptions> | null>(null);

function selectCommand(command: MorphCommand) {
  doMorph(props.statement, { ...command.identity, name: props.statement.name }, command);
  command.action?.();
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
  <Combobox as="div" class="relative flex w-full flex-col" @update:model-value="selectCommand($event)">
    <!-- :NestedActionComponentWidth -->
    <ComboboxOptions
      ref="commandOptionsRef"
      class="flex h-fit max-h-[360px] w-[160px] flex-col gap-1 overflow-y-auto p-1 py-1"
      static
    >
      <ComboboxOption v-for="command in commands" :key="command.label" :value="command" v-slot="{ active }">
        <li
          class="flex flex-row items-center justify-between gap-2"
          :class="[
            'cursor-pointer select-none px-2 py-0.5',
            active ? 'bg-orange-100 ' : '',
            morphIdentity == command.identity ? 'text-orange-600' : 'text-gray-900',
          ]"
        >
          <component :is="command.iconOutline" class="h-4 w-4 text-gray-900" />
          <span class="flex flex-1 flex-col">
            {{ command.label }}
          </span>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
