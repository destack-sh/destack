<script lang="ts" setup>
import { TypeTag, type TypeNodeData } from "@/gql/graphql";
import { computed, nextTick, ref, watch, type Ref } from "vue";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { useFocus } from "@vueuse/core";

const props = defineProps<{
  modelValue: TypeNodeData;
  readonly: boolean;
  editing: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: any): void;
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

function mapToMiniType(node: TypeNodeData): MiniType {
  if (PRIMITIVE_TYPES.includes(node.tag)) {
    return {
      tag: node.tag,
      rendered: node.tag,
    };
  } else {
    console.warn(`unexpected type node ${node.tag}`, node);
    return {
      tag: TypeTag.Any,
      rendered: "unknown",
    };
  }
}

const value: Ref<MiniType> = ref(mapToMiniType(props.modelValue));
const query: Ref<string> = ref("");
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const valueRef: Ref<HTMLInputElement | null> = ref(null);
const valueRefFocused = useFocus(valueRef);

type MiniType = {
  tag: TypeTag;
  reference?: string;
  isArray?: boolean;
  rendered: string;
};

const availableTypes: MiniType[] = [];

for (const primitiveType of PRIMITIVE_TYPES) {
  availableTypes.push({
    tag: primitiveType,
    rendered: primitiveType, // TODO @Incomplete: render types properly
  });
}

const filteredTypes = computed(() => availableTypes.filter((t) => t.rendered.includes(query.value)));

function writeValue(mtype: MiniType) {
  console.log("writeValue", mtype);
  // TODO @Incomplete: write type properly
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
  <Combobox v-else as="div" class="relative" :model-value="value" @update:model-value="writeValue" nullable>
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
