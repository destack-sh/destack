from rest_framework import mixins, permissions, serializers, viewsets
from rest_framework.exceptions import PermissionDenied

from bench.models.user import User


class UserSerializer(serializers.ModelSerializer):
    class Meta:
        model = User


class UserViewSet(
    mixins.RetrieveModelMixin,
    mixins.ListModelMixin,
    mixins.UpdateModelMixin,
    viewsets.GenericViewSet,
):
    queryset = User.objects.all()
    serializer_class = UserSerializer
    permission_classes = [permissions.IsAuthenticated]
    lookup_field = "id"

    def get_object(self):
        lookup_value = self.kwargs[self.lookup_field]
        if lookup_value == "@me":
            return self.request.user
        if not self.request.user.is_staff:
            raise PermissionDenied(f"cannot get user except self: {self.request.user}")
        return super().get_object()

    def get_queryset(self):
        queryset = super().get_queryset()
        if not self.request.user.is_staff:
            queryset = queryset.filter(id=self.request.user.id)
        return queryset
