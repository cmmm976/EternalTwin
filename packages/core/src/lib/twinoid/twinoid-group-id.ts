import { Ucs2StringType } from "kryo/lib/ucs2-string.js";

/**
 * A Twinoid group id.
 */
export type TwinoidGroupId = string;

export const $TwinoidGroupId: Ucs2StringType = new Ucs2StringType({
  minLength: 1,
  maxLength: 12,
  trimmed: true,
  pattern: /^[1-9]\d{0,11}$/,
});