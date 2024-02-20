<script lang="ts" setup>
import TypePreview from "@/components/interfaces/TypePreview.vue";
import { StatementType, TypeHint, TypeTag, type Field } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { TypeFlag, useCurrentModule } from "@/state/module";
import { ANY_FIELD, makeField } from "@/state/statement";
import { ICONS_BY_TAG_OUTLINE, renderBuiltinType, SUPPORTED_TYPEHINTS } from "@/state/type";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { ExclamationCircleIcon, ListBulletIcon, QuestionMarkCircleIcon } from "@heroicons/vue/24/outline";
import { computed, onMounted, ref, watch, type Ref, nextTick } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { useBenchState } from "@/state/bench";
import { ufSort } from "@/utils/search";
import { useElementRefs } from "@/composables/useGrid";

const props = defineProps<{
  modelValue?: Field;
  allowFreeform?: boolean;
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
    return TypeFlag.IS_SECRET | TypeFlag.IS_OPTIONAL;
  } else {
    return TypeFlag.IS_OPTIONAL;
  }
}

const BUILTIN_TYPES: (TypeHint | TypeTag)[] = [
  TypeTag.String,
  TypeTag.Boolean,
  TypeTag.Number,
  TypeTag.Blob,
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
  types: [StatementType.Class, StatementType.Choice, StatementType.Flow, StatementType.Task],
  // TODO @UX @Feature: also support code & database type references
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
      if (statement.type == StatementType.Class) {
        refType = TypeTag.Struct;
      } else if (statement.type == StatementType.Choice) {
        refType = TypeTag.Enum;
      } // ignore functions and databases for now, too confusing
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

const TRIMMED_PLURAL_FORMS = ["ies", "ie", "i", "es", "e", "s"];
const uf = new uFuzzy({ intraMode: 0 });
const filteredTypes = computed(() => {
  if (query.value.trim() == "") return availableTypes.value;
  const haystack = availableTypes.value.map((t) => {
    if (t.alias != null) {
      return `${renderField(t)} ${t.alias.join(" ")}}`;
    }
    return renderField(t) ?? "";
  });
  let q = query.value;
  // trim plural form for search
  const form = TRIMMED_PLURAL_FORMS.find((f) => q.length > f.length && q.endsWith(f));
  if (form != null) {
    q = q.slice(0, -form.length);
  }
  let idxs = uf.filter(haystack, q);
  if (idxs != null && idxs.length > 0) {
    const info = uf.info(idxs, haystack, q);
    const sort = ufSort(info, haystack, q);
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
  const filteredTypes = idxs?.map((idx) => availableTypes.value[idx]) ?? [];
  return filteredTypes;
});
const optionRefs = useElementRefs<InstanceType<typeof ComboboxOption>>(filteredTypes);

function writeValue(type: Field) {
  let newFlags = TypeFlag.ZERO;
  const wasArray = Boolean(type.flags & TypeFlag.IS_ARRAY);
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
  if (wasArray) {
    type.flags |= TypeFlag.IS_ARRAY; // restore since it was lost in getDefaultFlags
  }
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
    flag: TypeFlag.IS_OPTIONAL,
    invert: true,
    label: "required",
    setIcon: ExclamationCircleIcon,
    unsetIcon: QuestionMarkCircleIcon,
  },
  {
    flag: TypeFlag.IS_ARRAY,
    invert: false,
    label: "list",
    setIcon: ListBulletIcon,
    unsetIcon: ListBulletIcon,
  },
];

// constraint list & secret flags to UX-sensible types
// (internally we could support any permutation)
const NONNULL_TAGS = [TypeTag.Boolean];
const LISTABLE_TAGS = [TypeTag.Blob, TypeTag.TypeReference, TypeTag.Struct, TypeTag.Enum, TypeTag.Node];
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
  if (flag == TypeFlag.IS_OPTIONAL) {
    return !NONNULL_TAGS.includes(type.tag);
  } else if (flag == TypeFlag.IS_ARRAY) {
    return (
      !isFlagSet(TypeFlag.IS_SECRET) &&
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
  if (!isFlagSupported(value.value, flag)) return;
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

function toSignatureId(type: Field) {
  return `${type.tag}.${type.hint ?? ""}.${type.referenceCk ?? ""}`;
}

function findBySignatureId(id: string) {
  if (id.startsWith("freeform-")) {
    const tag = Object.values(TypeTag).find((t) => t.toLowerCase() == id.slice("freeform-".length).toLowerCase());
    if (!tag) {
      throw new Error(`unknown freeform tag ${id}`);
    }
    return makeField({
      projectVersionId: module.id.value,
      tag,
      flags: getDefaultFlags(TypeHint.Name),
      name: query.value,
    });
  }
  return availableTypes.value.find((t) => toSignatureId(t) == id);
}

// focus input once mounted
onMounted(() => {
  inputRef.value?.$el.focus();
  // scroll selected option into view
  nextTick(() => {
    const selectedOption = optionRefs.getRef(toSignatureId(value.value));
    selectedOption?.$el.scrollIntoView({ block: "nearest" });
  });
});

const appearance = useAppearance();

defineExpose({
  focus: () => {
    inputRef.value?.$el.focus();
  },
});
</script>
<template>
  <Combobox
    as="div"
    :model-value="toSignatureId(value)"
    @update:model-value="(id: any) => writeValue(findBySignatureId(id) ?? value)"
    @keydown.ctrl.r.exact.prevent.stop="toggleFlag(TypeFlag.IS_OPTIONAL)"
    @keydown.ctrl.o.exact.prevent.stop="toggleFlag(TypeFlag.IS_OPTIONAL)"
    @keydown.ctrl.m.exact.prevent.stop="toggleFlag(TypeFlag.IS_ARRAY)"
    @keydown.ctrl.l.exact.prevent.stop="toggleFlag(TypeFlag.IS_ARRAY)"
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
      class="w-full rounded-sm border border-orange-900/[12%] bg-orange-100 p-1 text-gray-900 outline-none ring-0 hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-[12%] focus:ring-0"
      :class="{
        'font-mono': appearance.fontMono,
        'text-sm placeholder:text-sm': appearance.textSmall,
        'text-md placeholder:text-md': !appearance.textSmall,
      }"
      @change="query = $event.target.value"
      :display-value="(el: any) => ''"
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
      <ComboboxOption
        v-for="ref in filteredTypes"
        :ref="(r: any) => optionRefs.registerRef(toSignatureId(ref), r)"
        :key="ref.id"
        :value="toSignatureId(ref)"
        v-slot="{ active, selected }"
      >
        <li
          :class="[
            'relative cursor-default select-none px-1 py-[3px] text-gray-900',
            active ? 'bg-orange-100' : '',
            selected ? 'text-orange-600' : '',
          ]"
        >
          <div class="flex max-w-full flex-row gap-1.5 whitespace-nowrap">
            <TypePreview :type="ref" class="mt-0.5" />
            <span>{{
              ref.referenceCk != null
                ? module.statementOf(ref.referenceCk)?.name
                : renderBuiltinType(ref.tag, ref.hint ?? null)
            }}</span>
            <!-- Source -->
            <span class="ml-auto truncate text-xs" :class="['truncate', active ? 'text-gray-700' : 'text-gray-500']">
              {{ ref.referenceCk == null ? "(builtin)" : module.pathOf(ref.referenceCk, { roffset: 1 }) }}
            </span>
          </div>
        </li>
      </ComboboxOption>
      <!-- Freeform -->
      <template v-if="props.allowFreeform && query.length > 0">
        <ComboboxOption
          v-for="tag of [TypeTag.String, TypeTag.Number, TypeTag.Boolean, TypeTag.Blob]"
          :key="'freeform-' + tag"
          :value="'freeform-' + tag"
          v-slot="{ active }"
        >
          <li
            class="relative cursor-default select-none px-1 py-[3px] text-gray-900"
            :class="['truncate', active ? 'bg-orange-100' : '']"
          >
            <div class="flex items-center">
              <component :is="ICONS_BY_TAG_OUTLINE[tag]" class="h-4 w-4" />
              <span class="ml-1"> {{ query }} </span>
            </div>
          </li>
        </ComboboxOption>
      </template>
    </ComboboxOptions>
  </Combobox>
</template>
