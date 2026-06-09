function formatCurrency(cents, currency = "USD") {
  const amount = (cents / 100).toFixed(2);
  return `${currency} ${amount}`;
}

module.exports = { formatCurrency };
