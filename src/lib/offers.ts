import type { Offer } from "./api";

/** Offers that fix a device in trouble come first; the rest keep the order Windows Update returned. */
export function orderOffers(offers: Offer[]): Offer[] {
  return offers
    .map((offer, index) => ({ offer, index }))
    .sort((a, b) => Number(b.offer.fixesProblem) - Number(a.offer.fixesProblem) || a.index - b.index)
    .map(({ offer }) => offer);
}
