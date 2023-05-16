<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { ANY_TYPE_NODE, renderSimpleType } from "@/components/statement";
import type { SimpleType } from "@/gql/graphql";
import { syncProperty } from "@/utils/sync";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { PencilIcon } from "@heroicons/vue/24/outline";
import TrashIcon from "@heroicons/vue/24/outline/TrashIcon";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{
  modelValue?: SimpleType;
  readonly: boolean;
  inlined?: boolean;
  structrefOnly?: boolean;
  hideFlags?: boolean;
  extraActions?: Action[];
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Pick<SimpleType, "name" | "tag" | "flags" | "reference">): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
  (e: "deleteLeft"): void;
  (e: "deleteSelf"): void;
  (e: "enter"): void;
  (e: "escape"): void;
  (e: "focus", event: FocusEvent): void;
}>();

const value: Ref<SimpleType> = ref(props.modelValue ?? ANY_TYPE_NODE);
const name: Ref<string> = ref(props.modelValue?.name ?? "");
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const typeButtonRef: Ref<InstanceType<typeof PopoverButton> | null> = ref(null);
const popoverButtonRef: Ref<InstanceType<typeof PopoverButton> | null> = ref(null);
const popoverOpenRef: Ref<HTMLSpanElement | null> = ref(null);

syncProperty({
  value: name,
  editing: computed(() => nameRef.value?.focused),
  read: () => (name.value = value.value.name ?? ""),
  write: () => {
    value.value = {
      ...value.value,
      name: name.value,
    };
    emit("update:modelValue", value.value);
  },
});

// sync model value into local value
watch(
  () => [props.modelValue],
  () => {
    value.value = props.modelValue ?? ANY_TYPE_NODE;
  }
);

type Action = {
  label: string;
  icon: any;
  action: () => boolean;
};
const actions = computed(() => {
  const actions = [];
  if (!props.readonly) {
    actions.push({
      label: "Edit type",
      icon: PencilIcon,
      action: () => {
        typeButtonRef.value?.$el.click();
        return false;
      },
    });
    actions.push({
      label: "Delete",
      icon: TrashIcon,
      action: () => {
        emit("deleteLeft");
        return true;
      },
    });
  }
  if (props.extraActions != null) {
    actions.push(...props.extraActions);
  }
  return actions;
});

function open() {
  if (popoverOpenRef.value == null) {
    popoverButtonRef.value?.$el.click();
    nextTick(() => nameRef.value?.focus());
  }
}

function focus() {
  buttonRef.value?.focus();
}

function blur() {
  buttonRef.value?.blur();
}

defineExpose({
  editing: computed(() => popoverOpenRef.value != null),
  focus,
  blur,
});
</script>
<template>
  <Popover as="div" class="relative" v-slot="{ close }">
    <!-- Preview -->
    <button
      ref="buttonRef"
      tabindex="-1"
      @keydown.left.exact.prevent="emit('navigateLeft')"
      @keydown.right.exact.prevent="emit('navigateRight')"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.delete.exact="editing || emit('deleteSelf')"
      class="h-full w-full text-left outline-none"
      @click="open"
      @keydown.enter.exact.prevent="open"
    >
      <span
        class="mr-2 text-gray-900"
        :class="inlined ? 'underline decoration-gray-400 decoration-dashed underline-offset-4' : ''"
        >{{ value.name }}</span
      >
      <span>
        {{ renderSimpleType(value) }}
      </span>
    </button>
    <!-- Hidden popover button to proxy the button to because I can't figure out key events on the popover button directly -->
    <PopoverButton ref="popoverButtonRef" @focus.prevent="focus" class="hidden" />
    <!-- Edit popover -->
    <FadeTransition>
      <PopoverPanel
        class="absolute -left-2 -top-2 z-10 flex w-64 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <span ref="popoverOpenRef" class="hidden" />
        <!-- Name & type -->
        <div class="flex flex-row items-baseline justify-between gap-2">
          <!-- Name -->
          <EditableSpan
            ref="nameRef"
            v-model="name"
            :readonly="readonly"
            class="w-full rounded-sm border border-orange-900 border-opacity-[12%] p-1 text-gray-900 focus:bg-orange-100"
            @navigate-right="typeButtonRef?.$el.focus()"
          />
          <!-- Type popover -->
          <Popover as="div" class="relative">
            <PopoverButton
              ref="typeButtonRef"
              :disabled="readonly"
              class="rounded-sm border border-orange-900 border-opacity-[12%] p-1 text-gray-900 focus:bg-orange-100"
              :class="readonly ? '' : 'hover:bg-orange-100'"
              @keydown.left.exact.prevent="nameRef?.focus()"
            >
              {{ renderSimpleType(value) }}
            </PopoverButton>
            <PopoverPanel
              class="absolute -left-1 -top-10 z-10 flex w-64 flex-col gap-2 rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
            >
              <SelectTypeCell :model-value="value" @update:model-value="emit('update:modelValue', $event)" />
            </PopoverPanel>
          </Popover>
        </div>
        <!-- Actions -->
        <div class="mt-0.5 flex flex-col gap-1" v-if="actions.length > 0">
          <button
            v-for="action in actions"
            :key="action.label"
            class="flex w-full flex-row items-center gap-2.5 rounded-sm px-2 py-1 hover:bg-orange-100"
            @click="action.action() && close()"
          >
            <component :is="action.icon" class="h-4 w-4 text-gray-500" />
            <span class="text-gray-700">{{ action.label }}</span>
          </button>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
