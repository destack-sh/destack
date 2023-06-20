<script lang="ts" setup>
import { useAppearance } from "@/state/appearance";
import { useCurrentModule, TypeFlag } from "@/state/module";
import { computed, type Ref, ref } from "vue";
import { Combobox, ComboboxButton, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { getEnumColor, type Field } from "@/state/statement";
import uFuzzy from "@leeoniya/ufuzzy";

const props = defineProps<{
  type: Field;
  modelValue?: string[];
  readonly?: boolean;
  preview?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string[]): void;
  (e: "close"): void;
}>();

const module = useCurrentModule();
const isArray = computed(() => Boolean(props.type.flags & TypeFlag.IsArray));
const runtimeType = computed(() => module.statementOf(props.type.reference?.id));

const members = computed(() => {
  return runtimeType.value?.fields ?? [];
});
const selectedMembers = computed(
  () => (props.modelValue?.map((v) => members.value.find((m) => m.key == v)).filter((m) => m != null) as Field[]) ?? []
);
const missingMembers = computed(() => members.value.filter((m) => !props.modelValue?.find((v) => v == m.key)));
const query = ref("");
const uf = new uFuzzy({ intraMode: 0 });
const filteredMembers = computed(() => {
  const baseMembers = isArray.value ? missingMembers.value : members.value;
  if (query.value.trim() == "") return baseMembers;
  const [idxs] = uf.search(
    baseMembers.map((m) => m.name),
    query.value
  );
  return idxs?.map((idx) => baseMembers[idx]) ?? [];
});

const queryRef: Ref<InstanceType<typeof ComboboxInput> | null> = ref(null);

function removeValue(key: string) {
  emit("update:modelValue", props.modelValue?.filter((v) => v != key) ?? []);
}

const appearance = useAppearance();

defineExpose({
  focus: () => queryRef.value?.$el.focus(),
  blur: () => queryRef.value?.$el.blur(),
});
</script>
<template>
  <div class="flex h-full w-full flex-row flex-wrap gap-1">
    <!-- :EnumStyle -->
    <!-- Existing members -->
    <template v-if="isArray || preview">
      <span
        v-for="member in selectedMembers"
        :key="member.key"
        class="inline-flex items-center gap-x-1.5 rounded-sm bg-gray-100 px-2 text-gray-900"
      >
        <svg class="h-1.5 w-1.5" :style="{ fill: getEnumColor(member) }" viewBox="0 0 6 6" aria-hidden="true">
          <circle cx="3" cy="3" r="3" />
        </svg>
        {{ member.name }}
        <!-- Delete button -->
        <button
          v-if="!preview && !readonly && isArray"
          class="p-0.5 text-gray-300 hover:text-gray-700"
          @click="removeValue(member.key)"
        >
          x
        </button>
      </span>
    </template>
    <!-- Ensure there's always something -->
    <template v-if="selectedMembers.length == 0 && preview">&nbsp;</template>
    <!-- TODO @Feature @UX: add missing enum members inline -->
    <Combobox
      v-if="!preview"
      as="div"
      class="flex w-full min-w-[300px] flex-col"
      :model-value="isArray ? null : selectedMembers[0]"
      @update:model-value="(val: Field) => {
        query = '';
        if (isArray) {
          emit('update:modelValue', [...(modelValue ?? []), val.key as string]);
        } else {
          emit('update:modelValue', [val.key as string]);
          emit('close')
        }
      }"
    >
      <!-- Hidden button to manage focus programmatically -->
      <ComboboxButton class="hidden" ref="comboboxButtonRef" />
      <ComboboxInput
        as="input"
        ref="queryRef"
        :default-value="query"
        @change="query = $event.target.value"
        spellcheck="false"
        class="w-full min-w-0 rounded-none border-none bg-transparent p-0 outline-none ring-0 placeholder:text-gray-400 focus:ring-0"
        :class="[appearance.textSmall ? 'text-sm' : '', isArray ? 'mt-1' : '']"
        :display-value="(val: any) => ''"
        :placeholder="(isArray ? 'Add ' : 'Select ') + runtimeType?.name"
        @keydown.backspace.exact="
          queryRef?.$el.value.length > 0 || removeValue(modelValue?.[modelValue.length - 1] ?? '')
        "
      >
      </ComboboxInput>
      <ComboboxOptions class="max-h-80 w-full overflow-auto py-1 focus:outline-none" static>
        <ComboboxOption
          v-for="member in filteredMembers"
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
