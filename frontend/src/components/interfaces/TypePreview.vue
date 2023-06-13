<script lang="ts" setup>
import { TypeHint, TypeTag, type Field } from "@/gql/graphql";
import { TypeFlag, useCurrentModule } from "@/state/module";
import { renderBuiltinType } from "@/state/type";
import {
  AdjustmentsHorizontalIcon,
  ArrowsRightLeftIcon,
  AtSymbolIcon,
  Bars3BottomLeftIcon,
  CalendarDaysIcon,
  CheckIcon,
  ChevronDoubleDownIcon,
  ClockIcon,
  CodeBracketIcon,
  DocumentIcon,
  FingerPrintIcon,
  HandThumbUpIcon,
  HashtagIcon,
  IdentificationIcon,
  KeyIcon,
  LinkIcon,
  ListBulletIcon,
  LockClosedIcon,
  MinusSmallIcon,
  PhoneIcon,
  PhotoIcon,
  QuestionMarkCircleIcon,
  RectangleGroupIcon,
  SparklesIcon,
  SpeakerWaveIcon,
  StarIcon,
  VideoCameraIcon,
} from "@heroicons/vue/24/outline";
import { computed } from "vue";

const props = defineProps<{
  type: Field;
  showTypeName?: boolean;
  hideIcon?: boolean;
  hideFlags?: boolean;
  hideReference?: boolean;
}>();

const module = useCurrentModule();
const resolvedReference = computed(() => {
  if (props.type.reference == null) {
    return;
  } else if (props.type.reference.name != null) {
    return props.type.reference;
  } else {
    return module.statementOf(props.type.reference.id);
  }
});

const resolvedTag = computed(() => resolvedReference.value?.rootTypeTag ?? props.type.tag);

const iconsByTag: Partial<Record<TypeTag, any>> = {
  [TypeTag.String]: Bars3BottomLeftIcon,
  [TypeTag.Number]: HashtagIcon,
  [TypeTag.Boolean]: CheckIcon,
  [TypeTag.Vector]: SparklesIcon,
  [TypeTag.Null]: MinusSmallIcon,
  [TypeTag.File]: DocumentIcon,
  [TypeTag.Struct]: RectangleGroupIcon,
  [TypeTag.Enum]: ChevronDoubleDownIcon,
};
const iconsByHint: Partial<Record<TypeHint, any>> = {
  // string
  [TypeHint.Name]: IdentificationIcon,
  [TypeHint.Uuid]: FingerPrintIcon,
  [TypeHint.Date]: CalendarDaysIcon,
  [TypeHint.Datetime]: CalendarDaysIcon,
  [TypeHint.Time]: ClockIcon,
  [TypeHint.Duration]: ClockIcon,
  [TypeHint.Url]: LinkIcon,
  [TypeHint.Email]: AtSymbolIcon,
  [TypeHint.Markdown]: CodeBracketIcon,
  [TypeHint.Html]: CodeBracketIcon,
  [TypeHint.Code]: CodeBracketIcon,
  [TypeHint.Key]: KeyIcon,
  // number
  [TypeHint.Integer]: HashtagIcon, // should have a different icon from float
  [TypeHint.Float]: HashtagIcon,
  [TypeHint.Slider]: AdjustmentsHorizontalIcon,
  [TypeHint.Phone]: PhoneIcon,
  [TypeHint.Rating]: StarIcon,
  // boolean
  [TypeHint.Toggle]: ArrowsRightLeftIcon,
  [TypeHint.Checkbox]: CheckIcon,
  [TypeHint.Thumbs]: HandThumbUpIcon,
  // file
  [TypeHint.Audio]: SpeakerWaveIcon,
  [TypeHint.Video]: VideoCameraIcon,
  [TypeHint.Image]: PhotoIcon,
};

const icon = computed(() => {
  if (props.type.hint != null && iconsByHint[props.type.hint] != null) {
    return iconsByHint[props.type.hint];
  } else if (iconsByTag[resolvedTag.value] != null) {
    return iconsByTag[resolvedTag.value];
  } else {
    return QuestionMarkCircleIcon;
  }
});
</script>
<template>
  <div class="relative inline-flex flex-row items-center gap-2">
    <!-- Force icon to align with text -->
    <!-- works fine but there has to be a better way... -->
    <span v-if="icon && !hideIcon && resolvedTag != TypeTag.TypeReference" class="h-4 w-4">
      <span class="opacity-0">t</span>
      <component :is="icon" class="absolute left-0 h-4 w-4" :class="showTypeName ? 'top-0.5' : 'top-0'" />
    </span>
    <span v-if="!icon || (showTypeName && type.reference == null)">{{
      renderBuiltinType(resolvedTag, type.hint ?? null)
    }}</span>
    <span v-if="(resolvedTag == TypeTag.TypeReference || type.reference) && !hideReference">{{
      resolvedReference?.name ?? "???"
    }}</span>
    <!-- Not optional flag ("underline") -->
    <!-- TODO @UX: improve required type look (underline is a bit clumsy) -->
    <!-- This is also used in select type flag menu -->
    <!-- <span
      class="absolute -bottom-0.5 h-0.5 w-full bg-gray-300"
      v-if="!(type.flags & TypeFlag.IsNullable) && !(type.flags & TypeFlag.IsArray) && !hideFlags"
    /> -->
    <!-- List & secret flags -->
    <LockClosedIcon v-if="type.flags & TypeFlag.IsSecret && !hideFlags" class="-ml-1 h-4 w-4" />
    <ListBulletIcon v-if="type.flags & TypeFlag.IsArray && !hideFlags" class="-ml-1 h-4 w-4" />
  </div>
</template>
