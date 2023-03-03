<script lang="ts" setup>
import {
  ANY_TYPE_NODE,
  makeTypeNode,
  PRIMITIVE_TYPE_NODES,
  renderSimpleType,
  type SimpleType,
} from "@/components/statement";
import { StatementType, SymbolType, TypeTag, type SimpleTypeNode } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { symbolsLike } from "@/state/runtime";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { onClickOutside, useFocus } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const props = defineProps<{
  modelValue?: SimpleType;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<SimpleType, "tag" | "reference">): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "enter"): void;
  (e: "escape"): void;
}>();

const value: Ref<SimpleType> = ref(props.modelValue ?? ANY_TYPE_NODE);
const editing: Ref<boolean> = ref(false);
const query: Ref<string> = ref("");
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const valueRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);
const valueRefFocused = useFocus(valueRef as any);
const optionsRef: Ref<HTMLDivElement | null> = ref(null);

const availableSymbols = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Type],
});
const availableTypes: Ref<SimpleType[]> = computed(() => {
  const basicTypes = [...PRIMITIVE_TYPE_NODES];
  // references
  for (const symbol of availableSymbols.value) {
    if (symbol.name == null) {
      continue; // ignore, shouldn't happen
    }
    basicTypes.push(
      makeTypeNode({
        tag: TypeTag.TypeReference,
        reference: symbol as { id: string; name: string },
      })
    );
  }
  // combine basic types with nullable & array options
  return [
    ...basicTypes,
    ...basicTypes
      .filter((t) => t.tag != TypeTag.Any && t.tag != TypeTag.Null)
      .map((t) => makeTypeNode({ ...t, isNullable: true })),
    ...basicTypes
      .filter((t) => t.tag != TypeTag.Any && t.tag != TypeTag.Null)
      .map((t) => makeTypeNode({ ...t, isArray: true })),
  ];
});

const filteredTypes = computed(() => availableTypes.value.filter((t) => renderSimpleType(t).includes(query.value)));

function writeValue(type: SimpleTypeNode) {
  editing.value = false;
  nextTick(() => buttonRef.value?.focus());
  value.value = type;
  emit("update:modelValue", type);
  emit("escape");
}

onClickOutside(optionsRef, (event: PointerEvent) => {
  // if outside options and not within valueRef stop editing
  if (event.target != valueRef.value?.$el) {
    editing.value = false;
  }
});

function edit() {
  editing.value = true;
  nextTick(() => (valueRefFocused.focused.value = true));
}

function cancel() {
  editing.value = false;
  nextTick(() => buttonRef.value?.focus());
  emit("escape");
}

function focus() {
  query.value = "";
  if (!editing.value) {
    buttonRef.value?.focus();
  } else {
    valueRefFocused.focused.value = true;
  }
}

const editor = useEditorState();

defineExpose({
  editing,
  focus,
  blur: () => {
    query.value = "";
    buttonRef.value?.blur();
    valueRefFocused.focused.value = false;
  },
});
</script>
<template>
  <button
    v-if="!editing"
    ref="buttonRef"
    tabeindex="-1"
    @keydown.left.exact.prevent="emit('navigateLeft')"
    @keydown.right.exact.prevent="emit('navigateRight')"
    @keydown.up.exact.prevent="emit('navigateUp')"
    @keydown.down.exact.prevent="emit('navigateDown')"
    @keydown.enter.exact.prevent="edit"
    @click="edit"
    :disabled="readonly"
    class="text-left outline-none"
  >
    {{ renderSimpleType(value) }}
  </button>
  <!-- Editable type :EditableCellStyle -->
  <Combobox v-else as="div" class="relative" :model-value="value" @update:model-value="writeValue">
    <ComboboxInput
      as="input"
      ref="valueRef"
      class="absolute -left-0.5 -top-0.5 z-10 rounded-sm border border-black bg-orange-50 p-1 outline-none ring-0 focus:border-black focus:underline focus:ring-0"
      :class="{
        'font-mono': editor.fontMono,
        'text-sm placeholder:text-sm': editor.textSmall,
        'text-md placeholder:text-md': !editor.textSmall,
      }"
      @change="query = $event.target.value"
      :display-value="(node: any) => node != null ? renderSimpleType(node) : null"
      placeholder="..."
      @keydown.escape.prevent=""
      @keyup.escape.prevent="cancel"
    />
    <ComboboxOptions
      ref="optionsRef"
      class="absolute z-20 mt-8 max-h-60 w-60 overflow-auto rounded-sm bg-white py-1 text-base shadow-md ring-1 ring-black ring-opacity-5 focus:outline-none"
      static
      v-show="editing"
      :class="{ 'font-mono': editor.fontMono, 'text-sm': editor.textSmall, 'text-md': !editor.textSmall }"
    >
      <ComboboxOption v-for="node in filteredTypes" :key="node.id" :value="node" v-slot="{ active, selected }">
        <li
          :class="[
            'relative cursor-default select-none py-0.5 px-2 ',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
            selected ? 'underline' : '',
          ]"
        >
          {{ renderSimpleType(node) }}
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
