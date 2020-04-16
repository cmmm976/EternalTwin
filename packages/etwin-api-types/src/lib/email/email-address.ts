import { Ucs2StringType } from "kryo/lib/ucs2-string.js";

/**
 * Any email address.
 *
 * It may be non-verified.
 */
export type EmailAddress = string;

/**
 * We only check that the adress is trimmed and non-empty, but leave-out verification.
 * (We only check for the `@` symbol).
 */
export const $EmailAddress: Ucs2StringType = new Ucs2StringType({
  trimmed: true,
  minLength: 1,
  maxLength: 100,
  pattern: /@/,
});