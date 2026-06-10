from decimal import Decimal, ROUND_HALF_UP


def to_cents(amount: Decimal) -> int:
    """Round a Decimal dollar amount to integer cents (banker-safe)."""
    return int((amount * 100).quantize(Decimal("1"), rounding=ROUND_HALF_UP))


def from_cents(cents: int) -> Decimal:
    return Decimal(cents) / 100
