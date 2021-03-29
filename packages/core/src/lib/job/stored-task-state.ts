import { CaseStyle } from "kryo";
import { $Any } from "kryo/lib/any";
import { $Uint32 } from "kryo/lib/integer";
import { RecordIoType, RecordType } from "kryo/lib/record";
import { $Ucs2String } from "kryo/lib/ucs2-string";

export interface StoredTaskState {
  kind: string,
  dataVersion: Number,
  options: any,
  state: any,
}

export const $StoredTaskState: RecordIoType<StoredTaskState> = new RecordType<StoredTaskState>({
  properties: {
    kind: {type: $Ucs2String},
    dataVersion: {type: $Uint32},
    options: {type: $Any},
    state: {type: $Any},
  },
  changeCase: CaseStyle.SnakeCase,
});
