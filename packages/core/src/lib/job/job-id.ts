import { Ucs2StringType } from "kryo/lib/ucs2-string";
import { $UuidHex, UuidHex } from "kryo/lib/uuid-hex";

export type JobId = UuidHex;

export const $JobId: Ucs2StringType = $UuidHex;