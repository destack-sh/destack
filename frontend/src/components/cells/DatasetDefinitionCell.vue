<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { TypeTag } from "@/gql/graphql";
import { generateKeyBetween } from "@/utils/fractional";
import { computed, ref, type Ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(description);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);

// assumes this is a struct dataset
if (context.typeNodeRoot.value?.tag != TypeTag.Struct) {
  throw new Error("unexpected type node tag: " + context.typeNodeRoot.value?.tag);
}
const fieldTypeNodes = computed(() => context.typeNodesChildren.value);
const records = computed(() => context.statement.value.records);
const recordsLength = computed(() => records.value?.length ?? 0);

function focusFirstIfExists() {
  console.log("focus first");
}

function focusLast() {
  console.log("focus last");
}

function insertBelow(recordId?: string) {
  let orderKey;
  if (recordId == null) {
    const lastRecord = records.value?.[recordsLength.value - 1];
    orderKey = generateKeyBetween(lastRecord?.orderKey ?? null, null);
  } else {
    const record = records.value?.find((r) => r.id === recordId);
    orderKey = generateKeyBetween(record?.orderKey ?? null, null);
  }
}

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
  },
});
</script>
<template>
  <!-- Declaration -->
  <DeclarationCell
    ref="declarationRef"
    @navigate-down="descriptionRef?.focus"
    @navigate-right="descriptionRef?.focus"
  />
  <!-- Reference type -->
  <!-- TODO @Incomplete: set dataset type to reference -->
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-left="declarationRef?.focus()"
    @navigate-up="declarationRef?.focus()"
    @navigate-down="focusFirstIfExists"
    @enter="context.insertBelow"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
  >
    +description
  </button>
  <!-- Dataset type headers -->
  <!-- TODO @Incomplete: dataset type editing -->
  <div class="grid w-fit gap-x-3">
    <div v-for="fieldNode in fieldTypeNodes" :key="fieldNode?.id">
      {{ fieldNode.name }}
    </div>
  </div>
  <!-- Dataset records -->
  <div class="grid w-fit gap-x-3">
    <template v-for="record in records" :key="record.id">
      <template v-for="fieldNode in fieldTypeNodes" :key="record.id + '.' + fieldNode?.id">
        {{ record.data?.[fieldNode.name] }}
      </template>
    </template>
    <button
      tabindex="-1"
      ref="addRecordRef"
      class="w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
      @click="insertBelow()"
      @enter="insertBelow()"
      @keydown.up.exact="focusLast"
      @keydown.down.exact="context.navigateDown"
    >
      +record
    </button>
  </div>
  <!-- TODO @Incomplete: dataset record editing -->
</template>
