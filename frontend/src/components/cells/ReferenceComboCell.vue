<script lang="ts" setup>
import type { InterpSymbol } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD, useEditorState, type StatementHeader } from "@/state/editor";
import { fileOf, relativePath, symbolOf, useSymbolNavigation } from "@/state/runtime";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { onClickOutside, onStartTyping, useFocus } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{
  self?: StatementHeader;
  reference?: StatementHeader | InterpSymbol | null;
  canDefineInPlace?: boolean;
  canDefineAnonymous?: boolean;
  availableSymbols: InterpSymbol[];
}>();
const emit = defineEmits<{
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "insertBelow"): void;
  (e: "escape"): void;
  (e: "deleteLeft"): void;
  (e: "defineInPlace", name: string): void;
  (e: "setReference", ref: InterpSymbol | null): void;
}>();

const inputRef: Ref<HTMLButtonElement | null> = ref(null);
const inputRefFocus = useFocus(inputRef);
const optionsRef: Ref<HTMLDivElement | null> = ref(null);
const selecting: Ref<boolean> = ref(false);

// TODO @Feature: use proper search for all searches (like uFuzzy)
const query = ref("");
const filteredSymbols = computed(() =>
  query.value === ""
    ? props.availableSymbols.filter((s) => s.id != props.self?.id)
    : props.availableSymbols
        .filter((s) => s.id != props.self?.id)
        .filter((s) => {
          return s.name?.toLowerCase().includes(query.value.toLowerCase());
        })
);

// define in place if query ends with :
watch(
  () => [query.value, props.canDefineInPlace, props.canDefineAnonymous],
  () => {
    if (props.canDefineInPlace && (query.value.length > 1 || props.canDefineAnonymous) && query.value.endsWith(":")) {
      emit("defineInPlace", query.value.slice(0, -1));
    }
  }
);

function setReference(ref: InterpSymbol | null) {
  // if (ref == null && query.value.length < 1) {
  //   // headless ui auto-selects an option when it matches the name
  //   // but we don't want that if we are defining in place
  //   return;
  // }
  selecting.value = false;
  if (ref == null && props.canDefineInPlace) {
    emit("defineInPlace", query.value);
  } else {
    emit("setReference", ref);
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

onClickOutside(optionsRef, (event: PointerEvent) => {
  // if outside options and not within valueRef stop editing
  if (event.target != inputRef.value?.$el) {
    selecting.value = false;
    escape();
  }
});

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

// alt-click focuses source reference
const { focusSymbol } = useSymbolNavigation();
function clickOpen(click: MouseEvent) {
  if (click.altKey) {
    if (props.reference != null) {
      focusSymbol(props.reference);
    }
  } else {
    open();
  }
}

// auto-open if the user starts typing and this is focused
onStartTyping(() => {
  if (query.value.length == 0 && inputRefFocus.focused.value) {
    open();
  }
});

function clearQuery() {
  query.value = "";
  // inputRef must be a ComboboxInput
  (inputRef.value?.$el as HTMLInputElement).value = "";
}

const selfSymbol = computed(() => symbolOf(props.self?.id));

function importSourceTo(symbol: InterpSymbol): string | undefined {
  const localFile = fileOf(symbol);
  if (localFile != null) {
    return "." + localFile.path;
  } else if (selfSymbol.value != null) {
    return relativePath(selfSymbol.value, symbol);
  }
  return undefined;
}

const editor = useEditorState();

defineExpose({
  focus: () => (inputRefFocus.focused.value = true),
  blur: () => ((inputRefFocus.focused.value = false), (selecting.value = false)),
  open,
  query,
  clearQuery,
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
    @keydown.shift.enter.exact.prevent="emit('insertBelow')"
    @keydown.delete.exact.prevent="emit('deleteLeft')"
    @click="clickOpen"
    class="rounded-sm decoration-dashed underline-offset-4 outline-transparent focus:underline focus:decoration-gray-900 focus:decoration-solid group-hover/statement:decoration-gray-900"
    :class="reference?.name ?? self?.name != null ? 'underline decoration-gray-400' : ''"
  >
    {{ reference?.name ?? self?.name ?? "..." }}
  </button>
  <Combobox v-else as="div" class="relative" :model-value="reference" @update:model-value="setReference" nullable>
    <ComboboxInput
      as="input"
      ref="inputRef"
      class="max-w-80 w-auto min-w-fit rounded-sm border-0 p-0 underline-offset-4 outline-none ring-0 focus:underline focus:ring-0 sm:text-sm"
      :class="{ 'font-mono': editor.fontMono }"
      @change="query = $event.target.value"
      :display-value="(stmt: any) => stmt?.name"
      placeholder="..."
      @keydown.escape.prevent=""
      @keyup.escape.prevent="escape"
      @keydown.shift.enter.exact.prevent="emit('insertBelow')"
      @keydown.delete="deleteLeftIfAtStart"
    />
    <ComboboxOptions
      ref="optionsRef"
      v-if="filteredSymbols.length > 0 || canDefineInPlace"
      class="absolute z-10 mt-1 max-h-60 w-80 overflow-auto rounded-sm bg-white py-1 text-base shadow-md ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
      static
      v-show="selecting"
    >
      <!-- define in-place option (weirdly, value must not be {} or headlessui will freak) -->
      <ComboboxOption v-if="query.length > 0 || canDefineAnonymous" :key="0" :value="null" v-slot="{ active }">
        <li
          :class="[
            'relative flex cursor-default select-none items-baseline justify-between py-0.5 px-2  text-sm',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
            editor.fontMono ? 'font-mono' : '',
          ]"
        >
          <span v-if="query.trim().length > 0">{{ query }}:</span>
          <span v-else class="">(unnamed)</span>
          <span class="text-xs" :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']">
            (define)
          </span>
        </li>
      </ComboboxOption>
      <!-- actual reference options -->
      <ComboboxOption
        v-for="symbol in filteredSymbols"
        :key="symbol.id"
        :value="symbol"
        as="template"
        v-slot="{ active, selected }"
      >
        <li
          :class="[
            'relative cursor-default select-none py-0.5 px-2 text-sm',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
            selected && !active ? 'text-orange-600' : '',
            editor.fontMono ? 'font-mono' : '',
          ]"
        >
          <div class="flex items-baseline justify-between">
            <span :class="['truncate']">
              {{ SYMBOL_TYPE_KEYWORD[symbol.symbolType] }}
              {{ symbol.name }}
            </span>
            <span class="text-xs" :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']">
              {{ importSourceTo(symbol) ?? "???" }}
            </span>
          </div>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
