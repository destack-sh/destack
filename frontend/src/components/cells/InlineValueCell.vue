<script lang="ts" setup>
import ObjectValueCell from "@/components/cells/ObjectValueCell.vue";
import { TypeTag, type SimpleType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { OBJECT_TYPETAGS } from "@/state/object";
import { symbolOf, TypeFlag } from "@/state/runtime";
import { syncProperty } from "@/utils/sync";
import { Combobox, ComboboxButton, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { EyeIcon } from "@heroicons/vue/24/outline";
import { onClickOutside, onStartTyping, useFocus } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const props = defineProps<{
  modelValue: any;
  placeholderValue?: any;
  type: SimpleType;
  readonly: boolean;
  immediate: boolean;
  active: boolean;
  debounced?: boolean;
  parentArray?: boolean;
  supportsDrop?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: any): void;
  (e: "dropFiles", p: "above" | "below", v: File[]): void;
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
const buttonRefFocused = useFocus(buttonRef as any);
const valueRef: Ref<HTMLInputElement | null> = ref(null);
const valueRefFocused = useFocus(valueRef as any);
const editableContainerRef: Ref<HTMLDivElement | null> = ref(null);
const comboboxButtonRef: Ref<InstanceType<typeof ComboboxButton> | null> = ref(null);

const readValue = computed(() => {
  let value;
  if (!editing.value && props.modelValue == null && props.placeholderValue) {
    value = props.placeholderValue;
  } else {
    value = props.modelValue;
  }
  // "coerce"
  if (props.type.tag == TypeTag.Boolean) {
    value = typeof value == "boolean" ? value : false;
  }
  return value;
});

// force is used for immediate updates from e.g. selects (that don't need debouncing)
function writeValue(val: any, force?: boolean) {
  value.value = val;
  if (props.immediate && (!props.debounced || force)) {
    emit("update:modelValue", val);
  }
}

if (props.debounced) {
  syncProperty({
    value,
    editing,
    read: () => (value.value = props.modelValue),
    write: () => emit("update:modelValue", value.value),
  });
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

onClickOutside(editableContainerRef, () => {
  if (editing.value) {
    confirm();
  }
});

// auto-start editing on typing if not editing and focused
onStartTyping(() => {
  if (!editing.value && buttonRefFocused.focused.value) {
    editing.value = true;
    nextTick(focus);
  }
});

function edit(event: KeyboardEvent | MouseEvent) {
  if (props.readonly) {
    return;
  }
  // ignore if click and click is on a different button
  // (inner elements may also be clicked)
  if (event instanceof MouseEvent && event.target != buttonRef.value && event.target instanceof HTMLButtonElement) {
    return;
  }
  if (OBJECT_TYPETAGS.includes(props.type.tag)) {
    valueRef.value?.open();
    return;
  } // don't prevent anything, need to open file chooser
  event.preventDefault();
  if (event instanceof KeyboardEvent) {
    // we want to propagate clicks to manage focus upstream
    event.stopPropagation();
  }
  if (editing.value) {
    return;
  }

  if (props.type.tag == TypeTag.Boolean) {
    writeValue(!readValue.value, true);
    confirm();
    return;
  }

  editing.value = true;
  nextTick(focus);
}

function focus() {
  if (!editing.value) {
    buttonRef.value?.focus({ preventScroll: true });
  } else {
    valueRefFocused.focused.value = true;
    comboboxButtonRef.value?.$el.click();
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
  <div class="relative">
    <!-- (not actually a button because inputs can't be inside a button) -->
    <div
      ref="buttonRef"
      tabindex="-1"
      :disabled="readonly"
      @click="edit"
      @keydown.enter.exact="edit"
      @keydown.space.exact="edit"
      @keydown.left.exact="editing || emitPrevent($event, 'navigateLeft')"
      @keydown.right.exact="editing || emitPrevent($event, 'navigateRight')"
      @keydown.tab.exact.prevent="editing || emitPrevent($event, 'navigateRight')"
      @keydown.shift.tab.exact.prevent="editing || emitPrevent($event, 'navigateLeft')"
      @keydown.up.exact.prevent="editing || emitPrevent($event, 'navigateUp')"
      @keydown.down.exact.prevent="editing || emitPrevent($event, 'navigateDown')"
      @keydown.backspace.exact.prevent="editing || emitPrevent($event, 'deleteLeft')"
      @keydown.delete.exact.prevent="editing || emitPrevent($event, 'deleteSelf')"
      @focus.stop.prevent="emit('focus', $event)"
      class="mousetrap-no-tab relative h-full max-h-28 w-full overflow-y-hidden text-left outline-none"
      :class="{
        'font-mono': editor.fontMono,
        'text-sm': editor.textSmall,
        'text-md': !editor.textSmall,
        'text-gray-300': readValue == placeholderValue,
      }"
    >
      <button
        v-if="type.flags & TypeFlag.IsSecret"
        class="group h-full w-full rounded-sm bg-gray-100"
        @click.prevent=""
      >
        &nbsp;
        <span
          class="absolute right-1 p-0.5 text-gray-400"
          :class="active ? '' : 'opacity-0 transition duration-150 group-hover:text-gray-700 group-hover:opacity-100'"
        >
          <EyeIcon class="h-4 w-4" />
        </span>
      </button>
      <!-- TODO @UX: edit and view arrays & structs -->
      <div
        v-else-if="type.flags & TypeFlag.IsArray && !parentArray"
        class="flex w-full flex-row flex-wrap gap-1.5 px-1"
      >
        <span class="text-xs text-gray-500" v-if="modelValue?.length == 0">({{ modelValue?.length }} elements)</span>
        <InlineValueCell
          v-for="(value, index) in modelValue"
          :type="type"
          :model-value="value"
          :key="index"
          :readonly="true"
          :immediate="false"
          :value="value"
          :active="active"
          parent-array
        />
      </div>
      <!-- Content preview -->
      <span ref="valueRef" class="text-left" v-else-if="type.tag == TypeTag.String">{{ readValue }}&nbsp;</span>
      <span ref="valueRef" class="text-right" v-else-if="type.tag == TypeTag.Number">{{ readValue }}&nbsp;</span>
      <input
        ref="valueRef"
        type="checkbox"
        class="h-4 w-4 rounded border-gray-300 text-orange-600 focus:ring-orange-600"
        v-else-if="type.tag == TypeTag.Boolean"
        :checked="readValue"
        :disabled="props.readonly"
      />
      <span ref="valueRef" class="" v-else-if="type.tag == TypeTag.Enum">{{ readValue }}&nbsp;</span>
      <ObjectValueCell
        ref="valueRef"
        v-else-if="OBJECT_TYPETAGS.includes(type.tag)"
        :type="type"
        :modelValue="readValue"
        @update:modelValue="writeValue($event, true)"
        :readonly="readonly"
        :active="active"
        :supportsDrop="props.supportsDrop"
        @dropFiles="(p, v) => emit('dropFiles', p, v)"
      />
      <div
        v-else-if="type.tag == TypeTag.Struct"
        class="flex w-full flex-row flex-wrap gap-2 border border-orange-900 border-opacity-[12%] p-1"
      >
        <div v-for="field in structFields" :key="field.name" class="flex flex-col">
          <span class="text-left text-xs text-gray-500">{{ field.name }}</span>
          <InlineValueCell
            v-if="field.tag != TypeTag.Struct"
            :type="field"
            :modelValue="readValue[field.name]"
            :placeholderValue="field.name"
            :readonly="true"
            :active="active"
            :immediate="false"
          />
        </div>
      </div>
      <!-- Can't render this type! -->
      <span ref="valueRef" v-else class="">{{ readValue }}&nbsp;</span>
    </div>
    <!-- Editable content (overlay) :EditableCellStyle -->
    <div
      class="absolute -left-1.5 -top-1.5 z-20 flex w-fit min-w-full flex-row items-baseline border border-solid border-orange-900 border-opacity-[12%] bg-white p-1.5 shadow-md"
      ref="editableContainerRef"
      v-if="editing"
      @click.prevent="emit('edit')"
    >
      <!-- Strings and numbers -->
      <textarea
        v-if="type.tag == TypeTag.String"
        :value="value"
        @input="(e: any) => writeValue(e.target?.value)"
        ref="valueRef"
        type="text"
        spellcheck="false"
        class="w-full rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
        :class="[editor.textSmall ? 'text-sm' : '', 'min-w-[300px]']"
        @keydown.enter.exact.prevent="confirm"
        @keydown.escape.exact.prevent="confirm"
        @keydown.tab.exact.prevent="
          confirm();
          emit('navigateRight');
        "
        :placeholder="placeholderValue ?? ''"
      />
      <input
        v-else-if="type.tag == TypeTag.Number"
        :value="value"
        @input="(e: any) => writeValue(Number.parseFloat(e.target?.value))"
        ref="valueRef"
        type="number"
        spellcheck="false"
        class="w-full min-w-0 rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
        :class="[editor.textSmall ? 'text-sm' : '']"
        @keydown.enter.exact.prevent="confirm"
        @keydown.escape.exact.prevent="confirm"
        @keydown.tab.exact.prevent="
          confirm();
          emit('navigateRight');
        "
        :placeholder="placeholderValue ?? ''"
      />
      <!-- Enum options -->
      <Combobox
        v-else-if="type.tag == TypeTag.Enum"
        as="div"
        class="flex w-full flex-col"
        :model-value="enumMembers.find((n) => n.value == value)"
        @update:model-value="(val: SimpleType) => (writeValue(val?.value, true), confirm())"
      >
        <!-- Hidden button to manage focus programmatically -->
        <ComboboxButton class="hidden" ref="comboboxButtonRef" />
        <ComboboxInput
          as="input"
          ref="valueRef"
          spellcheck="false"
          class="w-full min-w-0 rounded-none border-none bg-transparent p-0 outline-none ring-0 focus:ring-0"
          :class="[editor.textSmall ? 'text-sm' : '']"
          :display-value="(val: any) => ''"
          :placeholder="placeholderValue ?? '...'"
          @keyup.escape.prevent="confirm"
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
                'relative cursor-default select-none px-1 py-0.5',
                active ? 'bg-orange-100' : '',
                selected ? 'text-orange-600' : 'text-gray-900',
              ]"
            >
              {{ member.name }}
            </li>
          </ComboboxOption>
        </ComboboxOptions>
      </Combobox>
      <!-- Uneditable -->
      <span v-else class="text-red-500">{{ readValue || "panic (" + type.tag + ")" }}</span>
    </div>
  </div>
</template>
