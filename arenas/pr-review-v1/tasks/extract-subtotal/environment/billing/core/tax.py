from decimal import Decimal


# Tax rates are expressed as fractions of the taxable total.
RATES = {"US-CA": Decimal("0.0725"), "US-NY": Decimal("0.04"), "EU": Decimal("0.20")}


def tax_for(region: str, taxable_cents: int) -> int:
    from .money import to_cents, from_cents

    rate = RATES.get(region, Decimal("0"))
    return to_cents(from_cents(taxable_cents) * rate)
