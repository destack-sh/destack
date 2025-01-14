import pytest

from bench.language import (
    DEFAULT_CHECK_OPTIONS,
    NAME_CONSTRAINT,
    Session,
    check_value,
    on_invalid_raise,
    to_type,
)


@pytest.mark.parametrize(
    "name",
    [
        # source names
        "Start1",
        "Log1727",
        "123Run",
        "COMPLETED_RUN",
        # other names
        "a",
        "a-b",
        # person names
        "John Smith",
        "María García-López",
        "Li Wei",
        "Søren Østergård",
        "Fatima al-Hussein",
        "Jean-Claude O'Brien",
        "Aisha Patel-Williams",
        "Björn Güneş",
        "Zoe Ní Mhaonaigh",
        "Kwan-Ho Park",
        "Amélie Dubois-Chevalier",
        "Oluwaseun Adebayo",
        "Siobhán O'Sullivan",
        "Yuki Tanaka-Anderson",
        "François-Xavier Nguyễn",
        "اميرة الزهراء",  # Amira Al-Zahra (Arabic)
        "王芳",  # Wang Fang (Chinese)
        "Александр Иванов",  # Alexander Ivanov (Russian)
        "さくら田中",  # Sakura Tanaka (Japanese),
    ],
)
def test_validate_international_names(session: Session, name: str):
    name_type = to_type(str, constraint=NAME_CONSTRAINT)
    check_value(name, name_type, options=DEFAULT_CHECK_OPTIONS, invalid=on_invalid_raise)
