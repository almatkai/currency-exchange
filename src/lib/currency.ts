import { currencyCode2CountryAndRegion, currencyCode2Name } from "../currency";

export type Provider = "exchangerate-api" | "unirate";

export const PROVIDER_LABELS: Record<Provider, string> = {
  "exchangerate-api": "exchangerate-api.com",
  unirate: "unirateapi.com",
};

export const PROVIDER_DOC_URLS: Record<Provider, string> = {
  "exchangerate-api": "https://www.exchangerate-api.com/docs/standard-requests",
  unirate: "https://unirateapi.com/docs",
};

export function cacheKey(provider: Provider, historyDate: Date | null): string {
  const dateSuffix = historyDate
    ? `${historyDate.getFullYear()}-${historyDate.getMonth() + 1}-${historyDate.getDate()}`
    : "";
  return `${provider}-currency${dateSuffix}`;
}

export function getFlagEmoji(countryAndRegionCode: string): string {
  const codePoints = countryAndRegionCode
    .toUpperCase()
    .split("")
    .map((char) => 127397 + char.charCodeAt(0));
  let v = countryAndRegionCode.startsWith("X") ? "🇺🇳" : String.fromCodePoint(...codePoints) || "🇺🇳";
  v = countryAndRegionCode === "AN" ? "🇺🇳" : v;
  return v;
}

export function formatCurrency(amount: number, currencyCode: string): string {
  try {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: currencyCode,
      minimumFractionDigits: 0,
      maximumFractionDigits: 4,
    }).format(amount);
  } catch {
    return amount.toString();
  }
}

export interface ConversionRate {
  code: string;
  value: number;
}

export interface CurrencyResult {
  base_code: string;
  result: string;
  ["error-type"]: string;
  conversion_rates: { [key: string]: number };
  conversion_rate_exchanged?: Array<ConversionRate>;
  conversion_rate_pin_exchanged?: Array<ConversionRate>;
  time_last_update_unix?: number;
  time_next_update_unix?: number;
  time_last_update_utc?: string;
  year?: number;
  month?: number;
  day?: number;
}

export function enrichExchangeData(
  currencyData: CurrencyResult,
  amount: number,
  fromCode: string,
  filter: string,
  pinned?: Array<string>,
): CurrencyResult {
  const base = currencyData.conversion_rates[fromCode] || 0;

  currencyData.conversion_rate_exchanged = Object.keys(currencyData.conversion_rates)
    .filter(
      (it) =>
        (!pinned || pinned.indexOf(it) < 0) &&
        currencyCode2CountryAndRegion[it] !== undefined &&
        currencyCode2Name[it] !== undefined &&
        it !== fromCode &&
        (it.toLocaleLowerCase().indexOf(filter) >= 0 ||
          currencyCode2CountryAndRegion[it].toLocaleLowerCase().indexOf(filter) >= 0 ||
          currencyCode2Name[it].toLocaleLowerCase().indexOf(filter) >= 0),
    )
    .map<ConversionRate>((key: string) => ({
      code: key,
      value: (amount / base) * (currencyData.conversion_rates[key] || 0),
    }));

  currencyData.conversion_rate_pin_exchanged = pinned
    ?.filter(
      (it) =>
        currencyCode2CountryAndRegion[it] !== undefined &&
        currencyCode2Name[it] !== undefined &&
        it !== fromCode &&
        (it.toLocaleLowerCase().indexOf(filter) >= 0 ||
          currencyCode2CountryAndRegion[it].toLocaleLowerCase().indexOf(filter) >= 0 ||
          currencyCode2Name[it].toLocaleLowerCase().indexOf(filter) >= 0),
    )
    .map<ConversionRate>((key: string) => ({
      code: key,
      value: (amount / base) * (currencyData.conversion_rates[key] || 0),
    }));

  return currencyData;
}

export function parseAmountExpression(amountExpression: string): {
  amount: number;
  filter: string;
  historyDate: Date | null;
} {
  let amount = 0;
  let filter = "";
  let historyDate: Date | null = null;

  const matches = /(^[\d().+\-*/^]+)(.*)/.exec(amountExpression);
  if (matches) {
    try {
      // eslint-disable-next-line no-eval
      amount = eval(matches[1]);
    } catch {
      // ignore
    }

    if (matches[2]) {
      const historyMatches = /\s+at\s+(\d{4}\/\d{1,2}\/\d{1,2})/.exec(matches[2]);
      const historyMatchSize = historyMatches?.length;
      historyDate =
        historyMatchSize && historyMatchSize > 1 && historyMatches[1] ? new Date(historyMatches[1]) : null;
      historyDate = historyDate && !isNaN(historyDate?.getTime()) ? historyDate : null;

      const filterMatches = /\s+in\s+([a-zA-Z\s]*)/.exec(matches[2]);
      const filterMatchSize = filterMatches?.length;
      filter = filterMatchSize && filterMatchSize > 0 && filterMatches[1] ? filterMatches[1] : "";
      filter = filter.replace(/\s+at\s+/, " ").trim();
    }
  }

  return { amount, filter: filter.toLocaleLowerCase(), historyDate };
}
