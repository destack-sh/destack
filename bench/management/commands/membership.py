from django.core.management.base import BaseCommand, CommandError
from django.db import transaction

from bench.models.organization import Organization, OrganizationMembership, OrganizationRole
from bench.models.user import User


class Command(BaseCommand):
    help = "Add, remove or invite a user to an organization with a specific role."

    def add_arguments(self, parser):
        parser.add_argument("organization_slug", type=str, help="Slug of the organization")
        parser.add_argument(
            "action",
            type=str,
            choices=["add", "remove", "invite"],
            help="Whether to add, remove or invite the user from/to the organization",
        )
        parser.add_argument("user_identifier", type=str, help="Username or Email of the user")
        parser.add_argument(
            "--role",
            type=str,
            choices=[level.name for level in OrganizationRole],
            default=OrganizationRole.Guest,
            help="Role to be assigned when adding a user to the organization",
        )

    @transaction.atomic
    def handle(self, *args, **options):
        user_identifier = options["user_identifier"]
        organization_slug = options["organization_slug"]
        action = options["action"]
        role = options.get("role", OrganizationRole.Guest.name)
        role = OrganizationRole[role]

        organization = Organization.objects.get(owner_slug_id=organization_slug)
        # Determine if the user_identifier is a username or email
        if "@" in user_identifier:
            user = User.objects.filter(email=user_identifier).first()
        else:
            user = User.objects.get(username=user_identifier)
        if action == "add":
            # upsert to role
            if OrganizationMembership.objects.filter(user=user, organization=organization).exists():
                OrganizationMembership.objects.filter(user=user, organization=organization).update(
                    level=role
                )
                self.stdout.write(
                    self.style.SUCCESS(
                        f"Updated {user} in {organization} with role {OrganizationRole(role).label}"
                    )
                )
            else:
                OrganizationMembership.objects.create(
                    user=user, organization=organization, level=role
                )
                self.stdout.write(
                    self.style.SUCCESS(
                        f"Added {user } to {organization} with role {OrganizationRole(role).label}"
                    )
                )
        elif action == "remove":
            OrganizationMembership.objects.filter(user=user, organization=organization).delete()
            self.stdout.write(self.style.SUCCESS(f"Removed {user} from {organization}"))
        elif action == "invite":
            if "@" not in user_identifier:
                raise CommandError("You can only invite users by their email address.")
            organization.create_invite(user_identifier, role)
            self.stdout.write(
                self.style.SUCCESS(
                    f"Invitation sent to {user_identifier} to join {organization} as {role}"
                )
            )
