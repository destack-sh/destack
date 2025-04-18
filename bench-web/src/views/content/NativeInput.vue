<script lang="ts" setup>
import { STRING_TYPE, typeIsNumeric } from "@/language/core/type";
import { checkValueScalar, checkValueScalarConstraint } from "@/language/core/value";
import {
  Alignment,
  ColorShade,
  NodeReferenceData,
  NodeType,
  ViewType,
  type ViewData
} from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { IconInline } from "@/ui/icon";
import { getNativeConstraintProps, TEXT_DIRECTION_BY_ALIGNMENT } from "@/ui/view";
import { FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import { useElementSize } from "@vueuse/core";
import { computed, Ref, ref, toRef, watch } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; placeholder?: string; ariaHidden?: boolean } & Partial<
    Partial<
      Pick<
        ViewData,
        "type" | "name" | "title" | "icon" | "valueType" | "alignment" | "isInput" | "isDisabled" | "isMinimal"
      >
    >
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const valueType = computed(() => props.valueType ?? STRING_TYPE);
const modelValue = defineModel<string | number | bigint | Array<string | number | bigint>>();
const hasValue = computed(() => {
  if (modelValue.value == null) {
    return false;
  } else if (props.valueType?.isList) {
    return (modelValue.value as any[]).length > 0;
  } else {
    if (typeof modelValue.value == "string") {
      return modelValue.value.length > 0;
    } else {
      return true;
    }
  }
});
const values = computed(() => {
  if (modelValue.value == null) return [];
  else if (props.valueType?.isList) return modelValue.value as any[];
  else return [modelValue.value];
});
const inputType = computed(() => {
  if (props.valueType?.isSecret) return "password";
  else if (props.type == ViewType.NUMBER) return "number";
  else return "text";
});
const size = computed(() => {
  return Math.max(3, props.placeholder?.length ?? 0, (modelValue.value as string)?.length ?? 0);
});

const inputRef = ref<HTMLInputElement | null>(null);
const measureRef = ref<HTMLSpanElement | null>(null);
const measureSize = useElementSize(measureRef);

// sync currentValue (may be invalid) with modelValue
const currentValue: Ref<string | null | undefined> = ref(null);
watch(
  modelValue,
  (newValue) => (currentValue.value = Array.isArray(newValue) ? currentValue.value : newValue?.toString()),
  { immediate: true },
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

canvas.registerView(self, id);
defineExpose<ViewExpose & { select: () => void }>({
  self,
  id,
  focus: (anchor: FocusAnchor | NodeReferenceData | undefined = "right") => {
    if (anchor == "left") {
      inputRef.value?.focus();
      if (inputType.value != "number") {
        inputRef.value?.setSelectionRange(0, 0);
      }
    } else {
      inputRef.value?.focus();
      if (inputType.value != "number") {
        inputRef.value?.setSelectionRange(inputRef.value?.value?.length ?? 0, inputRef.value?.value?.length ?? 0);
      }
    }
  },
  select: () => {
    if (inputRef.value != null) inputRef.value.select();
  },
});
</script>
<template>
  <div
    v-if="isInput"
    class="group flex flex-row flex-wrap items-center gap-x-1 gap-y-1 rounded transition-colors duration-75 hover:border-gray-200"
    :class="[
      isDisabled ? 'bg-gray-100 text-gray-700' : !isMinimal ? 'bg-white text-gray-900' : 'text-gray-900',
      !isMinimal ? 'select-text border border-gray-200 px-2 py-0.5 outline-1 focus-within:outline' : '',
      validationError != null ? 'outline-danger-600' : 'outline-gray-400',
    ]"
  >
    <!-- Input -->
    <IconInline v-if="icon" v-bind="icon" :shade="ColorShade.S400" class="mr-0.5 w-5" />
    <!-- Current value -->
    <template v-if="!valueType?.isList">
      <!-- Scalar -->
      <input
        ref="inputRef"
        :value="currentValue"
        :placeholder="placeholder"
        spellcheck="false"
        :type="inputType"
        class="max-w-full flex-1 truncate border-0 bg-transparent p-0 outline-none ring-0 transition-colors duration-75 placeholder:text-gray-400 focus:ring-0"
        :class="[
          TEXT_DIRECTION_BY_ALIGNMENT[alignment ?? Alignment.START] ?? '',
          validationError != null ? 'text-danger-600' : '',
        ]"
        :style="{
          width: measureSize.width.value != null ? measureSize.width.value + 'px' : 'auto',
        }"
        v-bind="getNativeConstraintProps(valueType?.constraint)"
        :disabled="isDisabled"
        :aria-hidden="ariaHidden"
        @keydown.left.stop="
          () => {
            // only if we're at the start of the input
            if (inputRef?.selectionStart == 0) {
              emit('navigate', 'left');
            }
          }
        "
        @keydown.right.stop="
          () => {
            // only if we're at the end of the input
            if (inputRef?.selectionEnd == inputRef?.value?.length) {
              emit('navigate', 'right');
            }
          }
        "
        @keydown.up.stop.prevent="() => emit('navigate', 'up')"
        @keydown.down.stop.prevent="() => emit('navigate', 'down')"
        @keydown.enter.stop.prevent="() => (emit('apply', currentValue), emit('navigate', 'enter'))"
        @input="currentValue = ($event.target as HTMLInputElement).value"
      />
      <!-- Invisible input to measure width -->
      <span ref="measureRef" class="pointer-events-none invisible absolute whitespace-pre">
        {{ (currentValue?.length ?? 0) > 0 ? currentValue : placeholder }}
      </span>
      <!-- Clear -->
      <button
        v-if="!isMinimal && !isDisabled && !valueType?.isRequired && hasValue"
        class="ml-auto pl-1 text-gray-400 opacity-0 outline-none transition-colors duration-75 hover:text-gray-700 focus:ring-0 group-hover:opacity-100"
        aria-hidden
        tabindex="-1"
        @click.stop="clear"
      >
        <i class="fas fa-xmark" />
      </button>
    </template>
    <template v-else>
      <!-- List -->
      <div v-for="(v, i) in values" :key="i" class="rounded bg-gray-100 px-1 hover:text-gray-700">
        <span class="truncate">{{ v }}</span>
        <!-- Remove -->
        <button
          v-if="!isDisabled"
          class="ml-1.5 text-gray-400 opacity-0 transition-colors duration-75 hover:text-gray-700 group-hover:opacity-100"
          aria-hidden
          tabindex="-1"
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
        class="rounded border-0 bg-gray-100 p-0 px-1 outline-none ring-0 hover:text-gray-700 focus:ring-0"
        v-bind="getNativeConstraintProps(valueType?.constraint)"
        :size="isMinimal ? size : undefined"
        :disabled="isDisabled"
        :aria-hidden="ariaHidden"
        @keydown.enter.stop.prevent="(addCurrentValue(), $nextTick(() => inputRef?.focus()))"
        @input="currentValue = ($event.target as HTMLInputElement).value"
      />
      <!-- Add to list-->
      <button
        v-else-if="!isDisabled"
        class="hover:text-primary-700 mr-2 self-center text-gray-400 opacity-0 group-hover:opacity-100"
        aria-hidden
        tabindex="-1"
        @click.stop="(addNewValue(), $nextTick(() => inputRef?.focus()))"
      >
        <i class="fas fa-plus" />
      </button>
    </template>
  </div>
  <div
    v-else
    class="group flex flex-row flex-wrap items-center gap-x-1 gap-y-1 rounded text-gray-700 outline-1 outline-gray-400 focus-within:outline hover:border-gray-200"
    :class="[!isMinimal ? 'select-text border border-gray-200 px-2 py-1' : '']"
  >
    <!-- Read-only -->
    <template v-if="!valueType?.isList">
      <!-- Scalar -->
      <span
        :class="[
          TEXT_DIRECTION_BY_ALIGNMENT[alignment ?? Alignment.START] ?? '',
          modelValue != '' ? '' : 'text-gray-400',
        ]"
      >
        {{ modelValue || placeholder }}
      </span>
    </template>
    <template v-else>
      <!-- List -->
      <div v-for="(v, i) in values" :key="i" class="truncate rounded bg-gray-100 px-1">{{ v }}</div>
    </template>
  </div>
</template>
<style scoped>
/* hide up/down buttons for number input */
input[type="number"]:not(:focus)::-webkit-outer-spin-button,
input[type="number"]:not(:focus)::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
input[type="number"]:not(:focus) {
  appearance: textfield;
  -moz-appearance: textfield;
}
</style>
