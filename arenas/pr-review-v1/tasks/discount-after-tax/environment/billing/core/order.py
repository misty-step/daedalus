from decimal import Decimal

from .discount import apply_discount
from .tax import tax_for


def order_total(line_items, region, discount=Decimal("0")):
    """Compute the final order total in cents.

    line_items: list of (unit_price_cents, quantity).
    """
    subtotal = sum(price * qty for price, qty in line_items)
    tax = tax_for(region, subtotal)
    return apply_discount(subtotal + tax, discount)
