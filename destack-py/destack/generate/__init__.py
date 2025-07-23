def generate():
    """Generate all the derived things."""
    from .typescript import generate_language as generate_typescript_language

    generate_typescript_language()
