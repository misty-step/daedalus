const { formatCurrency } = require("./format");

function receiptLine(item) {
  return `${item.name}: ${formatCurrency(item.cents)}`;
}

module.exports = { receiptLine };
