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
import { fileOf, symbolsLike, TypeFlag } from "@/state/runtime";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { EyeIcon, EyeSlashIcon } from "@heroicons/vue/24/outline";
import { onClickOutside, useFocus } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

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
  (e: "deleteSelf"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "focus", event: FocusEvent): void;
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
  return basicTypes;
});
const filteredTypes = computed(() => availableTypes.value.filter((t) => renderSimpleType(t).includes(query.value)));

function writeValue(type: SimpleTypeNode) {
  editing.value = false;
  nextTick(() => buttonRef.value?.focus());
  value.value = type;
  emit("update:modelValue", type);
  emit("escape");
}

// sync modelValue if changed externally
watch(
  () => [props.modelValue],
  () => {
    if (props.modelValue != value.value) {
      value.value = props.modelValue ?? ANY_TYPE_NODE;
    }
  }
);

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
  nextTick(() => buttonRef.value?.focus({ preventScroll: true }));
  emit("escape");
}

function focus() {
  query.value = "";
  if (!editing.value) {
    buttonRef.value?.focus({ preventScroll: true });
  } else {
    valueRefFocused.focused.value = true;
  }
}

type FlagButton = {
  flag: TypeFlag;
  label: string;
  icon?: string;
  unsetIcon?: any;
  setIcon?: any;
};
const flagButtons: FlagButton[] = [
  {
    flag: TypeFlag.IsNullable,
    label: "Optional",
    icon: "?",
  },
  {
    flag: TypeFlag.IsArray,
    label: "List",
    icon: "[]",
  },
  {
    flag: TypeFlag.IsSecret,
    label: "Secret",
    unsetIcon: EyeIcon,
    setIcon: EyeSlashIcon,
  },
];

function isFlagSet(flag: TypeFlag) {
  return value.value.flags & flag;
}

function toggleFlag(flag: TypeFlag) {
  const newFlags = value.value.flags ^ flag;
  value.value = {
    ...value.value,
    flags: newFlags,
  };
  emit("update:modelValue", value.value);
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
    tabindex="-1"
    :disabled="readonly"
    @click="edit"
    @keydown.enter.exact.prevent="edit"
    @keydown.left.exact.prevent="emit('navigateLeft')"
    @keydown.right.exact.prevent="emit('navigateRight')"
    @keydown.up.exact.prevent="emit('navigateUp')"
    @keydown.down.exact.prevent="emit('navigateDown')"
    @keydown.delete.exact="editing || emit('deleteSelf')"
    @focus.stop.prevent="emit('focus', $event)"
    class="text-left outline-none"
  >
    {{ renderSimpleType(value) }}
  </button>
  <!-- Editable type :EditableCellStyle -->
  <Combobox v-else as="div" class="relative" :model-value="value" @update:model-value="writeValue">
    <ComboboxInput
      as="input"
      ref="valueRef"
      class="absolute -left-0.5 -top-0.5 z-10 rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 p-1 outline-none ring-0 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:underline focus:ring-0"
      :class="{
        'font-mono': editor.fontMono,
        'text-sm placeholder:text-sm': editor.textSmall,
        'text-md placeholder:text-md': !editor.textSmall,
      }"
      @change="query = $event.target.value"
      :display-value="(node: any) => node != null ? renderSimpleType(node) : null"
      placeholder="..."
      spellcheck="false"
      @keydown.escape.prevent=""
      @keyup.escape.prevent="cancel"
    />
    <ComboboxOptions
      ref="optionsRef"
      class="absolute z-20 mt-8 max-h-60 w-60 overflow-auto rounded-sm bg-white py-1 text-base shadow-md ring-1 ring-orange-900 ring-opacity-20 focus:outline-none"
      static
      v-show="editing"
      :class="{ 'font-mono': editor.fontMono, 'text-sm': editor.textSmall, 'text-md': !editor.textSmall }"
    >
      <div class="flex flex-row justify-around px-2 py-1">
        <button
          v-for="flagButton in flagButtons"
          :key="flagButton.label"
          class="flex flex-row items-center gap-1 rounded-sm px-1 py-0.5 hover:bg-orange-100"
          :class="isFlagSet(flagButton.flag) ? 'font-bold text-orange-600' : ''"
          @click="toggleFlag(flagButton.flag)"
        >
          <span> {{ flagButton.label }}</span>
          <span v-if="flagButton.icon">{{ flagButton.icon }}</span>
          <component
            v-else
            :is="isFlagSet(flagButton.flag) ? flagButton.setIcon : flagButton.unsetIcon"
            class="h-4 w-4"
          />
        </button>
      </div>
      <ComboboxOption v-for="node in filteredTypes" :key="node.id" :value="node" v-slot="{ active, selected }">
        <li
          :class="[
            'relative cursor-default select-none px-2 py-0.5',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
            selected ? 'underline' : '',
          ]"
        >
          <div class="flex items-baseline justify-between">
            <span class="truncate">
              {{ renderSimpleType(node) }}
            </span>
            <span
              v-if="node.tag == TypeTag.TypeReference && node.reference != null"
              class="text-xs"
              :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']"
            >
              {{ fileOf(node.reference)?.path }}
            </span>
          </div>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
