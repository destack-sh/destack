from typing import cast

from rest_framework import authentication, viewsets

from bench.models import User


class ProtectedViewSetMixin(viewsets.GenericViewSet):
    """
    Restricts access to this view set to authorized users
    """

    authentication_classes = [authentication.SessionAuthentication]

    def get_queryset(self):
        queryset = super().get_queryset()
        return queryset.filter(organization__in=cast(User, self.request.user).organizations)
