<script lang="ts" setup>
import { useStatementContext } from "@/components/statement";
import { StatementType } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD } from "@/state/editor";
import { fileOf, symbolsLike } from "@/state/runtime";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { CheckIcon } from "@heroicons/vue/24/outline";
import { useFocus } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{ canDefineInPlace?: boolean }>();
const emit = defineEmits<{
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "deleteRight"): void;
}>();

const context = useStatementContext();
const inputRef: Ref<HTMLButtonElement | null> = ref(null);
const inputRefFocus = useFocus(inputRef);
const selecting: Ref<boolean> = ref(false);

// TODO @Feature: use proper search for all searches (like uFuzzy)
const query = ref("");
const availableSymbols = symbolsLike({
  types: [StatementType.Definition],
});
const filteredSymbols = computed(() =>
  query.value === ""
    ? availableSymbols.value
    : availableSymbols.value.filter((s) => {
        return s.name?.toLowerCase().includes(query.value.toLowerCase());
      })
);

function setReference(ref: { id: string } | null) {
  selecting.value = false;
  if (ref == null && props.canDefineInPlace) {
    context.morphToDefinition(query.value);
  } else {
    context.setReference(ref);
  }
}

function escape() {
  if (selecting.value) {
    selecting.value = false;
  } else {
    emit("escape");
  }
}

defineExpose({
  focus: () => (inputRefFocus.focused.value = true),
  defocus: () => (inputRefFocus.focused.value = false),
});
</script>
<template>
  <button
    ref="inputRef"
    v-if="!context.editing.value || !selecting"
    @keydown.left.prevent="emit('navigateLeft')"
    @keydown.right.prevent="emit('navigateRight')"
    @keydown.enter.prevent="selecting = true"
    @click="selecting = true"
    class="rounded-sm outline-transparent focus:underline"
  >
    {{ context.reference.value?.name ?? "..." }}
  </button>
  <Combobox
    v-else
    as="div"
    class="relative"
    :model-value="context.reference"
    @update:model-value="setReference"
    nullable
  >
    <ComboboxInput
      as="input"
      ref="inputRef"
      class="rounded-sm border-0 p-0 font-mono outline-none ring-0 focus:underline focus:ring-0 sm:text-sm"
      @change="query = $event.target.value"
      :display-value="(stmt: any) => stmt?.name"
      placeholder="..."
      @keyup.escape="escape"
    />
    <!-- <ComboboxButton class="absolute inset-y-0 right-0 flex items-center rounded-r-md px-2 focus:outline-none">
      <ChevronUpDownIcon class="h-4 w-4 text-gray-400" aria-hidden="true" />
    </ComboboxButton> -->

    <ComboboxOptions
      v-if="filteredSymbols.length > 0 || canDefineInPlace"
      class="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-sm bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
    >
      <!-- define in-place option (weirdly, value must be null not {} or headlessui will freak) -->
      <ComboboxOption :key="0" :value="null" v-slot="{ active }">
        <li
          :class="[
            'relative flex cursor-default select-none items-baseline justify-between py-0.5 px-2 font-mono text-sm',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
          ]"
        >
          {{ query }}:
          <span class="text-xs" :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']">
            define
          </span>
        </li>
      </ComboboxOption>
      <!-- actual reference options -->
      <ComboboxOption
        v-for="stmt in filteredSymbols"
        :key="stmt.id"
        :value="stmt"
        as="template"
        v-slot="{ active, selected }"
      >
        <li
          :class="[
            'relative cursor-default select-none py-0.5 pr-9 font-mono text-sm',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
          ]"
        >
          <div class="flex items-baseline justify-between">
            <span :class="['truncate', selected && 'font-semibold']">
              {{ SYMBOL_TYPE_KEYWORD[stmt.symbolType] }}
              {{ stmt.name }}
            </span>
            <span class="text-xs" :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']">
              {{ fileOf(stmt)?.path }}
            </span>
          </div>

          <span
            v-if="selected"
            :class="['absolute inset-y-0 right-0 flex items-center pr-2', active ? 'text-white' : 'text-orange-600']"
          >
            <CheckIcon class="h-4 w-4" aria-hidden="true" />
          </span>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
