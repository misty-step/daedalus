# billing

Small billing service. Key invariant (see billing/core/order.py and
billing/core/discount.py): tax is charged on the post-discount subtotal,
never the pre-discount amount. Money is integer cents everywhere; convert
only at the Decimal boundary in billing/core/money.py.
