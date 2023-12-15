<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import { useCurrentModule, type Field } from "@/state/module";
import type TypeInterface from "@/components/interfaces/TypeInterface.vue";
import { TypeTag } from "@/gql/graphql";
import { useFields } from "@/state/statement";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { computed, ref, toRef } from "vue";
import type { StatementAction } from "@/state/bench";
import { CubeTransparentIcon } from "@heroicons/vue/24/solid";
import { pinAbsoluteElement } from "@/composables/useFixed";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import SelectTypeInterface from "@/components/interfaces/SelectTypeInterface.vue";
import { TrashIcon } from "@heroicons/vue/24/outline";
import { IS_DEBUG } from "@/utils/globals";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const module = useCurrentModule();
const open = ref(false);
const editingField = ref<Field | null>(null);
const popoverRef = ref<HTMLDivElement | null>(null);
const popoverPin = pinAbsoluteElement(popoverRef, { pos: true, keepInView: true });

const baseTypesRefs = useElementRefs<InstanceType<typeof TypeInterface>>();
const { baseTypes, updateField, deleteField, createUnionField } = useFields(toRef(props, "statement"));
const resolvedBaseTypes = computed(() => baseTypes.value.map((f) => module.statementOf(f.referenceCk)));

function onSelect(field: Field) {
  if (editingField.value != null) {
    updateField(editingField.value, { ...editingField.value, referenceCk: field.referenceCk });
  } else {
    createUnionField(field.referenceCk);
  }
}

function show() {
  open.value = true;
}

function edit(field: Field) {
  editingField.value = field;
  open.value = true;
}

function hide() {
  open.value = false;
  editingField.value = null;
}

defineExpose({
  focus: (focus: "first" | "last" = "first") => {
    if (focus == "first") {
      baseTypesRefs.focus(baseTypes.value[0]?.id);
    } else {
      baseTypesRefs.focus(baseTypes.value[baseTypes.value.length - 1]?.id);
    }
  },
  blur: () => {
    baseTypesRefs.refs.value.forEach((r) => r.blur?.());
    hide();
  },
  actions: [
    {
      label: "Inherit type",
      groupId: "edit",
      icon: CubeTransparentIcon,
      hideInline: true,
      disabled: props.readonly,
      action: () => {
        show();
      },
    },
  ] as StatementAction[],
});
</script>
<template>
  <!-- Base types -->
  <div class="flex flex-row gap-1.5">
    <!-- Existing bases -->
    <button
      v-for="(base, i) in baseTypes"
      :key="base.id"
      :ref="(ref: any) => baseTypesRefs.registerRef(base.id, ref)"
      class="inline-flex flex-row rounded-xl px-1 text-fuchsia-900 ring-inset ring-fuchsia-600/20 hover:bg-fuchsia-200 hover:ring-1 focus:bg-fuchsia-200 focus:outline-none focus:ring-fuchsia-600/60"
      @click="edit(base)"
      @keydown.left.exact.prevent="i == 0 ? emit('navigateLeft') : baseTypesRefs.focus(baseTypes[i - 1]?.id)"
      @keydown.right.exact.prevent="
        i == baseTypes.length - 1 ? emit('navigateRight') : baseTypesRefs.focus(baseTypes[i + 1]?.id)
      "
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
    >
      <CubeTransparentIcon class="mr-1 mt-0.5 h-4 w-4 text-fuchsia-900" />
      <span class="font-semibold">
        {{ resolvedBaseTypes[i]?.name ?? (IS_DEBUG ? base.referenceCk ?? "missing id" : "???") }}
      </span>
    </button>
    <!-- Create/edit popup right next to bases -->
    <div
      v-if="open"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @keydown.escape="hide()"
      @click.stop="hide()"
    />
    <FadeTransition>
      <div
        v-if="open"
        @keydown.escape="hide()"
        ref="popoverRef"
        class="z-50 flex w-80 flex-col rounded-sm bg-white p-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :class="popoverPin.pinned.value ? '' : 'absolute -top-8'"
      >
        <!-- Header -->
        <div class="flex flex-row justify-between">
          <h5 class="px-1 text-left text-xs font-semibold text-gray-500">Inherit type</h5>
          <div class="item-center flex flex-row gap-1 px-1">
            <button
              v-if="editingField != null"
              class="p-0.5 text-gray-400 hover:bg-orange-100"
              @click="deleteField(editingField), hide()"
            >
              <TrashIcon class="h-4 w-4" />
            </button>
          </div>
        </div>
        <!-- Body -->
        <SelectTypeInterface
          class="mt-2"
          hide-flags
          ref-only
          :ref-types="[TypeTag.Struct, TypeTag.Function]"
          @update:model-value="onSelect($event as Field), hide()"
        />
        <!-- Later the select type will move into top right like in field interface
           and the body will be about picking/omitting fields (also like in field interface) -->
      </div>
    </FadeTransition>
  </div>
</template>
