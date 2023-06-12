<script lang="ts" setup>
import TypePreview from "@/components/interfaces/TypePreview.vue";
import { ANY_FIELD, makeField, type SimpleType } from "@/state/statement";
import { StatementType, TypeHint, TypeTag, type Field } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { renderBuiltinType, SUPPORTED_TYPEHINTS } from "@/state/type";
import { TypeFlag, useCurrentModule } from "@/state/module";
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

const value: Ref<SimpleType> = ref(props.modelValue ?? ANY_FIELD);
const query: Ref<string> = ref("");
const inputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);

const module = useCurrentModule();

const BUILTIN_TYPES: (TypeHint | TypeTag)[] = [
  TypeTag.String,
  TypeTag.Boolean,
  TypeTag.Number,
  TypeTag.File,
  TypeTag.Vector,
  ...(Object.keys(SUPPORTED_TYPEHINTS) as TypeHint[]),
];
const BUILTINS_TYPES_NODES = BUILTIN_TYPES.map((tag) => {
  if (Object.values(TypeTag).includes(tag as TypeTag)) {
    return makeField({ tag: tag as TypeTag });
  } else if (tag in SUPPORTED_TYPEHINTS) {
    return makeField({ tag: SUPPORTED_TYPEHINTS[tag as TypeHint] as TypeTag, hint: tag as TypeHint });
  } else {
    throw new Error(`unknown primitive ${tag} ${typeof tag} ${Object.keys(TypeTag)}`);
  }
});

const availableSymbols = module.statementsLike({
  types: [StatementType.Type],
});
const availableTypes: Ref<SimpleType[] & { primitive?: boolean }> = computed(() => {
  const types = [];
  // builtin types
  if (!props.structrefOnly) {
    types.push(...BUILTINS_TYPES_NODES.map((t) => ({ ...t, primitive: true })));
  }
  // references
  for (const symbol of availableSymbols.value) {
    if (symbol.name == null || (symbol.rootTypeTag != TypeTag.Struct && props.structrefOnly)) {
      continue;
    }
    types.push(
      makeField({
        tag: TypeTag.TypeReference,
        reference: symbol as { id: string; name: string },
      })
    );
  }
  return types;
});
const filteredTypes = computed(() =>
  availableTypes.value.filter((t) => renderSimpleType(t).toLowerCase().includes(query.value.toLowerCase()))
);

function writeValue(type: Field) {
  // keep supported flags
  let newFlags = TypeFlag.Zero;
  for (let flag of Object.values(TypeFlag)) {
    flag = flag as TypeFlag;
    if (isFlagSet(flag) && isFlagSupported(type, flag)) {
      newFlags |= flag;
    }
  }
  type = {
    ...type,
    flags: newFlags,
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
      value.value = props.modelValue ?? ANY_FIELD;
    }
  }
);

type FlagButton = {
  flag: TypeFlag;
  invert: boolean;
  label: string;
  icon?: string;
  unsetIcon?: any;
  setIcon?: any;
};
const flagButtons: FlagButton[] = [
  {
    flag: TypeFlag.IsNullable,
    invert: true,
    label: "required",
    setIcon: ExclamationCircleIcon,
    unsetIcon: QuestionMarkCircleIcon,
  },
  {
    flag: TypeFlag.IsArray,
    invert: false,
    label: "many",
    setIcon: ListBulletIcon,
    unsetIcon: ListBulletIcon,
  },
  {
    flag: TypeFlag.IsSecret,
    invert: false,
    label: "secret",
    unsetIcon: LockOpenIcon,
    setIcon: LockClosedIcon,
  },
];

// constraint list & secret flags to UX-sensible types
// (internally we could support any permutation)
const NONNULL_TAGS = [TypeTag.Boolean];
const LISTABLE_TAGS = [TypeTag.File, TypeTag.TypeReference, TypeTag.Struct, TypeTag.Enum];
const LISTABLE_HINTS = [
  TypeHint.Name,
  TypeHint.Email,
  TypeHint.Phone,
  TypeHint.Url,
  TypeHint.Uuid,
  TypeHint.Audio,
  TypeHint.Image,
  TypeHint.Video,
];
const SECRETABLE_TAGS = [TypeTag.String, TypeTag.Number];
function isFlagSupported(type: SimpleType, flag: TypeFlag) {
  if (flag == TypeFlag.IsNullable) {
    return !isFlagSet(TypeFlag.IsArray) && !NONNULL_TAGS.includes(type.tag);
  } else if (flag == TypeFlag.IsArray) {
    return (
      isFlagSet(TypeFlag.IsNullable) &&
      !isFlagSet(TypeFlag.IsSecret) &&
      ((type.hint != null && LISTABLE_HINTS.includes(type.hint)) || LISTABLE_TAGS.includes(type.tag))
    );
  } else if (flag == TypeFlag.IsSecret) {
    return !isFlagSet(TypeFlag.IsArray) && SECRETABLE_TAGS.includes(type.tag);
  } else {
    return true;
  }
}

function isFlagSet(flag: TypeFlag): boolean {
  return Boolean(value.value.flags & flag);
}

function toggleFlag(flag: TypeFlag) {
  const newFlags = value.value.flags ^ flag;
  value.value = {
    ...value.value,
    flags: newFlags,
  };
  emit("update:modelValue", value.value);
}

function renderSimpleType(node: SimpleType): string {
  const builtin = renderBuiltinType(node.tag, node.hint ?? null);
  if (builtin != null) return builtin;
  if (node.tag == TypeTag.TypeReference || node.reference != null) {
    if (node.reference != null) {
      return module.statementOf(node.reference.id)?.name ?? "???";
    } else {
      return node.reference?.name ?? "...";
    }
  }
  throw new Error(`unexpected type node: ${JSON.stringify(node)}`);
}

// use 'combobox id' as a stable id

function toComboId(type: SimpleType) {
  return `${type.tag}.${type.hint ?? ""}.${type.reference?.id ?? ""}`;
}

function findByComboId(id: string) {
  return availableTypes.value.find((t) => toComboId(t) == id);
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
  <Combobox
    as="div"
    :model-value="toComboId(value)"
    @update:model-value="(id: any) => writeValue(findByComboId(id) ?? value)"
  >
    <!-- Flags -->
    <div v-if="!props.hideFlags" class="mb-2 flex flex-row justify-around">
      <button
        v-for="flagButton in flagButtons"
        :key="flagButton.label"
        :disabled="!isFlagSupported(value, flagButton.flag)"
        class="flex flex-row items-center gap-1 rounded-sm px-1 py-0.5 hover:bg-orange-100"
        :class="[
          isFlagSet(flagButton.flag) !== flagButton.invert ? 'font-bold text-orange-600' : '',
          isFlagSupported(value, flagButton.flag) ? 'text-gray-600' : 'cursor-not-allowed text-gray-400',
          isFlagSet(flagButton.flag) !== flagButton.invert && flagButton.flag == TypeFlag.IsNullable
            ? 'underline underline-offset-4'
            : '',
        ]"
        @click="toggleFlag(flagButton.flag)"
      >
        <span> {{ flagButton.label }}</span>
        <span v-if="flagButton.icon">{{ flagButton.icon }}</span>
        <component
          v-else
          :is="isFlagSet(flagButton.flag) !== flagButton.invert ? flagButton.setIcon : flagButton.unsetIcon"
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
      :display-value="(el: any) => renderSimpleType(findByComboId(el) ?? value)"
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
      <ComboboxOption
        v-for="node in filteredTypes"
        :key="node.id"
        :value="toComboId(node)"
        v-slot="{ active, selected }"
      >
        <li
          :class="[
            'relative cursor-default select-none px-1 py-1 text-gray-900',
            active ? 'bg-orange-100' : '',
            selected ? 'text-orange-600' : '',
          ]"
        >
          <div class="flex items-baseline justify-between">
            <TypePreview :type="node" show-type-name hide-flags />
            <!-- Source -->
            <span class="text-xs" :class="['truncate', active ? 'text-gray-700' : 'text-gray-500']">
              {{ node.primitive ? "(builtin)" : module.fileOf(node.reference)?.path }}
            </span>
          </div>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
