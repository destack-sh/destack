<script lang="ts" setup>
import { STRING_TYPE, typeIsNumeric } from "@/language/field";
import { checkValueScalar, checkValueScalarConstraint } from "@/language/value";
import { Alignment, ColorShade, NodeType, Orientation, Variant, ViewType, type ViewData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { IconInline } from "@/ui/icon";
import { getNativeConstraintProps, TEXT_DIRECTION_BY_ALIGNMENT } from "@/ui/view";
import { makeViewId, ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { computed, Ref, ref, toRef, watch } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<
      ViewData,
      "type" | "name" | "title" | "text" | "icon" | "variant" | "valueType" | "alignment" | "isInput" | "isDisabled"
    >
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const valueType = computed(() => props.valueType ?? STRING_TYPE);
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
const inputRef = ref<HTMLInputElement | null>(null);
const inputType = computed(() => {
  if (props.valueType?.isSecret) return "password";
  else if (props.type == ViewType.NUMBER) return "number";
  else return "text";
});

// sync currentValue (may be invalid) with modelValue
const currentValue: Ref<string | null | undefined> = ref(null);
watch(
  modelValue,
  (newValue) => (currentValue.value = Array.isArray(newValue) ? currentValue.value : newValue?.toString()),
  {
    immediate: true,
  },
);
const validationError: Ref<string | null> = ref(null);
watch(currentValue, (newValue) => {
  let value: any = newValue;
  if (typeIsNumeric(valueType.value)) {
    value = parseFloat(newValue as string);
    if (isNaN(value)) {
      value = null;
    }
  }

  // check
  const errors = [];
  if (value == null) {
    if (valueType.value.isRequired) {
      errors.push("missing value");
    }
  } else {
    checkValueScalar(value, valueType.value, (v, m, t) => errors.push(m));
    if (valueType.value.constraint != null) {
      checkValueScalarConstraint(value, valueType.value, valueType.value.constraint, (v, m, t) => errors.push(m));
    }
  }
  validationError.value = errors.length > 0 ? errors[0] : null;

  // auto-sync with modelValue for scalars (lists are added on demand)
  if (errors.length == 0 && !valueType.value.isList && value != modelValue.value) {
    emit("update:modelValue", value);
  }
});

function addNewValue() {
  if (!props.valueType?.isList) throw new Error(`cannot add to non-list`);
  if (currentValue.value != null) return;
  currentValue.value = "";
}
function addCurrentValue() {
  if (!props.valueType?.isList) throw new Error(`cannot add to non-list`);
  if (currentValue.value == null) return;
  if (validationError.value != null) return; // invalid
  emit("update:modelValue", ((modelValue.value as any[]) ?? []).concat(currentValue.value));
  currentValue.value = null;
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
defineExpose<ViewExposed & { select: () => void }>({
  self,
  id,
  focus: () => inputRef.value,
  select: () => {
    if (inputRef.value != null) inputRef.value.select();
  },
});
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- Input -->
    <div
      v-if="isInput"
      class="group flex select-text flex-row flex-wrap items-center gap-x-1 gap-y-1 rounded transition-colors duration-75 hover:border-gray-300"
      :class="[
        isDisabled
          ? 'bg-gray-100 text-gray-700'
          : variant != Variant.STEALTH
            ? 'bg-white text-gray-900'
            : 'text-gray-900',
        variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-1 outline-1 focus-within:outline' : '',
        validationError != null ? 'outline-danger-600' : 'outline-primary-900',
      ]"
    >
      <IconInline v-if="icon" v-bind="icon" :shade="ColorShade.S400" class="mr-0.5 w-5" />
      <!-- Current value -->
      <template v-if="!valueType?.isList">
        <!-- Scalar -->
        <input
          ref="inputRef"
          :value="currentValue"
          spellcheck="false"
          :type="inputType"
          class="flex-1 border-0 bg-transparent p-0 outline-none ring-0 transition-colors duration-75 focus:ring-0"
          :class="[
            TEXT_DIRECTION_BY_ALIGNMENT[alignment ?? Alignment.START] ?? '',
            validationError != null ? 'text-danger-600' : '',
          ]"
          :size="variant == Variant.STEALTH ? ((currentValue as string)?.length ?? 0) + 1 : undefined"
          v-bind="getNativeConstraintProps(valueType?.constraint)"
          :disabled="isDisabled"
          @input="currentValue = ($event.target as HTMLInputElement).value"
        />
        <!-- Clear -->
        <button
          v-if="variant != Variant.STEALTH && !isDisabled && !valueType?.isRequired && hasValue"
          class="ml-auto pl-1 text-gray-400 opacity-0 outline-none hover:text-primary-900 focus:ring-0 group-hover:opacity-100"
          @click.stop="clear"
        >
          <i class="fas fa-xmark" />
        </button>
      </template>
      <template v-else>
        <!-- List -->
        <div v-for="(v, i) in values" :key="i" class="rounded bg-gray-100 px-1 hover:text-primary-900">
          <span class="truncate">{{ v }}</span>
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
          v-if="currentValue != null"
          ref="inputRef"
          :value="currentValue"
          spellcheck="false"
          :type="inputType"
          class="rounded border-0 bg-gray-100 p-0 px-1 outline-none ring-0 hover:text-primary-900 focus:ring-0"
          v-bind="getNativeConstraintProps(valueType?.constraint)"
          :size="variant == Variant.STEALTH ? ((currentValue as string)?.length ?? 0) + 1 : undefined"
          :disabled="isDisabled"
          @keydown.enter.stop.prevent="addCurrentValue(), $nextTick(() => inputRef?.focus())"
          @input="currentValue = ($event.target as HTMLInputElement).value"
        />
        <!-- Add to list-->
        <button
          v-else-if="!isDisabled"
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
      class="group flex select-text flex-row flex-wrap items-center gap-x-1 gap-y-1 rounded text-gray-700 outline-1 outline-primary-900 focus-within:outline hover:border-gray-300"
      :class="[variant != Variant.STEALTH ? 'border border-gray-200 px-2 py-1' : '']"
    >
      <template v-if="!valueType?.isList">
        <!-- Scalar -->
        <span :class="[TEXT_DIRECTION_BY_ALIGNMENT[alignment ?? Alignment.START] ?? '']">
          {{ modelValue }}
        </span>
      </template>
      <template v-else>
        <!-- List -->
        <div v-for="(v, i) in values" :key="i" class="truncate rounded bg-gray-100 px-1">{{ v }}</div>
      </template>
    </div>
  </ViewContentWrapper>
</template>
