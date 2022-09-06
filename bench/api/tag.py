from typing import Optional, cast

from rest_framework import serializers, viewsets

from bench.models import Tag
from bench.models.tag import TaggableMixin, TaggedItem


class TagSerializer(serializers.ModelSerializer):
    references_count = serializers.SerializerMethodField()

    class Meta:
        model = Tag
        fields = [
            "type",
            "name",
            "description",
            "created_at",
            "updated_at",
            "metadata",
            "references_count",
        ]
        read_only_fields = ["type", "references_count"]

    def get_references_count(self, obj):
        return TaggedItem.objects.filter(tag_id=obj.id).count()


class TaggedItemSerializerMixin(serializers.Serializer):
    tags = serializers.ListField(required=False)

    def to_representation(self, obj):
        result = super(TaggedItemSerializerMixin, self).to_representation(obj)
        if hasattr(obj, "prefetched_tags"):
            result["tags"] = [t.tag.name for t in obj.prefetched_tags]
        else:
            result["tags"] = obj.tagged_items.values_list("tag__name", flat=True)
        return result

    def _set_tags(self, obj, tags: Optional[list[str]]):
        if obj is None or tags is None:
            return
        cast(TaggableMixin, obj).set_tags(tags)

    def create(self, validated_data):
        validated_data.pop("tags", None)
        instance = super(TaggedItemSerializerMixin, self).create(validated_data)
        self._set_tags(instance, self.initial_data.get("tags"))
        return instance

    def update(self, instance, validated_data):
        instance = super(TaggedItemSerializerMixin, self).update(instance, validated_data)
        self._set_tags(instance, self.initial_data.get("tags"))
        return instance


class TagViewSet(viewsets.ModelViewSet):
    queryset = Tag.objects.all()
    serializer_class = TagSerializer
    lookup_field = "name"


class TaggedItemViewSetMixin(viewsets.GenericViewSet):
    def get_queryset(self):
        queryset = super(TaggedItemViewSetMixin, self).get_queryset()
        return queryset.prefetch_related(
            "tagged_items",
            queryset=TaggedItem.objects.select_related("tag"),
            to_attr="prefetched_tags",
        )
