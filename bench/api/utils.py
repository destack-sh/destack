from typing import Any, ClassVar

from django.core.exceptions import ObjectDoesNotExist
from django.utils.translation import gettext_lazy as _
from rest_framework import serializers

from bench.models.flow import FlowVersion


class NameVersionListingField(serializers.RelatedField):
    default_error_messages = {
        "required": _("This field is required."),
        "does_not_exist": _('Invalid listing "{data}" - object does not exist.'),
        "incorrect_type": _("Incorrect type. Expected listing string, received {data_type}."),
    }
    parent_name_lookup: ClassVar[str]

    def to_representation(self, value: Any):
        return f"{getattr(value, self.parent_name_lookup).name}@{value.version}"

    def to_internal_value(self, data) -> FlowVersion:
        queryset = self.get_queryset()
        try:
            if not isinstance(data, str):
                raise TypeError
            parts = data.split("@")
            name = parts[0]
            version = parts[1]
            name_lookup = self.parent_name_lookup + "__name"
            return queryset.filter(**{name_lookup: name}, version=version).get()
        except ObjectDoesNotExist:
            self.fail("does_not_exist", data=data)
        except (TypeError, ValueError):
            self.fail("incorrect_type", data_type=type(data).__name__)


class FlowVersionListingField(NameVersionListingField):
    parent_name_lookup = "flow"


class ArtifactVersionListingField(NameVersionListingField):
    parent_name_lookup = "artifact"
