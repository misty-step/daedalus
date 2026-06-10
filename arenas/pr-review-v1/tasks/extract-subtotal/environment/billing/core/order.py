from decimal import Decimal

from .discount import apply_discount
from .tax import tax_for


def _subtotal(line_items):
    return sum(price * qty for price, qty in line_items)


def order_total(line_items, region, discount=Decimal("0")):
    """Compute the final order total in cents.

    line_items: list of (unit_price_cents, quantity).
    Tax is charged on the post-discount subtotal (see discount.py).
    """
    discounted = apply_discount(_subtotal(line_items), discount)
    return discounted + tax_for(region, discounted)
