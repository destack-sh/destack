<script lang="ts" setup>
import { StatementType, SymbolType, TypeTag, type TypeNodeData } from "@/gql/graphql";
import { TYPETAG_KEYWORD } from "@/state/editor";
import { symbolsLike } from "@/state/runtime";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { onClickOutside, useFocus } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue: TypeNodeData;
  readonly: boolean;
  editing: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<TypeNodeData, "tag" | "reference">): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "edit"): void;
}>();

const PRIMITIVE_TYPES = [TypeTag.Any, TypeTag.String, TypeTag.Boolean, TypeTag.Number, TypeTag.Null];

function mapToMiniType(node: TypeNodeData): RenderedMiniType {
  if (PRIMITIVE_TYPES.includes(node.tag) || node.tag == TypeTag.TypeReference) {
    return renderMiniType({
      tag: node.tag,
      reference: node.reference ?? undefined,
    });
  } else {
    console.warn(`unexpected type node ${node.tag}`, node);
    return renderMiniType({
      tag: TypeTag.Any,
    });
  }
}

const value: Ref<RenderedMiniType> = ref(mapToMiniType(props.modelValue));
const query: Ref<string> = ref("");
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const valueRef: Ref<HTMLInputElement | null> = ref(null);
const valueRefFocused = useFocus(valueRef);

// TODO @Cleanup: MiniType indicates that we may want a simpler TypeNodeData representation
//  for the UI & DB. Likely with proper references (instead of strings) as well. These are
//  not mutually dependent. More complex references (like functions) are useful as well..
type MiniType = {
  tag: TypeTag;
  reference?: string;
  isArray?: boolean;
  isUnionWithNull?: boolean;
};

type RenderedMiniType = MiniType & {
  rendered: string;
};

function renderMiniType(mtype: MiniType): RenderedMiniType {
  let renderedElement: string;
  if (PRIMITIVE_TYPES.includes(mtype.tag)) {
    renderedElement = TYPETAG_KEYWORD[mtype.tag];
  } else if (mtype.tag == TypeTag.TypeReference) {
    renderedElement = mtype.reference ?? "...";
  } else {
    throw new Error(`unexpected type node ${mtype.tag}`);
  }

  let rendered: string;
  if (mtype.isArray) {
    rendered = "list of " + renderedElement;
  } else {
    rendered = renderedElement;
  }
  return { ...mtype, rendered };
}

const availableSymbols = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Type],
});
const availableTypes: Ref<RenderedMiniType[]> = computed(() => {
  const availableTypes = [];

  // primitives
  for (const primitiveType of PRIMITIVE_TYPES) {
    availableTypes.push(
      renderMiniType({
        tag: primitiveType,
      })
    );
  }

  // references
  for (const symbol of availableSymbols.value) {
    if (symbol.name == null) {
      continue; // ignore
    }
    availableTypes.push(
      renderMiniType({
        tag: TypeTag.TypeReference,
        reference: symbol.name,
      })
    );
  }

  // TODO @Incomplete: edit list of primitives & lists of references

  return availableTypes;
});

const filteredTypes = computed(() => availableTypes.value.filter((t) => t.rendered.includes(query.value)));

function writeValue(mtype: RenderedMiniType) {
  if (mtype.isArray) {
    throw new Error("arrays not implemented");
  }
  value.value = mtype;
  emit("update:modelValue", mtype);
  emit("escape");
}

// re-focus when we start/stop editing
watch(
  () => props.editing,
  () => nextTick(focus)
);

function focus() {
  query.value = "";
  if (!props.editing) {
    buttonRef.value?.focus();
  } else {
    valueRefFocused.focused.value = true;
  }
}

defineExpose({
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
    ref="buttonRef"
    v-if="!editing"
    tabeindex="-1"
    @keydown.left.exact.prevent="emit('navigateLeft')"
    @keydown.right.exact.prevent="emit('navigateRight')"
    @keydown.up.exact.prevent="emit('navigateUp')"
    @keydown.down.exact.prevent="emit('navigateDown')"
    @keydown.enter.exact.prevent="emit('edit')"
    @click="emit('edit')"
    class="text-left outline-none"
  >
    {{ value.rendered }}
  </button>
  <!-- Editable type :EditableCellStyle -->
  <Combobox v-else as="div" class="relative" :model-value="value" @update:model-value="writeValue">
    <ComboboxInput
      as="input"
      ref="valueRef"
      class="absolute -left-0.5 -top-0.5 z-10 w-60 rounded-sm border border-black bg-orange-50 py-0 px-1 font-mono outline-none ring-0 focus:border-black focus:ring-0"
      @change="query = $event.target.value"
      :display-value="(stmt: any) => stmt?.name"
      placeholder="..."
      @keydown.escape.prevent=""
      @keyup.escape.prevent="emit('escape')"
    />
    <ComboboxOptions
      class="absolute z-20 mt-8 max-h-60 w-60 overflow-auto rounded-sm bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
      static
      v-show="editing"
    >
      <ComboboxOption
        v-for="mtype in filteredTypes"
        :key="mtype.rendered"
        :value="mtype"
        as="template"
        v-slot="{ active, selected }"
      >
        <li
          :class="[
            'relative cursor-default select-none py-0.5 px-2 font-mono text-sm',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
          ]"
        >
          {{ mtype.rendered }}
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
