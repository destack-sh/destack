<script lang="ts" setup>
import TypePreview from "@/components/interfaces/TypePreview.vue";
import { StatementType, TypeHint, TypeTag, type Field } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { TypeFlag, useCurrentModule } from "@/state/module";
import { ANY_FIELD, makeField } from "@/state/statement";
import { renderBuiltinType, SUPPORTED_TYPEHINTS } from "@/state/type";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { ExclamationCircleIcon, ListBulletIcon, QuestionMarkCircleIcon } from "@heroicons/vue/24/outline";
import { computed, onMounted, ref, watch, type Ref } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { useBenchState } from "@/state/bench";
import { ufSort } from "@/utils/search";

const props = defineProps<{
  modelValue?: Field;
  inlined?: boolean;
  refOnly?: boolean;
  refTypes?: TypeTag[];
  hideFlags?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<Field, "name" | "tag" | "flags" | "referenceCk" | "value">): void;
  (e: "escape"): void;
}>();

const value: Ref<Field> = ref(props.modelValue ?? ANY_FIELD);
const query: Ref<string> = ref("");
const inputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);

const bench = useBenchState();
const module = useCurrentModule();

function getDefaultFlags(t: TypeHint | TypeTag): number {
  if (t == TypeHint.Secret) {
    return TypeFlag.IsSecret | TypeFlag.IsOptional;
  } else {
    return TypeFlag.IsOptional;
  }
}

const BUILTIN_TYPES: (TypeHint | TypeTag)[] = [
  TypeTag.String,
  TypeTag.Boolean,
  TypeTag.Number,
  TypeTag.File,
  ...(Object.keys(SUPPORTED_TYPEHINTS) as TypeHint[]),
];
const BUILTIN_TYPES_ALIASES: Partial<Record<TypeHint | TypeTag, string[]>> = {
  [TypeTag.String]: ["string", "str", "char"],
  [TypeTag.Boolean]: ["bool", "boolean", "true", "false"],
  [TypeTag.Number]: ["number", "num", "int", "integer", "float", "double"],
};
const BUILTINS_TYPES_FIELDS = BUILTIN_TYPES.map((tag) => {
  if (Object.values(TypeTag).includes(tag as TypeTag)) {
    return makeField({ projectVersionId: module.id.value, tag: tag as TypeTag });
  } else if (tag in SUPPORTED_TYPEHINTS) {
    return makeField({
      projectVersionId: module.id.value,
      tag: SUPPORTED_TYPEHINTS[tag as TypeHint] as TypeTag,
      hint: tag as TypeHint,
    });
  } else {
    throw new Error(`unknown primitive ${tag} ${typeof tag} ${Object.keys(TypeTag)}`);
  }
});

const availableStatements = module.statementsLike({
  types: [StatementType.Type, StatementType.Flow, StatementType.Task],
  // TODO @UX @Feature: also support code & dataset type references
  //  (right now this is too noisy and confusing, too much code & 'does database mean relation?', also see :DbRecord)
});
type FieldInfo = { primitive?: boolean; alias?: string[] };
const availableTypes: Ref<Array<Field & FieldInfo>> = computed(() => {
  const types: Array<Field & FieldInfo> = [];
  // builtin types
  if (!props.refOnly) {
    types.push(
      ...BUILTINS_TYPES_FIELDS.map((t) => {
        const alias = BUILTIN_TYPES_ALIASES[t.hint ?? t.tag];
        return { ...t, flags: getDefaultFlags(t.hint ?? t.tag), primitive: true, alias };
      })
    );
  }
  // references
  for (const statement of availableStatements.value) {
    if (statement.name == null) continue;

    // filter references
    if (props.refTypes != null) {
      let refType: TypeTag | null = null;
      if (statement.type == StatementType.Type) {
        refType = statement.tag ?? null;
      } else if (statement.type == StatementType.Dataset) {
        refType = TypeTag.Struct;
      } else {
        refType = TypeTag.Function;
      }
      if (!props.refTypes.includes(refType as TypeTag)) continue;
    }

    types.push(
      makeField({
        projectVersionId: module.id.value,
        tag: TypeTag.TypeReference,
        referenceCk: statement.ck,
        flags: getDefaultFlags(TypeTag.TypeReference),
      })
    );
  }
  return types;
});

const uf = new uFuzzy({ intraMode: 0 });
const filteredTypes = computed(() => {
  if (query.value.trim() == "") return availableTypes.value;
  const haystack = availableTypes.value.map((t) => {
    if (t.alias != null) {
      return `${renderField(t)} ${t.alias.join(" ")}}`;
    }
    return renderField(t) ?? "";
  });
  let idxs = uf.filter(haystack, query.value);
  if (idxs != null && idxs.length > 0) {
    const info = uf.info(idxs, haystack, query.value);
    const sort = ufSort(info, haystack, query.value);
    const order = info.idx
      .map((v, i) => i)
      .sort((ia, ib) => {
        const aType = availableTypes.value[info.idx[ia]];
        const bType = availableTypes.value[info.idx[ib]];
        const aFocused = bench.focusedFileId == module.fileOf(aType?.referenceCk)?.id;
        const bFocused = bench.focusedFileId == module.fileOf(bType?.referenceCk)?.id;
        if (aFocused != bFocused) return aFocused ? -1 : 1;
        return sort(ia, ib);
      });
    idxs = order.map((i) => info.idx[i]);
  }
  return idxs?.map((idx) => availableTypes.value[idx]) ?? [];
});

function writeValue(type: Field) {
  let newFlags = TypeFlag.Zero;
  if (props.modelValue == null) {
    newFlags = getDefaultFlags(type.hint ?? type.tag);
  } else {
    // keep supported flags
    for (let flag of Object.values(TypeFlag)) {
      flag = flag as TypeFlag;
      if (isFlagSet(flag) && isFlagSupported(type, flag)) {
        newFlags |= flag;
      }
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
    flag: TypeFlag.IsOptional,
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
function isFlagSupported(type: Field, flag: TypeFlag) {
  if (flag == TypeFlag.IsOptional) {
    return !NONNULL_TAGS.includes(type.tag);
  } else if (flag == TypeFlag.IsArray) {
    return (
      !isFlagSet(TypeFlag.IsSecret) &&
      ((type.hint != null && LISTABLE_HINTS.includes(type.hint)) || LISTABLE_TAGS.includes(type.tag))
    );
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

function renderField(node: Field): string | undefined | null {
  const builtin = renderBuiltinType(node.tag, node.hint ?? null);
  if (builtin != null) return builtin;
  if (node.tag == TypeTag.TypeReference || node.referenceCk != null) {
    return module.statementOf(node.referenceCk)?.name;
  }
  throw new Error(`unexpected type node: ${JSON.stringify(node)}`);
}

// use 'combobox id' as a stable id

function toComboId(type: Field) {
  return `${type.tag}.${type.hint ?? ""}.${type.referenceCk ?? ""}`;
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
        class="flex flex-1 flex-row items-center justify-center gap-1 rounded-sm px-1 py-0.5 hover:bg-orange-100"
        :class="[
          isFlagSet(flagButton.flag) !== flagButton.invert ? 'font-bold text-orange-600' : '',
          isFlagSupported(value, flagButton.flag) ? 'text-gray-600' : 'cursor-not-allowed text-gray-400',
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
      class="w-full rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
      :class="{
        'font-mono': appearance.fontMono,
        'text-sm placeholder:text-sm': appearance.textSmall,
        'text-md placeholder:text-md': !appearance.textSmall,
      }"
      @change="query = $event.target.value"
      :display-value="(el: any) => props.modelValue == null ? '' : renderField(findByComboId(el) ?? value) ?? ''"
      placeholder="Search types"
      spellcheck="false"
      @keydown.enter.prevent.stop="emit('escape')"
    />
    <ComboboxOptions
      ref="optionsRef"
      class="mt-1 max-h-48 overflow-auto"
      static
      :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
    >
      <!-- Options -->
      <ComboboxOption v-for="ref in filteredTypes" :key="ref.id" :value="toComboId(ref)" v-slot="{ active, selected }">
        <li
          :class="[
            'relative cursor-default select-none px-1 py-[3px] text-gray-900',
            active ? 'bg-orange-100' : '',
            selected ? 'text-orange-600' : '',
          ]"
        >
          <div class="flex items-baseline justify-between">
            <TypePreview :type="ref" show-type-name hide-flags />
            <!-- Source -->
            <span class="text-xs" :class="['truncate', active ? 'text-gray-700' : 'text-gray-500']">
              {{ ref.referenceCk == null ? "(builtin)" : module.pathOf(ref.referenceCk, { roffset: 1 }) }}
            </span>
          </div>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
