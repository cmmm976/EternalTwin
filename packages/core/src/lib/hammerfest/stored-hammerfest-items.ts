import { CaseStyle } from "kryo";
import { $Date } from "kryo/lib/date.js";
import { $Null } from "kryo/lib/null.js";
import { RecordIoType, RecordType } from "kryo/lib/record.js";
import { TryUnionType } from "kryo/lib/try-union.js";

import { $HammerfestItemCounts, HammerfestItemCounts } from "./hammerfest-item-counts.js";

export interface StoredHammerfestItems {
  firstArchivedAt: Date;
  lastArchivedAt: Date;
  items: HammerfestItemCounts;
}

export const $StoredHammerfestItems: RecordIoType<StoredHammerfestItems> = new RecordType<StoredHammerfestItems>({
  properties: {
    firstArchivedAt: {type: $Date},
    lastArchivedAt: {type: $Date},
    items: {type: $HammerfestItemCounts},
  },
  changeCase: CaseStyle.SnakeCase,
});

export type NullableStoredHammerfestItems = null | StoredHammerfestItems;

export const $NullableStoredHammerfestItems: TryUnionType<NullableStoredHammerfestItems> = new TryUnionType({variants: [$Null, $StoredHammerfestItems]});