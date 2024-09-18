<script lang="ts" setup>
import { ColorShade, ColorType, NodeType, Variant, ViewType, type ViewData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { IconInline } from "@/ui/icon";
import { canvas } from "@/system/space";
import { getNativeConstraintProps, guardNativeInput } from "@/ui/view";
import { ViewContentWrapper, makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { computed, Ref, ref, toRef } from "vue";
import { STRING_TYPE_IDENTITY } from "@/language/field";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<
      ViewData,
      "type" | "name" | "title" | "text" | "icon" | "variant" | "valueType" | "orientation" | "isInput" | "isDisabled"
    >
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const modelValue = defineModel<string | number | bigint | Array<string | number | bigint>>();
const hasValue = computed(() => {
  if (modelValue.value == null) return false;
  if (props.valueType?.isList) return (modelValue.value as any[]).length > 0;
  else return true;
});
const values = computed(() => {
  if (modelValue.value == null) return [];
  else if (props.valueType?.isList) return modelValue.value as any[];
  else return [modelValue.value];
});
const addingValue: Ref<string | number | bigint | null> = ref(null);
const inputRef = ref<HTMLInputElement | null>(null);
const inputType = computed(() => {
  if (props.valueType?.isSecret) return "password";
  else if (props.type == ViewType.NUMBER) return "number";
  else return "text";
});

function addNewValue() {
  if (!props.valueType?.isList) throw new Error(`cannot add to non-list`);
  if (addingValue.value != null) return;
  if (props.type == ViewType.NUMBER) addingValue.value = 0;
  else addingValue.value = "";
}
function addCurrentValue() {
  if (!props.valueType?.isList) throw new Error(`cannot add to non-list`);
  if (addingValue.value == null) return;
  emit("update:modelValue", ((modelValue.value as any[]) ?? []).concat(addingValue.value));
  addingValue.value = null;
}
function clear() {
  if (!props.valueType?.isList) emit("update:modelValue", undefined);
  else emit("update:modelValue", []);
}
function remove(idx: number) {
  if (!props.valueType?.isList) throw new Error(`cannot remove from non-list`);
  emit(
    "update:modelValue",
    (modelValue.value as any[]).filter((_, i) => i != idx),
  );
}

const id = makeViewId(props);
canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, focus: () => inputRef.value });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- Input -->
    <div
      v-if="isInput"
      class="group flex flex-row flex-wrap items-center gap-x-1 gap-y-1 rounded outline-1 outline-primary-900 focus-within:outline hover:border-gray-300"
      :class="[
        isDisabled ? 'bg-gray-100 text-gray-700' : 'bg-white text-gray-900',
        variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-1' : '',
      ]"
    >
      <IconInline v-if="icon" v-bind="icon" :shade="ColorShade.S400" class="mr-0.5 w-5" />
      <!-- Current value -->
      <template v-if="!valueType?.isList">
        <!-- Scalar -->
        <input
          ref="inputRef"
          :value="modelValue"
          spellcheck="false"
          :type="inputType"
          class="w-full border-0 bg-transparent p-0 outline-none ring-0 focus:ring-0"
          v-bind="getNativeConstraintProps(valueType?.constraint)"
          :disabled="isDisabled"
          @input="
            guardNativeInput(
              valueType ?? STRING_TYPE_IDENTITY,
              valueType?.constraint,
              $event,
              values[0],
              (newValue) => {
                emit('update:modelValue', newValue, typeof newValue);
              },
            )
          "
        />
      </template>
      <template v-else>
        <!-- List -->
        <div v-for="(v, i) in values" :key="i" class="rounded bg-gray-100 px-1 hover:text-primary-900">
          <span>{{ v }} </span>
          <!-- Remove -->
          <button
            v-if="!isDisabled"
            class="ml-1.5 text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
            @click.stop="remove(i)"
          >
            <i class="fas fa-xmark" />
          </button>
        </div>
        <!-- New value -->
        <input
          v-if="addingValue != null"
          ref="inputRef"
          :value="addingValue"
          spellcheck="false"
          :type="inputType"
          class="rounded border-0 bg-gray-100 p-0 px-1 outline-none ring-0 hover:text-primary-900 focus:ring-0"
          v-bind="getNativeConstraintProps(valueType?.constraint)"
          :disabled="isDisabled"
          @keydown.enter.stop.prevent="addCurrentValue(), $nextTick(() => inputRef?.focus())"
          @input="
            guardNativeInput(
              valueType ?? STRING_TYPE_IDENTITY,
              valueType?.constraint,
              $event,
              values[0],
              (newValue) => (addingValue = newValue),
            )
          "
        />
        <!-- Add to list-->
        <button
          v-else
          class="mr-2 self-center text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
          @click.stop="addNewValue(), $nextTick(() => inputRef?.focus())"
        >
          <i class="fas fa-plus" />
        </button>
      </template>
    </div>
    <!-- Read-only -->
    <div
      v-else
      class="group flex flex-row flex-wrap items-center gap-x-1 gap-y-1 rounded text-gray-700 outline-1 outline-primary-900 focus-within:outline hover:border-gray-300"
      :class="[variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-1' : '']"
    >
      <template v-if="!valueType?.isList">
        <!-- Scalar -->
        <span>{{ modelValue }}</span>
      </template>
      <template v-else>
        <!-- List -->
        <div v-for="(v, i) in values" :key="i" class="rounded bg-gray-100 px-1">{{ v }}</div>
      </template>
    </div>
  </ViewContentWrapper>
</template>
