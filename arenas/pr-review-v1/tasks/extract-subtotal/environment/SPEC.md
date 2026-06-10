# Billing rules

1. All money is integer cents internally.
2. Discounts apply to the pre-tax subtotal. Tax is computed on the
   already-discounted subtotal.
3. Tax rates (billing/core/tax.py) are fractions of the taxable amount.
4. Invoices are immutable once paid; payment is idempotent.
