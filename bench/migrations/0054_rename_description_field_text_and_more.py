import django.core.validators
from django.db import migrations, models


class Migration(migrations.Migration):
    dependencies = [
        ("bench", "0053_alter_issue_kind"),
    ]

    operations = [
        migrations.RenameField(
            model_name="field",
            old_name="description",
            new_name="text",
        ),
        migrations.RenameField(
            model_name="statement",
            old_name="root_type_flags",
            new_name="flags",
        ),
        migrations.RenameField(
            model_name="statement",
            old_name="root_type_tag",
            new_name="tag",  # Renaming root_type_tag to tag
        ),
        # move content of description to text if text is empty
        migrations.RunSQL(
            """
            UPDATE bench_statement SET text = description WHERE text IS NULL;
            """
        ),
        migrations.RemoveField(
            model_name="statement",
            name="description",
        ),
        migrations.RemoveField(
            model_name="statement",
            name="lang",
        ),
        migrations.AddField(
            model_name="statement",
            name="heading_level",
            field=models.IntegerField(blank=True, null=True),
        ),
        migrations.AlterField(
            model_name="field",
            name="name",
            field=models.CharField(
                blank=True,
                max_length=256,
                null=True,
                validators=[django.core.validators.RegexValidator(r"^[a-zA-Z0-9_.\-:/ \xa0]*$")],
            ),
        ),
        migrations.AlterField(
            model_name="statement",
            name="key",
            field=models.CharField(blank=True, max_length=32, null=True),
        ),
        migrations.AlterField(
            model_name="statement",
            name="name",
            field=models.CharField(
                blank=True,
                max_length=256,
                null=True,
                validators=[django.core.validators.RegexValidator(r"^[a-zA-Z0-9_.\-:/ \xa0]*$")],
            ),
        ),
        migrations.AlterField(
            model_name="statement",
            name="type",
            field=models.CharField(
                choices=[
                    ("tag", "TAG"),
                    ("text", "TEXT"),
                    ("blank", "BLANK"),
                    ("type", "TYPE"),
                    ("task", "TASK"),
                    ("code", "CODE"),
                    ("flow", "FLOW"),
                    ("model", "MODEL"),
                    ("variable", "VARIABLE"),
                    ("dataset", "DATASET"),
                    ("reference", "REFERENCE"),
                ],
                max_length=32,
            ),
        ),
        migrations.RunSQL("UPDATE bench_statement SET type='variable' WHERE type='value';"),
        migrations.RunSQL("UPDATE bench_statement SET type='text' WHERE type='expectation';"),
    ]
