import { Ucs2StringType } from "kryo/lib/ucs2-string";
import { $UuidHex, UuidHex } from "kryo/lib/uuid-hex";

export type TaskId = UuidHex;

export const $TaskId: Ucs2StringType = $UuidHex;