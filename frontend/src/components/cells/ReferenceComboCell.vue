<script lang="ts" setup>
import { useStatementContext } from "@/components/statement";
import { StatementType, type InterpSymbol } from "@/gql/graphql";
import { SYMBOL_TYPE_BY_KEYWORD, SYMBOL_TYPE_KEYWORD } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { fileOf, symbolsLike } from "@/state/runtime";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { onStartTyping, useFocus } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{ canDefineInPlace?: boolean }>();
const emit = defineEmits<{
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "defineInPlace", name: string): void;
  (e: "referenceSet", ref: InterpSymbol): void;
}>();

const context = useStatementContext();
const inputRef: Ref<HTMLButtonElement | null> = ref(null);
const inputRefFocus = useFocus(inputRef);
const selecting: Ref<boolean> = ref(false);

// TODO @Feature: use proper search for all searches (like uFuzzy)
const query = ref("");
// TODO @Robustness: trim reference selection to reachable symbols (from runtime)
const availableSymbols = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: context.statement.value.symbolType != null ? [context.statement.value?.symbolType] : undefined,
});
const filteredSymbols = computed(() =>
  query.value === ""
    ? availableSymbols.value.filter((s) => s.id != context.statement.value.id)
    : availableSymbols.value
        .filter((s) => s.id != context.statement.value.id)
        .filter((s) => {
          return s.name?.toLowerCase().includes(query.value.toLowerCase());
        })
);

// set symbol type if query starts with it and it's not yet set (like in SelectTypeCell)
// (used in ProtoCell)
watch(query, (newContent) => {
  if (context.statement.value.type != StatementType.Blank || context.statement.value.symbolType != null) {
    return;
  }
  // :ParseStatementInput
  const endsInSpace = newContent.endsWith(" ") || newContent.endsWith(" "); // non-breaking spaces
  newContent = newContent.trim();
  if (endsInSpace && SYMBOL_TYPE_BY_KEYWORD[newContent]) {
    context.setSymbolType(SYMBOL_TYPE_BY_KEYWORD[newContent]);
    // reset query
    query.value = "";
    // inputRef must be a ComboboxInput
    (inputRef.value?.$el as HTMLInputElement).value = "";
  }
});

// define in place if query ends with :
watch(
  () => [query.value, props.canDefineInPlace],
  () => {
    if (props.canDefineInPlace && query.value.length > 1 && query.value.endsWith(":")) {
      emit("defineInPlace", query.value.slice(0, -1));
    }
  }
);

const operations = useOperations();
function setReference(ref: InterpSymbol | null) {
  if (ref == null && query.value.length < 1) {
    // headless ui auto-selects an option when it matches the name
    // but we don't want that if we are defining in place
    return;
  }
  selecting.value = false;
  if (ref == null && props.canDefineInPlace) {
    emit("defineInPlace", query.value);
  } else {
    // TODO @Cleanup: setting reference and naming a reference shouldn't be separate
    //  Indeed, we probably don't want names on references at all (creates weird aliasing).
    operations.statement.rename(context.statement.value.id, context.statement.value.name ?? null, ref.name);
    context.setReference(ref);
    if (ref != null) {
      emit("referenceSet", ref);
    }
  }
}

function deleteLeftIfAtStart(event: KeyboardEvent) {
  // check if cursor is at start of the text
  const input = event.target as HTMLInputElement;
  if (input.selectionStart == input.selectionEnd && input.selectionStart == 0) {
    event.preventDefault();
    event.stopPropagation();
    emit("deleteLeft");
  }
}

function escape() {
  if (selecting.value) {
    selecting.value = false;
    // focus button once we've switched back
    nextTick(() => {
      inputRefFocus.focused.value = true;
    });
  } else {
    emit("escape");
  }
}

function open() {
  selecting.value = true;
  // focus input ref once we've switched to the combobox
  nextTick(() => {
    inputRefFocus.focused.value = true;
  });
}

// auto-open if the user starts typing and this is focused
onStartTyping(() => {
  if (query.value.length == 0 && inputRefFocus.focused.value) {
    open();
  }
});

defineExpose({
  focus: () => (inputRefFocus.focused.value = true),
  blur: () => ((inputRefFocus.focused.value = false), (selecting.value = false)),
  open,
});
</script>
<template>
  <button
    ref="inputRef"
    v-if="!selecting"
    tabeindex="-1"
    @keydown.left.exact.prevent="emit('navigateLeft')"
    @keydown.right.exact.prevent="emit('navigateRight')"
    @keydown.up.exact.prevent="emit('navigateUp')"
    @keydown.down.exact.prevent="emit('navigateDown')"
    @keydown.enter.exact.prevent="open"
    @keydown.delete.exact.prevent="emit('deleteLeft')"
    @click="open"
    class="rounded-sm outline-transparent focus:underline"
  >
    {{ context.reference.value?.name ?? context.statement.value.name ?? "..." }}
  </button>
  <Combobox
    v-else
    as="div"
    class="relative"
    :model-value="context.reference.value"
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
      @keydown.escape.prevent=""
      @keyup.escape.prevent="escape"
      @keydown.shift.enter.exact.prevent="context.insertBelow"
      @keydown.delete="deleteLeftIfAtStart"
    />
    <ComboboxOptions
      ref="optionsRef"
      v-if="filteredSymbols.length > 0 || canDefineInPlace"
      class="absolute z-10 mt-1 max-h-60 w-80 overflow-auto rounded-sm bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
      static
      v-show="selecting"
    >
      <!-- define in-place option (weirdly, value must not be {} or headlessui will freak) -->
      <ComboboxOption v-if="query.length > 0" :key="0" :value="null" v-slot="{ active }">
        <li
          :class="[
            'relative flex cursor-default select-none items-baseline justify-between py-0.5 px-2 font-mono text-sm',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
          ]"
        >
          {{ query }}:
          <span class="text-xs" :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']">
            (define)
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
            'relative cursor-default select-none py-0.5 px-2 font-mono text-sm',
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
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
