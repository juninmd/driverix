import type { Offer } from "./api";

export type OfferSort = "source" | "size" | "date";

export const OFFER_SORTS: Record<OfferSort, string> = {
  source: "Ordem do Windows Update",
  size: "Maior primeiro (tamanho)",
  date: "Mais recente primeiro",
};

/** Returns a new array; the input keeps the Windows Update order. Ties keep that order too. */
export function sortOffers(offers: Offer[], sort: OfferSort): Offer[] {
  const copy = [...offers];
  switch (sort) {
    case "size":
      return copy.sort((a, b) => b.candidate.sizeBytes - a.candidate.sizeBytes);
    case "date":
      // Unknown dates sort last; ISO strings compare correctly as text.
      return copy.sort((a, b) => (b.candidate.date ?? "").localeCompare(a.candidate.date ?? ""));
    default:
      return copy;
  }
}
