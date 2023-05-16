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
import { computed, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue?: SimpleType;
  inlined?: boolean;
  structrefOnly?: boolean;
  hideFlags?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<SimpleType, "name" | "tag" | "flags" | "reference">): void;
  (e: "escape"): void;
}>();

const value: Ref<SimpleType> = ref(props.modelValue ?? ANY_TYPE_NODE);
const query: Ref<string> = ref("");
const inputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);

const availableSymbols = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Type],
});
const availableTypes: Ref<SimpleType[]> = computed(() => {
  const basicTypes = [];
  if (!props.structrefOnly) {
    basicTypes.push(...PRIMITIVE_TYPE_NODES);
  }
  // references
  for (const symbol of availableSymbols.value) {
    if (symbol.name == null || (symbol.rootTypeTag != TypeTag.Struct && props.structrefOnly)) {
      continue;
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
  // keep flags
  type = {
    ...type,
    flags: value.value.flags,
  };
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
    label: "optional",
    icon: "?",
  },
  {
    flag: TypeFlag.IsArray,
    label: "list",
    icon: "[]",
  },
  {
    flag: TypeFlag.IsSecret,
    label: "secret",
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
  focus: () => inputRef.value?.$el.focus(),
});
</script>
<template>
  <Combobox as="div" :model-value="value" @update:model-value="writeValue">
    <!-- Flags -->
    <div v-if="!props.hideFlags" class="mb-2 flex flex-row justify-around">
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
    <ComboboxInput
      as="input"
      ref="inputRef"
      class="w-full rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:underline focus:ring-0"
      :class="{
        'font-mono': editor.fontMono,
        'text-sm placeholder:text-sm': editor.textSmall,
        'text-md placeholder:text-md': !editor.textSmall,
      }"
      @change="query = $event.target.value"
      :display-value="(node: any) => node != null ? renderSimpleType(node) : null"
      placeholder="..."
      spellcheck="false"
    />
    <ComboboxOptions
      ref="optionsRef"
      class="mt-2 max-h-48 w-60 overflow-auto"
      static
      :class="{ 'font-mono': editor.fontMono, 'text-sm': editor.textSmall, 'text-md': !editor.textSmall }"
    >
      <!-- Options -->
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
