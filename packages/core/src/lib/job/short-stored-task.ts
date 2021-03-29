import { CaseStyle } from "kryo";
import { $Date } from "kryo/lib/date";
import { $Float64 } from "kryo/lib/float64";
import { $Uint32 } from "kryo/lib/integer";
import { RecordIoType, RecordType } from "kryo/lib/record";
import { $Ucs2String } from "kryo/lib/ucs2-string";

import { $TaskId, TaskId } from "./task-id";
import { $TaskStatus, TaskStatus } from "./task-status";

export interface ShortStoredTask {
  id: TaskId,
  parent?: TaskId,
  status: TaskStatus,
  statusMessage?: string,
  createdAt: Date,
  advancedAt: Date,
  stepCount: Number,
  runningTime: Number,
}

export const $ShortStoredTask: RecordIoType<ShortStoredTask> = new RecordType<ShortStoredTask>({
  properties: {
    id: {type: $TaskId},
    parent: {type: $TaskId, optional: true},
    status: {type: $TaskStatus},
    statusMessage: {type: $Ucs2String, optional: true},
    createdAt: {type: $Date},
    advancedAt: {type: $Date},
    stepCount: {type: $Uint32},
    runningTime: {type: $Float64},
  },
  changeCase: CaseStyle.SnakeCase,
});
