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
}>();

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
    buttonRef.value?.focus();
  });
}

function cancel() {
  emit("escape");
  editing.value = false;
  value.value = props.modelValue;
  nextTick(() => buttonRef.value?.focus());
}

onClickOutside(editableContainerRef, () => {
  if (editing.value) {
    cancel();
  }
});

function edit() {
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
    buttonRef.value?.focus();
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
  <div
    class="relative"
    :class="{ 'font-mono': editor.fontMono, 'text-sm': editor.textSmall, 'text-md': !editor.textSmall }"
  >
    <button
      class="flex h-full w-full outline-none outline-transparent ring-0"
      :class="readValue == placeholderValue ? 'text-gray-300' : ''"
      tabindex="-1"
      ref="buttonRef"
      @click="edit"
      @keydown.enter.exact="edit"
      @keydown.left.exact="editing || emit('navigateLeft')"
      @keydown.right.exact="editing || emit('navigateRight')"
      @keydown.up.exact="editing || emit('navigateUp')"
      @keydown.down.exact="editing || emit('navigateDown')"
      @keydown.backspace.exact="editing || emit('deleteLeft')"
      @keydown.delete.exact="editing || emit('deleteSelf')"
    >
      <!-- Default content if empty and no special rendering-->
      <span v-if="readValue == null && type.tag != TypeTag.Boolean">&nbsp;</span>
      <!-- Content preview -->
      <!-- TODO @Incomplete: edit array values -->
      <span ref="valueRef" class="text-left" v-if="type.tag == TypeTag.String">{{ readValue }}</span>
      <span ref="valueRef" class="text-right" v-else-if="type.tag == TypeTag.Number">{{ readValue }}</span>
      <input
        ref="valueRef"
        type="checkbox"
        class="h-4 w-4 rounded border-gray-300 text-orange-600 focus:ring-orange-500"
        v-else-if="type.tag == TypeTag.Boolean"
        :checked="readValue"
      />
      <span ref="valueRef" class="" v-else-if="type.tag == TypeTag.Enum">{{ readValue }}</span>
      <!-- Can't render this type! -->
      <span ref="valueRef" v-else class="text-red-500">{{ readValue }}</span>
    </button>
    <!-- Editable content (overlay) :EditableCellStyle -->
    <div
      class="absolute -left-0.5 -top-0.5 z-20 flex w-fit min-w-[200px] flex-row items-baseline rounded-sm border border-solid border-black bg-orange-50 p-1"
      ref="editableContainerRef"
      v-if="editing"
      @click.prevent="emit('edit')"
    >
      <!-- Button to confirm if not immediate and there are pending changes -->
      <button class="absolute right-1 text-xs text-gray-500" @click="confirm" v-if="!immediate && value != modelValue">
        *
      </button>
      <!-- Strings and numbers -->
      <input
        v-if="type.tag == TypeTag.String || type.tag == TypeTag.Number"
        :value="value"
        @input="(e: any) => writeValue(e.target?.value)"
        ref="valueRef"
        :type="type.tag == TypeTag.String ? 'text' : 'number'"
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
          :display-value="(val: any) => val?.name"
          :placeholder="placeholderValue ?? '...'"
          @keyup.escape.prevent="cancel"
        >
        </ComboboxInput>
        <ComboboxOptions class="max-h-80 w-full overflow-auto py-1 text-base focus:outline-none" static>
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
  </div>
</template>
