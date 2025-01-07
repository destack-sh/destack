<script lang="ts" setup>
import { NodeType, TypeData, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { viewEmits, type ViewExposed } from "@/views/common";
import Picker from "@/views/content/Picker.vue";
import { computed, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "valueType" | "isInput" | "isMinimal">
  >
>();
const modelValue = defineModel<TypeData | undefined>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const supportsList = computed(() => modelValue.value?.baseTypePtr?.nodeType != NodeType.BLOCK);

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div class="flex flex-row items-center">
    <Picker
      id="picker"
      :is-input="isInput"
      :value-type="valueType"
      :model-value="modelValue"
      class="rounded-r-none"
      @update:model-value="
        (value) =>
          emit('update:modelValue', { ...value, isRequired: modelValue?.isRequired, isList: modelValue?.isList })
      "
    />
    <button
      v-tooltip="{ title: () => (modelValue?.isRequired ? 'Required' : 'Optional'), small: true, group: 'type' }"
      class="h-[30px] rounded rounded-l-none border border-l-0 border-gray-200 px-0.5 text-gray-900 transition-colors duration-75 hover:bg-gray-100"
      :class="[supportsList ? 'rounded-r-none' : '']"
      @click="emit('update:modelValue', { ...modelValue, isRequired: !modelValue?.isRequired })"
    >
      <span class="w-5 text-center" :class="modelValue?.isRequired ? 'fas fa-check' : 'fas fa-question'" />
    </button>
    <button
      v-if="supportsList"
      v-tooltip="{ title: () => (modelValue?.isList ? 'List' : 'Scalar'), small: true, group: 'type' }"
      class="border-l-none h-[30px] rounded rounded-l-none border border-l-0 border-gray-200 px-0.5 text-gray-900 transition-colors duration-75 hover:bg-gray-100"
      @click="emit('update:modelValue', { ...modelValue, isList: !modelValue?.isList })"
    >
      <span class="w-5 text-center" :class="modelValue?.isList ? 'fas fa-list' : 'fas fa-1'" />
    </button>
  </div>
</template>
