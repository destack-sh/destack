<script lang="ts" setup>
import SimpleTypePreview from "@/components/cells/SimpleTypePreview.vue";
import { ANY_TYPE_NODE, makeTypeNode, type SimpleType } from "@/components/statement";
import { StatementType, SymbolType, TypeHint, TypeTag, type SimpleTypeNode } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { renderSimpleType, SUPPORTED_TYPEHINTS } from "@/state/editor";
import { fileOf, symbolsLike, TypeFlag } from "@/state/runtime";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import {
  ExclamationCircleIcon,
  ListBulletIcon,
  LockClosedIcon,
  LockOpenIcon,
  QuestionMarkCircleIcon,
} from "@heroicons/vue/24/outline";
import { computed, onMounted, ref, watch, type Ref } from "vue";

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

const BUILTIN_TYPES: (TypeHint | TypeTag)[] = [
  TypeTag.String,
  TypeTag.Boolean,
  TypeTag.Number,
  TypeTag.File,
  TypeTag.Embedding,
  ...(Object.keys(SUPPORTED_TYPEHINTS) as TypeHint[]),
];
const BUILTINS_TYPES_NODES = BUILTIN_TYPES.map((tag) => {
  if (Object.values(TypeTag).includes(tag as TypeTag)) {
    return makeTypeNode({ tag: tag as TypeTag });
  } else if (tag in SUPPORTED_TYPEHINTS) {
    return makeTypeNode({ tag: SUPPORTED_TYPEHINTS[tag as TypeHint] as TypeTag, hint: tag as TypeHint });
  } else {
    throw new Error(`unknown primitive ${tag} ${typeof tag} ${Object.keys(TypeTag)}`);
  }
});

const availableSymbols = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Type],
});
const availableTypes: Ref<SimpleType[] & { primitive?: boolean }> = computed(() => {
  const basicTypes = [];
  // builtin types
  if (!props.structrefOnly) {
    basicTypes.push(...BUILTINS_TYPES_NODES.map((t) => ({ ...t, primitive: true })));
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
const filteredTypes = computed(() =>
  availableTypes.value.filter((t) => renderSimpleType(t).toLowerCase().includes(query.value.toLowerCase()))
);

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
    setIcon: QuestionMarkCircleIcon,
    unsetIcon: ExclamationCircleIcon,
  },
  {
    flag: TypeFlag.IsArray,
    label: "many",
    setIcon: ListBulletIcon,
    unsetIcon: ListBulletIcon,
  },
  {
    flag: TypeFlag.IsSecret,
    label: "secret",
    unsetIcon: LockOpenIcon,
    setIcon: LockClosedIcon,
  },
];

function isFlagEnabled(flag: TypeFlag) {
  if (flag == TypeFlag.IsArray) {
    return !isFlagSet(TypeFlag.IsSecret);
  } else if (flag == TypeFlag.IsSecret) {
    return !isFlagSet(TypeFlag.IsArray) && value.value.reference == null;
  } else {
    return true;
  }
}

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

// focus input once mounted
onMounted(() => {
  inputRef.value?.$el.focus();
});

const appearance = useAppearance();

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
        :disabled="!isFlagEnabled(flagButton.flag)"
        class="flex flex-row items-center gap-1 rounded-sm px-1 py-0.5 hover:bg-orange-100"
        :class="[
          isFlagSet(flagButton.flag) ? 'font-bold text-orange-600' : '',
          isFlagEnabled(flagButton.flag) ? '' : 'cursor-not-allowed text-gray-400',
        ]"
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
        'font-mono': appearance.fontMono,
        'text-sm placeholder:text-sm': appearance.textSmall,
        'text-md placeholder:text-md': !appearance.textSmall,
      }"
      @change="query = $event.target.value"
      :display-value="(el) => null"
      placeholder="..."
      spellcheck="false"
      @keydown.enter.prevent.stop="emit('escape')"
    />
    <ComboboxOptions
      ref="optionsRef"
      class="mt-1 max-h-48 w-60 overflow-auto"
      static
      :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
    >
      <!-- Options -->
      <ComboboxOption v-for="node in filteredTypes" :key="node.id" :value="node" v-slot="{ active, selected }">
        <li
          :class="[
            'relative cursor-default select-none px-1 py-1 text-gray-900',
            active ? 'bg-orange-100' : '',
            selected ? 'text-orange-600' : '',
          ]"
        >
          <div class="flex items-baseline justify-between">
            <SimpleTypePreview :type="node" show-type-name />
            <!-- Ref source -->
            <span
              v-if="node.tag == TypeTag.TypeReference && node.reference != null"
              class="text-xs"
              :class="['truncate', active ? 'text-gray-700' : 'text-gray-500']"
            >
              {{ fileOf(node.reference)?.path }}
            </span>
            <!-- Builtin -->
            <span v-else-if="node.primitive" class="text-xs" :class="[active ? 'text-gray-700' : 'text-gray-500']"
              >(builtin)</span
            >
          </div>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
