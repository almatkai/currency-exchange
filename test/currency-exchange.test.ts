import assert from "node:assert/strict";
import test, { describe, it } from "node:test";
import {
  cacheKey,
  enrichExchangeData,
  formatCurrency,
  getFlagEmoji,
  parseAmountExpression,
  PROVIDER_LABELS,
} from "../src/lib/currency";

describe("cacheKey", () => {
  it("returns provider-currency without date", () => {
    assert.equal(cacheKey("exchangerate-api", null), "exchangerate-api-currency");
    assert.equal(cacheKey("unirate", null), "unirate-currency");
  });
  it("includes date suffix when historyDate provided", () => {
    const d = new Date(2022, 8, 25); // 2022/9/25
    assert.equal(cacheKey("exchangerate-api", d), "exchangerate-api-currency2022-9-25");
    assert.equal(cacheKey("unirate", d), "unirate-currency2022-9-25");
  });
});

describe("getFlagEmoji", () => {
  it("returns flag for valid country code and fallback for X and AN", () => {
    assert.equal(getFlagEmoji("US"), "🇺🇸");
    assert.equal(getFlagEmoji("GB"), "🇬🇧");
    assert.equal(getFlagEmoji("JP"), "🇯🇵");
    assert.equal(getFlagEmoji("XK"), "🇺🇳");
    assert.equal(getFlagEmoji("AN"), "🇺🇳");
    assert.equal(getFlagEmoji("XA"), "🇺🇳");
  });
});

describe("formatCurrency", () => {
  it("formats USD and handles invalid code fallback", () => {
    const usd = formatCurrency(1234.5, "USD");
    assert.match(usd, /\$/);
    assert.match(usd, /1,234\.5/);
    assert.equal(formatCurrency(10, "USD"), "$10");
    // invalid code should fallback to string
    assert.equal(formatCurrency(10, "INVALID"), "10");
  });
});

describe("parseAmountExpression", () => {
  it("parses plain amount", () => {
    const r = parseAmountExpression("10");
    assert.equal(r.amount, 10);
    assert.equal(r.filter, "");
    assert.equal(r.historyDate, null);
  });
  it("evaluates expression and extracts filter and date", () => {
    const r = parseAmountExpression("1+2 in EUR");
    assert.equal(r.amount, 3);
    assert.equal(r.filter, "eur");
    assert.equal(r.historyDate, null);
  });
  it("parses historical date", () => {
    const r = parseAmountExpression("5.5 in USD at 2022/9/25");
    assert.equal(r.amount, 5.5);
    assert.equal(r.filter, "usd");
    assert.ok(r.historyDate instanceof Date);
    assert.equal(r.historyDate!.getFullYear(), 2022);
    assert.equal(r.historyDate!.getMonth(), 8);
    assert.equal(r.historyDate!.getDate(), 25);
  });
  it("parses in filter with at clause interleaved", () => {
    const r = parseAmountExpression("10 in GBP at 2023/01/15");
    assert.equal(r.amount, 10);
    assert.equal(r.filter, "gbp");
    assert.ok(r.historyDate);
  });
  it("handles invalid expression gracefully", () => {
    const r = parseAmountExpression("not-a-number");
    assert.equal(r.amount, 0);
  });
});

describe("enrichExchangeData", () => {
  const baseData = {
    base_code: "USD",
    result: "success",
    "error-type": "",
    conversion_rates: { USD: 1, EUR: 0.9, GBP: 0.8, JPY: 110, AED: 3.67 },
  };

  it("computes conversion for amount and filters", () => {
    const data = JSON.parse(JSON.stringify(baseData));
    const out = enrichExchangeData(data, 100, "USD", "", undefined);
    const eur = out.conversion_rate_exchanged!.find((c) => c.code === "EUR")!;
    assert.equal(eur.value, 90);
    const jpy = out.conversion_rate_exchanged!.find((c) => c.code === "JPY")!;
    assert.equal(jpy.value, 11000);
  });

  it("filters by substring and excludes pinned and base", () => {
    const data = JSON.parse(JSON.stringify(baseData));
    const out = enrichExchangeData(data, 10, "USD", "", ["GBP"]);
    assert.ok(out.conversion_rate_exchanged!.some((c) => c.code === "EUR"));
    assert.ok(!out.conversion_rate_exchanged!.some((c) => c.code === "GBP"));
    assert.ok(!out.conversion_rate_exchanged!.some((c) => c.code === "USD"));
    assert.ok(out.conversion_rate_exchanged!.some((c) => c.code === "JPY"));
    assert.equal(out.conversion_rate_pin_exchanged!.length, 1);
    assert.equal(out.conversion_rate_pin_exchanged![0].code, "GBP");
  });

  it("filters by filter string", () => {
    const data = JSON.parse(JSON.stringify(baseData));
    const out = enrichExchangeData(data, 10, "USD", "eur", undefined);
    assert.ok(out.conversion_rate_exchanged!.some((c) => c.code === "EUR"));
    assert.ok(!out.conversion_rate_exchanged!.some((c) => c.code === "JPY"));
    assert.ok(!out.conversion_rate_exchanged!.some((c) => c.code === "GBP"));
  });

  it("pin filtering respects filter", () => {
    const data = JSON.parse(JSON.stringify(baseData));
    const out = enrichExchangeData(data, 1, "USD", "jpy", ["EUR", "JPY"]);
    // pinned EUR should be filtered out because filter is jpy
    assert.equal(out.conversion_rate_pin_exchanged!.length, 1);
    assert.equal(out.conversion_rate_pin_exchanged![0].code, "JPY");
  });
});

describe("provider labels", () => {
  it("has labels for both providers", () => {
    assert.equal(PROVIDER_LABELS["exchangerate-api"], "exchangerate-api.com");
    assert.equal(PROVIDER_LABELS["unirate"], "unirateapi.com");
  });
});

test("package.json declares Windows+macOS and Tezbar metadata", async () => {
  const pkg = await import("../package.json", { with: { type: "json" } });
  const data = (pkg as unknown as { default: Record<string, unknown> }).default ??
    (pkg as unknown as Record<string, unknown>);
  assert.deepEqual((data as { platforms: string[] }).platforms, ["macOS", "Windows"]);
  const tezbar = (data as { tezbar: { platforms: string[]; repository: string; upstream: { commit: string; path: string } } }).tezbar;
  assert.deepEqual(tezbar.platforms, ["macos", "windows"]);
  assert.equal(tezbar.repository, "https://github.com/almatkai/currency-exchange");
  assert.equal(tezbar.upstream.path, "extensions/currency-exchange");
  assert.equal(tezbar.upstream.commit, "6734d85992290066fd31a7f5bee3cca3d3545a1a");
  const commands = (data as { commands: { platforms: string[] }[] }).commands;
  for (const cmd of commands) {
    assert.deepEqual(cmd.platforms, ["macOS", "Windows"]);
  }
  const deps = (data as { dependencies: Record<string, string> }).dependencies;
  assert.ok(deps["@raycast/api"]);
  assert.ok(deps["node-fetch"]);
});
