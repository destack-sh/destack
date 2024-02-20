<script lang="ts" setup>
import { TypeTag, type Field, StatementType } from "@/gql/graphql";
import { useCurrentModule, useNavigation } from "@/state/module";
import { STATEMENT_TYPE_TAGS, getStatementIconOutline } from "@/state/statement";
import { ICONS_BY_HINT_OUTLINE, ICONS_BY_TAG_OUTLINE, renderBuiltinType } from "@/state/type";
import { computed } from "vue";

const props = defineProps<{
  type: Field;
  showTypeName?: boolean;
  hideIcon?: boolean;
  hideReference?: boolean;
}>();

const module = useCurrentModule();
const resolvedReference = computed(() => {
  if (props.type.referenceCk != null) {
    return module.statementOf(props.type.referenceCk);
  } else {
    return null;
  }
});

const resolvedTag = computed(
  () => STATEMENT_TYPE_TAGS[resolvedReference.value?.type as StatementType] ?? props.type.tag
);

const icon = computed(() => {
  if (
    resolvedReference.value != null &&
    [StatementType.Code, StatementType.Flow, StatementType.Task, StatementType.Database].includes(
      resolvedReference.value?.type
    )
  ) {
    return getStatementIconOutline(resolvedReference.value?.type);
  } else if (props.type.hint != null && ICONS_BY_HINT_OUTLINE[props.type.hint] != null) {
    return ICONS_BY_HINT_OUTLINE[props.type.hint];
  } else if (ICONS_BY_TAG_OUTLINE[resolvedTag.value] != null) {
    return ICONS_BY_TAG_OUTLINE[resolvedTag.value];
  } else {
    return null;
  }
});
</script>
<template>
  <!-- TODO @Cleanup: merge TypePreview component into call sites (just use getIcon function above) -->
  <component :is="icon" class="h-4 w-4" />
</template>
