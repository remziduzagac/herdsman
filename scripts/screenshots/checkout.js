import { charge } from "../payments/client.js";
import { total } from "../cart/total.js";

const RETRYABLE = new Set(["timeout", "5xx"]);
const MAX_ATTEMPTS = 3;

const sleep = (ms) =>
  new Promise((done) => setTimeout(done, ms));

export async function checkout(cart, card) {
  const amount = total(cart.items);
  const key = crypto.randomUUID();

  for (let attempt = 1; ; attempt++) {
    const res = await charge({ card, amount, key });
    if (res.ok) {
      return { status: "paid", id: res.id };
    }
    if (res.declined) {
      return { status: "declined" };
    }
    const retry = RETRYABLE.has(res.error);
    if (!retry || attempt === MAX_ATTEMPTS) {
      throw new Error(`payment: ${res.error}`);
    }
    await sleep(200 * 2 ** attempt);
  }
}
