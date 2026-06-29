"""Dummy script to satisfy mypy type checking in CI."""


def greet(name: str) -> str:
    """Returns a greeting."""
    return f"Hello, {name}!"


if __name__ == "__main__":
    print(greet("Aether"))
