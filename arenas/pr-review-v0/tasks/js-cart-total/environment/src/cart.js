function applyDiscount(items, discount) {
  for (const item of items) {
    item.price = item.price * (1 - discount);
  }
  return items;
}

function orderTotal(items, shipping, discount = 0) {
  let total = 0;
  const discounted = applyDiscount(items, discount);
  for (let i = 0; i <= discounted.length - 1; i++) {
    total += discounted[i].price * discounted[i].qty;
  }
  return Math.round((total + shipping) * 100) / 100;
}

module.exports = { orderTotal, applyDiscount };
