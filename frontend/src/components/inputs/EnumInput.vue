<script lang="ts" setup>
import { useAppearance } from "@/state/appearance";
import { useCurrentModule, TypeFlag, type Field, useNavigation } from "@/state/module";
import { computed, type Ref, ref } from "vue";
import { Combobox, ComboboxButton, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { getEnumColor } from "@/state/statement";
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
const nav = useNavigation();
const isArray = computed(() => Boolean(props.type.flags & TypeFlag.IS_ARRAY));
const runtimeType = computed(() => module.statementOf(props.type.referenceCk));

const members = computed(() => {
  return (
    runtimeType.value?.fields.filter((f) => f.deletedAt == null).sort((a, b) => a.orderKey.localeCompare(b.orderKey)) ??
    []
  );
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
  const [idxs, info, order] = uf.search(
    baseMembers.map((m) => m.name ?? ""),
    query.value,
    true
  );
  if (idxs && order) {
    return order.map((i) => baseMembers[idxs[i]]);
  }
  return baseMembers;
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
    <span
      v-for="member in selectedMembers"
      :key="member.key"
      class="inline-flex items-center gap-x-1 rounded-sm px-1.5 text-gray-900 hover:cursor-pointer hover:bg-amber-100"
    >
      <svg class="h-[8px] w-[8px]" :style="{ fill: getEnumColor(member) }" viewBox="0 0 6 6" aria-hidden="true">
        <rect rx="2" ry="2" width="5" height="6" />
      </svg>
      <span class="underline decoration-gray-300 underline-offset-4">
        {{ member.name }}
      </span>
      <!-- Delete button -->
      <button
        v-if="!preview && !readonly"
        class="p-0.5 text-gray-300 hover:text-gray-700"
        @click.stop="removeValue(member.key)"
      >
        x
      </button>
    </span>
    <!-- Ensure there's always something -->
    <template v-if="selectedMembers.length == 0 && preview">&nbsp;</template>
    <!-- TODO @UX: add new enum members inline -->
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
        class="mx-1.5 mt-1 w-full min-w-0 rounded-none border-none bg-transparent p-0 outline-none ring-0 placeholder:text-gray-400 focus:ring-0"
        :class="[appearance.textSmall ? 'text-sm' : '', isArray ? 'mt-1' : '']"
        :display-value="(val: any) => ''"
        :placeholder="(isArray ? 'Add ' : 'Select ') + runtimeType?.name"
        @keydown.backspace.exact="
          queryRef?.$el.value.length > 0 || removeValue(modelValue?.[modelValue.length - 1] ?? '')
        "
      >
      </ComboboxInput>
      <ComboboxOptions
        class="mt-1 flex max-h-80 w-full flex-col gap-y-0.5 overflow-auto border-t py-1 pt-1.5 focus:outline-none"
        static
      >
        <ComboboxOption
          v-for="member in filteredMembers"
          :key="member.name ?? ''"
          :value="member"
          v-slot="{ active, selected }"
        >
          <div
            :class="[
              'relative w-full cursor-default select-none hover:cursor-pointer ',
              selected ? 'text-amber-600' : 'text-gray-900',
            ]"
          >
            <li
              class="mx-1 flex w-fit flex-row items-center gap-1.5 px-1.5 py-0.5"
              :class="[active ? 'bg-amber-100' : '']"
            >
              <svg class="h-[8px] w-[8px]" :style="{ fill: getEnumColor(member) }" viewBox="0 0 6 6" aria-hidden="true">
                <rect rx="2" ry="2" width="5" height="6" />
              </svg>
              <span class="">
                {{ member.name }}
              </span>
            </li>
          </div>
        </ComboboxOption>
      </ComboboxOptions>
    </Combobox>
  </div>
</template>
