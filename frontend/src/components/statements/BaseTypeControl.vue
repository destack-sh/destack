<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import type { Field } from "@/state/module";
import TypeInterface from "@/components/interfaces/TypeInterface.vue";
import { TypeTag } from "@/gql/graphql";
import { useFields } from "@/state/statement";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { toRef } from "vue";
import type { StatementAction } from "@/state/bench";
import { CubeTransparentIcon } from "@heroicons/vue/24/outline";

const props = defineProps<Pick<StatementProps, "statement" | "readonly" | "focused" | "editing">>();
const emit = defineEmits<StatementEmit>();

const baseTypesRefs = useElementRefs<InstanceType<typeof TypeInterface>>();
const { baseTypes, updateField, deleteField, createUnionField } = useFields(toRef(props, "statement"));

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
  },
  actions: [
    {
      label: "Inherit type",
      groupId: "edit",
      icon: CubeTransparentIcon,
      hideInline: true,
      action: () => {
        createUnionField();
      },
    },
  ] as StatementAction[],
});
</script>
<template>
  <!-- Base types -->
  <!-- TODO @UX: clean up base types control -->
  <div class="inline-flex flex-row gap-x-1">
    <TypeInterface
      v-for="field of baseTypes"
      :key="field.id"
      :ref="(el: any) => baseTypesRefs.registerRef(field.id, el)"
      :model-value="field"
      @update:model-value="(val) => updateField(field, val as Field)"
      @delete-self="deleteField(field)"
      @navigate-left="
        field.id == baseTypes[0].id
          ? emit('navigateLeft')
          : baseTypesRefs.focus(baseTypes[baseTypes.findIndex((n) => n.id == field.id) - 1].id)
      "
      @navigate-right="
        field.id == baseTypes[baseTypes.length - 1].id
          ? emit('navigateRight')
          : baseTypesRefs.focus(baseTypes[baseTypes.findIndex((n) => n.id == field.id) + 1].id)
      "
      @navigate-down="emit('navigateDown')"
      @navigate-up="emit('navigateUp')"
      :active="focused"
      :readonly="readonly"
      ref-only
      :ref-types="[TypeTag.Struct, TypeTag.Function]"
      hide-flags
      class="-mt-[1px] w-full rounded-sm border border-transparent border-opacity-[15%] text-orange-600 focus-within:border-solid focus-within:border-orange-900 focus-within:bg-orange-100 hover:bg-orange-100 focus:bg-orange-100"
    />
  </div>
</template>
