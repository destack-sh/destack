def generate():
    """Generate all the derived things."""
    from .python import generate as generate_python
    from .typescript import generate as generate_typescript

    generate_python()
    generate_typescript()
