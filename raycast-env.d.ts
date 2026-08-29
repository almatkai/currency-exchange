/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** Rate Provider - Which exchange rate API to query. Switch this if you prefer UniRateAPI's pricing or coverage. */
  "provider": "exchangerate-api" | "unirate",
  /** API Key - Key for the selected provider. ExchangeRate-API: https://app.exchangerate-api.com/dashboard — UniRateAPI: https://unirateapi.com/dashboard */
  "api_key": string
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `index` command */
  export type Index = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `index` command */
  export type Index = {}
}

