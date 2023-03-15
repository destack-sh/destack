<script lang="ts" setup>
import { TypeTag, type SimpleType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { symbolOf } from "@/state/runtime";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { onClickOutside, useFocus } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const props = defineProps<{
  modelValue: any;
  placeholderValue?: any;
  type: SimpleType;
  readonly: boolean;
  immediate: boolean;
  slim?: boolean;
  parentArray?: boolean; // hack to prevent recursion, doesn't work for nested arrays
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: any): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "deleteSelf"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "edit"): void;
  (e: "focus", event: FocusEvent): void;
}>();

function emitPrevent(event: any, e: string, ...args: any[]) {
  event.preventDefault();
  emit(e, ...args);
}

// local copy of value
const value: Ref<any> = ref(props.modelValue);
const editing: Ref<boolean> = ref(false);
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const valueRef: Ref<HTMLInputElement | null> = ref(null);
const valueRefFocused = useFocus(valueRef as any);
const editableContainerRef: Ref<HTMLDivElement | null> = ref(null);

const readValue = computed(() => {
  if (!editing.value && !props.modelValue && props.placeholderValue) {
    return props.placeholderValue;
  } else {
    return props.modelValue;
  }
});
function writeValue(val: any) {
  if (props.type.tag == TypeTag.String) {
    // trim whitespace
    val = val.trim();
  }
  value.value = val;
  if (props.immediate) {
    emit("update:modelValue", val);
  }
}

function confirm() {
  if (!props.immediate) {
    emit("update:modelValue", value.value);
  }
  editing.value = false;
  nextTick(() => {
    emit("escape");
    buttonRef.value?.focus({ preventScroll: true });
  });
}

function cancel() {
  emit("escape");
  editing.value = false;
  value.value = props.modelValue;
  nextTick(() => buttonRef.value?.focus({ preventScroll: true }));
}

onClickOutside(editableContainerRef, () => {
  if (editing.value) {
    cancel();
  }
});

function edit(event: KeyboardEvent | MouseEvent) {
  if (props.readonly) {
    return;
  }
  event.preventDefault();
  if (event instanceof KeyboardEvent) {
    // we want to propagate clicks to manage focus upstream
    event.stopPropagation();
  }

  if (props.type.tag == TypeTag.Boolean) {
    writeValue(!readValue.value);
    confirm();
    return;
  }

  editing.value = true;
  nextTick(focus);
}

function focus() {
  if (!editing.value) {
    console.log("focus inline value cell");
    buttonRef.value?.focus({ preventScroll: true });
  } else {
    valueRefFocused.focused.value = true;
  }
}

// get enum members from runtime type of reference
const enumMembers = computed(() => {
  if (props.type.tag != TypeTag.Enum || props.type.reference == null) {
    return [];
  }
  const runtimeType = symbolOf(props.type.reference?.id);
  return runtimeType?.typeNodes ?? [];
});

const structFields = computed(() => {
  if (props.type.tag != TypeTag.Struct) {
    return [];
  }
  const runtimeType = symbolOf(props.type.reference?.id);
  return runtimeType?.typeNodes ?? [];
});

const editor = useEditorState();

defineExpose({
  editing,
  focus,
  blur: () => {
    buttonRef.value?.blur();
    valueRefFocused.focused.value = false;
  },
});
</script>
<template>
  <!-- Wrapper for selectable value container -->
  <button
    ref="buttonRef"
    tabindex="-1"
    :disabled="readonly"
    @click="edit"
    @keydown.enter.exact="edit"
    @keydown.left.exact="editing || emitPrevent($event, 'navigateLeft')"
    @keydown.right.exact="editing || emitPrevent($event, 'navigateRight')"
    @keydown.up.exact="editing || emitPrevent($event, 'navigateUp')"
    @keydown.down.exact="editing || emitPrevent($event, 'navigateDown')"
    @keydown.backspace.exact="editing || emitPrevent($event, 'deleteLeft')"
    @keydown.delete.exact="editing || emitPrevent($event, 'deleteSelf')"
    @focus.stop.prevent="emit('focus', $event)"
    class="relative text-left outline-none"
    :class="{
      'font-mono': editor.fontMono,
      'text-sm': editor.textSmall,
      'text-md': !editor.textSmall,
      'text-gray-300': readValue == placeholderValue,
    }"
  >
    <!-- Default content if empty and no special rendering-->
    <!-- TODO @Incomplete: edit array & struct values values -->
    <!-- TODO @UX: array & struct rendering (esp. nested) is ugly and hacky (nested InlineValueCells, see below) -->
    <div v-if="type.isArray && !parentArray" class="flex w-full flex-row flex-wrap gap-1.5 px-1">
      <span class="text-xs text-gray-500" v-if="modelValue?.length == 0">({{ modelValue?.length }} elements)</span>
      <InlineValueCell
        v-for="(value, index) in modelValue"
        :type="type"
        :model-value="value"
        :key="index"
        :readonly="true"
        :immediate="false"
        :value="value"
        parent-array
      />
    </div>
    <span v-else-if="readValue == null && type.tag != TypeTag.Boolean">&nbsp;</span>
    <!-- Content preview -->
    <span ref="valueRef" class="text-left" v-else-if="type.tag == TypeTag.String">{{ readValue }}</span>
    <span ref="valueRef" class="text-right" v-else-if="type.tag == TypeTag.Number">{{ readValue }}</span>
    <input
      ref="valueRef"
      type="checkbox"
      class="h-4 w-4 rounded border-gray-300 text-orange-600 focus:ring-orange-500"
      v-else-if="type.tag == TypeTag.Boolean"
      :checked="readValue"
      :disabled="props.readonly"
    />
    <span ref="valueRef" class="" v-else-if="type.tag == TypeTag.Enum">{{ readValue }}</span>
    <div
      v-else-if="type.tag == TypeTag.Struct"
      class="flex w-full flex-row flex-wrap gap-2 rounded-sm border border-orange-900 border-opacity-[12%] p-1"
    >
      <div v-for="field in structFields" :key="field.name" class="flex flex-col">
        <span class="text-left text-xs text-gray-500">{{ field.name }}</span>
        <InlineValueCell
          :type="field"
          :modelValue="readValue[field.name]"
          :placeholderValue="field.name"
          :readonly="true"
          :immediate="false"
        />
      </div>
    </div>
    <!-- Can't render this type! -->
    <span ref="valueRef" v-else class="">{{ readValue }}</span>
    <!-- Editable content (overlay) :EditableCellStyle -->
    <div
      class="absolute -left-0.5 -top-0.5 z-20 flex w-fit flex-row items-baseline rounded-sm border border-solid border-orange-600 bg-orange-100 p-1"
      ref="editableContainerRef"
      v-if="editing"
      @click.prevent="emit('edit')"
    >
      <!-- Button to confirm if not immediate and there are pending changes -->
      <button class="absolute right-1 text-xs text-gray-500" @click="confirm" v-if="!immediate && value != modelValue">
        *
      </button>
      <!-- Strings and numbers -->
      <textarea
        v-if="type.tag == TypeTag.String"
        :value="value"
        @input="(e: any) => writeValue(e.target?.value)"
        ref="valueRef"
        type="text"
        spellcheck="false"
        class="w-full rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
        :class="[editor.textSmall ? 'text-sm' : '', slim ? 'min-w-[200px]' : ' min-w-[300px]']"
        @keydown.enter.exact.prevent="confirm"
        @keydown.escape.exact.prevent="cancel"
        :placeholder="placeholderValue ?? ''"
        :rows="slim ? 1 : 3"
      />
      <input
        v-else-if="type.tag == TypeTag.Number"
        :value="value"
        @input="(e: any) => writeValue(e.target?.value)"
        ref="valueRef"
        type="number"
        spellcheck="false"
        class="w-full min-w-0 rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
        :class="[editor.textSmall ? 'text-sm' : '']"
        @keydown.enter.exact.prevent="confirm"
        @keydown.escape.exact.prevent="cancel"
        :placeholder="placeholderValue ?? ''"
      />
      <!-- Enum options -->
      <Combobox
        v-else-if="type.tag == TypeTag.Enum"
        as="div"
        class="flex flex-col"
        :model-value="enumMembers.find((n) => n.value == value)"
        @update:model-value="(val: SimpleType) => (writeValue(val?.value), confirm())"
      >
        <ComboboxInput
          as="input"
          ref="valueRef"
          spellcheck="false"
          class="w-full min-w-0 rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
          :class="[editor.textSmall ? 'text-sm' : '']"
          :display-value="(val: any) => val?.name"
          :placeholder="placeholderValue ?? '...'"
          @keyup.escape.prevent="cancel"
        >
        </ComboboxInput>
        <ComboboxOptions class="max-h-80 w-full overflow-auto py-1 focus:outline-none" static>
          <ComboboxOption
            v-for="member in enumMembers"
            :key="member.name"
            :value="member"
            v-slot="{ active, selected }"
          >
            <li
              :class="[
                'relative cursor-default select-none py-0.5 px-2',
                active ? 'bg-orange-600 text-white' : 'text-gray-900',
                selected ? 'underline' : '',
              ]"
            >
              {{ member.name }}
            </li>
          </ComboboxOption>
        </ComboboxOptions>
      </Combobox>
      <!-- Uneditable -->
      <span v-else class="text-red-500">{{ readValue || "panic!" }}</span>
    </div>
  </button>
</template>
