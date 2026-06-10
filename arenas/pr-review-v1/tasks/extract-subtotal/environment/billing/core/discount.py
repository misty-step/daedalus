from decimal import Decimal


def apply_discount(subtotal_cents: int, fraction: Decimal) -> int:
    """Return the discounted subtotal in cents. Discounts apply to the
    pre-tax subtotal only; tax is computed on the discounted amount."""
    if not 0 <= fraction <= 1:
        raise ValueError("discount fraction must be in [0, 1]")
    return int(subtotal_cents * (1 - fraction))
