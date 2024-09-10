import pytest

from bench.language.field import to_type
from bench.language.session import Session
from bench.language.validation import NAME_CONSTRAINT, on_invalid_raise
from bench.language.value import check_value


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
        "さくら田中",  # Sakura Tanaka (Japanese)],
    ],
)
def test_support_international_names(shared_session: Session, name: str):
    name_type = to_type(str, constraint=NAME_CONSTRAINT)
    check_value(name, name_type, on_invalid_raise)
