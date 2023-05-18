<script lang="ts" setup>
import { useAppearance } from "@/state/appearance";
import { symbolOf, TypeFlag } from "@/state/runtime";
import { computed, type Ref, ref } from "vue";
import { Combobox, ComboboxButton, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { getEnumColor, type SimpleType } from "@/components/statement";

const props = defineProps<{
  type: SimpleType;
  readonly?: boolean;
  modelValue?: string[];
  preview?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string[]): void;
}>();

const isArray = computed(() => props.type.flags & TypeFlag.IsArray);
const runtimeType = computed(() => symbolOf(props.type.reference?.id));
const members = computed(() => {
  return runtimeType.value?.typeNodes ?? [];
});
const missingMembers = computed(() => members.value.filter((m) => !props.modelValue?.find((v) => v == m.name)));

const inputRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);

function getColorByValue(value: string) {
  const member = members.value.find((m) => m.name == value);
  if (member != null) {
    return getEnumColor(member);
  } else {
    return "gray";
  }
}

function removeValue(value: string) {
  emit("update:modelValue", props.modelValue?.filter((v) => v != value) ?? []);
}

const appearance = useAppearance();

defineExpose({
  focus: () => inputRef.value?.$el.focus(),
  blur: () => inputRef.value?.$el.blur(),
});
</script>
<template>
  <div class="flex h-full w-full flex-row flex-wrap gap-1">
    <!-- :EnumStyle -->
    <!-- Existing members (same as above but with edit button) -->
    <span
      v-for="value in modelValue"
      :key="value"
      class="inline-flex items-center gap-x-1.5 rounded-sm bg-gray-100 px-2 text-gray-900"
    >
      <svg class="h-1.5 w-1.5" :style="{ fill: getColorByValue(value) }" viewBox="0 0 6 6" aria-hidden="true">
        <circle cx="3" cy="3" r="3" />
      </svg>
      {{ value }}
      <button
        v-if="!preview && !readonly && isArray"
        class="p-0.5 text-gray-300 hover:text-gray-700"
        @click="removeValue(value)"
      >
        x
      </button>
    </span>
    <!-- TODO @Feature @UX: add missing enum members inline -->
    <Combobox
      v-if="!preview && missingMembers.length > 0"
      as="div"
      class="flex w-full flex-col"
      :model-value="modelValue"
      @update:model-value="(val: SimpleType) => {
        if (isArray) {
          emit('update:modelValue', [...(modelValue ?? []), val.name as string]);
        } else {
          emit('update:modelValue', [val.name as string]);
        }
      }"
    >
      <!-- Hidden button to manage focus programmatically -->
      <ComboboxButton class="hidden" ref="comboboxButtonRef" />
      <ComboboxInput
        as="input"
        ref="inputRef"
        spellcheck="false"
        class="mt-1.5 w-full min-w-0 rounded-none border-none bg-transparent p-0 outline-none ring-0 placeholder:text-gray-400 focus:ring-0"
        :class="[appearance.textSmall ? 'text-sm' : '']"
        :display-value="(val: any) => ''"
        :placeholder="isArray ? 'Add ' : 'Select ' + runtimeType?.name"
        @keydown.backspace.exact.prevent="
          inputRef?.$el.value.length > 0 || removeValue(modelValue?.[modelValue.length - 1] ?? '')
        "
      >
      </ComboboxInput>
      <ComboboxOptions class="max-h-80 w-full overflow-auto py-1 focus:outline-none" static>
        <ComboboxOption
          v-for="member in isArray ? missingMembers : members"
          :key="member.name ?? ''"
          :value="member"
          v-slot="{ active, selected }"
        >
          <li
            :class="[
              'relative flex cursor-default select-none flex-row items-center gap-1.5 px-1 py-0.5',
              active ? 'bg-orange-100' : '',
              selected ? 'text-orange-600' : 'text-gray-900',
            ]"
          >
            <svg class="h-1.5 w-1.5" :style="{ fill: getEnumColor(member) }" viewBox="0 0 6 6" aria-hidden="true">
              <circle cx="3" cy="3" r="3" />
            </svg>
            {{ member.name }}
          </li>
        </ComboboxOption>
      </ComboboxOptions>
    </Combobox>
  </div>
</template>
