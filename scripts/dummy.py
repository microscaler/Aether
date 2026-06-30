"""Dummy script to satisfy mypy type checking in CI."""


def greet(name: str) -> str:
    """Return a greeting."""
    return f"Hello, {name}!"


def main() -> None:
    """Run the dummy script."""
    print(greet("Aether"))


if __name__ == "__main__":
    main()
