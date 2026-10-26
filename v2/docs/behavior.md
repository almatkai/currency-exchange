# Source behavior and bounded differences

The authority is the complete fresh own source at
`8759bec78fa594cc3c56e9c3746bfd31e78d0c69`, not the older one-provider Raycast
implementation. Both providers always fetch USD data. Conversion is the ordered
IEEE f64 expression `(amount / selectedBaseRate) * targetRate`, with JS `|| 0`
for missing/zero rates. No arithmetic reassociation or decimal financial model
is substituted. Results and copy bytes are tested against actual source execution.

Each open session keeps its selected base currency; another session
saving a different default does not change that existing dropdown. New sessions
load the latest persisted base. Pin mutations merge freshly under the storage lock.

The initial arithmetic prefix is exactly `^[\d().+\-*/^]+`; whitespace stops it.
The recursive grammar accepts decimal tokens with digits on both sides of a dot,
parentheses and unary signs, with unary before right-associative power. The final
value must be finite. Invalid, zero and negative amounts do not exchange. The
native resource bounds are 4 KiB per query, 1 KiB arithmetic and depth 128;
over-limit expressions evaluate to the same safe zero fallback. No eval exists.

`in` and `at YYYY/M/D` use the source regexes, with ECMAScript whitespace and ASCII
digits, rather than natural-language parsing. JavaScript's non-dotAll rest capture,
legacy padded years 0000..0099, and day rollover through day 31 are preserved.
In the pinned V8 parser, leading 0001..0012 selects month/day/year (so
0001/2/3 is January 2, 2003), 0013..0031 is invalid, and 0000 or 0032..0099
selects the legacy year. The executed source oracle covers each of these years
with valid, invalid and reordered fields, plus century boundaries.
Dates are civil calendar values, independent of timezone conversion. The tested
source timezone is Asia/Almaty. DatePicker submits an ISO civil date, removes only
the first existing source `at` match, appends the new date, and returns to the List.
An empty date leaves the query unchanged; Cancel keeps its exact original text.
The source's `Currecy History Date` spelling is retained.

All 161 code/name/country rows, flags (including X*/AN => UN) and their order come
from the pinned source. Server rate-object insertion order determines ordinary
results; pins keep their independent stored order, including duplicate valid pins
already present. Pin appends only when absent; Unpin removes every occurrence.
Unknown/overlong stored pin codes are excluded, with a 512-entry bound. Filters
match code, country or currency name and exclude the selected base. Missing pinned
rates are still calculated using the source fallback. No invented currencies,
history lists or preferences are added.

The en-US currency formatter uses the pinned source oracle's ICU symbols, decimal
half-expand rounding of JS shortest digits to four fraction places, grouping and
signed zero. Copy uses `ryu-js` Number.toString-compatible output, including
scientific notation and zero canonicalization. All currencies are tested with
finite extremes, subnormals, rounding ties, NaN and both infinities. Intermediate
non-finite conversions are kept as classified internal f64 values; only text goes
into native JSON. Positive Infinity displays `No Currency` for pinned rows and
`∞` otherwise; NaN/negative Infinity use currency Intl formatting. Their raw copy
strings remain `Infinity`, `NaN` or `-Infinity`, as in source. They are never
silently replaced by a finite financial result.

Provider contracts:

* ExchangeRate-API: GET fixed `https://v6.exchangerate-api.com/v6/<key>/latest/USD`
  or `.../history/USD/Y/M/D`. The key is an encoded path segment. Success/result,
  numeric rates, USD base, current update metadata and requested history date are
  validated. Its known errors map to static, credential-free messages.
* UniRate: GET fixed `https://api.unirateapi.com/api/rates?from=USD&api_key=...`
  or `.../historical/rates?from=USD&date=YYYY-MM-DD&api_key=...`. Numeric strings,
  including decimal exponent/empty string and representable unsigned radix
  integers, normalize like Number. NaN strings are skipped. Missing base defaults
  to USD. Current metadata uses rounded current seconds and next = current+86400;
  historical metadata uses the requested civil date. HTTP 401 maps to invalid key.

Exact validation exclusions, deliberately stronger than permissive V1:

* Unknown/non-USD provider base, missing/invalid ExchangeRate current metadata (UTC date must be strict RFC2822
  UTC and agree with its timestamp), mismatched
  history date, nonnumeric ExchangeRate rates, empty or >512-entry tables, or code
  keys over 32 bytes are rejected before any conversion/cache commit.
* UniRate string Infinity, decimal overflow and radix integers exceeding u128
  are rejected. V1's intermediate `JSON.stringify` can turn Infinity into null,
  later producing zero; this port displays a static rate error instead. No finite
  result is accepted in place of that overflow. Radix integers through u128 round
  once on conversion to f64. Invalid numeric spellings such as `inf` are skipped,
  matching Number's NaN behavior.
* Credentials are required, <=4 KiB, without controls; `.` and `..` cannot be a
  path key. Invalid provider preferences need correction and command reopen.
* Redirects and non-2xx HTTP are failures. Redirects/proxies are disabled; TLS is
  verified. Connect timeout is 5s, complete request/body timeout 15s, outer deadline
  16s, body <=1 MiB. No automatic retry or provider URL override is compiled into
  the product. Raw provider errors/bodies/headers/URLs are never logged or shown.
* The rate date subtitle displays the preserved provider UTC metadata verbatim,
  rather than V1's OS-locale-dependent `toLocaleString()`. Historical date text
  matches source. Numeric subtitles use the tested en-US formatter.
* Date submit awaits its one fetch while the generic patched Form disables
  editable fields; this prevents a response from overwriting a newer field draft.
  The source pops to its List immediately. Query/date contents are retained.
* Cache data is scoped additionally by key identity and profile dataDir; V1 only
  scopes by provider/date. Root corruption is preserved with an editable recovery
  view; malformed owned cache fields can be replaced after a successful fetch.
  Unknown root keys survive updates; the bounded owned cache namespace is evicted
  oldest-insertion first. Symlink/nonregular descendants below the host-owned
  dataDir are rejected. Store files are private, <=1 MiB, with fresh reads under a
  process lane and nonblocking interprocess lock. Busy writes require manual retry.

Provider calls in QA use fake keys and synthetic rates. No live provider data,
paid API call, user key, real clipboard, foreground window or profile is exercised.
The standard/historical URL shapes were checked against primary ExchangeRate-API
[standard docs](https://www.exchangerate-api.com/docs/standard-requests) and
[history docs](https://www.exchangerate-api.com/docs/historical-data-requests).
The source's UniRate docs URL returned 404 during this run; UniRate behavior is
anchored to the pinned own TSX and synthetic transport contract, not an asserted
live service acceptance.
